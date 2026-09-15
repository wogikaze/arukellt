"""Tests for the benchmark percentile contract."""

from __future__ import annotations

import unittest
import sys
import tempfile
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parent.parent
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

from util.percentiles import percentile_linear
from selfhost.measure_overlay_goal import parse_rss_max_kb


class PercentileTest(unittest.TestCase):
    def test_ten_sample_p95_uses_linear_interpolation(self) -> None:
        samples = [8915, 9146, 8984, 9077, 8834, 8980, 9010, 8970, 9013, 8887]
        self.assertEqual(percentile_linear(samples, 50.0), 8982.0)
        self.assertEqual(percentile_linear(samples, 95.0), 9114.95)

    def test_empty_and_invalid_inputs(self) -> None:
        self.assertIsNone(percentile_linear([], 95.0))
        with self.assertRaises(ValueError):
            percentile_linear([1.0], 100.1)

    def test_rss_parser_ignores_gnu_time_failure_prefix(self) -> None:
        with tempfile.TemporaryDirectory() as raw_dir:
            path = Path(raw_dir) / "rss"
            path.write_text("Command exited with non-zero status 134\n1097924\n")
            self.assertEqual(parse_rss_max_kb(path), 1097924)


if __name__ == "__main__":
    unittest.main()
