"""Tests for bootstrap overlay patch guards in selfhost.checks.

Run from the repo root:
    python3 -m pytest scripts/tests/test_selfhost_overlay.py -v
    python3 scripts/tests/test_selfhost_overlay.py

These tests verify the checked-helper behaviour (_sub_required, _replace_required,
_sub_optional, _replace_optional) and the BootstrapOverlayError exception.
They do NOT require wasmtime or the pinned wasm — they are pure unit tests.
"""
from __future__ import annotations

import io
import re
import unittest
from contextlib import redirect_stdout, redirect_stderr

import sys
from pathlib import Path

_SCRIPTS_DIR = Path(__file__).resolve().parent.parent
if str(_SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(_SCRIPTS_DIR))

from selfhost.checks import (  # noqa: E402
    BootstrapOverlayError,
    _replace_optional,
    _replace_required,
    _sub_optional,
    _sub_required,
)


class TestSubRequired(unittest.TestCase):
    """_sub_required must raise BootstrapOverlayError on zero matches."""

    def test_matches_and_replaces(self) -> None:
        text = "pub fn foo() -> i32 { 1 }"
        result = _sub_required(
            text,
            r"pub fn (\w+)\(\) -> i32",
            r"pub fn \1() -> i64",
            "test replace",
        )
        self.assertEqual(result, "pub fn foo() -> i64 { 1 }")

    def test_raises_on_no_match(self) -> None:
        with self.assertRaises(BootstrapOverlayError) as ctx:
            _sub_required(
                "nothing here",
                r"pub fn missing\(\)",
                "pub fn missing()",
                "stub missing function",
            )
        self.assertIn("stub missing function", str(ctx.exception))
        self.assertIn("pub fn missing", str(ctx.exception))

    def test_count_param_respected(self) -> None:
        text = "a a a"
        result = _sub_required(text, r"a", "b", "replace a with b", count=2)
        self.assertEqual(result, "b b a")

    def test_flags_param_respected(self) -> None:
        text = "FN foo()\nfn bar()"
        result = _sub_required(text, r"^fn ", "pub fn ", "promote fn", flags=re.M)
        self.assertIn("pub fn bar()", result)
        self.assertNotIn("pub fn foo()", result)


class TestReplaceRequired(unittest.TestCase):
    """_replace_required must raise BootstrapOverlayError when old is absent."""

    def test_replaces_when_present(self) -> None:
        text = "use wasm::sections_tail"
        result = _replace_required(
            text,
            "use wasm::sections_tail",
            "use wasm::sections_data",
            "rewrite import",
        )
        self.assertEqual(result, "use wasm::sections_data")

    def test_raises_when_absent(self) -> None:
        with self.assertRaises(BootstrapOverlayError) as ctx:
            _replace_required(
                "no match here",
                "use wasm::sections_tail",
                "use wasm::sections_data",
                "rewrite import",
            )
        self.assertIn("rewrite import", str(ctx.exception))
        self.assertIn("use wasm::sections_tail", str(ctx.exception))

    def test_error_message_truncates_long_snippet(self) -> None:
        long_old = "x" * 300
        with self.assertRaises(BootstrapOverlayError) as ctx:
            _replace_required("text", long_old, "y", "long snippet test")
        msg = str(ctx.exception)
        self.assertIn("long snippet test", msg)
        self.assertLessEqual(len(msg), 500)


class TestSubOptional(unittest.TestCase):
    """_sub_optional must NOT raise on zero matches; must print a skip notice."""

    def test_matches_and_replaces(self) -> None:
        text = "fn foo()"
        result = _sub_optional(text, r"^fn ", "pub fn ", "promote fn", flags=re.M)
        self.assertEqual(result, "pub fn foo()")

    def test_no_match_does_not_raise(self) -> None:
        buf = io.StringIO()
        with redirect_stderr(buf):
            result = _sub_optional(
                "nothing here",
                r"^fn ",
                "pub fn ",
                "promote fn",
                flags=re.M,
            )
        self.assertEqual(result, "nothing here")
        self.assertIn("optional patch skipped", buf.getvalue())
        self.assertIn("promote fn", buf.getvalue())


class TestReplaceOptional(unittest.TestCase):
    """_replace_optional must NOT raise when old is absent; must print a skip notice."""

    def test_replaces_when_present(self) -> None:
        text = "chars::skip_whitespace"
        result = _replace_optional(
            text,
            "chars::skip_whitespace",
            "char_skip::skip_whitespace_impl",
            "lexer chars rename",
        )
        self.assertEqual(result, "char_skip::skip_whitespace_impl")

    def test_no_match_does_not_raise(self) -> None:
        buf = io.StringIO()
        with redirect_stderr(buf):
            result = _replace_optional(
                "nothing here",
                "chars::skip_whitespace",
                "char_skip::skip_whitespace_impl",
                "lexer chars rename",
            )
        self.assertEqual(result, "nothing here")
        self.assertIn("optional patch skipped", buf.getvalue())
        self.assertIn("lexer chars rename", buf.getvalue())


class TestBootstrapOverlayError(unittest.TestCase):
    """BootstrapOverlayError is a RuntimeError subclass."""

    def test_is_runtime_error(self) -> None:
        self.assertTrue(issubclass(BootstrapOverlayError, RuntimeError))

    def test_message_preserved(self) -> None:
        err = BootstrapOverlayError("test message")
        self.assertEqual(str(err), "test message")


if __name__ == "__main__":
    unittest.main()
