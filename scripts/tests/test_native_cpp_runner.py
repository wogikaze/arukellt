"""Unit + smoke tests for the public native-cpp host launcher."""

from __future__ import annotations

import importlib.util
import os
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

RUNNER_PATH = ROOT / "scripts" / "run" / "native-cpp-runner.py"
SPEC = importlib.util.spec_from_file_location("native_cpp_runner", RUNNER_PATH)
assert SPEC and SPEC.loader
runner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(runner)

WRAPPER = ROOT / "scripts" / "run" / "arukellt-selfhost.sh"
FIXTURE = ROOT / "tests" / "fixtures" / "native_cpp" / "constant_return.ark"
PUBLIC = ROOT / "tests" / "fixtures" / "native_cpp_public"
PUBLIC_BAD_MAIN = PUBLIC / "main_with_param.ark"
PUBLIC_ARGS = PUBLIC / "args_print.ark"
PUBLIC_STDIO = PUBLIC / "stdio_hello.ark"
PUBLIC_EXIT = PUBLIC / "process_exit_7.ark"
PUBLIC_FS = PUBLIC / "fs_roundtrip.ark"
PUBLIC_STDIN = PUBLIC / "stdin_echo.ark"
PUBLIC_PANIC = PUBLIC / "panic_message.ark"
PUBLIC_TRAP_DIV = PUBLIC / "trap_div_zero.ark"
PUBLIC_FS_WRITE_ERR = PUBLIC / "fs_write_missing_parent.ark"
PUBLIC_HOF = PUBLIC / "hof_named_callback.ark"
PUBLIC_PHI = PUBLIC / "phi_if_join_i32.ark"
PUBLIC_SCALAR = PUBLIC / "scalar_cfg.ark"
PUBLIC_UNSUPPORTED = PUBLIC / "unsupported_array_new.ark"
RUNTIME_C = ROOT / "src" / "compiler" / "native_c" / "runtime" / "ark_native_runtime.c"


class NativeCppRunnerParseTest(unittest.TestCase):
    def test_splits_program_args_after_ddash(self) -> None:
        compiler, program = runner.parse_run_argv(
            ["run", "prog.ark", "--target", "native-cpp", "--", "--target", "wasm32-gc", "x"]
        )
        self.assertEqual(compiler, ["run", "prog.ark", "--target", "native-cpp"])
        self.assertEqual(program, ["--target", "wasm32-gc", "x"])

    def test_extract_target_equals_form(self) -> None:
        self.assertEqual(
            runner.extract_target(["run", "a.ark", "--target=native-cpp"]),
            "native-cpp",
        )

    def test_opt_level_mapping(self) -> None:
        self.assertEqual(runner.clang_opt_flag(0), "-O0")
        self.assertEqual(runner.clang_opt_flag(1), "-O1")
        self.assertEqual(runner.clang_opt_flag(2), "-O2")
        self.assertEqual(runner.clang_opt_flag(3), "-O2")

    def test_signal_exit_mapping(self) -> None:
        self.assertEqual(runner.map_child_exit(0), 0)
        self.assertEqual(runner.map_child_exit(7), 7)
        self.assertEqual(runner.map_child_exit(-2), 130)

    def test_program_args_after_ddash_do_not_select_target(self) -> None:
        compiler, program = runner.parse_run_argv(
            ["run", "prog.ark", "--", "--target", "native-cpp"]
        )
        self.assertIsNone(runner.extract_target(compiler))
        self.assertEqual(program, ["--target", "native-cpp"])


class NativeCppRunnerSmokeTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        if not FIXTURE.is_file():
            raise unittest.SkipTest("native_cpp fixture missing")
        build_dir = os.environ.get("ARUKELLT_BUILD_DIR", "").strip()
        s2_candidates = []
        if build_dir:
            root = Path(build_dir)
            s2_candidates.extend(
                [
                    root / "selfhost" / "arukellt-s2-runtime.wasm",
                    root / "selfhost" / "arukellt-s2.wasm",
                ]
            )
        s2_candidates.extend(
            [
                ROOT / ".build" / "selfhost" / "arukellt-s2-runtime.wasm",
                ROOT / ".build" / "selfhost" / "arukellt-s2.wasm",
            ]
        )
        cls.s2 = next((path for path in s2_candidates if path.is_file()), None)
        if cls.s2 is None:
            raise unittest.SkipTest("selfhost s2 wasm is required for native-cpp run smoke")

    def _env(self, build_dir: Path) -> dict[str, str]:
        env = os.environ.copy()
        env["ARUKELLT_SELFHOST_WASM"] = str(self.s2)
        env["ARUKELLT_BUILD_DIR"] = str(build_dir)
        return env

    def test_wrapper_routes_native_cpp_run(self) -> None:
        build_dir = ROOT / ".build" / "native-run-smoke-test"
        build_dir.mkdir(parents=True, exist_ok=True)
        env = self._env(build_dir)
        env["ARUKELLT_NATIVE_CPP_VERBOSE"] = "1"
        result = subprocess.run(
            [
                str(WRAPPER),
                "run",
                str(FIXTURE.relative_to(ROOT)),
                "--target",
                "native-cpp",
            ],
            cwd=ROOT,
            env=env,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(
            result.returncode,
            0,
            f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}",
        )

    def test_rejects_main_with_parameters(self) -> None:
        build_dir = ROOT / ".build" / "native-run-smoke-test"
        build_dir.mkdir(parents=True, exist_ok=True)
        result = subprocess.run(
            [
                str(WRAPPER),
                "compile",
                str(PUBLIC_BAD_MAIN.relative_to(ROOT)),
                "--target",
                "native-cpp",
                "--emit",
                "c",
                "-o",
                ".build/native-run-smoke-test/bad-main.c",
            ],
            cwd=ROOT,
            env=self._env(build_dir),
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertNotEqual(result.returncode, 0)
        combined = result.stdout + result.stderr
        self.assertIn("requires `fn main()` with no parameters", combined)

    def test_args_exclude_argv0(self) -> None:
        build_dir = ROOT / ".build" / "native-run-smoke-test"
        build_dir.mkdir(parents=True, exist_ok=True)
        result = subprocess.run(
            [
                str(WRAPPER),
                "run",
                str(PUBLIC_ARGS.relative_to(ROOT)),
                "--target",
                "native-cpp",
                "--",
                "hello",
                "a b",
                "--flag",
            ],
            cwd=ROOT,
            env=self._env(build_dir),
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("count=3", result.stdout)
        self.assertIn("arg=hello", result.stdout)
        self.assertNotIn(".exe", result.stdout)

    def test_stdio_stdout_and_stderr(self) -> None:
        build_dir = ROOT / ".build" / "native-run-smoke-test"
        build_dir.mkdir(parents=True, exist_ok=True)
        result = subprocess.run(
            [str(WRAPPER), "run", str(PUBLIC_STDIO.relative_to(ROOT)), "--target", "native-cpp"],
            cwd=ROOT,
            env=self._env(build_dir),
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("hello-stdout", result.stdout)
        self.assertIn("hello-stderr", result.stderr)

    def test_process_exit_code_passthrough(self) -> None:
        build_dir = ROOT / ".build" / "native-run-smoke-test"
        build_dir.mkdir(parents=True, exist_ok=True)
        result = subprocess.run(
            [str(WRAPPER), "run", str(PUBLIC_EXIT.relative_to(ROOT)), "--target", "native-cpp"],
            cwd=ROOT,
            env=self._env(build_dir),
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 7, result.stdout + result.stderr)
        self.assertIn("before-exit", result.stdout)
        self.assertNotIn("after-exit", result.stdout)

    def test_fs_roundtrip_uses_launcher_cwd(self) -> None:
        build_dir = ROOT / ".build" / "native-run-smoke-test"
        work = build_dir / "cwd-work"
        work.mkdir(parents=True, exist_ok=True)
        out_file = work / "native_cpp_public_fs_roundtrip.txt"
        out_file.unlink(missing_ok=True)
        result = subprocess.run(
            [str(WRAPPER), "run", str(PUBLIC_FS.resolve()), "--target", "native-cpp"],
            cwd=work,
            env=self._env(build_dir),
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("fs-ok", result.stdout)
        self.assertTrue(out_file.is_file(), "relative write must land in launcher cwd")
        self.assertEqual(out_file.read_text(encoding="utf-8"), "fs-ok")

    def test_stdin_passthrough(self) -> None:
        build_dir = ROOT / ".build" / "native-run-smoke-test"
        build_dir.mkdir(parents=True, exist_ok=True)
        result = subprocess.run(
            [str(WRAPPER), "run", str(PUBLIC_STDIN.relative_to(ROOT)), "--target", "native-cpp"],
            cwd=ROOT,
            env=self._env(build_dir),
            input="stdin-bytes",
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("in=stdin-bytes", result.stdout)

    def test_public_runner_defaults_gc_on(self) -> None:
        """Public launcher must force GC on when unset (process env inheritance contract)."""
        source = RUNNER_PATH.read_text(encoding="utf-8")
        self.assertIn('env["ARUKELLT_NATIVE_GC"] = "1"', source)
        self.assertIn("os.execve", source)
        self.assertNotIn("ARUKELLT_NATIVE_GC_SKIP_CLEARS", source)
        self.assertNotIn("allow_high_rss", source)
        self.assertNotIn("ARUKELLT_NATIVE_ALLOW_HIGH_RSS", source)

    def test_runtime_defaults_gc_on_when_unset(self) -> None:
        source = RUNTIME_C.read_text(encoding="utf-8")
        self.assertIn("if (enable == NULL) return 1;", source)
        self.assertIn("void ark_rt_panic(ark_string *message)", source)
        self.assertIn("void ark_rt_trap_kind(ark_trap_kind kind)", source)

    def test_panic_prints_message_and_exits_nonzero(self) -> None:
        build_dir = ROOT / ".build" / "native-run-smoke-test"
        build_dir.mkdir(parents=True, exist_ok=True)
        result = subprocess.run(
            [str(WRAPPER), "run", str(PUBLIC_PANIC.relative_to(ROOT)), "--target", "native-cpp"],
            cwd=ROOT,
            env=self._env(build_dir),
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("boom-native", result.stderr)
        self.assertIn("panic", result.stderr)

    def test_trap_div_zero_prints_kind(self) -> None:
        build_dir = ROOT / ".build" / "native-run-smoke-test"
        build_dir.mkdir(parents=True, exist_ok=True)
        result = subprocess.run(
            [str(WRAPPER), "run", str(PUBLIC_TRAP_DIV.relative_to(ROOT)), "--target", "native-cpp"],
            cwd=ROOT,
            env=self._env(build_dir),
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("divide by zero", result.stderr)

    def test_fs_write_error_is_result_not_trap(self) -> None:
        build_dir = ROOT / ".build" / "native-run-smoke-test"
        build_dir.mkdir(parents=True, exist_ok=True)
        result = subprocess.run(
            [str(WRAPPER), "run", str(PUBLIC_FS_WRITE_ERR.relative_to(ROOT)), "--target", "native-cpp"],
            cwd=ROOT,
            env=self._env(build_dir),
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("write-err", result.stdout)
        self.assertNotIn("runtime trap", result.stderr)
        self.assertNotIn("divide by zero", result.stderr)

    def test_hof_zero_capture_named_callback(self) -> None:
        build_dir = ROOT / ".build" / "native-run-smoke-test"
        build_dir.mkdir(parents=True, exist_ok=True)
        result = subprocess.run(
            [str(WRAPPER), "run", str(PUBLIC_HOF.relative_to(ROOT)), "--target", "native-cpp"],
            cwd=ROOT,
            env=self._env(build_dir),
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("result=42", result.stdout)

    def test_phi_if_join_runs(self) -> None:
        build_dir = ROOT / ".build" / "native-run-smoke-test"
        build_dir.mkdir(parents=True, exist_ok=True)
        result = subprocess.run(
            [str(WRAPPER), "run", str(PUBLIC_PHI.relative_to(ROOT)), "--target", "native-cpp"],
            cwd=ROOT,
            env=self._env(build_dir),
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("value=10", result.stdout)

    def test_scalar_cfg_corpus(self) -> None:
        build_dir = ROOT / ".build" / "native-run-smoke-test"
        build_dir.mkdir(parents=True, exist_ok=True)
        result = subprocess.run(
            [str(WRAPPER), "run", str(PUBLIC_SCALAR.relative_to(ROOT)), "--target", "native-cpp"],
            cwd=ROOT,
            env=self._env(build_dir),
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("scalar-ok", result.stdout)

    def test_unsupported_opcode_is_capability_diagnostic(self) -> None:
        build_dir = ROOT / ".build" / "native-run-smoke-test"
        build_dir.mkdir(parents=True, exist_ok=True)
        result = subprocess.run(
            [
                str(WRAPPER),
                "compile",
                str(PUBLIC_UNSUPPORTED.relative_to(ROOT)),
                "--target",
                "native-cpp",
                "--emit",
                "c",
                "-o",
                ".build/native-run-smoke-test/unsupported.c",
            ],
            cwd=ROOT,
            env=self._env(build_dir),
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertNotEqual(result.returncode, 0)
        combined = result.stdout + result.stderr
        self.assertIn("does not support MIR opcode MIR_ARRAY_NEW", combined)
        self.assertNotIn("compiler ICE", combined)


if __name__ == "__main__":
    unittest.main()
