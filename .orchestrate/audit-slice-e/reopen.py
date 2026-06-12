#!/usr/bin/env python3
"""Batch reopen false-done issues for Slice E audit (2026-06-12). Orchestration-only."""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DONE = ROOT / "issues" / "done"
OPEN = ROOT / "issues" / "open"

REOPENS: dict[int, dict[str, str]] = {
    236: {
        "reason": "CLI LSP stdio contract unmet: `cmd_lsp` reads a script file path, not stdin JSON-RPC; docs/extension advertise bare `arukellt lsp` over stdio.",
        "violated": "LSP stdio usage documented and usable; extension can connect without script-file workaround",
        "evidence": "src/compiler/main/editor.ark (cmd_lsp); src/compiler/main/dispatch.ark; extensions/arukellt-all-in-one/src/extension.js",
    },
    333: {
        "reason": "Project-wide symbol index absent in selfhost LSP after #572 deleted `crates/ark-lsp`.",
        "violated": "workspace/symbol searches indexed project; did_open/change updates index",
        "evidence": "src/compiler/lsp/* (no index); issues/done/572-phase7-delete-ark-lsp.md",
    },
    334: {
        "reason": "Stdlib definition resolution not manifest-driven in selfhost LSP.",
        "violated": "goto_definition/hover resolve stdlib via manifest metadata",
        "evidence": "src/compiler/lsp/completion.ark (hardcoded items); no manifest loader in LSP path",
    },
    335: {
        "reason": "Cross-file goto-definition not implemented in selfhost analysis/LSP.",
        "violated": "Qualified-name and cross-file definition resolution",
        "evidence": "src/compiler/analysis/symbols.ark (single-buffer symbol_at only)",
    },
    336: {
        "reason": "Hover returns AST signature text, not type-inferred hover with stdlib docs.",
        "violated": "Type-inferred hover and stdlib documentation in hover payload",
        "evidence": "src/compiler/lsp/feature_hover.ark; src/compiler/analysis/symbols.ark",
    },
    337: {
        "reason": "Signature help handler and capability missing in selfhost LSP.",
        "violated": "textDocument/signatureHelp for stdlib functions",
        "evidence": "src/compiler/lsp/responses_lifecycle.ark; src/compiler/lsp/dispatch_features.ark",
    },
    338: {
        "reason": "Semantic references / documentHighlight not implemented.",
        "violated": "textDocument/references with symbol-id semantics",
        "evidence": "src/compiler/lsp/dispatch.ark (no references handler)",
    },
    339: {
        "reason": "Semantic rename not implemented in selfhost LSP.",
        "violated": "textDocument/rename and prepareRename",
        "evidence": "src/compiler/lsp/dispatch.ark (no rename handler)",
    },
    340: {
        "reason": "Auto-import from manifest/project not implemented.",
        "violated": "Code actions / completion insert missing imports from manifest",
        "evidence": "src/compiler/lsp/completion.ark (static list only)",
    },
    341: {
        "reason": "Organize imports code action absent.",
        "violated": "source.organizeImports removes unused / sorts imports",
        "evidence": "src/compiler/lsp/dispatch.ark (no codeAction handler)",
    },
    342: {
        "reason": "Context-aware completion absent; flat static completion list only.",
        "violated": "Dot/pattern/type/use-context completion",
        "evidence": "src/compiler/lsp/completion.ark",
    },
    355: {
        "reason": "LSP protocol E2E coverage is partial (2 script fixtures only); acceptance requires broader protocol/error coverage.",
        "violated": "Protocol error handling and handler coverage beyond lifecycle+hover/def",
        "evidence": "tests/fixtures/selfhost/lsp_*.lsp-script (2 files); scripts/check/check-lsp-lifecycle.py",
    },
    450: {
        "reason": "Definition response span covers declaration block, not identifier-only; no name_span in parser AST.",
        "violated": "gotoDefinition range covers identifier token only",
        "evidence": "tests/fixtures/selfhost/lsp_hover_definition.lsp-expected; src/compiler/lsp/feature_symbol.ark",
    },
    451: {
        "reason": "Partial selfhost hover behavior; cited `crates/ark-lsp/tests/lsp_e2e.rs` evidence deleted with #572.",
        "violated": "Semantic-only hover contract with regression tests on selfhost path",
        "evidence": "src/compiler/lsp/feature_hover.ark; absent crates/ark-lsp/",
    },
    452: {
        "reason": "E0100 unresolved-name diagnostic E2E is skipped in extension tests; CLI/LSP parity not fully verified.",
        "violated": "Diagnostics parity including E0100 regression",
        "evidence": "extensions/arukellt-all-in-one/src/test/extension.test.js (test.skip E0100)",
    },
    454: {
        "reason": "Nine LSP snapshot regression tests lived in deleted `crates/ark-lsp`; not ported to selfhost.",
        "violated": "Snapshot regression fixtures and runner for LSP responses",
        "evidence": "absent crates/ark-lsp/tests/; only 2 selfhost lsp fixtures",
    },
    463: {
        "reason": "LSP performance smoke tests and `tests/fixtures/lsp_perf/` absent after #572.",
        "violated": "Performance smoke tests with baseline recording",
        "evidence": "no lsp_perf fixtures; no lsp_perf.rs equivalent",
    },
    502: {
        "reason": "Multi-root workspace LSP resolution not implemented in selfhost LSP state.",
        "violated": "Multi-root index, dependency graph walk, cross-package resolution",
        "evidence": "src/compiler/lsp/state_record.ark; tests/package-workspace/ (package fixtures only)",
    },
    566: {
        "reason": "Parser error-node contract (NK_ERROR/NK_MISSING, sync_to_decl_start) not present in modular parser.",
        "violated": "Explicit error AST nodes and sync recovery per acceptance",
        "evidence": "src/compiler/parser/kinds.ark; rg NK_ERROR/NK_MISSING under src/compiler/parser/",
    },
    626: {
        "reason": "Aggregate IDE-ready frontend claim fails while #566 parser error-node acceptance is unmet.",
        "violated": "Full Phase 6/A error-recovery frontend including #566 contract",
        "evidence": "issues/done/566-phase6-partial-ast-recovery.md audit gap; src/compiler/parser/",
    },
    628: {
        "reason": "LSP MVP handlers exist via script replay, but user-visible stdio transport entrypoint is not wired (`cmd_lsp` requires input file).",
        "violated": "stdio transport for JSON-RPC usable by VS Code LanguageClient",
        "evidence": "src/compiler/main/editor.ark; extensions/arukellt-all-in-one/src/extension.js; docs/current-state.md",
    },
    183: {
        "reason": "Epic rollup closed while child issues (#191, #479, legacy LSP nav) remain false-done or unmet.",
        "violated": "All decomposed extension/LSP children truly done with repo proof",
        "evidence": "issues/done/191-*.md; issues/done/479-*.md; selfhost LSP feature gaps",
    },
    184: {
        "reason": "Foundation rollup depends on #189-191; #191 command-graph UI and audit-reopen debt remain.",
        "violated": "Extension foundation children complete with user-visible entrypoints",
        "evidence": "issues/done/191-vscode-setup-doctor-command-graph-and-environment-inspection.md",
    },
    191: {
        "reason": "Command graph is output-channel text dump, not executable graph UI/workflow claimed in acceptance.",
        "violated": "Command graph UI and environment inspection surfaces",
        "evidence": "extensions/arukellt-all-in-one/src/extension.js (showCommandGraph text output)",
    },
    271: {
        "reason": "Extension test runner wired locally but CI workflow does not run extension E2E job.",
        "violated": "CI job runs extension tests on every PR",
        "evidence": ".github/workflows/ci.yml (no extension test job); extensions/arukellt-all-in-one/package.json",
    },
    273: {
        "reason": "Acceptance cites `arukellt:build` / `ark build` task execution; repo tests use compile/fmt-check tasks only.",
        "violated": "Task provider E2E for build task name and argv",
        "evidence": "extensions/arukellt-all-in-one/src/test/extension.test.js (#622 suite differs from #273 acceptance text)",
    },
    462: {
        "reason": "Parent rollup closed on false-done #479; settings behavior claims lack LSP server-side proof.",
        "violated": "Settings change LSP CodeLens/hover/diagnostics/on-save behavior",
        "evidence": "issues/done/479-lsp-config-struct-and-handler-behavior.md; absent crates/ark-lsp/",
    },
    479: {
        "reason": "LspConfig struct and handler behavior cited in `crates/ark-lsp` — crate deleted; no selfhost equivalent.",
        "violated": "All five initializationOptions affect LSP handler behavior with tests",
        "evidence": "absent crates/ark-lsp/; rg LspConfig under src/ (no matches)",
    },
    480: {
        "reason": "README claims five settings control LSP server behaviour; #479 server-side implementation missing.",
        "violated": "README accuracy for settings → LSP behavior mapping",
        "evidence": "extensions/arukellt-all-in-one/README.md; absent LspConfig in src/",
    },
}


def find_file(issue_id: int) -> Path | None:
    pattern = f"{issue_id:03d}-"
    for d in (DONE, OPEN):
        for p in d.glob(f"{pattern}*.md"):
            return p
    return None


def reopen_section(meta: dict[str, str]) -> str:
    lines = [
        "## Reopened by audit — 2026-06-12",
        "",
        f"**Reopen reason:** {meta['reason']}",
        "",
        f"**Violated acceptance:** {meta['violated']}",
        "",
        "**Evidence files:**",
    ]
    for part in meta["evidence"].split("; "):
        lines.append(f"- `{part.strip()}`")
    lines.extend(["", "**Follow-up split issue:** see #634 for stdio LSP/DAP transport where applicable", ""])
    return "\n".join(lines)


def update_frontmatter(text: str) -> str:
    text = re.sub(r"^Status:\s*done\s*$", "Status: open", text, count=1, flags=re.MULTILINE)
    text = re.sub(r"^Updated:\s*[^\n]+\s*$", "Updated: 2026-06-12", text, count=1, flags=re.MULTILINE)
    if "Updated:" not in text[:500]:
        text = re.sub(r"(^ID:\s*\d+\s*$)", r"\1\nUpdated: 2026-06-12", text, count=1, flags=re.MULTILINE)
    return text


def process(issue_id: int, meta: dict[str, str]) -> None:
    src = find_file(issue_id)
    if src is None:
        print(f"WARN: issue #{issue_id} not found", file=sys.stderr)
        return
    if src.parent == OPEN:
        print(f"SKIP: #{issue_id} already in open/")
        return

    text = src.read_text(encoding="utf-8")
    text = update_frontmatter(text)
    section = reopen_section(meta)
    if "## Reopened by audit — 2026-06-12" not in text:
        # Insert after frontmatter closing ---
        m = re.match(r"(---\n.*?\n---\n)", text, flags=re.DOTALL)
        if m:
            insert_at = m.end()
            text = text[:insert_at] + "\n" + section + text[insert_at:]
        else:
            text = section + text

    dest = OPEN / src.name
    dest.write_text(text, encoding="utf-8")
    src.unlink()
    print(f"REOPEN #{issue_id}: {dest.name}")


def main() -> int:
    for iid, meta in sorted(REOPENS.items()):
        process(iid, meta)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
