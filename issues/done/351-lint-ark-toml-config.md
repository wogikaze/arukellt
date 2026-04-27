---
Status: done
Created: 2026-03-31
Updated: 2026-04-01
Track: linter
Orchestration class: implementation-ready
Depends on: 350
Closed: 2026-04-01
ID: 351
Blocks v1 exit: no
Priority: 20
---

# Linter: ark.toml で lint 設定を管理する
- `crates/ark-manifest/src/lib.rs`: "LintConfig struct (allow/warn/deny), LintLevel enum, severity_for() method"
- `crates/ark-driver/src/session.rs`: "lint_allow/lint_deny fields, filtering in check()"
- `crates/arukellt/src/commands.rs`: Load lint config from ark.toml in cmd_check/cmd_lint
- `docs/ark-toml.md`: "[lint] section documentation"
# Linter: ark.toml で lint 設定を管理する

## Summary

`ark.toml` に `[lint]` セクションを追加し、project ごとに lint rule の severity を設定できるようにする。

## Acceptance

- [x] `ark.toml` で `[lint]` セクションが parse される
- [x] `[lint]` で `allow = ["rule-name"]` / `warn = ["rule-name"]` / `deny = ["rule-name"]` が設定可能
- [x] CLI / LSP が `ark.toml` の lint 設定を尊重する (CLI check/lint で反映済み)
- [x] `docs/ark-toml.md` に `[lint]` セクションが文書化される

## Implementation

- `crates/ark-manifest/src/lib.rs`: LintConfig struct (allow/warn/deny), LintLevel enum, severity_for() method
- `crates/ark-driver/src/session.rs`: lint_allow/lint_deny fields, filtering in check()
- `crates/arukellt/src/commands.rs`: Load lint config from ark.toml in cmd_check/cmd_lint
- `docs/ark-toml.md`: [lint] section documentation
- 3 unit tests for parsing and severity lookup