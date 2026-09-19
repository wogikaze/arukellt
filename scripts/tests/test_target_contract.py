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

    def test_cmd_targets_uses_generated_display_lines(self) -> None:
        """cmd_targets() must call target_display_line for each canonical target."""
        targets_ark = (ROOT / "src/compiler/main/targets.ark").read_text(encoding="utf-8")
        canonical = [entry["id"] for entry in self.state["target_profiles"]]
        for target in canonical:
            needle = f'target_contract_generated::target_display_line(String_from("{target}"))'
            self.assertIn(needle, targets_ark, f"cmd_targets() missing generated display for {target}")

    def test_emit_matrix_is_present_for_every_target(self) -> None:
        for profile in self.state["target_profiles"]:
            self.assertIsInstance(profile.get("default_emit_kind"), str)
            self.assertIsInstance(profile.get("allowed_emit_kinds"), list)
            self.assertIn(profile["default_emit_kind"], profile["allowed_emit_kinds"])
        native = next(p for p in self.state["target_profiles"] if p["id"] == "native-cpp")
        self.assertEqual(native["default_emit_kind"], "c")
        self.assertEqual(native["allowed_emit_kinds"], ["c"])
        self.assertFalse(native["run_supported"])
        wasm_gc = next(p for p in self.state["target_profiles"] if p["id"] == "wasm32-gc")
        self.assertNotIn("c", wasm_gc["allowed_emit_kinds"])

    def test_memory_width_and_gc_axes_are_explicit(self) -> None:
        profiles = {profile["id"]: profile for profile in self.state["target_profiles"]}
        self.assertEqual(profiles["wasm32"]["gc_availability"], "linear-memory")
        self.assertEqual(profiles["wasm32"]["default_memory_width"], "memory32")
        self.assertEqual(profiles["wasm32"]["allowed_memory_widths"], ["memory32"])
        self.assertEqual(profiles["wasm32-gc"]["gc_availability"], "wasm-gc")
        self.assertEqual(profiles["wasm32-gc"]["default_memory_width"], "memory64")
        self.assertEqual(profiles["wasm32-gc"]["allowed_memory_widths"], ["memory32", "memory64"])
        self.assertIn("wasm32", next(p for p in self.state["host_profiles"] if p["id"] == "wasi-p1")["supported_targets"])
        self.assertIn("wasm32-gc", next(p for p in self.state["host_profiles"] if p["id"] == "wasi-p2")["supported_targets"])

    def test_target_capability_fixtures_cover_positive_and_diagnostic_paths(self) -> None:
        positive_fixtures = [
            ROOT / "tests/fixtures/target_capability/gc_linear_same_source.ark",
            ROOT / "tests/fixtures/target_capability/wasm32_same_source.ark",
        ]
        diagnostic = ROOT / "tests/fixtures/diagnostics/target_memory_width_unavailable.ark"
        diagnostic_flags = diagnostic.with_suffix(".flags")
        diagnostic_golden = diagnostic.with_suffix(".diag")
        positive_sources = []
        for positive in positive_fixtures:
            positive_flags = positive.with_suffix(".flags")
            positive_expected = positive.with_suffix(".expected")
            self.assertTrue(positive.is_file())
            self.assertTrue(positive_expected.is_file())
            self.assertIn("--memory-width memory32", positive_flags.read_text(encoding="utf-8"))
            positive_sources.append(positive.read_text(encoding="utf-8"))
        self.assertIn("--target wasm32-gc", positive_fixtures[0].with_suffix(".flags").read_text(encoding="utf-8"))
        self.assertIn("--target wasm32", positive_fixtures[1].with_suffix(".flags").read_text(encoding="utf-8"))
        self.assertEqual(positive_sources[0], positive_sources[1])
        self.assertTrue(diagnostic.is_file())
        self.assertIn("--target wasm32", diagnostic_flags.read_text(encoding="utf-8"))
        self.assertIn("--memory-width memory64", diagnostic_flags.read_text(encoding="utf-8"))
        self.assertIn("does not support memory width", diagnostic_golden.read_text(encoding="utf-8"))

    def test_generated_emit_helpers_cover_native_cpp(self) -> None:
        compiler = (ROOT / "src/compiler/main/target_contract_generated.ark").read_text(
            encoding="utf-8"
        )
        self.assertIn("fn target_run_supported(", compiler)
        self.assertIn("fn target_default_emit_kind(", compiler)
        self.assertIn("fn target_allows_emit_kind(", compiler)
        self.assertIn("fn target_gc_availability(", compiler)
        self.assertIn("fn target_default_memory_width(", compiler)
        self.assertIn("fn target_allows_memory_width(", compiler)
        self.assertIn("fn target_allows_host_profile(", compiler)
        self.assertIn('String_from("native-cpp")', compiler)
        self.assertIn('return String_from("c")', compiler)


if __name__ == "__main__":
    unittest.main()
