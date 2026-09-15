"""Tests for the benchmark percentile contract."""

from __future__ import annotations

import unittest
import sys
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parent.parent
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

from util.percentiles import percentile_linear


class PercentileTest(unittest.TestCase):
    def test_ten_sample_p95_uses_linear_interpolation(self) -> None:
        samples = [8915, 9146, 8984, 9077, 8834, 8980, 9010, 8970, 9013, 8887]
        self.assertEqual(percentile_linear(samples, 50.0), 8982.0)
        self.assertEqual(percentile_linear(samples, 95.0), 9114.95)

    def test_empty_and_invalid_inputs(self) -> None:
        self.assertIsNone(percentile_linear([], 95.0))
        with self.assertRaises(ValueError):
            percentile_linear([1.0], 100.1)


if __name__ == "__main__":
    unittest.main()
