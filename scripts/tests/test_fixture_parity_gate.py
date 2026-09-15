from __future__ import annotations

import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from selfhost import checks
from selfhost.fixture_parity import (
    _select_smoke_fixtures,
    _worker_count,
    run_fixture_parity,
    run_reference_fixture_parity,
)


class FixtureParityMigrationTests(unittest.TestCase):
    def test_public_checks_entrypoint_is_migrated(self) -> None:
        self.assertIs(checks.run_fixture_parity, run_fixture_parity)
        self.assertIsNot(run_reference_fixture_parity, run_fixture_parity)

    def test_smoke_selection_spans_domains_and_prefers_goldens(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            fixtures_root = root / "tests" / "fixtures"
            fixtures_root.mkdir(parents=True)
            fixtures = [
                "alpha/a.ark",
                "alpha/b.ark",
                "beta/x.ark",
                "gamma/y.ark",
            ]
            for fixture in fixtures:
                path = fixtures_root / fixture
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("fn main() {}\n", encoding="utf-8")
            (fixtures_root / "alpha" / "b.expected").write_text("ok\n", encoding="utf-8")
            (fixtures_root / "gamma" / "y.expected").write_text("ok\n", encoding="utf-8")

            with patch.dict(os.environ, {}, clear=False):
                os.environ.pop("ARUKELLT_FIXTURE_EXECUTE_ALL", None)
                selected = _select_smoke_fixtures(root, fixtures)

            self.assertEqual(selected, {"alpha/b.ark", "beta/x.ark", "gamma/y.ark"})

    def test_execute_all_selects_every_fixture(self) -> None:
        fixtures = ["a/x.ark", "b/y.ark"]
        with tempfile.TemporaryDirectory() as tmp, patch.dict(
            os.environ, {"ARUKELLT_FIXTURE_EXECUTE_ALL": "1"}
        ):
            self.assertEqual(_select_smoke_fixtures(Path(tmp), fixtures), set(fixtures))

    def test_worker_count_honors_environment(self) -> None:
        with patch.dict(os.environ, {"ARUKELLT_FIXTURE_WORKERS": "7"}):
            self.assertEqual(_worker_count(), 7)
        with patch.dict(os.environ, {"ARUKELLT_FIXTURE_WORKERS": "0"}):
            self.assertEqual(_worker_count(), 1)


if __name__ == "__main__":
    unittest.main()
