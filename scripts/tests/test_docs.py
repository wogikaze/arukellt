"""Unit tests for docs_domain checks (stdlib unittest, no pytest)."""

from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path
from unittest.mock import MagicMock, patch

# Ensure scripts/ is on path
_SCRIPTS_DIR = Path(__file__).resolve().parent.parent
if str(_SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(_SCRIPTS_DIR))

from docs_domain.checks import (  # noqa: E402
    run_consistency,
    run_examples,
    run_freshness,
    run_regenerate,
)

_ROOT = Path(__file__).resolve().parent.parent.parent
_ANCHOR_CHECK = _SCRIPTS_DIR / "check" / "check-anchor-fragments.py"
_anchor_spec = importlib.util.spec_from_file_location(
    "check_anchor_fragments", _ANCHOR_CHECK
)
_anchor_mod = importlib.util.module_from_spec(_anchor_spec)
assert _anchor_spec.loader is not None
_anchor_spec.loader.exec_module(_anchor_mod)
gfm_slug = _anchor_mod.gfm_slug
build_anchor_set = _anchor_mod.build_anchor_set


class TestGfmSlug(unittest.TestCase):
    def test_em_dash_heading(self):
        self.assertEqual(
            gfm_slug("Memory Model — GC native"),
            "memory-model--gc-native",
        )

    def test_numbered_section(self):
        self.assertEqual(
            gfm_slug("1. Lexical Structure"),
            "1-lexical-structure",
        )

    def test_strips_non_ascii(self):
        self.assertEqual(
            gfm_slug("Result / Option エラー処理"),
            "result--option-",
        )


class TestAnchorSet(unittest.TestCase):
    def test_explicit_id(self):
        content = '<a id="error-result-type"></a>\n## Error Handling: Result Type\n'
        self.assertIn("error-result-type", build_anchor_set(content))

    def test_duplicate_headings(self):
        content = "## Foo\n\nbody\n\n## Foo\n"
        anchors = build_anchor_set(content)
        self.assertIn("foo", anchors)
        self.assertIn("foo-1", anchors)

    def test_html_comment_stripped_from_heading(self):
        content = "## 1. Lexical Structure <!-- stability: stable -->\n"
        self.assertIn("1-lexical-structure", build_anchor_set(content))

    def test_adr_example_explicit_and_generated(self):
        content = (
            '<a id="memory-model-gc-native"></a>\n'
            "## Memory Model — GC native\n"
        )
        anchors = build_anchor_set(content)
        self.assertIn("memory-model-gc-native", anchors)
        self.assertIn("memory-model--gc-native", anchors)


class TestDryRun(unittest.TestCase):
    def _assert_dry(self, fn, *args, **kwargs):
        rc, out = fn(_ROOT, dry_run=True, *args, **kwargs)
        self.assertEqual(rc, 0)
        self.assertEqual(out, "")

    def test_consistency_dry(self):
        self._assert_dry(run_consistency)

    def test_freshness_dry(self):
        self._assert_dry(run_freshness)

    def test_examples_dry(self):
        self._assert_dry(run_examples)

    def test_regenerate_dry(self):
        self._assert_dry(run_regenerate)

    def test_regenerate_check_only_dry(self):
        self._assert_dry(run_regenerate, check_only=True)


class TestCommands(unittest.TestCase):
    """Verify the correct commands are assembled (no actual subprocess)."""

    def _captured_cmd(self, fn, *args, **kwargs):
        with patch("docs_domain.checks.subprocess.run") as mock_run:
            mock_result = MagicMock()
            mock_result.returncode = 0
            mock_result.stdout = "ok"
            mock_run.return_value = mock_result
            fn(_ROOT, dry_run=False, *args, **kwargs)
            return mock_run.call_args[0][0]

    def test_consistency_cmd(self):
        cmd = self._captured_cmd(run_consistency)
        self.assertEqual(cmd, ["python3", "scripts/check/check-docs-consistency.py"])

    def test_freshness_cmd(self):
        cmd = self._captured_cmd(run_freshness)
        self.assertEqual(cmd, ["python3", "scripts/check/check-docs-freshness.py"])

    def test_examples_cmd(self):
        cmd = self._captured_cmd(run_examples)
        self.assertEqual(cmd, ["python3", "scripts/check/check-doc-examples.py", "docs/"])

    def test_regenerate_cmd_no_check(self):
        cmd = self._captured_cmd(run_regenerate, check_only=False)
        self.assertEqual(cmd, ["python3", "scripts/gen/generate-docs.py"])

    def test_regenerate_cmd_check_only(self):
        cmd = self._captured_cmd(run_regenerate, check_only=True)
        self.assertEqual(cmd, ["python3", "scripts/gen/generate-docs.py", "--check"])


class TestVerificationEvidenceRendering(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        sys.path.insert(0, str(_SCRIPTS_DIR / "gen"))
        spec = importlib.util.spec_from_file_location(
            "generate_docs_review", _SCRIPTS_DIR / "gen" / "generate-docs.py"
        )
        cls.generator = importlib.util.module_from_spec(spec)
        sys.modules[spec.name] = cls.generator
        spec.loader.exec_module(cls.generator)

    def test_current_observation_is_separate_from_historical_release_snapshot(self):
        state = self.generator.load_toml(self.generator.PROJECT_STATE)
        state["verification"]["current_observation"] = {
            "observed_at": "2026-09-17",
            "base_commit": "edc010bb9",
            "working_tree": "uncommitted fixture gate fixes",
            "fixture_command": "python3 scripts/manager.py verify fixtures",
            "compile_validate_passed": 1691,
            "fixture_failed": 0,
            "runtime_executed": 100,
            "runtime_unsupported": 2,
            "quick_command": "python3 scripts/manager.py verify quick",
            "quick_passed": 151,
            "quick_total": 151,
        }
        readme = self.generator.render_readme_status(state, 2871, {"public_functions": []})
        health = self.generator.render_current_state_test_health(state, 2871, [])
        for rendered in (readme, health):
            with self.subTest(rendered=rendered[:40]):
                self.assertIn("1691 compile/validate passed", rendered)
                self.assertIn("100 runtime executed", rendered)
                self.assertIn("151/151", rendered)
                self.assertIn("uncommitted fixture gate fixes", rendered)
                self.assertIn("Historical", rendered)
                self.assertIn("1089", rendered)
                self.assertIn("not exhaustive runtime", rendered)
                self.assertIn("not a release-readiness receipt", rendered)

    def test_absent_current_observation_does_not_invent_current_results(self):
        state = self.generator.load_toml(self.generator.PROJECT_STATE)
        state["verification"].pop("current_observation", None)
        rendered = self.generator.render_readme_status(state, 2871, {"public_functions": []})
        self.assertNotIn("1691 compile/validate passed", rendered)
        self.assertIn("Historical", rendered)
        self.assertIn("166/166", rendered)


if __name__ == "__main__":
    unittest.main()
