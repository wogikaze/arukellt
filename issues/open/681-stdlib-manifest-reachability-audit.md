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
を約束する。`docs/capability-surface.md` と `std/manifest.toml` は
`std::host::http` / `sockets` / `udp` を **not user-reachable** と明記している。
「API はあるが selfhost から呼べない」モジュールが reference / cookbook で
利用可能に見えるギャップを監査する。

## Audit checklist (section 4)

| チェック | 現状 (2026-06-17) | 起票/追跡 |
|----------|-------------------|-----------|
| manifest 掲載だが selfhost user-reachable でない | http/sockets/udp（#633 方針） | **#675** |
| source-backed module docs が reachability を十分伝えない | generated `docs/stdlib/reference.md` に警告バッジあり、一覧弱い | 本 issue |
| `--deny-*` と `--allow-*` / default policy の一致 | deny-clock/random のみ強い；http/net deny 未実装 | **#675** |
| T1/T3 availability が reference に十分 | 要 scoreboard 横断 | 本 issue |
| host_http runtime dispatch なし（docs 上） | `call_host_network.ark` 存在 vs manifest 文言矛盾 | **#675**, **#679** |
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
- `issues/open/675-host-capability-reachability-flags.md`
