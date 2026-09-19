---
Status: done
Created: 2026-09-19
Updated: 2026-09-19
ID: 863
Parent: 852
Track: language-v2
Depends on: "854"
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 24"
---

# 863 — String / Unicode semantics を確定する

## Scope

RFC-011: **24**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [x] String encoding を固定する
- [x] byte index と文字単位 API を分離する
- [x] `char` API の整数 sentinel を廃止または移行する
- [x] invalid UTF/boundary/slicing を規定する
- [x] ASCII/BMP/supplementary/combining fixture を追加する

## Required evidence

- implementation commit SHA: `e4becb596`
- normative spec / ADR/RFC update reference: [ADR-069](../../docs/adr/ADR-069-language-v2-string-unicode-semantics.md), [RFC-011 item 24](../../docs/rfcs/011-language-v2-semantics-redesign.md)
- positive fixture + negative/diagnostic fixture: `stdlib_text/unicode_scalar_semantics.ark`, `diagnostics/unicode_codepoint_option_mismatch.ark/.diag`
- applicable compile/validate/runtime gate output: current-source s2 の `wasm32-gc` component と `wasm32` linear module が compile・`wasm-tools validate`・wasmtime runtime を両方 PASS

## False-done guard

- [x] Acceptance の一部だけで done にしない
- [x] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [x] 親 #852 の担当項目を evidence 付きで更新する

## Completion receipt

- `String` は valid UTF-8、`char` は Unicode scalar とし、byte API (`byte_at`, `slice_bytes`、
  `to_utf8_bytes`) と scalar API (`len_chars`, `codepoint_at`, `chars`, `slice_chars`) を分離した。
- `char_at` は deprecated raw-byte compatibility API として残し、in-tree の parser/compiler/stdlib
  call site は `byte_at` へ移行した。`codepoint_at` は範囲外を integer sentinel ではなく `Option::None`
  で返す。
- strict `try_from_utf8`、lossy `from_utf8`、UTF-8 boundary-aware slicing、canonical UTF-8 の
  `char_to_string` / `push_char` を GC／linear emitter と native compatibility runtime に反映した。
- positive fixture は ASCII `A`、BMP `é`、supplementary `😀`、combining mark `é`、invalid UTF-8、
  byte/scalar boundary、append を検証する。negative fixture は `Option<char>` を `i32` に代入した
  sentinel 回帰を拒否する。
- `python3 scripts/manager.py fmt --check` / lint は変更 200 件を `failed=0`、生成 core-op binding
  と registry は fresh、`python3 scripts/manager.py docs regenerate` は up to date だった。
- 正規 canonical `selfhost build-compiler` は、依存 #854 の既知 bootstrap mismatch
  (`unbox` / `slice_get`) で停止する。これは #863 の source failure ではなく、targeted current-source
  s2 では compiler audit、positive/negative check、GC/linear compile・validate・runtime を確認した。
- `docs check` は freshness・deprecated API・consistency は PASS。doc examples は canonical artifact
  では上記 #854 mismatch、targeted s2 では未変更の ADR-066 example の既存 i32/i64 mismatch が残る。
  #863 はその既存 example や bootstrap workaround を変更せず、未実装を隠す SKIP/allow-list も追加していない。
