"""Tests for the doc-example gate's statement wrapping contract.

`arukellt check` ignores top-level statements, so the gate must put them
inside a synthetic `fn main()` before invoking the compiler (issue #683).
These tests pin that contract without requiring a compiler build.
"""

from __future__ import annotations

import contextlib
import importlib.util
import io
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import MagicMock, patch

_SCRIPTS_DIR = Path(__file__).resolve().parent.parent
if str(_SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(_SCRIPTS_DIR))

_CHECK = _SCRIPTS_DIR / "check" / "check-doc-examples.py"
_spec = importlib.util.spec_from_file_location("check_doc_examples", _CHECK)
assert _spec is not None and _spec.loader is not None
check_doc_examples = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(check_doc_examples)


class TestWrapStatementSnippet(unittest.TestCase):
    def test_statement_only_snippet_is_wrapped(self):
        wrapped = check_doc_examples.wrap_statement_snippet('Stdio::println("hi")\n')
        self.assertIsNotNone(wrapped)
        self.assertIn("fn main() {", wrapped)
        self.assertIn('Stdio::println("hi")', wrapped)

    def test_complete_module_is_left_unchanged(self):
        code = 'fn main() {\n    Stdio::println("hi")\n}\n'
        self.assertIsNone(check_doc_examples.wrap_statement_snippet(code))

    def test_top_level_struct_is_treated_as_complete_module(self):
        code = "struct Point {\n    x: i32,\n}\n"
        self.assertIsNone(check_doc_examples.wrap_statement_snippet(code))

    def test_imports_stay_at_file_scope(self):
        code = 'use std::host::stdio::Stdio\n\nStdio::println("hi")\n'
        wrapped = check_doc_examples.wrap_statement_snippet(code)
        self.assertIsNotNone(wrapped)
        self.assertLess(
            wrapped.index("use std::host::stdio::Stdio"),
            wrapped.index("fn main() {"),
        )

    def test_import_only_snippet_is_not_wrapped(self):
        code = "use std::core\nuse std::text as t\n"
        self.assertIsNone(check_doc_examples.wrap_statement_snippet(code))

    def test_leading_comments_stay_outside_main(self):
        code = "// Parse a value.\nlet value = 1\n"
        wrapped = check_doc_examples.wrap_statement_snippet(code)
        self.assertIsNotNone(wrapped)
        self.assertLess(wrapped.index("// Parse a value."), wrapped.index("fn main() {"))


class TestNonCanonicalBody(unittest.TestCase):
    def test_inline_reject_comment_is_non_canonical(self):
        code = "let total = 1i64 + 1  // reject: i64 and i32\n"
        self.assertTrue(check_doc_examples.is_non_canonical_body(code))

    def test_migration_before_after_headers_are_non_canonical(self):
        for marker in ("// before", "// After", "  // BEFORE  "):
            with self.subTest(marker=marker):
                self.assertTrue(
                    check_doc_examples.is_non_canonical_body(f"{marker}\nlet x = 1\n")
                )

    def test_ordinary_comment_is_canonical(self):
        for comment in (
            "// after this call, the value is consumed",
            "// reject invalid input before parsing",
        ):
            with self.subTest(comment=comment):
                self.assertFalse(
                    check_doc_examples.is_non_canonical_body(f"{comment}\nlet x = 1\n")
                )


class TestCheckBlockInput(unittest.TestCase):
    def _checked_code(self, code: str) -> str:
        captured: dict[str, str] = {}

        def fake_run(cmd, **kwargs):
            path = Path(kwargs["cwd"]) / cmd[2]
            captured["code"] = path.read_text(encoding="utf-8")
            result = MagicMock()
            result.returncode = 0
            result.stdout = ""
            result.stderr = ""
            return result

        with patch.object(check_doc_examples.subprocess, "run", side_effect=fake_run):
            passed, error = check_doc_examples.check_block(
                "arukellt", code, Path("docs/test.md"), 0, "wasm32"
            )
        self.assertTrue(passed, error)
        return captured["code"]

    def test_module_snippet_is_checked_verbatim(self):
        code = 'fn main() {\n    Stdio::println("hi")\n}\n'
        self.assertEqual(self._checked_code(code), code)

    def test_wrapped_snippet_is_compiled_to_a_temp_wasm(self):
        commands: list[list[str]] = []

        def fake_run(cmd, **kwargs):
            commands.append(cmd)
            result = MagicMock()
            result.returncode = 0
            result.stdout = ""
            result.stderr = ""
            return result

        with patch.object(check_doc_examples.subprocess, "run", side_effect=fake_run):
            passed, error = check_doc_examples.check_block(
                "arukellt",
                'Stdio::println("hi")\n',
                Path("docs/test.md"),
                0,
                "wasm32",
                compile_check=True,
            )
        self.assertTrue(passed, error)
        self.assertEqual([cmd[1] for cmd in commands], ["check", "compile"])
        self.assertIn("-o", commands[1])
        self.assertTrue(commands[1][-1].endswith(".wasm"))
        self.assertFalse((check_doc_examples.ROOT / commands[1][-1]).exists())


class TestMainSkipsNotation(unittest.TestCase):
    def test_notation_block_is_not_sent_to_the_compiler(self):
        markdown = (
            "```ark\n"
            "let total = 1i64 + 1  // reject: i64 and i32\n"
            "```\n"
            "\n"
            "```ark\n"
            'Stdio::println("hi")\n'
            "```\n"
        )
        checked: list[tuple[str, bool]] = []

        def fake_check_block(_arukellt, code, _md, _idx, _target, compile_check=False):
            checked.append((code, compile_check))
            return True, ""

        with tempfile.TemporaryDirectory() as tmp:
            docs_dir = Path(tmp)
            (docs_dir / "example.md").write_text(markdown, encoding="utf-8")
            with (
                patch.object(check_doc_examples, "find_arukellt", return_value="arukellt"),
                patch.object(check_doc_examples, "check_block", side_effect=fake_check_block),
                patch.object(sys, "argv", ["check-doc-examples.py", str(docs_dir)]),
                contextlib.redirect_stdout(io.StringIO()),
            ):
                self.assertEqual(check_doc_examples.main(), 0)

        self.assertEqual(len(checked), 1)
        code, compile_check = checked[0]
        self.assertIn("fn main() {", code)
        self.assertNotIn("// reject:", code)
        self.assertTrue(compile_check)


if __name__ == "__main__":
    unittest.main()
