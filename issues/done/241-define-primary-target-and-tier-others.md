---
Status: done
Created: 2026-03-30
Updated: 2026-04-01
ID: 241
Track: main
Depends on: 221
Orchestration class: implementation-ready
---
# 本線 target を選定し、experimental/blocked target を明示的に区分する
**Blocks v1 exit**: yes

## Summary

Arukellt は Wasm T1/T3・component model・LLVM scaffold・各種 runtime surface まで視野が広いが、
「安心して使える本線」が明確でない。
この issue では、「今、本気で出荷する target」を 1 本選定し、その他を experimental/blocked として明示的に区分する。

## Acceptance

- [x] 本線 target（primary target）が 1 本、ADR として明記されている
- [x] 本線 target 以外がそれぞれ `experimental` か `blocked` としてラベル付けされている
- [x] 本線 target は CI のすべての品質ゲートを通過している
- [x] experimental target は日常の品質ゲートから切り離されている（CI が別途）

## Scope

### 本線 target の選定

- 現行実装の成熟度・テスト状況・ユーザー価値の観点からの評価
- T1（Wasm MVP）/ T3（Wasm GC）/ component model の現状比較
- ADR として選定理由を文書化

### target tier 定義

- primary：出荷品質を保証。CI 全通過が必須
- experimental：使えるかもしれないが保証なし。CI は別途
- blocked：upstream 待ちなど外部依存で進められない
- scaffold：構造のみ、実装なし

### CI 分離

- 本線 target の CI ゲートと experimental target の CI の分離実装
- blocked target を CI から除外する設定

## References

- `docs/adr/`
- `issues/open/242-ci-layer-structure.md`
- `issues/open/225-document-release-criteria-based-on-guarantees.md`