---
Status: done
Created: 2026-09-27
Updated: 2026-09-28
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

- [x] 全 5 ファミリが初期容量を超える insert で成長し、既存 entry が
      再ハッシュされて lookup/remove が継続して正しい（両 backend）。
- [x] `HashMap::insert` / adapter 層が容量超過を silent drop しない。
      成長実装までの移行期に失敗を残す場合は `Option`/`Result` で
      呼び出し側に返すか、少なくとも panic する。
- [x] `bench_stdlib_hash_ops` が両 backend で expected に一致する
      （stale な `35050` を算出済みの `35000` へ修正 — 下記参照）。
- [x] gc / linear で同一意味論（capacity の内部値は問わないが
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

- [x] Acceptance の一部だけで done にしない
- [x] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [x] 親 #879 の担当項目を evidence 付きで更新する

## Open design questions

- 成長の実装場所: emitter に `__hm_*_grow` / `__hm_*_new_with_cap` を
  追加するか、`__hm_*` backing をやめて Ark 側の Vec/record
  （`std::collections::hash` と同じ構造）に移すか。後者は #687 の
  growth 実装を再利用できるが generic specialization 周りの
  intrinsic 最適化を失う可能性がある。
- `insert` の失敗表示: growth が入れば不要になるが、途中経過で
  silent drop を残さない方針を ADR-015/semantic-debt の観点で確定する。

## Done (2026-09-28)

Implementation commit: `2f39ba9fa`.

設計（既存 ADR への参照）: backing store は compiler-private intrinsic
固定レイアウトのまま拡張 — ADR-002（memory model: std HashMap 実装）、
ADR-066（generic 公開 API: 変更なし）、ADR-075（compiler-private
runtime intrinsic のみで public source API を追加しない原則と同型）。
成長ポリシー（0.75 load factor・rehash・probing）は全て Ark 側に置き、
両 backend で同一コードが同一意味論を駆動する。

- `__hm_{si,ss,is,if,sf}_new_with_cap(cap)`: donor table の確保。
  wasm32-gc は inline struct emit の cap を arg local 化、linear は
  `hashmap_with_capacity`（#687 の flat Vec<i32> runtime と同一
  layout `[cap, size, keys, vals, flags]`）を call + reachability
  alias。
- `__hm_{si,ss,is,if,sf}_adopt(dst, src)`: donor storage を live map
  へ移譲して map identity を保持（GC: 5 struct field copy、linear:
  Vec header {len, cap, data} 12B copy）。
- 各ファミリ `*_set` を ensure-insert-capacity（0.75 load、
  `hash::hashmap_set` と同ポリシー）+ `*_set_probe` に分割し、
  live entry を donor へ再挿入して adopt。insert は silent drop し得
  ない（失敗経路は OOM のみ）。

Evidence:

- `tests/fixtures/stdlib_hashmap/hashmap_growth.ark`（si 100 inserts
  + remove/reinsert、is/ss 60 inserts）と `hashmap_fn_growth.ark`
  （if/sf 40 inserts + fn call 検証）が wasm32-gc component / wasm32
  linear の両 backend で `growth-ok` / `fn-growth-ok` を出力。
  manifest で run: + t3-run 登録。
- `benchmarks/bench_stdlib_hash_ops`: 両 backend で `35000`。
  `.expected` の `35050` は誤記だったため、bench source から
  算出した正しい checksum（100 len + Σ7·i + 100 contains +
  50×2 removes + 50 len）へ修正。bench コメントも更新。
- `verify lane` PASS（touched-code ratchet 8 files clean）。
- 既知の無関係失敗: `stdlib_hashmap/hashmap_keys_generic` の
  wasm32-gc validate 失敗は 10c661240（#894 前）でも同一 offset で
  再現する既存回帰（T3 gate の pre-existing failure set）。
