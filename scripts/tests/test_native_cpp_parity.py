"""Wasm(wasi-p1 hosted) ↔ native-cpp stdout/stderr/exit parity for a small public corpus."""

from __future__ import annotations

import os
import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WRAPPER = ROOT / "scripts" / "run" / "arukellt-selfhost.sh"
HOSTED = ROOT / "scripts" / "run" / "arukellt-run-hosted.sh"
PUBLIC = ROOT / "tests" / "fixtures" / "native_cpp_public"

# Program argv is covered by native-only E2E; arukellt-host-run accepts no trailing args.
PARITY_FIXTURES = [
    ("stdio_hello.ark", 0, ["hello-stdout"], ["hello-stderr"]),
    ("phi_if_join_i32.ark", 0, ["value=10"], []),
    ("hof_named_callback.ark", 0, ["result=42"], []),
    ("scalar_cfg.ark", 0, ["scalar-ok"], []),
    ("process_exit_7.ark", 7, ["before-exit"], []),
]


class NativeCppParityTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        build_dir = os.environ.get("ARUKELLT_BUILD_DIR", "").strip()
        candidates = []
        if build_dir:
            root = Path(build_dir)
            candidates.extend(
                [
                    root / "selfhost" / "arukellt-s2-runtime.wasm",
                    root / "selfhost" / "arukellt-s2.wasm",
                ]
            )
        candidates.extend(
            [
                ROOT / ".build" / "selfhost" / "arukellt-s2-runtime.wasm",
                ROOT / ".build" / "selfhost" / "arukellt-s2.wasm",
            ]
        )
        cls.s2 = next((path for path in candidates if path.is_file()), None)
        if cls.s2 is None:
            raise unittest.SkipTest("selfhost s2 wasm is required")
        host_bins = [
            ROOT / "target" / "release" / "arukellt-host-run",
            ROOT / "tools" / "host-linker" / "target" / "release" / "arukellt-host-run",
            ROOT / "target" / "debug" / "arukellt-host-run",
            ROOT / "tools" / "host-linker" / "target" / "debug" / "arukellt-host-run",
        ]
        if not any(path.is_file() for path in host_bins):
            raise unittest.SkipTest("arukellt-host-run is required for wasm parity")

    def _env(self) -> dict[str, str]:
        env = os.environ.copy()
        env["ARUKELLT_SELFHOST_WASM"] = str(self.s2)
        build_dir = os.environ.get("ARUKELLT_BUILD_DIR", "").strip()
        env["ARUKELLT_BUILD_DIR"] = build_dir or str(ROOT / ".build" / "native-parity")
        return env

    def _run_wasm(self, fixture: Path) -> subprocess.CompletedProcess[str]:
        build_dir = Path(self._env()["ARUKELLT_BUILD_DIR"])
        out = build_dir / "parity-wasm" / f"{fixture.stem}.wasm"
        out.parent.mkdir(parents=True, exist_ok=True)
        compile = subprocess.run(
            [
                str(WRAPPER),
                "compile",
                str(fixture.relative_to(ROOT)),
                "--target",
                "wasm32",
                "-o",
                str(out.relative_to(ROOT)),
            ],
            cwd=ROOT,
            env=self._env(),
            capture_output=True,
            text=True,
            check=False,
            timeout=120,
        )
        if compile.returncode != 0:
            return compile
        return subprocess.run(
            [str(HOSTED), "--dir", str(ROOT), str(out)],
            cwd=ROOT,
            env=self._env(),
            capture_output=True,
            text=True,
            check=False,
            timeout=120,
        )

    def _run_native(self, fixture: Path) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [str(WRAPPER), "run", str(fixture.relative_to(ROOT)), "--target", "native-cpp"],
            cwd=ROOT,
            env=self._env(),
            capture_output=True,
            text=True,
            check=False,
            timeout=120,
        )

    def test_public_parity_corpus(self) -> None:
        for name, expected_exit, stdout_needles, stderr_needles in PARITY_FIXTURES:
            with self.subTest(fixture=name):
                fixture = PUBLIC / name
                wasm = self._run_wasm(fixture)
                native = self._run_native(fixture)
                self.assertEqual(
                    wasm.returncode,
                    expected_exit,
                    f"wasm exit for {name}: {wasm.stdout}\n{wasm.stderr}",
                )
                self.assertEqual(
                    native.returncode,
                    expected_exit,
                    f"native exit for {name}: {native.stdout}\n{native.stderr}",
                )
                for needle in stdout_needles:
                    self.assertIn(needle, wasm.stdout)
                    self.assertIn(needle, native.stdout)
                for needle in stderr_needles:
                    self.assertIn(needle, wasm.stderr)
                    self.assertIn(needle, native.stderr)


if __name__ == "__main__":
    unittest.main()
