"""Regression tests for scripts/check/check-core-ops.py."""
from __future__ import annotations

import copy
import importlib.util
import io
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

try:
    import tomllib  # type: ignore
except ModuleNotFoundError:
    import tomli as tomllib  # type: ignore


SCRIPT = Path(__file__).resolve().parents[1] / "check" / "check-core-ops.py"
SPEC = importlib.util.spec_from_file_location("check_core_ops", SCRIPT)
assert SPEC and SPEC.loader
check_core_ops = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = check_core_ops
SPEC.loader.exec_module(check_core_ops)

CORE_OPS_PATH = Path(__file__).resolve().parents[2] / "data" / "core-ops.toml"
MANIFEST_PATH = Path(__file__).resolve().parents[2] / "std" / "manifest.toml"


class CoreOpsCheckerTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.core_ops_orig = tomllib.loads(CORE_OPS_PATH.read_text(encoding="utf-8"))
        cls.manifest_orig = tomllib.loads(MANIFEST_PATH.read_text(encoding="utf-8"))

    def setUp(self) -> None:
        self.core_ops = copy.deepcopy(self.core_ops_orig)
        self.manifest = copy.deepcopy(self.manifest_orig)

    def _run(self, core_ops=None, manifest=None, status=None, strict=False) -> list[str]:
        errors: list[str] = []
        warnings: list[str] = []
        co = core_ops if core_ops is not None else self.core_ops
        ma = manifest if manifest is not None else self.manifest
        st = status if status is not None else co.get("status", "scaffold")
        check_core_ops.validate_core_ops_and_manifest(co, ma, st, strict, errors, warnings)
        return errors

    def test_pass_scaffold_default(self) -> None:
        self.assertEqual(self._run(), [])

    def test_strict_rejects_placeholders(self) -> None:
        errors = self._run(strict=True)
        self.assertEqual(len(errors), 5)
        self.assertTrue(all("example.invalid" in e for e in errors))

    def test_generic_params_must_be_list(self) -> None:
        self.core_ops["operations"][0]["signature"]["generic_params"] = "T"
        errors = self._run()
        self.assertTrue(any("generic_params" in e and "list" in e for e in errors))

    def test_generic_signature_roundtrip(self) -> None:
        identity = {
            "id": "vec.identity",
            "visibility": "public",
            "classification": {"layer": "semantic_stdlib"},
            "binding": {"policy": "required"},
            "signature": {
                "generic_params": ["T"],
                "inputs": [
                    {"name": "value", "type": {"kind": "ref", "name": "vec", "args": [{"kind": "var", "name": "T"}]}}
                ],
                "outputs": [
                    {"type": {"kind": "ref", "name": "vec", "args": [{"kind": "var", "name": "T"}]}}
                ],
            },
            "lowering": {"kind": "normal_call"},
            "fallback": {"required": True, "implementation_symbol": "example.invalid.fallback.vec_identity"},
            "semantics": {
                "const_evaluable": True, "overflow": "none", "nan": "none", "trap": "none", "equivalence": "exact_bitwise"
            },
            "effect": {
                "memory": "none", "allocates": False, "may_trap": False, "noreturn": False,
                "external_io": False, "nondeterminism": "deterministic", "atomic": False, "volatile": False,
            },
            "inline": {"policy": "hint"},
        }
        self.core_ops["operations"].append(identity)
        for fn in self.manifest["functions"]:
            if fn.get("name") == "starts_with" and fn.get("module") == "std::text":
                fn["core_op_id"] = "vec.identity"
                fn["generic_params"] = ["T"]
                fn["params"] = ["Vec<T>"]
                fn["returns"] = "Vec<T>"
                break
        self.assertEqual(self._run(), [])

    def test_generic_manifest_without_generic_params_fails(self) -> None:
        identity = {
            "id": "vec.identity",
            "visibility": "public",
            "classification": {"layer": "semantic_stdlib"},
            "binding": {"policy": "required"},
            "signature": {
                "generic_params": ["T"],
                "inputs": [
                    {"name": "value", "type": {"kind": "ref", "name": "vec", "args": [{"kind": "var", "name": "T"}]}}
                ],
                "outputs": [
                    {"type": {"kind": "ref", "name": "vec", "args": [{"kind": "var", "name": "T"}]}}
                ],
            },
            "lowering": {"kind": "normal_call"},
            "fallback": {"required": True, "implementation_symbol": "example.invalid.fallback.vec_identity"},
            "semantics": {
                "const_evaluable": True, "overflow": "none", "nan": "none", "trap": "none", "equivalence": "exact_bitwise"
            },
            "effect": {
                "memory": "none", "allocates": False, "may_trap": False, "noreturn": False,
                "external_io": False, "nondeterminism": "deterministic", "atomic": False, "volatile": False,
            },
            "inline": {"policy": "hint"},
        }
        self.core_ops["operations"].append(identity)
        for fn in self.manifest["functions"]:
            if fn.get("name") == "starts_with" and fn.get("module") == "std::text":
                fn["core_op_id"] = "vec.identity"
                fn["params"] = ["Vec<T>"]
                fn["returns"] = "Vec<T>"
                break
        errors = self._run()
        self.assertTrue(any("unknown type identifier" in e for e in errors))

    def test_optional_binding_requires_reason_and_tracking(self) -> None:
        self.core_ops["operations"][0]["binding"] = {"policy": "optional"}
        errors = self._run()
        self.assertTrue(any("reason" in e for e in errors))
        self.assertTrue(any("tracking_issue" in e for e in errors))

    def test_internal_forbidden_with_manifest_ref(self) -> None:
        self.core_ops["operations"][0]["visibility"] = "internal"
        self.core_ops["operations"][0]["binding"] = {"policy": "forbidden"}
        errors = self._run()
        self.assertTrue(any("internal operation must not be referenced" in e for e in errors))

    def test_internal_optional_is_error(self) -> None:
        self.core_ops["operations"][0]["visibility"] = "internal"
        self.core_ops["operations"][0]["binding"] = {"policy": "optional", "reason": "x", "tracking_issue": "#1"}
        errors = self._run()
        self.assertTrue(any("internal operation cannot have binding.policy = optional" in e for e in errors))

    def test_public_forbidden_is_error(self) -> None:
        self.core_ops["operations"][0]["binding"] = {"policy": "forbidden"}
        errors = self._run()
        self.assertTrue(any("public operation cannot have binding.policy = forbidden" in e for e in errors))

    def test_internal_required_is_error(self) -> None:
        self.core_ops["operations"][0]["visibility"] = "internal"
        self.core_ops["operations"][0]["binding"] = {"policy": "required"}
        errors = self._run()
        self.assertTrue(any("internal operation cannot have binding.policy = required" in e for e in errors))

    def test_fallback_public_path_is_error(self) -> None:
        self.core_ops["operations"][0]["fallback"]["implementation_symbol"] = "std::text::starts_with"
        errors = self._run()
        self.assertTrue(any("must not be a public path" in e for e in errors))

    def test_runtime_forbidden_field(self) -> None:
        for op in self.core_ops["operations"]:
            if op["id"] == "panic":
                op["lowering"]["runtime"]["package"] = "foo"
                break
        errors = self._run()
        self.assertTrue(any("must not contain field" in e and "package" in e for e in errors))

    def test_semantics_const_evaluable_must_be_bool(self) -> None:
        self.core_ops["operations"][0]["semantics"]["const_evaluable"] = "false"
        errors = self._run()
        self.assertTrue(any("const_evaluable must be a boolean" in e for e in errors))

    def test_effect_flags_must_be_bool(self) -> None:
        self.core_ops["operations"][0]["effect"]["may_trap"] = 1
        errors = self._run()
        self.assertTrue(any("may_trap must be a boolean" in e for e in errors))

    def test_semantics_enum_values(self) -> None:
        self.core_ops["operations"][0]["semantics"]["overflow"] = "banana"
        self.core_ops["operations"][0]["semantics"]["trap"] = "banana"
        errors = self._run()
        self.assertTrue(any("semantics.overflow" in e for e in errors))
        self.assertTrue(any("semantics.trap" in e for e in errors))

    def test_when_value_invalid(self) -> None:
        self.core_ops["operations"][3]["specializations"][0]["when"]["portable_simd_lowering"] = "ScalarFallback"
        errors = self._run()
        self.assertTrue(any("ScalarFallback" in e or "portable_simd_lowering" in e for e in errors))

    def test_required_capabilities_not_enum(self) -> None:
        self.core_ops["operations"][-1]["lowering"]["target"]["required_capabilities"].append("portable_simd_lowering")
        errors = self._run()
        self.assertTrue(any("portable_simd_lowering" in e and "capability" in e for e in errors))

    def test_unknown_manifest_type_identifier(self) -> None:
        for fn in self.manifest["functions"]:
            if fn.get("core_op_id") == "string.starts_with":
                fn["params"] = ["Banana", "String"]
                break
        errors = self._run()
        self.assertTrue(any("unknown type identifier" in e for e in errors))

    def test_malformed_constraints_not_list(self) -> None:
        self.core_ops["operations"][0]["signature"]["constraints"] = None
        errors = self._run()
        self.assertTrue(any("constraints" in e and "list" in e for e in errors))

    def test_malformed_specializations_not_list(self) -> None:
        self.core_ops["operations"][3]["specializations"] = {"x": "y"}
        errors = self._run()
        self.assertTrue(any("specializations" in e and "list" in e for e in errors))

    def test_malformed_effect_not_dict(self) -> None:
        self.core_ops["operations"][0]["effect"] = []
        errors = self._run()
        self.assertTrue(any("effect" in e and "table" in e for e in errors))

    def test_malformed_signature_not_dict(self) -> None:
        self.core_ops["operations"][0]["signature"] = None
        errors = self._run()
        self.assertTrue(any("signature" in e and "table" in e for e in errors))

    def test_ref_primitive_name_is_error(self) -> None:
        self.core_ops["operations"][0]["signature"]["inputs"][0]["type"] = {"kind": "ref", "name": "i32"}
        errors = self._run()
        self.assertTrue(any("primitive name" in e for e in errors))

    def test_ref_arity_mismatch(self) -> None:
        self.core_ops["operations"][0]["signature"]["inputs"][0]["type"] = {"kind": "ref", "name": "vec"}
        errors = self._run()
        self.assertTrue(any("generic args" in e for e in errors))

    def test_public_symbol_collision(self) -> None:
        # Add a duplicate prelude::panic entry pointing to a different CoreOpId.
        self.manifest["functions"].append({
            "name": "panic",
            "kind": "prelude_wrapper",
            "prelude": True,
            "core_op_id": "string.starts_with",
            "params": ["String"],
            "returns": "()",
        })
        errors = self._run()
        self.assertTrue(any("public symbol" in e for e in errors))

    def test_specialization_ambiguity(self) -> None:
        spec = copy.deepcopy(self.core_ops["operations"][3]["specializations"][0])
        spec["when"] = {"backend": "wasm"}
        self.core_ops["operations"][3]["specializations"].append(spec)
        errors = self._run()
        self.assertTrue(any("ambiguity" in e for e in errors))

    def test_when_cross_axis_invalid(self) -> None:
        self.core_ops["operations"][3]["specializations"][0]["when"]["target_family"] = "native"
        errors = self._run()
        self.assertTrue(any("target_family" in e and "backend" in e for e in errors))

    def test_required_public_binding_missing(self) -> None:
        for fn in self.manifest["functions"]:
            if fn.get("core_op_id") == "string.starts_with":
                fn["core_op_id"] = "string.starts_with_REMOVED"
        errors = self._run()
        self.assertTrue(any("public required binding has no manifest reference" in e for e in errors))

    def test_missing_validation_python(self) -> None:
        self.core_ops["validation"] = {}
        errors = self._run()
        self.assertTrue(any("validation.python" in e for e in errors))

    def test_validation_false(self) -> None:
        self.core_ops["validation"]["python"]["check_signature_compat"] = False
        errors = self._run()
        self.assertTrue(any("validation.python.check_signature_compat must be true" in e for e in errors))

    def test_validation_unknown_key(self) -> None:
        self.core_ops["validation"]["python"]["check_unknown"] = True
        errors = self._run()
        self.assertTrue(any("validation.python.check_unknown" in e for e in errors))

    def test_duplicate_operation_id(self) -> None:
        self.core_ops["operations"].append({"id": "string.starts_with"})
        errors = self._run()
        self.assertTrue(any("duplicated" in e and "operation" in e for e in errors))

    def test_duplicate_type_id(self) -> None:
        self.core_ops["types"].append({"id": "string", "kind": "string"})
        errors = self._run()
        self.assertTrue(any("duplicated" in e and "type" in e for e in errors))

    def test_manifest_type_id_not_found(self) -> None:
        for t in self.manifest["types"]:
            if t.get("name") == "String":
                t["type_id"] = "not_a_type"
                break
        errors = self._run()
        self.assertTrue(any("not_a_type" in e for e in errors))

    def test_manifest_core_op_id_not_found(self) -> None:
        for fn in self.manifest["functions"]:
            if fn.get("core_op_id") == "string.starts_with":
                fn["core_op_id"] = "not_an_op"
                break
        errors = self._run()
        self.assertTrue(any("not_an_op" in e for e in errors))

    def test_normal_call_without_fallback(self) -> None:
        self.core_ops["operations"][0]["lowering"]["kind"] = "normal_call"
        self.core_ops["operations"][0].pop("fallback", None)
        errors = self._run()
        self.assertTrue(any("normal_call requires" in e for e in errors))

    def test_lowering_extraneous_subtable(self) -> None:
        self.core_ops["operations"][2]["lowering"]["target"] = {"target_family": "wasm", "target_id": "panic"}
        errors = self._run()
        self.assertTrue(any("must not contain lowering.target" in e for e in errors))

    def test_unknown_typeexpr_field(self) -> None:
        self.core_ops["operations"][0]["signature"]["inputs"][0]["type"]["extra"] = 1
        errors = self._run()
        self.assertTrue(any("unknown TypeExpr fields" in e for e in errors))

    def test_unknown_constraint_field(self) -> None:
        self.core_ops["operations"][0]["signature"]["constraints"].append({"kind": "trait", "extra": 1})
        errors = self._run()
        self.assertTrue(any("unknown fields" in e for e in errors))

    def test_unknown_semantics_field(self) -> None:
        self.core_ops["operations"][0]["semantics"]["extra"] = 1
        errors = self._run()
        self.assertTrue(any("semantics has unknown fields" in e for e in errors))

    def test_unknown_effect_field(self) -> None:
        self.core_ops["operations"][0]["effect"]["extra"] = 1
        errors = self._run()
        self.assertTrue(any("effect has unknown fields" in e for e in errors))

    def test_unknown_binding_field(self) -> None:
        self.core_ops["operations"][0]["binding"]["extra"] = 1
        errors = self._run()
        self.assertTrue(any("binding has unknown fields" in e for e in errors))

    def test_unknown_classification_field(self) -> None:
        self.core_ops["operations"][0]["classification"]["extra"] = 1
        errors = self._run()
        self.assertTrue(any("classification has unknown fields" in e for e in errors))

    def test_target_unknown_feature(self) -> None:
        self.core_ops["operations"][-1]["lowering"]["target"]["required_target_features"].append("bulk-memory")
        errors = self._run()
        self.assertTrue(any("bulk-memory" in e for e in errors))


class CoreOpsCheckerMainTest(unittest.TestCase):
    @staticmethod
    def _core_ops_text(status: str, placeholder_prefix: str = "example.invalid") -> str:
        text = CORE_OPS_PATH.read_text(encoding="utf-8")
        return text.replace('status = "scaffold"', f'status = "{status}"').replace("example.invalid", placeholder_prefix)

    def _temp_core_ops(self, status: str, placeholder_prefix: str = "example.invalid") -> Path:
        tmp = Path(tempfile.mkdtemp()) / "core-ops.toml"
        tmp.write_text(self._core_ops_text(status, placeholder_prefix), encoding="utf-8")
        return tmp

    def _run_main(self, *args: str, core_ops_path: Path = CORE_OPS_PATH, manifest_path: Path = MANIFEST_PATH) -> int:
        argv = ["check-core-ops.py", *args]
        with mock.patch("sys.argv", argv), \
             mock.patch.object(check_core_ops, "CORE_OPS", core_ops_path), \
             mock.patch.object(check_core_ops, "MANIFEST", manifest_path), \
             mock.patch("sys.stdout", new_callable=io.StringIO), \
             mock.patch("sys.stderr", new_callable=io.StringIO):
            return check_core_ops.main()

    def test_main_default_passes(self) -> None:
        self.assertEqual(self._run_main(), 0)

    def test_main_strict_fails_on_placeholders(self) -> None:
        self.assertEqual(self._run_main("--strict"), 1)

    def test_main_production_status_scaffold_fails(self) -> None:
        tmp = self._temp_core_ops("scaffold", "core.invalid")
        self.assertEqual(self._run_main("--production-structural-readiness", core_ops_path=tmp), 1)

    def test_main_production_status_production_passes(self) -> None:
        tmp = self._temp_core_ops("production", "core.invalid")
        self.assertEqual(self._run_main("--production-structural-readiness", core_ops_path=tmp), 0)


if __name__ == "__main__":
    unittest.main()
