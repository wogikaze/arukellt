"""Native-cpp scalar, CFG, and direct-call execution checks."""

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
FIXTURE = Path("tests/fixtures/native_cpp/scalar_cfg_call.ark")
RUNTIME_DIR = ROOT / "src/compiler/native_c/runtime"
RUNTIME_SOURCE = RUNTIME_DIR / "ark_native_runtime.c"


def resolve_clang() -> str | None:
    candidates = [shutil.which("clang"), "/usr/local/swift/usr/bin/clang", "/usr/bin/clang"]
    candidates.extend(str(path) for path in sorted(Path("/usr/bin").glob("clang-[0-9]*"), reverse=True))
    for candidate in candidates:
        if candidate and Path(candidate).is_file():
            return candidate
    return None


class NativeCppScalarCfgTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        configured = os.environ.get("ARUKELLT_NATIVE_CPP_COMPILER_WASM")
        cls.compiler = Path(configured) if configured else ROOT / ".build/selfhost/arukellt-s2.wasm"
        cls.clang = resolve_clang()
        if not cls.compiler.is_file() or cls.clang is None:
            raise unittest.SkipTest("current-source s2 and clang are required")

    def test_scalar_cfg_and_direct_call_return_42(self) -> None:
        with tempfile.TemporaryDirectory(dir=ROOT / ".build") as temp_dir:
            temp = Path(temp_dir)
            generated = temp / "scalar_cfg_call.c"
            env = os.environ.copy()
            env["ARUKELLT_SELFHOST_WASM"] = str(self.compiler)
            compile_result = subprocess.run(
                [
                    str(WRAPPER),
                    "compile",
                    str(FIXTURE),
                    "--target",
                    "native-cpp",
                    "-o",
                    str(generated.relative_to(ROOT)),
                ],
                cwd=ROOT,
                env=env,
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(compile_result.returncode, 0, compile_result.stdout + compile_result.stderr)
            source = generated.read_text(encoding="utf-8")
            symbols = re.findall(r"int32_t (ark_f_\d+__P_[^\s]+__R_i32)\(", source)
            main_call = re.search(r"\(void\)(ark_f_\d+__P_v__R_i32)\(\);", source)
            self.assertIsNotNone(main_call, source)
            self.assertGreaterEqual(len(symbols), 3)
            self.assertIn("goto ark_ctl_", source)

            generated_object = temp / "generated.o"
            object_result = subprocess.run(
                [
                    self.clang,
                    "-std=c99",
                    "-O0",
                    "-Wall",
                    "-Wextra",
                    "-Werror",
                    "-I",
                    str(RUNTIME_DIR),
                    "-Dmain=ark_generated_entry",
                    "-c",
                    str(generated),
                    "-o",
                    str(generated_object),
                ],
                cwd=ROOT,
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(object_result.returncode, 0, object_result.stdout + object_result.stderr)
            ark_main = main_call.group(1)
            harness = temp / "harness.c"
            harness.write_text(
                f"#include <stdint.h>\nint32_t {ark_main}(void);\n"
                f"int main(void) {{ return {ark_main}() == INT32_C(42) ? 0 : 1; }}\n",
                encoding="utf-8",
            )
            executable = temp / "scalar_cfg_call"
            link_result = subprocess.run(
                [
                    self.clang,
                    "-std=c99",
                    "-O0",
                    str(harness),
                    str(generated_object),
                    str(RUNTIME_SOURCE),
                    "-I",
                    str(RUNTIME_DIR),
                    "-o",
                    str(executable),
                ],
                cwd=ROOT,
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(link_result.returncode, 0, link_result.stdout + link_result.stderr)
            run_result = subprocess.run([str(executable)], cwd=ROOT, capture_output=True, text=True, check=False)
            self.assertEqual(run_result.returncode, 0, run_result.stdout + run_result.stderr)


if __name__ == "__main__":
    unittest.main()
