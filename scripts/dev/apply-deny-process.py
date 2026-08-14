#!/usr/bin/env python3
"""Temporary #676 deny-process migration; removed before PR readiness."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

args = ROOT / "src/compiler/main/args_record.ark"
text = args.read_text(encoding="utf-8")
text = text.replace(
    "fn cli_set_error(opts: CliOptions, error_msg: String) { opts.has_error = true; opts.error_msg = error_msg }",
    "fn cli_set_error(opts: CliOptions, error_msg: String) {\n    opts.has_error = true\n    opts.error_msg = error_msg\n}",
)
args.write_text(text, encoding="utf-8")

core = ROOT / "src/compiler/main/compile_core.ark"
text = core.read_text(encoding="utf-8")
if "fn source_uses_process_intrinsics(" not in text:
    anchor = '''fn source_uses_random_intrinsics(source: String) -> bool {
    contains(clone(source), String_from("random::")) ||
    contains(clone(source), String_from("random_i32")) ||
    contains(source, String_from("std::host::random"))
}
'''
    addition = anchor + '''
fn source_uses_process_intrinsics(source: String) -> bool {
    contains(clone(source), String_from("process::exit")) ||
    contains(clone(source), String_from("process::abort")) ||
    contains(clone(source), String_from("std::host::process")) ||
    contains(clone(source), String_from("__runtime_abi_process_exit")) ||
    contains(source, String_from("__runtime_abi_process_abort"))
}
'''
    if anchor not in text:
        raise SystemExit("compile_core capability helper anchor missing")
    text = text.replace(anchor, addition, 1)
text = text.replace(
    '''    if !args_record::cli_deny_clock(opts) && !args_record::cli_deny_random(opts) {
        return true
    }''',
    '''    if !args_record::cli_deny_clock(opts) && !args_record::cli_deny_random(opts) && !args_record::cli_deny_process(opts) {
        return true
    }''',
    1,
)
old = '''            if args_record::cli_deny_random(opts) && source_uses_random_intrinsics(source) {
                stdio::eprintln(String_from("--deny-random: this program uses random intrinsics"))
                return false
            }
            true'''
new = '''            if args_record::cli_deny_random(opts) && source_uses_random_intrinsics(clone(source)) {
                stdio::eprintln(String_from("--deny-random: this program uses random intrinsics"))
                return false
            }
            if args_record::cli_deny_process(opts) && source_uses_process_intrinsics(source) {
                stdio::eprintln(String_from("--deny-process: this program uses process-control intrinsics"))
                return false
            }
            true'''
if old in text:
    text = text.replace(old, new, 1)
elif "--deny-process: this program uses process-control intrinsics" not in text:
    raise SystemExit("compile_core deny block anchor missing")
core.write_text(text, encoding="utf-8")
