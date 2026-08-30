#!/usr/bin/env python3
"""Enable only gc_hint in the bootstrap mir_opt stub for the focused proof gate.

The committed bootstrap overlay intentionally stubs the full mir_opt namespace
because LICM/GC passes from the historical snapshot trap in flat-overlay
selfhost builds. This script makes a deterministic CI-worktree-only edit:
retain stdlib inline and add the current gc_hint pass plus its exact source
closure. No repository source file is silently modified outside the checkout.
"""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CHECKS = ROOT / "scripts" / "selfhost" / "checks.py"

OLD_SOURCES = '''BOOTSTRAP_REQUIRED_MIR_OPT_SOURCES = (
    "mir_opt/stdlib_inline.ark",
    "mir_opt/stdlib_inline_eligibility.ark",
    "mir_opt/stdlib_inline_locals.ark",
    "mir_opt/stdlib_inline_rewrite.ark",
)
'''
NEW_SOURCES = '''BOOTSTRAP_REQUIRED_MIR_OPT_SOURCES = (
    "mir_opt/stdlib_inline.ark",
    "mir_opt/stdlib_inline_eligibility.ark",
    "mir_opt/stdlib_inline_locals.ark",
    "mir_opt/stdlib_inline_rewrite.ark",
    "mir_opt/gc_hint.ark",
    "mir_opt/gc_hint_core.ark",
    "mir_opt/loop_regions.ark",
    "mir_opt/summary_record.ark",
)
'''

OLD_STUB = '''BOOTSTRAP_MIR_OPT_STUB = """// Bootstrap overlay stub — retain only the bounded stdlib pass.
use mir_opt_stdlib_inline

pub fn optimize_module(m: MirModule, opt_level: i32, target: String) -> MirModule {
    if opt_level >= 1 {
        mir_opt_stdlib_inline::stdlib_inline_module(m)
    }
    mir_opt_stdlib_inline::stdlib_resolve_normal_calls(m)
    m
}
"""
'''
NEW_STUB = '''BOOTSTRAP_MIR_OPT_STUB = """// Focused proof overlay — stdlib plus validated gc_hint only.
use mir_module_functions
use mir_opt_gc_hint
use mir_opt_stdlib_inline

pub fn optimize_module(m: MirModule, opt_level: i32, target: String) -> MirModule {
    let _target = target
    if opt_level >= 1 {
        mir_opt_stdlib_inline::stdlib_inline_module(m)
    }
    mir_opt_stdlib_inline::stdlib_resolve_normal_calls(m)
    if opt_level < 2 {
        return m
    }
    let fn_count = mir_module_functions::MirModule_function_count(m)
    let updated = Vec::new<MirFunction>()
    let mut fi = 0
    while fi < fn_count {
        let f = mir_module_functions::MirModule_function_at(m, fi)
        mir_opt_gc_hint::run_gc_hint(f)
        push(updated, f)
        fi = fi + 1
    }
    mir_module_functions::MirModule_set_functions(m, updated)
    m
}
"""
'''


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise ValueError(f"{label}: expected one match, found {count}")
    return text.replace(old, new, 1)


def main() -> int:
    text = CHECKS.read_text(encoding="utf-8")
    if "mir_opt/gc_hint.ark" in text and "mir_opt_gc_hint::run_gc_hint" in text:
        print("enable-gc-hint-overlay: already applied")
        return 0
    text = replace_once(text, OLD_SOURCES, NEW_SOURCES, "required source closure")
    text = replace_once(text, OLD_STUB, NEW_STUB, "mir_opt bootstrap stub")
    CHECKS.write_text(text, encoding="utf-8")
    print("enable-gc-hint-overlay: PASS")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError) as exc:
        print(f"enable-gc-hint-overlay: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
