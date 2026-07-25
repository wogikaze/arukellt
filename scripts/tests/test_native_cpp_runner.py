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

    def test_wrapper_routes_native_cpp_run(self) -> None:
        env = os.environ.copy()
        env["ARUKELLT_SELFHOST_WASM"] = str(self.s2)
        env["ARUKELLT_NATIVE_CPP_VERBOSE"] = "1"
        build_dir = ROOT / ".build" / "native-run-smoke-test"
        build_dir.mkdir(parents=True, exist_ok=True)
        env["ARUKELLT_BUILD_DIR"] = str(build_dir)
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


if __name__ == "__main__":
    unittest.main()
