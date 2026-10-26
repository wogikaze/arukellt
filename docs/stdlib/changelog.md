# Stdlib API Changelog

> Stdlib の公開 API lifecycle（deprecation / removal / tier change）を記録する手動管理の changelog。
>
> ADR-014 と [ADR-087](../adr/ADR-087-0x-deprecation-exception.md) が要求する changelog
> entry の置き場である。現在の stability と replacement は `std/manifest.toml` と生成された
> `reference.md` / `eradication.md` を正とし、このファイルは変更履歴を持つ。生成物では
> ない。entry は日付順に追記し、過去の entry を書き換えない。

## 記録ルール

- 1 entry = 1 日付 + 1 wave。ADR / issue、対象シンボル、変更、replacement を書く。
- `stable` の削除は [ADR-087](../adr/ADR-087-0x-deprecation-exception.md) の 3 条件
  （W0009、migration guide、この changelog）を満たす。条件を満たさない削除を先に
  記録しない。
- このファイルの過去 entry は削除時点の履歴であり、現在の API 一覧ではない。現在の
  事実は manifest と生成 docs から取得する。

## 2026-10-26 — 0.x deprecation exception (ADR-014 addendum)

- **Policy**: [ADR-087](../adr/ADR-087-0x-deprecation-exception.md) を採択した。
  ADR-014 は維持し、0.x 期に限り `stable` の削除窓を明示的例外で短縮する。
- **Scope**: 0.x 期の `stable` 公開 stdlib エントリのみ。language spec、CLI、ABI、
  target、compiler phase、stdlib 以外の API には適用しない。
- **Conditions**: 削除 wave ごとに (1) W0009 が対象 API に対して実装・発火する、
  (2) `migration-guidance.md` に before/after と到達可能な replacement がある、
  (3) この changelog に対象シンボル・削除理由・replacement を記録する。
- **`remove_in`**: 0.x 例外の下では上限として扱う。`remove_in = "1.0.0"` の
  `stable` エントリを 0.x の minor で削除してよいが、`remove_in` より後には削除しない。
- **1.0.0**: 今は切らない。release-blocking な 3 項目（fixture harness、selfhost CLI
  parity、selfhost diag parity）の再証跡が揃うまでタグを切らない。1.0.0 以降は
  ADR-014 の通常方針に戻り、この例外は失効する。
- **Immediate wave**: `experimental` の `must delete` 2 件、`provisional` の
  `must delete` 21 件（個別判断）、`associated-or-method` 1129 件の replacement 追加。
- **Not in this wave**: `stable` の `must delete` 64 件。特に prelude monomorphic
  seq/combinator の 33 件は W0009 実装（別 lane）まで削除しない。314 件の `stable`
  free→method 置換は replacement の追加として進める。

## 2026-10-26 — #917 core-helper wave (`wave/917-1124`)

- **Wave**: ADR-046 associated-or-method execution, one `std::core` module
  family. No `std::manifest.toml` entry was involved, so the ADR-087 0.x
  exception was not needed.
- **Removed**: 22 public free functions (18 scorecard-counted) — the 12
  `std::core::either` helpers, the 7 `std::core::math` numeric helpers (`gcd`,
  `lcm`, `is_power_of_two`, `next_power_of_two`, `leading_zeros`,
  `trailing_zeros`, `popcount`; `gcd` shares its name with the prelude wrapper
  and is not counted), and the 3 Option-returning `std::core::convert` parse
  adapters (not counted: the prelude `parse_*` entries own those names).
- **Reason**: the replacements already shipped as `Either` enum constructors /
  inherent methods, `i32` inherent `gcd` / `lcm` plus `Integer` trait methods,
  and `TryFrom<String>` associated functions.
- **Replacement**: `migration-guidance.md#removed-in-the-917-core-helper-wave`.
- **Scorecard**: 1246 → 1228 symbols (`associated-or-method` 1124 → 1106,
  unregistered 431 → 413); 1418 → 1396 `pub fn` declarations.
- **Not in this wave**: `stable` manifest entries (still need the ADR-087
  W0009 sequence), the 20 `HashMap_*` builtins, the stable 64 / `std::seq` 33,
  and the deferred provisional items.
