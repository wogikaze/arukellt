#!/usr/bin/env python3
"""Audit the language-v2 compatibility layer for orphan entries (#878).

The compat layer spans four artifacts that must agree:

  1. std/manifest.toml           - deprecated_by pointers + windows + prelude surface
  2. std/prelude.ark             - the actual auto-imported decl surface
  3. src/compiler/lint/deprecated_table.ark - W0009/W0002 live-call warnings
  4. src/compiler/resolver/removed_names.ark - E0100 migration hints for
     names that no longer resolve

An orphan is any of:
  - a deprecated_by target that resolves to no manifest entry, decl,
    or `Type::method` implementation
  - a lint-table entry whose call can never resolve (dead warning)
  - a removed_names entry for a name that still resolves (dormant hint)
  - a manifest-deprecated prelude name with no diagnostic coverage
  - a prelude decl missing from the manifest entirely

Usage: python3 scripts/check/check-compat-orphans.py [--json]
Exit: 0 when the compat layer is consistent, 1 otherwise.
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # python < 3.11
    import tomli as tomllib

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "scripts" / "gen"))

from stdlib_eradication import method_decl_exists  # noqa: E402
MANIFEST = ROOT / "std" / "manifest.toml"
PRELUDE = ROOT / "std" / "prelude.ark"
LINT_TABLE = ROOT / "src" / "compiler" / "lint" / "deprecated_table.ark"
REMOVED_TABLE = ROOT / "src" / "compiler" / "resolver" / "removed_names.ark"
STABLE_TABLE = ROOT / "src" / "compiler" / "resolver" / "prelude_stable.ark"

# Qualified W0009 table keys use the call-site module alias.  Map each alias
# to the std module that owns the called free function.
QUALIFIED_WARN_MODULES = {
    "cmp": "std::core::cmp",
    "io": "std::io",
    "convert": "std::core::convert",
    "hash": "std::core::hash",
    "math": "std::core::math",
    "seq": "std::seq",
}


def manifest_entries(manifest: dict) -> list[dict]:
    out: list[dict] = []
    for section in ("functions", "types", "values", "traits"):
        out.extend(manifest.get(section, []))
    return out


def module_file(module: str) -> Path | None:
    """Map a std::a::b module path to its std/ source file."""
    rel = module.removeprefix("std::").replace("::", "/")
    base = ROOT / "std" / rel
    candidates = [
        base.with_suffix(".ark"),
        base / "mod.ark",
    ]
    for cand in candidates:
        if cand.is_file():
            return cand
    return None


def pub_decls(path: Path) -> set[str]:
    text = path.read_text(encoding="utf-8")
    names = set(re.findall(r"^pub fn ([A-Za-z_][A-Za-z0-9_]*)", text, re.M))
    names |= set(re.findall(r"^pub (?:enum|struct|resource|trait) ([A-Za-z_][A-Za-z0-9_]*)", text, re.M))
    return names


def resolve_deprecated_target(
    target: str,
    names_in_manifest: set[str],
    module_names: set[tuple[str, str]],
) -> bool:
    """Resolve a manifest `deprecated_by` target.

    Bare names resolve against manifest function names. ``std::a::b``
    paths resolve against manifest `(module, name)` pairs or the mapped
    std file. Other qualified targets are `Type::method` references and
    must name a real method declared in an `impl` block under std.
    """
    if "::" not in target:
        return target in names_in_manifest
    module, _, leaf = target.rpartition("::")
    if module.startswith("std::"):
        if (module, leaf) in module_names:
            return True
        source = module_file(module)
        return source is not None and leaf in pub_decls(source)
    return method_decl_exists(ROOT, module, leaf)


def table_names(text: str, fn_name: str) -> list[str]:
    """Extract string literals used in eq(clone(x), "NAME") comparisons of one fn."""
    m = re.search(rf"fn {re.escape(fn_name)}\(.*?(?=\n(?:pub )?fn |\ntest mod |\Z)", text, re.S)
    if not m:
        return []
    body = m.group(0)
    return re.findall(r'eq\(clone\([^)]*\), "([^"]+)"\)', body)


def resolver_builtin_names() -> set[str]:
    names: set[str] = set()
    for path in (ROOT / "src" / "compiler" / "resolver").glob("builtins*.ark"):
        text = path.read_text(encoding="utf-8")
        names |= set(re.findall(r'push\(names, String_from\("([^"]+)"\)\)', text))
        names |= set(re.findall(r'push\(names, "([^"]+)"\)', text))
    return names


def main() -> int:
    manifest = tomllib.loads(MANIFEST.read_text(encoding="utf-8"))
    entries = manifest_entries(manifest)
    prelude_text = PRELUDE.read_text(encoding="utf-8")
    prelude_decls = pub_decls(PRELUDE)
    lint_text = LINT_TABLE.read_text(encoding="utf-8")
    removed_text = REMOVED_TABLE.read_text(encoding="utf-8")
    stable_text = STABLE_TABLE.read_text(encoding="utf-8")
    builtins = resolver_builtin_names()

    use_warn_paths = table_names(lint_text, "deprecated_use_message")
    call_warn_names = table_names(lint_text, "deprecated_call_message")
    qual_warn_paths = []
    for fn in (
        "deprecated_math_qualified_message",
        "deprecated_cmp_qualified_message",
        "deprecated_convert_qualified_message",
        "deprecated_hash_qualified_message",
        "deprecated_seq_qualified_message",
        "deprecated_io_qualified_message",
    ):
        qual_warn_paths += table_names(lint_text, fn)
    # The bare-name table is split into family helpers for readability; the
    # coverage checks must see every entry, not only the dispatcher body.
    for fn in (
        "deprecated_numeric_call_message",
        "deprecated_vec_call_message",
        "deprecated_assert_call_message",
        "deprecated_seq_call_message",
    ):
        call_warn_names += table_names(lint_text, fn)
    removed_names = table_names(removed_text, "removed_name_hint")
    removed_names += table_names(removed_text, "removed_name_specific_hint")
    removed_names += table_names(removed_text, "is_gated_prelude_name")
    stable_names = set(table_names(stable_text, "is_stable_prelude_name"))

    policy = manifest.get("prelude_policy", {})
    if policy.get("legacy_auto_import"):
        auto = prelude_decls
    else:
        auto = set(policy.get("stable_functions", [])) | set(
            policy.get("intrinsic_compatibility_functions", [])
        )
    auto_resolvable = auto | builtins
    # Gated prelude decls are not auto-imported, but an explicit
    # `use std::prelude::{name}` makes the call resolve, so a W0009 entry for
    # them is live. Only the removed-name hints (rule 4) require the stricter
    # auto-resolvable set.
    callable_names = auto_resolvable | prelude_decls
    problems: list[str] = []
    notes: list[str] = []

    # 1. deprecated_by targets must resolve to a real manifest entry or decl.
    names_in_manifest = {e.get("name") for e in entries}
    module_names = {(e.get("module"), e.get("name")) for e in entries if e.get("module")}
    deprecated_entries = [e for e in entries if e.get("deprecated_by")]
    for e in deprecated_entries:
        target = e["deprecated_by"]
        if not resolve_deprecated_target(target, names_in_manifest, module_names):
            problems.append(
                f"orphan deprecated_by: {e.get('name')} -> {target} resolves to nothing"
            )

    # 2. deprecation windows: defaults or per-entry overrides must exist.
    dep_policy = manifest.get("deprecation_policy")
    if not dep_policy or not dep_policy.get("default_since") or not dep_policy.get("default_remove_in"):
        problems.append(
            "deprecation_policy lacks default_since/default_remove_in; "
            "deprecated entries have no removal window"
        )
    for e in deprecated_entries:
        since = e.get("deprecated_since") or (dep_policy or {}).get("default_since")
        remove_in = e.get("remove_in") or (dep_policy or {}).get("default_remove_in")
        if not since or not remove_in:
            problems.append(
                f"deprecated entry {e.get('name')} lacks an effective window"
            )

    # 3. Lint table liveness.
    for path in use_warn_paths:
        if module_file(path) is None:
            problems.append(
                f"orphan W0002 entry: deprecated module path {path} does not resolve"
            )
    for name in call_warn_names:
        if name not in callable_names:
            problems.append(
                f"orphan W0009 entry: {name} cannot be called (not a prelude decl or builtin)"
            )
    for path in qual_warn_paths:
        mod, _, leaf = path.partition("::")
        module = QUALIFIED_WARN_MODULES.get(mod)
        if module is None:
            problems.append(
                f"orphan W0009 qualified entry: {path} has no warning-module mapping"
            )
            continue
        f = module_file(module)
        if f is None or leaf not in pub_decls(f):
            problems.append(
                f"orphan W0009 qualified entry: {path} resolves to no pub decl"
            )

    # 4. removed_names entries must be genuinely unreachable as bare names.
    for name in removed_names:
        if name in auto_resolvable:
            problems.append(
                f"dormant removed_names entry: {name} still resolves as a bare name"
            )

    # 5. Manifest-deprecated prelude names need diagnostic coverage.
    for e in entries:
        if e.get("prelude") and e.get("stability") == "deprecated":
            name = e.get("name")
            live = name in call_warn_names
            gone = name in removed_names
            if not live and not gone:
                problems.append(
                    f"undagnosed deprecation: {name} is deprecated in manifest but "
                    "has no W0009 entry and no removed-name hint"
                )

    # 6. Prelude decls must be recorded in the manifest.
    prelude_manifest_names = {e.get("name") for e in entries if e.get("prelude")}
    for decl in sorted(prelude_decls):
        if decl not in prelude_manifest_names:
            problems.append(
                f"unrecorded prelude decl: {decl} is pub in std/prelude.ark but "
                "absent from manifest prelude surface"
            )

    # 7. Resolver stable table must match the manifest prelude policy.
    # Compiler-builtin stable types (Option/Result/String/Vec) have no prelude
    # declaration to register; only stable types declared in std/prelude.ark
    # (currently `Std`) enter the resolver's bare-name table.
    declared_stable_types = set(policy.get("stable_types", [])) & prelude_decls
    manifest_stable = (
        set(policy.get("stable_functions", []))
        | set(policy.get("intrinsic_compatibility_functions", []))
        | declared_stable_types
    )
    for name in sorted(stable_names - manifest_stable):
        problems.append(
            f"prelude_stable drift: {name} is stable in resolver but not in "
            "manifest prelude_policy"
        )
    for name in sorted(manifest_stable - stable_names):
        problems.append(
            f"prelude_stable drift: {name} is stable in manifest but missing "
            "from resolver::prelude_stable"
        )

    # 8. Every gated prelude decl needs a migration hint.
    gated = prelude_decls - stable_names
    hinted = set(removed_names)
    for decl in sorted(gated - hinted):
        problems.append(
            f"gated decl without hint: {decl} is outside the stable set but "
            "resolver::removed_names gives no migration hint"
        )

    if "--json" in sys.argv:
        print(json.dumps({"problems": problems, "notes": notes}, indent=2))
        return 1 if problems else 0

    for n in notes:
        print(f"note: {n}")
    if problems:
        print(f"compat-orphan audit: {len(problems)} problem(s)")
        for p in problems:
            print(f"  FAIL {p}")
        return 1
    print("compat-orphan audit: clean")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
