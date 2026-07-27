"""Tests for the native-cpp capability registry validator."""

from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
VALIDATOR_PATH = ROOT / "scripts/check/check-native-cpp-capabilities.py"


def load_validator():
    spec = importlib.util.spec_from_file_location("native_cpp_capabilities", VALIDATOR_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {VALIDATOR_PATH}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class NativeCppCapabilityValidatorTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp_dir = tempfile.TemporaryDirectory()
        self.root = Path(self.temp_dir.name)
        (self.root / "src/compiler/mir").mkdir(parents=True)
        (self.root / "data").mkdir()
        (self.root / "src/compiler/mir/opcodes.ark").write_text(
            "fn MIR_CONST_I32() -> i32 { 1 }\nfn MIR_RETURN() -> i32 { 42 }\n",
            encoding="utf-8",
        )
        (self.root / "data/core-ops.toml").write_text(
            '[[operations]]\nid = "text.len"\n', encoding="utf-8"
        )

    def tearDown(self) -> None:
        self.temp_dir.cleanup()

    def write_capabilities(self, body: str) -> None:
        (self.root / "data/native-cpp-capabilities.toml").write_text(
            """schema_version = 1
target = "native-cpp"
[status_schema]
allowed = ["supported", "planned", "unsupported"]
supported_requires = ["implementation"]
planned_requires = ["phase", "implementation"]
unsupported_requires = ["reason"]
"""
            + body,
            encoding="utf-8",
        )

    def test_complete_registry_reports_counts(self) -> None:
        self.write_capabilities(
            """
[[mir_opcodes]]
id = "MIR_CONST_I32"
status = "supported"
implementation = "src/compiler/native_c/"
[[mir_opcodes]]
id = "MIR_RETURN"
status = "planned"
phase = 1
implementation = "c_emitter"
[[core_ops]]
id = "text.len"
status = "unsupported"
reason = "outside fixture"
"""
        )
        validator = load_validator()
        summary = validator.validate_repository(self.root)
        self.assertEqual((summary.mir_opcode_count, summary.core_op_count), (2, 1))
        self.assertEqual(summary.status_counts, {"planned": 1, "supported": 1, "unsupported": 1})
        generated = validator.render_generated_ark(
            [
                {"id": "MIR_CONST_I32", "status": "supported", "implementation": "emitter"},
                {"id": "MIR_RETURN", "status": "planned", "phase": 1, "implementation": "emitter"},
            ],
            [{"id": "text.len", "status": "unsupported", "reason": "outside fixture"}],
        )
        self.assertIn("native_c_core_capability_is_supported", generated)

    def test_missing_unknown_and_duplicate_ids_are_rejected(self) -> None:
        self.write_capabilities(
            """
[[mir_opcodes]]
id = "MIR_CONST_I32"
status = "planned"
phase = 1
implementation = "c_emitter"
[[mir_opcodes]]
id = "MIR_CONST_I32"
status = "planned"
phase = 1
implementation = "c_emitter"
[[mir_opcodes]]
id = "MIR_UNKNOWN"
status = "unsupported"
reason = "unknown"
[[core_ops]]
id = "unknown.op"
status = "unsupported"
reason = "unknown"
"""
        )
        with self.assertRaisesRegex(ValueError, "missing MIR opcode"):
            load_validator().validate_repository(self.root)

    def test_status_required_fields_are_enforced(self) -> None:
        cases = [("supported", "implementation"), ("planned", "phase"), ("unsupported", "reason")]
        for status, required in cases:
            with self.subTest(status=status):
                self.write_capabilities(
                    f"""
[[mir_opcodes]]
id = "MIR_CONST_I32"
status = "{status}"
[[mir_opcodes]]
id = "MIR_RETURN"
status = "planned"
phase = 1
implementation = "c_emitter"
[[core_ops]]
id = "text.len"
status = "unsupported"
reason = "outside fixture"
"""
                )
                with self.assertRaisesRegex(ValueError, required):
                    load_validator().validate_repository(self.root)

    def test_unknown_status_is_rejected(self) -> None:
        self.write_capabilities(
            """
[[mir_opcodes]]
id = "MIR_CONST_I32"
status = "scaffold"
[[mir_opcodes]]
id = "MIR_RETURN"
status = "planned"
phase = 1
implementation = "c_emitter"
[[core_ops]]
id = "text.len"
status = "unsupported"
reason = "outside fixture"
"""
        )
        with self.assertRaisesRegex(ValueError, "unknown status"):
            load_validator().validate_repository(self.root)

    def test_registry_schema_is_enforced(self) -> None:
        self.write_capabilities(
            """
[[mir_opcodes]]
id = "MIR_CONST_I32"
status = "supported"
implementation = "src/compiler/native_c/"
[[mir_opcodes]]
id = "MIR_RETURN"
status = "planned"
phase = 1
implementation = "c_emitter"
[[core_ops]]
id = "text.len"
status = "unsupported"
reason = "outside fixture"
"""
        )
        path = self.root / "data/native-cpp-capabilities.toml"
        path.write_text(path.read_text(encoding="utf-8").replace("schema_version = 1", "schema_version = 2"), encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "schema_version must be 1"):
            load_validator().validate_repository(self.root)


if __name__ == "__main__":
    unittest.main()
