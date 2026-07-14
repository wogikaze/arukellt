from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "check" / "check-trivial-tests.py"
SPEC = importlib.util.spec_from_file_location("trivial_tests", SCRIPT)
assert SPEC and SPEC.loader
trivial_tests = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = trivial_tests
SPEC.loader.exec_module(trivial_tests)


class TrivialTestsTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.scan_dir = self.root / "src" / "compiler"
        self.scan_dir.mkdir(parents=True)
        self.old_scan = trivial_tests.SCAN_DIRS
        trivial_tests.SCAN_DIRS = [self.scan_dir]

    def tearDown(self) -> None:
        trivial_tests.SCAN_DIRS = self.old_scan
        self.temp.cleanup()

    def _write(self, name: str, content: str) -> Path:
        p = self.scan_dir / name
        p.write_text(content, encoding="utf-8")
        return p

    def test_pass_when_no_trivial(self) -> None:
        self._write("good.ark", 'test mod "real" {\n    test "add" { assert(add(1, 2) == 3) }\n}\n')
        self.assertEqual(trivial_tests.find_trivial_tests(self.scan_dir / "good.ark"), [])

    def test_detect_probe_n(self) -> None:
        p = self._write("bad.ark", 'test mod "bulk" {\n    test "probe_0" { assert(0 >= 0) }\n}\n')
        findings = trivial_tests.find_trivial_tests(p)
        self.assertTrue(len(findings) > 0)
        self.assertTrue(any("probe_N" in f for f in findings))

    def test_detect_sanity_trivial(self) -> None:
        p = self._write("bad.ark", 'test mod "x" {\n    test "sanity" { assert(1 == 1) }\n}\n')
        findings = trivial_tests.find_trivial_tests(p)
        self.assertTrue(len(findings) > 0)

    def test_no_false_positive_real_sanity(self) -> None:
        p = self._write("good.ark", 'test "sanity" {\n    assert(1 + 1 == 2)\n}\n')
        findings = trivial_tests.find_trivial_tests(p)
        # assert(1 + 1 == 2) is NOT a trivial pattern — should not be flagged
        self.assertEqual(findings, [])

    def test_detect_assert_true(self) -> None:
        p = self._write("bad.ark", 'test "x" { assert(true) }\n')
        findings = trivial_tests.find_trivial_tests(p)
        self.assertTrue(len(findings) > 0)

    def test_detect_assert_0_eq_0(self) -> None:
        p = self._write("bad.ark", 'test "x" { assert(0 == 0) }\n')
        findings = trivial_tests.find_trivial_tests(p)
        self.assertTrue(len(findings) > 0)

    def test_no_false_positive_real_test(self) -> None:
        p = self._write("good.ark", 'test "len" { assert(len("abc") == 3) }\n')
        findings = trivial_tests.find_trivial_tests(p)
        self.assertEqual(findings, [])

    def test_detect_multiline_trivial_assert(self) -> None:
        """Multi-line test block with trivial assert must be detected."""
        p = self._write("bad.ark", 'test "x" {\n    assert(true)\n}\n')
        findings = trivial_tests.find_trivial_tests(p)
        self.assertTrue(len(findings) > 0)
        self.assertTrue(any("assert(true)" in f for f in findings))

    def test_detect_multiline_assert_1_eq_1(self) -> None:
        """Multi-line test block with assert(1 == 1) must be detected."""
        p = self._write("bad.ark", 'test "trait_exists" {\n    assert(1 == 1)\n}\n')
        findings = trivial_tests.find_trivial_tests(p)
        self.assertTrue(len(findings) > 0)
        self.assertTrue(any("assert(1 == 1)" in f for f in findings))

    def test_no_false_positive_n_eq_0(self) -> None:
        """assert(n == 0) where n is a variable must NOT be flagged as trivial."""
        p = self._write("good.ark", 'test "default" {\n    let n: i32 = Default_default()\n    assert(n == 0)\n}\n')
        findings = trivial_tests.find_trivial_tests(p)
        self.assertEqual(findings, [])

    def test_detect_self_comparison(self) -> None:
        """assert(x == x) self-comparison must be detected."""
        p = self._write("bad.ark", 'test "self" {\n    let x = 5\n    assert(x == x)\n}\n')
        findings = trivial_tests.find_trivial_tests(p)
        self.assertTrue(len(findings) > 0)


if __name__ == "__main__":
    unittest.main()
