"""End-to-end checks for the native-cpp constant-return vertical slice."""

from __future__ import annotations

import os
import re
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
WRAPPER = ROOT / "scripts/run/arukellt-selfhost.sh"
CONSTANT_FIXTURE = Path("tests/fixtures/native_cpp/constant_return.ark")
UNSUPPORTED_FIXTURE = Path("tests/fixtures/native_cpp/unsupported_array.ark")
RUNTIME_DIR = ROOT / "src/compiler/native_c/runtime"
RUNTIME_SOURCE = RUNTIME_DIR / "ark_native_runtime.c"


def resolve_clang() -> str | None:
    candidates = [shutil.which("clang"), "/usr/local/swift/usr/bin/clang", "/usr/bin/clang"]
    candidates.extend(str(path) for path in sorted(Path("/usr/bin").glob("clang-[0-9]*"), reverse=True))
    for candidate in candidates:
        if candidate and Path(candidate).is_file():
            return candidate
    return None


class NativeCppPhase1ATests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        configured = os.environ.get("ARUKELLT_NATIVE_CPP_COMPILER_WASM")
        cls.compiler = Path(configured) if configured else ROOT / ".build/selfhost/arukellt-s2.wasm"
        if not cls.compiler.is_file():
            raise unittest.SkipTest("current-source s2 compiler wasm is not built")
        cls.clang = resolve_clang()
        if cls.clang is None:
            raise unittest.SkipTest("clang is required")

    def compile_fixture(self, fixture: Path, output: Path) -> subprocess.CompletedProcess[str]:
        env = os.environ.copy()
        env["ARUKELLT_SELFHOST_WASM"] = str(self.compiler)
        return subprocess.run(
            [str(WRAPPER), "compile", str(fixture), "--target", "native-cpp", "-o", str(output.relative_to(ROOT))],
            cwd=ROOT,
            env=env,
            capture_output=True,
            text=True,
            check=False,
        )

    def test_constant_return_emits_compiles_and_runs(self) -> None:
        with tempfile.TemporaryDirectory(dir=ROOT / ".build") as temp_dir:
            output = Path(temp_dir) / "constant_return.c"
            compile_result = self.compile_fixture(CONSTANT_FIXTURE, output)
            self.assertEqual(compile_result.returncode, 0, compile_result.stdout + compile_result.stderr)
            source = output.read_text(encoding="utf-8")
            self.assertIn("#include <stdint.h>", source)
            symbol_match = re.search(r"int32_t (ark_f_\d+__P_v__R_i32)\(void\)", source)
            self.assertIsNotNone(symbol_match, source)
            local_match = re.search(r"int32_t (ark_v_\d+);", source)
            self.assertIsNotNone(local_match, source)
            local = local_match.group(1)
            self.assertIn(f"{local} = INT32_C(42);", source)
            self.assertIn(f"return {local};", source)
            self.assertIn("int main(int argc, char **argv)", source)
            self.assertIn("return 0;", source)
            self.assertNotIn(".globl", source)
            self.assertNotIn("xor %eax", source)

            executable = Path(temp_dir) / "constant_return"
            clang_result = subprocess.run(
                [
                    self.clang,
                    "-std=c99",
                    "-O0",
                    "-Wall",
                    "-Wextra",
                    "-Werror",
                    "-I",
                    str(RUNTIME_DIR),
                    str(output),
                    str(RUNTIME_SOURCE),
                    "-o",
                    str(executable),
                ],
                cwd=ROOT,
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(clang_result.returncode, 0, clang_result.stdout + clang_result.stderr)
            run_result = subprocess.run([str(executable)], cwd=ROOT, capture_output=True, text=True, check=False)
            self.assertEqual(run_result.returncode, 0)
            self.assertEqual(run_result.stdout, "")
            self.assertEqual(run_result.stderr, "")

    def test_unsupported_array_is_rejected_without_output(self) -> None:
        with tempfile.TemporaryDirectory(dir=ROOT / ".build") as temp_dir:
            output = Path(temp_dir) / "unsupported.c"
            compile_result = self.compile_fixture(UNSUPPORTED_FIXTURE, output)
            diagnostic = compile_result.stdout + compile_result.stderr
            self.assertNotEqual(compile_result.returncode, 0, diagnostic)
            self.assertIn("target `native-cpp`", diagnostic)
            self.assertIn("MIR_ARRAY_NEW", diagnostic)
            self.assertIn("function `main`", diagnostic)
            self.assertIn("planned phase 2", diagnostic)
            self.assertFalse(output.exists())
            self.assertNotIn("native-llvm scaffold", diagnostic)


if __name__ == "__main__":
    unittest.main()
