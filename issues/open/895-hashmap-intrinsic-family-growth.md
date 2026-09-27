---
Status: open
Created: 2026-09-27
Updated: 2026-09-27
ID: 895
Parent: 879
Track: language-v2
Depends on: ""
Related: "ADR-058, RFC-011, #514, #687, #879, #852"
Orchestration class: design-ready
Orchestration upstream: 879
Blocks v{N}: language-v2
Priority: 2
Source: "#879 benchmark receipt — bench_stdlib_hash_ops correctness failure"
---

# 895 — HashMap intrinsic ファミリの容量成長（growth/rehash）が存在しない

## Scope

`__hm_si_*` / `__hm_ss_*` / `__hm_is_*` / `__hm_if_*` / `__hm_sf_*` の
5 intrinsic ファミリ（`std::collections::hash_string` / `hash_str_fn` /
`hash_fn` が使う backing store）には成長機構がない。
`emit_hm_*_new_gc` / linear 側の対応 emitter は容量を 16 に固定して
生成し（`src/compiler/wasm/intrinsic_hashmap_str_new.ark`）、
`cap` より多くの insert は `hashmap_str_set` 系の probing が
`probes < cap` を打ち切って `false` を返すだけで静かに drop される。
公開 API の `HashMap::insert`（`hashmap_str_i32_insert` 等の adapter）は
その戻り値を `_stored` として捨て、失敗を呼び出し側に一切通知しない。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## 背景

- #687 で `std::collections::hash`（raw i32→i32 map）には
  growth/reserve/shrink が実装済みだが、`__hm_*` intrinsic 経由の
  String/i32/f64/fn キー・値ファミリは対象外だった。
- #879 の benchmark 計測で再現:
  `benchmarks/bench_stdlib_hash_ops.ark`（100 件 insert）は
  wasm32 / wasm32-gc の両 backend で `len() == 16` までしか入らず、
  正規意味論の期待値 `35050` に対し wasm32 は `1680896` を返す。
  残り 84 件の insert は無通知で失われる。
- 各ファミリの intrinsic は `new/cap/size/get_flag/set_flag/
  get_key/set_key/get_val/set_val/set_size` の 10 個のみで、
  新しい容量で table を再確保する経路がない（`new` に cap 引数もない）。

## Acceptance

- [ ] 全 5 ファミリが初期容量を超える insert で成長し、既存 entry が
      再ハッシュされて lookup/remove が継続して正しい（両 backend）。
- [ ] `HashMap::insert` / adapter 層が容量超過を silent drop しない。
      成長実装までの移行期に失敗を残す場合は `Option`/`Result` で
      呼び出し側に返すか、少なくとも panic する。
- [ ] `bench_stdlib_hash_ops` が両 backend で expected `35050` に一致する。
- [ ] gc / linear で同一意味論（capacity の内部値は問わないが
      観測可能な insert/get/remove/len/keys/values/entries は一致）。

## Required evidence

- implementation commit SHA
- 成長設計の ADR または既存 ADR への参照（backing store の形を
  intrinsic 固定レイアウトのまま拡張するか、Ark 側の Vec/record へ
  移すかは設計判断）
- positive fixture（capacity 超過 insert → 全 entry 存活）+
  必要なら negative/diagnostic fixture
- wasm32 / wasm32-gc 両経路の compile/validate/runtime gate 出力

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #879 の担当項目を evidence 付きで更新する

## Open design questions

- 成長の実装場所: emitter に `__hm_*_grow` / `__hm_*_new_with_cap` を
  追加するか、`__hm_*` backing をやめて Ark 側の Vec/record
  （`std::collections::hash` と同じ構造）に移すか。後者は #687 の
  growth 実装を再利用できるが generic specialization 周りの
  intrinsic 最適化を失う可能性がある。
- `insert` の失敗表示: growth が入れば不要になるが、途中経過で
  silent drop を残さない方針を ADR-015/semantic-debt の観点で確定する。

