from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "check" / "check-infile-test-adoption.py"
SPEC = importlib.util.spec_from_file_location("infile_test_adoption", SCRIPT)
assert SPEC and SPEC.loader
ita = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = ita
SPEC.loader.exec_module(ita)


class InfileTestAdoptionTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self._old_root = ita.ROOT
        ita.ROOT = str(self.root)

    def tearDown(self) -> None:
        ita.ROOT = self._old_root
        self.temp.cleanup()

    def _write(self, name: str, content: str) -> Path:
        p = self.root / name
        p.write_text(content, encoding="utf-8")
        return p

    def test_standalone_test(self) -> None:
        """A standalone `test "name" { ... }` is extracted and meaningful."""
        text = 'test "add" { assert(1 + 1 == 2) }'
        blocks = ita._extract_test_blocks(text)
        self.assertEqual(len(blocks), 1)
        name, is_mod, body, start_line = blocks[0]
        self.assertEqual(name, "add")
        self.assertFalse(is_mod)
        self.assertEqual(start_line, 1)
        self.assertIn("assert(", body)
        self.assertFalse(ita.is_trivial_test(name, body))

    def test_mod_with_multiple_inner_tests(self) -> None:
        """A test mod containing two unit tests yields one module and two cases."""
        content = 'test mod "math" {\n    test "add" { assert(1 + 1 == 2) }\n    test "sub" { assert(3 - 1 == 2) }\n}'
        self._write("mod.ark", content)
        mods, cases, meaningful = ita.count_tests_in_dir("")
        self.assertEqual(mods, 1)
        self.assertEqual(cases, 2)
        self.assertEqual(meaningful, 2)

    def test_function_bound_test(self) -> None:
        """`test <identifier> "name" { ... }` is treated as a function-bound test case."""
        text = 'test add "add_one" { assert(add(1, 2) == 3) }'
        blocks = ita._extract_test_blocks(text)
        self.assertEqual(len(blocks), 1)
        name, is_mod, body, start_line = blocks[0]
        self.assertEqual(name, "add_one")
        self.assertFalse(is_mod)
        self.assertEqual(start_line, 1)
        self.assertIn("assert(", body)

    def test_test_inside_comments_ignored(self) -> None:
        """Test declarations inside line comments are ignored."""
        text = '// test "skip" { assert(true) }\ntest "real" { assert(1 + 1 == 2) }'
        blocks = ita._extract_test_blocks(text)
        self.assertEqual(len(blocks), 1)
        self.assertEqual(blocks[0][0], "real")
        self.assertEqual(blocks[0][3], 2)

    def test_test_inside_strings_ignored(self) -> None:
        """Test declarations and asserts inside string literals are ignored."""
        text = 'x = "test mod \\"inner\\" { assert(true) }"\ntest "real" { let s = "}" ; assert(1 + 1 == 2) }'
        blocks = ita._extract_test_blocks(text)
        self.assertEqual(len(blocks), 1)
        name, is_mod, body, start_line = blocks[0]
        self.assertEqual(name, "real")
        self.assertEqual(start_line, 2)
        self.assertFalse(ita.is_trivial_test(name, body))

    def test_trivial_test_exclusion(self) -> None:
        """Probe tests and trivial asserts are excluded from meaningful count."""
        content = 'test "probe_0" { assert(0 >= 0) }\ntest "real" { assert(1 + 1 == 2) }'
        self._write("trivial.ark", content)
        mods, cases, meaningful = ita.count_tests_in_dir("")
        self.assertEqual(cases, 2)
        self.assertEqual(meaningful, 1)
        self.assertTrue(ita.is_trivial_test("probe_0", "assert(0 >= 0)"))
        self.assertFalse(ita.is_trivial_test("real", "assert(1 + 1 == 2)"))

    def test_meaningful_le_executable_cases_invariant(self) -> None:
        """The meaningful count is never greater than the executable case count."""
        content = 'test "trivial" { assert(true) }\ntest "real" { assert(1 + 1 == 2) }'
        self._write("mix.ark", content)
        mods, cases, meaningful = ita.count_tests_in_dir("")
        self.assertEqual(cases, 2)
        self.assertEqual(meaningful, 1)
        self.assertLessEqual(meaningful, cases)


if __name__ == "__main__":
    unittest.main()
