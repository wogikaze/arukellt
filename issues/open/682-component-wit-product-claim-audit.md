---
Status: open
Created: 2026-06-17
Updated: 2026-06-17
ID: 682
Track: docs-audit
Depends on: 679, 680
Orchestration class: audit-ready
Orchestration upstream: wasm-tools (CI install policy)
Blocks v{N}: none
Priority: 1
Source: Component/WIT product-claim verification audit 2026-06-17
Child tracks: 668, 671, 673, 674, 677, 030
---

# 682 — Component / WIT product-claim verification audit

## Summary

README と Quickstart は `wasm32-wasi-p2` で WIT/component を **利用可能** と読める。
`target-contract` は component emit を **smoke tier** とし、CI では `wasm-tools` 不在時
**skip-on-CI** と明記。読者期待（「Wasm-first」「Component/WIT target」）と
verify 実態の差分を監査し、#668–#674 の実装 issue と役割分担する。

## Audit checklist (section 5 + 8)

| チェック | 現状 (2026-06-17) | 起票/追跡 |
|----------|-------------------|-----------|
| export ↔ import 対称性 | export fixture 豊富、import per-type matrix 薄い | **#671** |
| `--emit wit/component/all` + `--wit` が実態と一致 | #034/#652–654 done；Quickstart は success fixture 未リンク | **#683** |
| reject だけの型を “supported” と読ませない | stream/future E0402；flags/resource は #651/#473 で supported | 本 issue |
| canonical ABI type matrix ↔ docs ↔ fixtures | `current-state` tier table 詳細；多く E0401 | **#673** |
| jco / JS interop が docs のみ | 103 jco scenarios；#030 open | **#030** |
| wasm-tools compose が build workflow として読めるか | `arukellt compose` あり；deps wasm 未解決 | **#674** |
| ark.toml dependency が未実装に見えないか | WIT vendor #663 done；component wasm 未 | **#674** |
| component fixture CI skip | wasm-tools 不在で skip | 本 issue |
| external tool missing が release OK 扱い | release-checklist に component interop なし | **#678** |
| P2 native が “available” に見えるか | gate_074 あり；target-contract deferred 矛盾 | **#668**, **#680** |

## Acceptance

- [ ] Product claim 文書（README 1段落 + Quickstart Component 節）に **smoke tier** と
      **wasm-tools 必須** を明示
- [ ] CI policy 決定: wasm-tools を CI に入れる **か** README から “CI-guaranteed” 表現を削除
- [ ] Component/WIT claim ↔ manifest fixture kind マップ（`component-compile`, `wit_import`,
      `component-interop/*`）を `docs/process/` に公開
- [ ] `verify quick` に「component claim coverage」サマリ gate（skip 率閾値 or 必須 subset）
- [ ] Gate `scripts/check/gate-682-component-wit-claim-audit.py`
- [ ] `python3 scripts/manager.py verify quick` exits 0

## References

- `docs/target-contract.md` (Component output tier)
- `docs/quickstart.md` (Component Build)
- `issues/done/034-wit-cli-integration.md`
- `issues/open/030-036-jco-javascript-interop.md`
