"""Tests for the native-cpp selfhost coverage receipt parser."""

from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
COLLECTOR_PATH = ROOT / "scripts/check/collect-native-cpp-coverage.py"


def load_collector():
    spec = importlib.util.spec_from_file_location("native_cpp_coverage", COLLECTOR_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {COLLECTOR_PATH}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


class NativeCppCoverageTests(unittest.TestCase):
    def test_core_ops_use_canonical_registry_order(self) -> None:
        source = """[[legacy_bindings]]
id = "not-an-operation"
[[operations]]
id = "z.last"
[[operations]]
id = "a.first"
"""
        self.assertEqual(load_collector().core_op_runtime_ids(source), ["a.first", "z.last"])

    def test_report_parser_preserves_measured_ids_and_types(self) -> None:
        report = """noise
NATIVE_CPP_COVERAGE_V1
summary|functions|2
summary|instructions|5
opcode|1|3
core_op|7|2
legacy_type|1|4
type_entry|2|1|4|0|i32
host_function|9|1|std::fs::read
unresolved_call|compile|missing::callee
NATIVE_CPP_COVERAGE_END
more noise
"""
        parsed = load_collector().parse_coverage_report(report)
        self.assertEqual(parsed["summary"], {"functions": 2, "instructions": 5})
        self.assertEqual(parsed["opcodes"], {1: 3})
        self.assertEqual(parsed["core_ops"], {7: 2})
        self.assertEqual(parsed["legacy_types"], {1: 4})
        self.assertEqual(
            parsed["types"],
            [{"type_id": 2, "kind": 1, "use_count": 4, "type_parameter_count": 0, "name": "i32"}],
        )
        self.assertEqual(
            parsed["host_functions"],
            [{"function_id": 9, "count": 1, "name": "std::fs::read"}],
        )
        self.assertEqual(
            parsed["unresolved_calls"],
            [{"function": "compile", "callee": "missing::callee"}],
        )

    def test_missing_end_marker_is_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "markers"):
            load_collector().parse_coverage_report("NATIVE_CPP_COVERAGE_V1\nsummary|functions|1\n")

    def test_malformed_line_is_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "malformed"):
            load_collector().parse_coverage_report(
                "NATIVE_CPP_COVERAGE_V1\nopcode|not-an-id|1\nNATIVE_CPP_COVERAGE_END"
            )


if __name__ == "__main__":
    unittest.main()
