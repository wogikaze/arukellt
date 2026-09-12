---
Status: open
Created: 2026-06-17
Updated: 2026-06-17
ID: 681
Track: docs-audit
Depends on: 679
Orchestration class: audit-ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 2
Source: Stdlib manifest reachability audit framework 2026-06-17
Child tracks: 675, 676
---

# 681 — Stdlib manifest reachability contract audit

## Summary

README は「Stdlib manifest-backed public API: 619 functions」と読者に **公開 API 存在**
を約束する。この監査の起票時点では、`docs/capability-surface.md` と
`std/manifest.toml` が `std::host::http` / `sockets` / `udp` を
**not user-reachable** と明記していた一方、compiler/runtime 側に旧 surface が
残っていた。ADR-054 により、その HTTP/TCP/UDP/stream facade、compiler intrinsic、
互換 fixture、Rust host runtime は削除済みである。残る監査対象は、削除済み API を
公開 API として再掲載しないことと、現行の公式 WASI Component boundary の記述を
manifest・reference・cookbook で一致させることである。

## Audit checklist (section 4)

| チェック | 現状 (2026-06-17) | 起票/追跡 |
|----------|-------------------|-----------|
| 旧 manifest 掲載だが selfhost user-reachable でない | http/sockets/udp（ADR-054 で削除済み） | **#675 done** |
| source-backed module docs が reachability を十分伝えない | generated `docs/stdlib/reference.md` に警告バッジあり、一覧弱い | 本 issue |
| `--deny-*` と `--allow-*` / default policy の一致 | 旧 network permission flag は追加せず、公式 Component boundary を使用 | **#675 done** |
| T1/T3 availability が reference に十分 | 要 scoreboard 横断 | 本 issue |
| 旧 network runtime dispatch なし（docs 上） | 旧 `call_host_network.ark` と network facade を削除済み | **#675 done**, **#679** |
| fs/env/process が docs 期待より狭い | `read_dir`/`metadata` stub | **#676** |
| error type が signature と一致 | 要 spot-check gate | 本 issue |

## Acceptance

- [ ] manifest 全 `std::host::*` モジュールの `availability` と runtime dispatch の
      対応表（machine-readable TOML または generated markdown）を公開
- [ ] `scripts/gen/generate-docs.py` が user-reachable ドリフト時に **verify を fail**
      （#675 と連携；本 issue は audit + gate 定義）
- [ ] `docs/stdlib/scoreboard.md` に reachability 列を追加し manifest と同期
- [ ] cookbook / quickstart の host 例が reachability tier とリンク
- [ ] Gate `scripts/check/gate-681-stdlib-reachability-audit.py`
- [ ] `python3 scripts/manager.py verify quick` exits 0

## References

- `std/manifest.toml`
- `docs/capability-surface.md`
- `issues/done/633-host-capability-surface-honesty-vs-selfhost-runtime.md`
- `issues/done/675-host-capability-reachability-flags.md`
