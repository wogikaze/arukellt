"""Unit tests for spec.md stability-marker / [[features]] sync (#876).

The checker compares each feature row in
docs/data/language-doc-classifications.toml against the effective
stability label of the matching spec.md section (own marker or nearest
marked ancestor) and against the section title so that renumbered
sections cannot silently reattach labels to a different feature.
"""

from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path

_CHECK = (
    Path(__file__).resolve().parent.parent
    / "check"
    / "check-docs-consistency.py"
)
_spec = importlib.util.spec_from_file_location(
    "check_docs_consistency", _CHECK
)
_mod = importlib.util.module_from_spec(_spec)
assert _spec.loader is not None
_spec.loader.exec_module(_mod)

_SPEC_TEXT = """# Spec

## 1. Lexical Structure <!-- stability: stable -->

### 1.1 Source Encoding

## 2. Type System <!-- stability: stable -->

### 2.8 Let-Generalization (TypeScheme) <!-- stability: provisional -->

### 2.9 Traits and Impl Blocks <!-- stability: provisional -->

## 6. Items <!-- stability: stable -->

### 6.1 Function Definition

#### 6.1.1 Multi-Clause Function Definitions <!-- stability: provisional -->

### 6.4 Trait Definition <!-- stability: provisional -->

## 9. Standard Library API <!-- stability: see individual entries -->

### 9.1 String — Construction & Operations (prelude)
"""


def _write_tree(root: Path, spec_text: str, features: str) -> None:
    spec_dir = root / "docs" / "language"
    data_dir = root / "docs" / "data"
    spec_dir.mkdir(parents=True)
    data_dir.mkdir(parents=True)
    (spec_dir / "spec.md").write_text(spec_text, encoding="utf-8")
    (data_dir / "language-doc-classifications.toml").write_text(
        features, encoding="utf-8"
    )


def _feature_rows(*rows: tuple[str, str, str]) -> str:
    return "".join(
        f'[[features]]\nid = "{sid}"\nname = "{name}"\n'
        f'stability = "{stab}"\n'
        for sid, name, stab in rows
    )


class TestSpecSectionStability(unittest.TestCase):
    def setUp(self) -> None:
        self.effective, self.explicit, self.names = (
            _mod._spec_section_stability(_SPEC_TEXT)
        )

    def test_top_level_marker(self) -> None:
        self.assertEqual(self.effective["1"], "stable")
        self.assertEqual(self.explicit["1"], "stable")

    def test_subsection_inherits_section(self) -> None:
        self.assertEqual(self.effective["1.1"], "stable")
        self.assertNotIn("1.1", self.explicit)

    def test_subsection_explicit_marker(self) -> None:
        self.assertEqual(self.effective["2.8"], "provisional")
        self.assertEqual(self.effective["2.9"], "provisional")

    def test_h4_marker_under_unmarked_h3(self) -> None:
        self.assertEqual(self.effective["6.1"], "stable")
        self.assertEqual(self.effective["6.1.1"], "provisional")

    def test_non_label_marker_is_unmarked(self) -> None:
        self.assertEqual(self.effective["9"], "")
        self.assertNotIn("9", self.explicit)
        self.assertEqual(self.effective["9.1"], "")

    def test_titles_captured_without_comments(self) -> None:
        self.assertEqual(self.names["2.8"], "Let-Generalization (TypeScheme)")
        self.assertEqual(self.names["6.4"], "Trait Definition")


class TestNormFeatureName(unittest.TestCase):
    def test_strips_trailing_parens(self) -> None:
        self.assertEqual(
            _mod._norm_feature_name("Traits and Impl Blocks (v1)"),
            "traits and impl blocks",
        )
        self.assertEqual(
            _mod._norm_feature_name("Let-Generalization (TypeScheme)"),
            "let-generalization",
        )

    def test_case_and_whitespace(self) -> None:
        self.assertEqual(
            _mod._norm_feature_name("  Import   Syntax "),
            "import syntax",
        )


class TestCheckSpecStabilityMarkerSync(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)
        self._old_root = _mod.ROOT
        _mod.ROOT = self.root
        _mod.errors.clear()

    def tearDown(self) -> None:
        _mod.ROOT = self._old_root
        _mod.errors.clear()
        self._tmp.cleanup()

    def _run(self, features: str, spec_text: str = _SPEC_TEXT) -> int:
        _write_tree(self.root, spec_text, features)
        _mod.errors.clear()
        return _mod.check_spec_stability_marker_sync()

    def test_synced_tree_passes(self) -> None:
        rc = self._run(
            _feature_rows(
                ("1", "Lexical Structure", "stable"),
                ("2", "Type System", "stable"),
                ("2.8", "Let-Generalization (TypeScheme)", "provisional"),
                ("2.9", "Traits and Impl Blocks (v1)", "provisional"),
                ("6", "Items", "stable"),
                ("6.1.1", "Multi-Clause Function Definitions", "provisional"),
                ("6.4", "Trait Definition", "provisional"),
            )
        )
        self.assertEqual(rc, 0)
        self.assertEqual(_mod.errors, [])

    def test_label_drift_detected(self) -> None:
        rc = self._run(
            _feature_rows(("2.9", "Traits and Impl Blocks (v1)", "stable"))
        )
        self.assertEqual(rc, 1)
        self.assertIn("2.9", _mod.errors[-1])

    def test_name_drift_detected(self) -> None:
        # Renumbered section: TOML name belongs to a different feature.
        rc = self._run(
            _feature_rows(("2.8", "Traits and Impl Blocks (v1)", "provisional"))
        )
        self.assertEqual(rc, 1)
        self.assertIn("does not match", _mod.errors[-1])

    def test_missing_spec_section_detected(self) -> None:
        rc = self._run(_feature_rows(("42", "No Such Section", "stable")))
        self.assertEqual(rc, 1)
        self.assertIn("no matching spec.md section", _mod.errors[-1])

    def test_explicit_marker_without_row_detected(self) -> None:
        rc = self._run(_feature_rows(("1", "Lexical Structure", "stable")))
        self.assertEqual(rc, 1)
        self.assertIn("no [[features]] row", _mod.errors[-1])


if __name__ == "__main__":
    unittest.main()
