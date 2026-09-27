#!/usr/bin/env python3
"""#878 codemod: make legacy prelude uses explicit.

After the v2 prelude gate, std::prelude only auto-registers the ADR-072
stable set. This codemod rewrites tracked .ark sources so every other
prelude decl used bare is either:

  * imported explicitly:  use std::prelude::{name, ...}
  * for 'concat' (deprecated): rewritten to 'text::concat(...)' with
    'use std::text'
  * for 'char_at' (deprecated): rewritten to 'byte_at(...)' with
    'use std::prelude::{byte_at}'

Usage:
  python3 scripts/util/migrate-prelude-imports.py          # dry-run report
  python3 scripts/util/migrate-prelude-imports.py --apply  # rewrite files
  python3 scripts/util/migrate-prelude-imports.py --fix-placement --apply
      # hoist use lines that were inserted between a doc comment and its
      # item back into the file's leading use block (CQ-DOC-006 repair)
"""
from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent

STABLE = {
    "panic", "assert", "String_from", "String_new", "char_to_string",
    "len", "push", "pop", "get", "set", "get_unchecked",
}

# Names that are NOT prelude decls: they resolve via builtins / other
# modules, or no longer resolve at all. The codemod must not import them.
NOT_PRELUDE_DECLS = {
    "Option", "Result", "String", "Vec", "HashMap",
    "Some", "None", "Ok", "Err",
    "ok", "err", "expect", "unwrap_or_else", "ok_or",
    "i32_to_string", "i64_to_string", "f64_to_string", "bool_to_string",
    "Vec_new_i32", "Vec_new_String", "Vec_new",
    "Vec_with_capacity_i32", "Vec_with_capacity_String", "Vec_with_capacity",
    "read_to_string", "exists", "get_var", "eprintln",
    "HashMap_new_i32_i32", "HashMap_new_String_String", "HashMap_new_String_i32",
    "HashMap_new_i32_String",
    "HashMap_i32_i32_new",
    "HashMap_i32_i32_get", "HashMap_i32_i32_insert", "HashMap_i32_i32_contains_key",
    "HashMap_i32_i32_len",
    "HashMap_String_String_get", "HashMap_String_String_insert",
    "HashMap_String_String_contains_key", "HashMap_String_String_len",
    "HashMap_String_i32_get", "HashMap_String_i32_insert",
    "HashMap_String_i32_contains_key", "HashMap_String_i32_len",
    "HashMap_i32_String_get", "HashMap_i32_String_insert",
    "HashMap_i32_String_contains_key", "HashMap_i32_String_len",
    "__intrinsic_string_is_empty",
    "map_option_String_String", "map_result_i32_i32",
    "u8", "u16", "u32", "u64", "i8", "i16",
}

# Deprecated names with a non-prelude canonical target.
REWRITE_RULES = {
    "concat": ("std::text", "text::concat"),
    "char_at": ("std::prelude", "byte_at"),
    "println": ("std::host::stdio", "stdio::println"),
    "print": ("std::host::stdio", "stdio::print"),
}


def prelude_decls() -> set[str]:
    text = (ROOT / "std" / "prelude.ark").read_text(encoding="utf-8")
    names = set(re.findall(r"^pub fn ([A-Za-z_][A-Za-z0-9_]*)", text, re.M))
    names |= set(re.findall(r"^pub (?:enum|struct|resource|trait) ([A-Za-z_][A-Za-z0-9_]*)", text, re.M))
    return names


IMPORTABLE = prelude_decls() - STABLE - set(REWRITE_RULES)

DECL_RE = re.compile(
    r"\b(?:pub\s+)?(?:fn|struct|enum|trait|type|const)\s+([A-Za-z_][A-Za-z0-9_]*)"
)

LINE_COMMENT_RE = re.compile(r"//[^\n]*")
STRING_RE = re.compile(r'f?"(?:[^"\\]|\\.)*"')
USE_LINE_RE = re.compile(r"^\s*(?:pub\s+)?use\s+.*$", re.M)
IMPORT_LINE_RE = re.compile(r"^\s*import\s+.*$", re.M)


def _blank(m: re.Match) -> str:
    return " " * (m.end() - m.start())


def strip_noise(text: str) -> str:
    """Mask comments/strings/use-lines with same-length blanks so every
    offset in the masked text maps 1:1 to the original."""
    t = USE_LINE_RE.sub(_blank, text)
    t = IMPORT_LINE_RE.sub(_blank, t)
    t = STRING_RE.sub(_blank, t)
    t = LINE_COMMENT_RE.sub(_blank, t)
    return t


def decl_keywords_before(text: str, pos: int) -> bool:
    seg = text[max(0, pos - 16):pos]
    return bool(re.search(r"\b(?:fn|struct|enum|trait|type|mod|test|const|use|as|impl|for)\s+$", seg))


def bare_uses(scanned: str, name: str) -> list[int]:
    out = []
    for m in re.finditer(rf"(?<![\w:.]){re.escape(name)}\b", scanned):
        rest = scanned[m.end():m.end() + 8]
        if re.match(r"^\s*:", rest):
            continue
        if decl_keywords_before(scanned, m.start()):
            continue
        out.append(m.start())
    return out


def rewrite_call(original: str, name: str, new_callee: str) -> str:
    pattern = re.compile(rf"(?<![\w:.]){re.escape(name)}\s*\(")
    stripped = strip_noise(original)
    keep = []
    for m in pattern.finditer(stripped):
        if decl_keywords_before(stripped, m.start()):
            continue
        keep.append((m.start(), m.end() - 1))
    out = original
    for s, e in reversed(keep):
        out = out[:s] + new_callee + out[e:]
    return out


def existing_use_line(text: str, module: str) -> re.Match | None:
    return re.search(rf"^use {re.escape(module)}\s*$", text, re.M)


def existing_selective(text: str, module: str) -> re.Match | None:
    return re.search(rf"^use {re.escape(module)}\{{([^}}]*)\}}\s*$", text, re.M)


def selective_bound_names(text: str) -> set[str]:
    names = set()
    for m in re.finditer(r"^use [^\n{]*\{([^}]*)\}", text, re.M):
        for part in m.group(1).split(","):
            part = part.strip()
            if part:
                names.add(part)
    return names


def insert_use(text: str, line: str) -> str:
    lines = text.split("\n")
    last_use = -1
    for i, l in enumerate(lines):
        s = l.strip()
        if s.startswith("use ") or s.startswith("import "):
            last_use = i
            continue
        if s != "" and not s.startswith("//"):
            break
    if last_use >= 0:
        idx = last_use + 1
    else:
        idx = len(lines)
        for i, l in enumerate(lines):
            s = l.strip()
            if s == "" or s.startswith("//"):
                continue
            idx = i
            break
    lines.insert(idx, line)
    return "\n".join(lines)


def chunk_use_lines(module: str, names: list[str]) -> list[str]:
    lines = []
    cur_names: list[str] = []
    prefix = f"use {module}::{{"
    for n in names:
        trial = ", ".join(cur_names + [n])
        if len(prefix + trial + "}") > 110 and cur_names:
            lines.append(prefix + ", ".join(cur_names) + "}")
            cur_names = [n]
        else:
            cur_names.append(n)
    if cur_names:
        lines.append(prefix + ", ".join(cur_names) + "}")
    return lines


def process(path: Path) -> tuple[str, dict] | None:
    text = path.read_text(encoding="utf-8")
    scanned = strip_noise(text)
    defined = set(DECL_RE.findall(scanned))
    bound = selective_bound_names(text)

    new_text = text
    needed: set[str] = set()
    text_imports: set[str] = set()

    for name, (mod, callee) in REWRITE_RULES.items():
        if name in defined:
            continue
        if bare_uses(scanned, name):
            new_text = rewrite_call(new_text, name, callee)
            text_imports.add(mod)
            if callee != name and mod == "std::prelude":
                needed.add(callee)
    rescan = strip_noise(new_text)
    for name in sorted(IMPORTABLE):
        if name in defined or name in bound:
            continue
        if bare_uses(rescan, name):
            needed.add(name)

    if not needed and not text_imports:
        return None

    m = existing_selective(new_text, "std::prelude")
    if m:
        have = {p.strip() for p in m.group(1).split(",") if p.strip()}
        merged = sorted(have | needed)
        if needed:
            replacement = "\n".join(chunk_use_lines("std::prelude", merged))
            new_text = new_text[:m.start()] + replacement + new_text[m.end():]
            needed = set()
    if needed:
        for line in chunk_use_lines("std::prelude", sorted(needed)):
            new_text = insert_use(new_text, line)
    for mod in sorted(text_imports):
        if mod == "std::prelude":
            continue
        if existing_use_line(new_text, mod) or existing_selective(new_text, mod):
            continue
        new_text = insert_use(new_text, f"use {mod}")

    return new_text, {"prelude_names": sorted(needed)}


USE_OR_IMPORT = re.compile(r"^(use |import )")


def fix_placement(text: str) -> str:
    """Hoist use/import lines stranded inside the doc-comment region.

    An earlier codemod pass inserted generated use lines immediately
    before the consuming decl, which lands between a /// doc comment and
    its item (CQ-DOC-006).  Move every use/import line that appears after
    the first /// line of the file prefix back into the leading use block
    and merge duplicate selective prelude imports.
    """
    lines = text.split("\n")
    first_item = None
    for i, l in enumerate(lines):
        s = l.strip()
        if s == "" or s.startswith("//") or USE_OR_IMPORT.match(s):
            continue
        first_item = i
        break
    if first_item is None:
        return text
    prefix = lines[:first_item]
    doc_idx = next(
        (i for i, l in enumerate(prefix) if l.strip().startswith("///")), None
    )
    if doc_idx is None:
        return text
    displaced = [
        (i, l)
        for i, l in enumerate(prefix)
        if i > doc_idx and USE_OR_IMPORT.match(l.strip())
    ]
    if not displaced:
        return text
    drop = {i for i, _ in displaced}
    lines = [l for i, l in enumerate(lines) if i not in drop]
    anchor = max(
        (i for i, l in enumerate(lines[:doc_idx]) if USE_OR_IMPORT.match(l.strip())),
        default=None,
    )
    at = anchor + 1 if anchor is not None else doc_idx
    moved = [l for _, l in displaced]
    lines = lines[:at] + moved + lines[at:]
    text = "\n".join(lines)
    return merge_prelude_selectives(text)


def merge_prelude_selectives(text: str) -> str:
    matches = list(existing_selective_iter(text, "std::prelude"))
    if len(matches) <= 1:
        return text
    names: list[str] = []
    for m in matches:
        for part in m.group(1).split(","):
            p = part.strip()
            if p and p not in names:
                names.append(p)
    first, rest = matches[0], matches[1:]
    out = text
    for m in reversed(rest):
        out = out[: m.start()] + out[m.end() :]
    out = out[: first.start()] + "\n".join(
        chunk_use_lines("std::prelude", sorted(names))
    ) + out[first.end() :]
    return out


def existing_selective_iter(text: str, module: str):
    return re.finditer(rf"^use {re.escape(module)}\{{([^}}]*)\}}\s*$", text, re.M)


def main() -> int:
    apply = "--apply" in sys.argv
    only = [
        a
        for a in sys.argv[1:]
        if a not in ("--apply", "--fix-placement")
    ]
    files = subprocess.run(
        ["git", "ls-files", "*.ark"], cwd=ROOT, capture_output=True, text=True
    ).stdout.split()
    if only:
        files = [f for f in files if any(o in f for o in only)]
    fix = "--fix-placement" in sys.argv
    changed = 0
    for rel in files:
        if rel == "std/prelude.ark":
            continue
        p = ROOT / rel
        if fix:
            try:
                old = p.read_text(encoding="utf-8")
                new_text = fix_placement(old)
            except Exception as e:  # noqa: BLE001
                print(f"ERROR {rel}: {e}")
                continue
            if new_text == old:
                continue
            changed += 1
            if apply:
                p.write_text(new_text, encoding="utf-8")
            print(f"{'APPLY' if apply else 'WOULD'} {rel}")
            continue
        try:
            res = process(p)
        except Exception as e:  # noqa: BLE001
            print(f"ERROR {rel}: {e}")
            continue
        if res is None:
            continue
        new_text, meta = res
        if new_text == p.read_text(encoding="utf-8"):
            continue
        changed += 1
        if apply:
            p.write_text(new_text, encoding="utf-8")
        print(f"{'APPLY' if apply else 'WOULD'} {rel} +{meta['prelude_names']}")
    print(f"{'applied' if apply else 'would change'}: {changed} files")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
