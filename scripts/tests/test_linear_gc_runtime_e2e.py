"""Exercise the linear fallback collector through its emitted private ABI."""

from __future__ import annotations

import os
import re
import shutil
import subprocess
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / "tests/fixtures/target_capability/wasm32_same_source.ark"
COLLECT_FIXTURE = ROOT / "tests/fixtures/target_capability/linear_gc_entry_collect.ark"
FUNCTION_DEFINITION = re.compile(r"^\s+\(func .*?\(;([0-9]+);\)", re.MULTILINE)
MAIN_DEFINITION = re.compile(r"^\s+\(func \$main \(;([0-9]+);\)", re.MULTILINE)
GLOBAL_DEFINITION = re.compile(r"^\s+\(global \(;", re.MULTILINE)

# Linear modules emit exactly the heap pointer plus the seven fallback
# globals declared by wasm::constants; there is no GC literal-cache global
# on this target, so the ordinals below are stable.
FREE_HEAD_GLOBAL = 2
ALLOC_WATERMARK_GLOBAL = 7
COLLECT_THRESHOLD_BYTES = 67108864


class LinearGcRuntimeE2eTest(unittest.TestCase):
    def setUp(self) -> None:
        compiler_candidates = (
            ROOT / ".build/selfhost/arukellt-s2-runtime.wasm",
            ROOT / ".build/selfhost/arukellt-s2.wasm",
        )
        compiler = next((path for path in compiler_candidates if path.is_file()), None)
        if compiler is None:
            self.skipTest("current-source selfhost wasm is not built")
        if shutil.which("wasm-tools") is None or shutil.which("wasmtime") is None:
            self.skipTest("wasm-tools and wasmtime are required")
        self.compiler = compiler

    def compile_linear_fixture(self, fixture: Path = FIXTURE, output_name: str = "linear_gc_runtime_e2e.wasm") -> Path:
        output = ROOT / ".build/tests" / output_name
        output.parent.mkdir(parents=True, exist_ok=True)
        result = subprocess.run(
            [
                str(ROOT / "scripts/run/arukellt-selfhost.sh"),
                "compile",
                str(fixture.relative_to(ROOT)),
                "--target",
                "wasm32",
                "--wasi-version",
                "wasi-p1",
                "--memory-width",
                "memory32",
                "--emit",
                "wasm",
                "-o",
                str(output.relative_to(ROOT)),
            ],
            cwd=ROOT,
            env={**os.environ, "ARUKELLT_SELFHOST_WASM": str(self.compiler)},
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertTrue(output.is_file())
        return output

    def runtime_function_indices(self, wat: str) -> tuple[int, ...]:
        indices = [int(match.group(1)) for match in FUNCTION_DEFINITION.finditer(wat)]
        self.assertGreaterEqual(len(indices), 8, "linear artifact is missing runtime functions")
        runtime_indices = tuple(indices[-8:])
        self.assertEqual(
            runtime_indices,
            tuple(range(runtime_indices[0], runtime_indices[0] + 8)),
            "runtime functions are not the final contiguous function group",
        )
        return runtime_indices

    def add_runtime_probe(self, wat: str) -> str:
        (
            root_frame_enter,
            root_push,
            root_update,
            root_frame_leave,
            _commit_top,
            _mark_candidate,
            collect,
            alloc,
        ) = self.runtime_function_indices(wat)
        probe = f"""
  (func (export "linear_gc_runtime_probe") (result i32)
    (local $frame i32)
    (local $root i32)
    (local $first i32)
    (local $second i32)
    (local $third i32)
    (local $fourth i32)
    (local $fifth i32)
    (local $cycle_live i32)
    (local $pair_reused i32)
    call {root_frame_enter}
    local.set $frame
    i32.const 16
    call {alloc}
    local.set $first
    i32.const 16
    call {alloc}
    local.set $second
    local.get $first
    local.get $second
    i32.store
    local.get $second
    local.get $first
    i32.store
    local.get $first
    call {root_push}
    local.set $root
    local.get $root
    local.get $first
    call {root_update}
    call {collect}
    i32.const 16
    call {alloc}
    local.set $third
    local.get $third
    local.get $first
    i32.ne
    local.get $third
    local.get $second
    i32.ne
    i32.and
    local.set $cycle_live
    local.get $root
    i32.const 0
    call {root_update}
    call {collect}
    i32.const 16
    call {alloc}
    local.set $fourth
    i32.const 16
    call {alloc}
    local.set $fifth
    local.get $fourth
    local.get $first
    i32.eq
    local.get $fifth
    local.get $second
    i32.eq
    i32.and
    local.get $fourth
    local.get $second
    i32.eq
    local.get $fifth
    local.get $first
    i32.eq
    i32.and
    i32.or
    local.set $pair_reused
    local.get $frame
    call {root_frame_leave}
    local.get $cycle_live
    local.get $pair_reused
    i32.and
  )
"""
        module_end = wat.rstrip()
        self.assertTrue(module_end.endswith(")"), "printed artifact is not a WAT module")
        return module_end[:-1] + probe + ")\n"

    def add_entry_collect_probe(self, wat: str) -> str:
        (_, _, _, _, _, _, _, alloc) = self.runtime_function_indices(wat)
        main_match = MAIN_DEFINITION.search(wat)
        self.assertIsNotNone(main_match, "compiled artifact is missing $main")
        main_index = int(main_match.group(1))
        # 256 blocks x 64 bytes of unrooted garbage, then a watermark forced
        # past the threshold. Entering the compiled function must run the
        # collector: the watermark is reset and the garbage records move to
        # the free list.
        probe = f"""
  (func (export "linear_gc_entry_collect_probe") (result i32)
    (local $i i32)
    block ;; label = @1
      loop ;; label = @2
        local.get $i
        i32.const 256
        i32.ge_u
        br_if 1 (;@1;)
        i32.const 64
        call {alloc}
        drop
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br 0 (;@2;)
      end
    end
    i32.const -1
    global.set {ALLOC_WATERMARK_GLOBAL}
    call {main_index}
    global.get {ALLOC_WATERMARK_GLOBAL}
    i32.const {COLLECT_THRESHOLD_BYTES}
    i32.lt_u
    global.get {FREE_HEAD_GLOBAL}
    i32.const 0
    i32.ne
    i32.and
  )
"""
        module_end = wat.rstrip()
        self.assertTrue(module_end.endswith(")"), "printed artifact is not a WAT module")
        return module_end[:-1] + probe + ")\n"

    def print_artifact(self, artifact: Path) -> str:
        printed = subprocess.run(
            ["wasm-tools", "print", str(artifact)],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(printed.returncode, 0, printed.stderr)
        return printed.stdout

    def run_probe(self, probe_wat: str, probe_name: str, output_name: str) -> str:
        probe = ROOT / ".build/tests" / output_name
        parsed = subprocess.run(
            ["wasm-tools", "parse", "-o", str(probe)],
            cwd=ROOT,
            input=probe_wat,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(parsed.returncode, 0, parsed.stderr)
        validated = subprocess.run(
            ["wasm-tools", "validate", str(probe)],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(validated.returncode, 0, validated.stderr)
        executed = subprocess.run(
            [
                "wasmtime",
                "run",
                "--invoke",
                probe_name,
                "--dir=.",
                str(probe),
            ],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(executed.returncode, 0, executed.stderr)
        return executed.stdout

    def test_rooted_cycle_survives_and_unreachable_cycle_is_reused(self) -> None:
        artifact = self.compile_linear_fixture()
        wat = self.print_artifact(artifact)
        self.assertNotIn("struct.", wat)
        self.assertNotIn("array.", wat)
        self.assertNotIn("ref.", wat)

        probe_wat = self.add_runtime_probe(wat)
        stdout = self.run_probe(
            probe_wat,
            "linear_gc_runtime_probe",
            "linear_gc_runtime_e2e_probe.wasm",
        )
        self.assertEqual(stdout.strip().splitlines()[-1], "1", stdout)

    def test_source_level_live_data_survives_entry_collection(self) -> None:
        artifact = self.compile_linear_fixture(
            COLLECT_FIXTURE,
            "linear_gc_entry_collect.wasm",
        )
        wat = self.print_artifact(artifact)
        self.assertNotIn("struct.", wat)
        self.assertNotIn("array.", wat)
        # The fixture allocates ~160 MiB of Vec garbage across 160 calls,
        # so the 64 MiB entry gate must have fired at least twice while the
        # kept strings stayed reachable through main's root slots. The
        # in-source assertions panic on any corruption; exit 0 proves the
        # collector preserved live data reachable via transitive marking.
        executed = subprocess.run(
            [
                "wasmtime",
                "run",
                "--wasm",
                "gc",
                "--wasm",
                "function-references",
                "--dir=.",
                str(artifact),
            ],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(executed.returncode, 0, executed.stderr)

    def test_function_entry_collects_when_watermark_exceeds_threshold(self) -> None:
        artifact = self.compile_linear_fixture()
        wat = self.print_artifact(artifact)
        self.assertEqual(
            len(GLOBAL_DEFINITION.findall(wat)),
            8,
            "linear artifact must emit the heap pointer plus 7 fallback globals",
        )
        probe_wat = self.add_entry_collect_probe(wat)
        stdout = self.run_probe(
            probe_wat,
            "linear_gc_entry_collect_probe",
            "linear_gc_entry_collect_probe.wasm",
        )
        self.assertEqual(stdout.strip().splitlines()[-1], "1", stdout)


if __name__ == "__main__":
    unittest.main()
