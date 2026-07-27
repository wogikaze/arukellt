from pathlib import Path
import re
import shutil
import subprocess
import unittest


ROOT = Path(__file__).resolve().parents[2]
FIXTURE = "tests/fixtures/stdlib_inline/abs_probe.ark"


class StdlibInlineIntegrationTests(unittest.TestCase):
    def exported_body(self, wat: str, export_name: str) -> str:
        export = re.search(rf'\(export "{export_name}" \(func (\d+)\)\)', wat)
        self.assertIsNotNone(export)
        function_index = export.group(1)
        body = re.search(rf'\(func \(;{function_index};\)(.*?)(?=\n  \(func|\n\))', wat, re.S)
        self.assertIsNotNone(body)
        return body.group(1)

    def compile_probe(self, opt_level: int) -> Path:
        output = ROOT / ".build/tests" / f"stdlib_inline_abs_o{opt_level}.wasm"
        output.parent.mkdir(parents=True, exist_ok=True)
        result = subprocess.run(
            [
                str(ROOT / "scripts/run/arukellt-selfhost.sh"),
                "compile",
                FIXTURE,
                "--target",
                "wasm32",
                "--opt-level",
                str(opt_level),
                "-o",
                str(output.relative_to(ROOT)),
            ],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        return output

    def test_normal_fallback_and_inlined_body_are_equivalent(self):
        if not (ROOT / ".build/selfhost/arukellt-s2.wasm").is_file():
            self.skipTest("current-source selfhost wasm is not built")
        wasmtime = shutil.which("wasmtime")
        wasm_tools = shutil.which("wasm-tools")
        if wasmtime is None or wasm_tools is None:
            self.skipTest("wasmtime and wasm-tools are required")

        outputs = [self.compile_probe(0), self.compile_probe(1)]
        printed = []
        probes = (
            ("probe_abs", ("-7",), "7"),
            ("probe_min", ("7", "-2"), "-2"),
            ("probe_max", ("7", "-2"), "7"),
            ("probe_clamp", ("12", "0", "9"), "9"),
            ("probe_gcd", ("54", "24"), "6"),
            ("probe_pow", ("3", "4"), "81"),
            ("probe_range_contains", ("5",), "1"),
            ("probe_range_len", (), "5"),
            ("probe_string_starts_with", (), "1"),
            ("probe_string_ends_with", (), "1"),
            ("probe_string_contains", (), "1"),
            ("probe_string_index_of", (), "9"),
            ("probe_string_len", (), "14"),
            ("probe_string_char_at", (), "115"),
            ("probe_string_slice", (), "1"),
            ("probe_string_to_lower", (), "1"),
            ("probe_string_to_upper", (), "1"),
            ("probe_string_trim", (), "1"),
            ("probe_string_trim_start", (), "1"),
            ("probe_string_trim_end", (), "1"),
            ("probe_string_repeat", (), "1"),
            ("probe_string_padding", (), "1"),
            ("probe_string_replace", (), "1"),
            ("probe_string_split", (), "1"),
            ("probe_string_lines", (), "1"),
            ("probe_string_storage_ops", (), "1"),
            ("probe_scalar_format_ops", (), "1"),
            ("probe_string_join", (), "1"),
            ("probe_string_concat", (), "1"),
            ("probe_vec_read_ops", (), "1"),
            ("probe_vec_string_mutation", (), "1"),
            ("probe_seq_read_ops", (), "1"),
            ("probe_seq_allocation_ops", (), "1"),
            ("probe_seq_sort_i32", (), "1"),
        )
        for output in outputs:
            validate = subprocess.run(
                [wasm_tools, "validate", str(output)],
                cwd=ROOT,
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(validate.returncode, 0, validate.stdout + validate.stderr)
            for probe_name, args, expected in probes:
                invoke = subprocess.run(
                    [wasmtime, "run", "--invoke", probe_name, str(output), *args],
                    cwd=ROOT,
                    capture_output=True,
                    text=True,
                    check=False,
                )
                self.assertEqual(invoke.returncode, 0, invoke.stdout + invoke.stderr)
                self.assertEqual(invoke.stdout.strip(), expected)
            wat = subprocess.run(
                [wasm_tools, "print", str(output)],
                cwd=ROOT,
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(wat.returncode, 0, wat.stdout + wat.stderr)
            printed.append(wat.stdout)

        self.assertIn("call ", self.exported_body(printed[0], "probe_abs"))
        self.assertNotIn("call ", self.exported_body(printed[1], "probe_abs"))
        self.assertIn("call ", self.exported_body(printed[1], "probe_gcd"))

    def test_recursion_and_budget_guards_are_explicit(self):
        eligibility = (ROOT / "src/compiler/mir_opt/stdlib_inline_eligibility.ark").read_text(
            encoding="utf-8"
        )
        self.assertIn("instruction_count > stdlib_inline_instruction_budget()", eligibility)
        self.assertIn("instruction_count * 8 > stdlib_inline_code_size_budget()", eligibility)
        self.assertIn("MirFunction_name(caller)", eligibility)
        self.assertIn("MirFunction_name(callee)", eligibility)
        self.assertIn("op == opcodes::MIR_CALL()", eligibility)

    def test_bootstrap_overlay_ships_the_bounded_pass(self):
        checks = (ROOT / "scripts/selfhost/checks.py").read_text(encoding="utf-8")
        self.assertIn('"mir_opt/stdlib_inline.ark"', checks)
        self.assertIn("stdlib_resolve_normal_calls", checks)


if __name__ == "__main__":
    unittest.main()
