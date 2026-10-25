"""Regression tests for the [[types]] registry schema and Owner::member targets.

ADR-086 §7 makes `std/manifest.toml` `[[types]]` the public type/trait
registry: `kind` is required so generated docs group by declaration kind
instead of a hardcoded name list, and a deprecated free function may point at
`Type::method` when the type entry lists that method.
"""
from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path

_GEN_SCRIPT = Path(__file__).resolve().parents[1] / "gen" / "generate-docs.py"
_SPEC = importlib.util.spec_from_file_location("generate_docs", _GEN_SCRIPT)
assert _SPEC and _SPEC.loader
sys.path.insert(0, str(_GEN_SCRIPT.parent))
generate_docs = importlib.util.module_from_spec(_SPEC)
sys.modules[_SPEC.name] = generate_docs
_SPEC.loader.exec_module(generate_docs)


def _manifest(types, functions=None, policy=None):
    return {
        "deprecation_policy": policy
        or {"default_since": "0.1.0", "default_remove_in": "1.0.0", "minimum_complete_releases": 1},
        "types": types,
        "functions": functions or [],
    }


def _function(name, **extra):
    entry = {
        "name": name,
        "params": [],
        "returns": "()",
        "stability": "stable",
        "doc_category": "misc",
    }
    entry.update(extra)
    return entry


class TypeSchemaTests(unittest.TestCase):
    def test_kind_doc_methods_and_trait_impls_are_accepted(self) -> None:
        manifest = _manifest(
            [
                {"name": "Box", "kind": "struct", "doc": "Boxed value.", "trait_impls": ["Clone"]},
                {"name": "Ordering", "kind": "enum", "doc": "Ordering result."},
                {"name": "Iterator", "kind": "trait", "methods": ["next", "collect", "count"]},
                {"name": "u8", "kind": "primitive"},
            ]
        )
        self.assertEqual(generate_docs.validate_manifest_schema(manifest), [])

    def test_missing_kind_is_rejected(self) -> None:
        violations = generate_docs.validate_manifest_schema(_manifest([{"name": "Deque"}]))
        self.assertTrue(any("missing required field 'kind'" in item for item in violations), violations)

    def test_invalid_kind_is_rejected(self) -> None:
        violations = generate_docs.validate_manifest_schema(
            _manifest([{"name": "Deque", "kind": "record"}])
        )
        self.assertTrue(any("invalid kind 'record'" in item for item in violations), violations)

    def test_methods_must_be_a_string_list(self) -> None:
        violations = generate_docs.validate_manifest_schema(
            _manifest([{"name": "Deque", "kind": "struct", "methods": "push_back"}])
        )
        self.assertTrue(any("'methods' must be a list of strings" in item for item in violations), violations)

    def test_duplicate_type_name_is_rejected(self) -> None:
        violations = generate_docs.validate_manifest_schema(
            _manifest([{"name": "Deque", "kind": "struct"}, {"name": "Deque", "kind": "struct"}])
        )
        self.assertTrue(any("duplicate [[types]] entry" in item for item in violations), violations)


class DeprecatedByOwnerMemberTests(unittest.TestCase):
    def _deprecated_function(self, target: str):
        return _function(
            "deque_push_back",
            module="std::collections::linear",
            stability="deprecated",
            deprecated_by=target,
            deprecated_since="0.1.0",
            remove_in="1.0.0",
        )

    def test_owner_member_target_resolves_through_types_methods(self) -> None:
        manifest = _manifest(
            [{"name": "Deque", "kind": "struct", "methods": ["push_back"]}],
            [self._deprecated_function("Deque::push_back")],
        )
        self.assertEqual(generate_docs.validate_manifest_schema(manifest), [])

    def test_unlisted_method_target_is_rejected(self) -> None:
        manifest = _manifest(
            [{"name": "Deque", "kind": "struct", "methods": ["push_front"]}],
            [self._deprecated_function("Deque::push_back")],
        )
        violations = generate_docs.validate_manifest_schema(manifest)
        self.assertTrue(any("deprecated_by must resolve" in item for item in violations), violations)

    def test_deprecated_type_does_not_export_method_targets(self) -> None:
        manifest = _manifest(
            [{"name": "Deque", "kind": "struct", "stability": "deprecated", "methods": ["push_back"]}],
            [self._deprecated_function("Deque::push_back")],
        )
        violations = generate_docs.validate_manifest_schema(manifest)
        self.assertTrue(any("deprecated_by must resolve" in item for item in violations), violations)


class PublicTypesRenderTests(unittest.TestCase):
    def test_types_are_grouped_by_kind_not_name(self) -> None:
        manifest = _manifest(
            [
                {"name": "String", "kind": "struct", "prelude": True, "doc": "Text."},
                {"name": "Ordering", "kind": "enum", "doc": "Ordering."},
                {"name": "Iterator", "kind": "trait", "methods": ["next"]},
                {"name": "u8", "kind": "primitive"},
            ]
        )
        rendered = generate_docs.render_stdlib_reference(manifest)
        self.assertIn("## Public Types", rendered)
        self.assertIn("### Structs", rendered)
        self.assertIn("### Enums", rendered)
        self.assertIn("### Traits", rendered)
        self.assertIn("### Primitives", rendered)
        self.assertLess(rendered.index("### Structs"), rendered.index("### Enums"))
        self.assertLess(rendered.index("### Enums"), rendered.index("### Traits"))
        self.assertNotIn("## Prelude Types", rendered)

    def test_methods_and_trait_impls_are_rendered_from_the_registry(self) -> None:
        manifest = _manifest(
            [
                {
                    "name": "Deque",
                    "kind": "struct",
                    "methods": ["new", "push_back"],
                    "trait_impls": ["Clone"],
                }
            ]
        )
        rendered = generate_docs.render_stdlib_reference(manifest)
        self.assertIn("### Type Methods and Trait Implementations", rendered)
        self.assertIn("| `Deque` | `new`, `push_back` | `Clone` |", rendered)


if __name__ == "__main__":
    unittest.main()
