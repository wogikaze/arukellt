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
import os
import re
import tempfile
import unittest
import unittest.mock
from contextlib import redirect_stdout

import sys
from pathlib import Path

_SCRIPTS_DIR = Path(__file__).resolve().parent.parent
if str(_SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(_SCRIPTS_DIR))

from selfhost.checks import (  # noqa: E402
    BootstrapOverlayError,
    _replace_optional,
    _replace_required,
    _patch_bootstrap_prelude_structural_box_slice_text,
    _patch_bootstrap_wit_type_paths,
    _rewrite_bootstrap_wit_type_paths_text,
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
        with unittest.mock.patch.dict(os.environ, {"ARUKELLT_OVERLAY_VERBOSE": "1"}):
            with redirect_stdout(buf):
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
        with unittest.mock.patch.dict(os.environ, {"ARUKELLT_OVERLAY_VERBOSE": "1"}):
            with redirect_stdout(buf):
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


class TestStructuralBoxSliceOverlay(unittest.TestCase):
    """The compatibility transform stays source-only and narrowly scoped."""

    def test_rewrites_generic_enum_wrappers_to_structs(self) -> None:
        source = (
            "pub enum Box<T> {\n"
            "    Owned(T),\n"
            "}\n\n"
            "pub enum Slice<T> {\n"
            "    View(Vec<T>, i32, i32),\n"
            "}\n\n"
            "// Box is an owned heap wrapper.\n"
            "pub fn Box_new<T>(value: T) -> Box<T> {\n"
            "    Box::Owned(value)\n"
            "}\n\n"
            "pub fn unbox<T>(boxed: Box<T>) -> T {\n"
            "    match boxed { Box::Owned(value) => value, }\n"
            "}\n\n"
            "pub fn as_slice<T>(source: Vec<T>, start: i32, end: i32) -> Slice<T> {\n"
            "    Slice::View(source, start, end)\n"
            "}\n\n"
            "pub fn slice_len<T>(view: Slice<T>) -> i32 {\n"
            "    0\n"
            "}\n\n"
            "pub fn slice_get<T>(view: Slice<T>, index: i32) -> T {\n"
            "    match view { Slice::View(source, start, _) => get_unchecked(source, index), }\n"
            "}\n\n"
            "// Collections\n"
        )
        result = _patch_bootstrap_prelude_structural_box_slice_text(source)
        self.assertIn("pub struct Box<T>", result)
        self.assertIn("pub struct Slice<T>", result)
        self.assertNotIn("pub enum Box<T>", result)
        self.assertNotIn("Box::Owned", result)
        self.assertIn("get_unchecked(view.source", result)

    def test_rejects_drifted_prelude(self) -> None:
        with self.assertRaises(BootstrapOverlayError):
            _patch_bootstrap_prelude_structural_box_slice_text("pub enum Box<T> {}\n")


class TestWitTypePathOverlay(unittest.TestCase):
    """WIT type names are normalized only inside the flat bootstrap overlay."""

    def test_rewrites_type_names_but_not_calls(self) -> None:
        source = (
            "fn parse(ty: types::WitType) -> ast::WitNode {\n"
            "    types::wit_type_from_id(0)\n"
            "}\n"
            "fn make(func: world::WitFunc) -> types::WitParam { func }\n"
        )
        result = _rewrite_bootstrap_wit_type_paths_text(source)
        self.assertIn("ty: WitType", result)
        self.assertIn("-> WitNode", result)
        self.assertIn("types::wit_type_from_id(0)", result)
        self.assertIn("func: WitFunc", result)
        self.assertIn("-> WitParam", result)

    def test_requires_the_wit_closure(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            wit_root = root / "std" / "wit"
            wit_root.mkdir(parents=True)
            for name in ("ast.ark", "parser.ark", "types.ark", "world.ark"):
                (wit_root / name).write_text(
                    "types::WitType\n" if name == "parser.ark" else "WitType\n",
                    encoding="utf-8",
                )
            _patch_bootstrap_wit_type_paths(root)
            self.assertEqual(
                (wit_root / "parser.ark").read_text(encoding="utf-8"),
                "WitType\n",
            )

    def test_reports_missing_wit_source(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaises(BootstrapOverlayError):
                _patch_bootstrap_wit_type_paths(Path(tmp))


if __name__ == "__main__":
    unittest.main()
