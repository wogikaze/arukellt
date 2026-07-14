from __future__ import annotations

import copy
import importlib.util
import json
from pathlib import Path
import sys
import unittest

try:
    import tomllib
except ModuleNotFoundError:
    import tomli as tomllib


ROOT = Path(__file__).resolve().parents[2]
GENERATOR = ROOT / "scripts/gen/generate-docs.py"
SPEC = importlib.util.spec_from_file_location("generate_docs", GENERATOR)
assert SPEC and SPEC.loader
generate_docs = importlib.util.module_from_spec(SPEC)
sys.path.insert(0, str(GENERATOR.parent))
sys.modules[SPEC.name] = generate_docs
SPEC.loader.exec_module(generate_docs)


class TargetContractTest(unittest.TestCase):
    def setUp(self) -> None:
        self.state = tomllib.loads(
            (ROOT / "docs/data/project-state.toml").read_text(encoding="utf-8")
        )

    def test_alias_contract_matches_adr_007(self) -> None:
        aliases = {entry["input"]: entry for entry in self.state["target_aliases"]}
        expected = {
            "wasm32-wasi-p1": ("wasm32", "wasi-p1"),
            "wasm32-wasi": ("wasm32", "wasi-p1"),
            "wasm32-wasi-p2": ("wasm32-gc", "wasi-p2"),
            "wasm-gc": ("wasm32-gc", "wasi-p2"),
            "wasm-gc-wasi-p2": ("wasm32-gc", "wasi-p2"),
            "wasm32-wasi-p3": ("wasm32-gc", "wasi-p3"),
        }
        for spelling, (target, host) in expected.items():
            self.assertEqual(aliases[spelling]["policy"], "warning")
            self.assertEqual(aliases[spelling]["warning_id"], "W0002")
            self.assertEqual(aliases[spelling]["canonical_target"], target)
            self.assertEqual(aliases[spelling]["host_profile"], host)
        self.assertEqual(aliases["native"]["policy"], "error")
        self.assertEqual(aliases["wasm32-freestanding"]["policy"], "error")

    def test_generated_outputs_are_deterministic_and_current(self) -> None:
        compiler = generate_docs.render_compiler_target_contract(self.state).rstrip() + "\n"
        extension = generate_docs.render_extension_target_contract(self.state).rstrip() + "\n"
        self.assertEqual(
            compiler,
            (ROOT / "src/compiler/main/target_contract_generated.ark").read_text(
                encoding="utf-8"
            ),
        )
        self.assertEqual(
            extension,
            (ROOT / "extensions/arukellt-all-in-one/src/target-contract.generated.js").read_text(
                encoding="utf-8"
            ),
        )
        self.assertEqual(compiler, generate_docs.render_compiler_target_contract(self.state).rstrip() + "\n")

    def test_extension_enum_matches_canonical_targets(self) -> None:
        package = json.loads(
            (ROOT / "extensions/arukellt-all-in-one/package.json").read_text(encoding="utf-8")
        )
        enum = package["contributes"]["configuration"]["properties"]["arukellt.target"]["enum"]
        expected = [entry["id"] for entry in self.state["target_profiles"]] + [None]
        self.assertEqual(enum, expected)

    def test_invalid_alias_schema_is_explained(self) -> None:
        state = copy.deepcopy(self.state)
        state["target_aliases"].append(copy.deepcopy(state["target_aliases"][0]))
        errors = generate_docs.validate_target_contract(state)
        self.assertTrue(any("duplicate target alias input" in error for error in errors))
        state = copy.deepcopy(self.state)
        state["target_aliases"][0]["canonical_target"] = "missing"
        errors = generate_docs.validate_target_contract(state)
        self.assertTrue(any("unknown canonical_target" in error for error in errors))

    def test_operational_compiler_sources_use_only_canonical_target_values(self) -> None:
        allowed = ROOT / "src/compiler/main/target_contract_generated.ark"
        forbidden = ("wasm32-wasi", "wasm32-freestanding", "wasm-gc")
        findings: list[str] = []
        for path in sorted((ROOT / "src/compiler").rglob("*.ark")):
            if path == allowed:
                continue
            text = path.read_text(encoding="utf-8")
            for spelling in forbidden:
                if spelling in text:
                    findings.append(f"{path.relative_to(ROOT)}: {spelling}")
        self.assertEqual(findings, [])


if __name__ == "__main__":
    unittest.main()
