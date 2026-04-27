---
Status: done
Created: 2026-03-28
Updated: 2026-03-28
ID: 038
Track: component-model
Depends on: 028, 029, 030, 031
Orchestration class: implementation-ready
---
# WIT 型マッピング全 16 種の component fixture 整備
**Blocks v2 exit**: yes

## Summary

v2 完了条件 §8 #4 「WIT 型マッピング全 16 種のテスト fixture が pass する」を達成するため、
現在 5 種 (s32/bool/f64/s64/multi) のみ存在する component fixture を拡充し、
残り 11 種 (string, list, option, result, record, variant, resource, tuple, flags,
enum(WIT), char) の fixture と compile-pass テストを追加する。

---

## 背景

`tests/fixtures/component/` に現在存在する fixture:

| ファイル | カバーする型 |
|---------|------------|
| `export_add.ark` | s32 |
| `export_i64.ark` | s64 |
| `export_f64.ark` | f64 |
| `export_bool.ark` | bool |
| `multi_export.ark` | s32 (複数 export) |

`roadmap-v2.md` §2 到達目標 #2 では以下 16 種のマッピングを定義している:

> s32, s64, f64, bool, char, string, list, option, result, record, variant,
> resource, tuple, flags, enum(WIT)

残り 11 種の fixture が存在しない。また canonical ABI の string/list lift-lower が
未実装のため、string/list を含む component は `wasm-tools component new` の段階で失敗する。

---

## 受け入れ条件

### Phase 1: compile-only fixture (WIT 型認識・診断のみ)

以下の fixture を `tests/fixtures/component/` に追加し、`component-compile:` エントリとして
`tests/fixtures/manifest.txt` に登録する。各 fixture は `arukellt compile --emit component`
が exit 0 で完了することを確認する (実行時の正確性は Phase 2 で検証)。

| fixture ファイル | カバーする WIT 型 | 実装難度 |
|----------------|----------------|---------|
| `export_char.ark` | char | 低 (i32 として ABI 通過) |
| `export_tuple.ark` | tuple | 低 (複数戻り値フラット化) |
| `export_enum_wit.ark` | enum (WIT, payload なし) | 低 (i32 discriminant) |
| `export_option.ark` | option<s32> | 中 (i32 + i32 の 2-word) |
| `export_result.ark` | result<s32, s32> | 中 (i32 + i32 の 2-word) |
| `export_record.ark` | record (複数フィールド) | 中 (struct フラット化) |
| `export_variant.ark` | variant (payload あり) | 中-高 (discriminant + payload) |
| `export_flags.ark` | flags | 中 (bitmask, E0090 診断可) |
| `export_string.ark` | string | 高 (canonical ABI lift/lower 必要) |
| `export_list.ark` | list<s32> | 高 (canonical ABI lift/lower 必要) |
| `export_resource.ark` | resource | 高 (handle + drop hook) |

### Phase 2: runtime correctness (wasmtime 実行検証)

Phase 1 で追加した fixture のうち、canonical ABI が実装された型について
wasmtime `--invoke` で実際の出力を検証するテストを追加する。
string / list / resource は canonical ABI 実装 (#029) 完了後に対応。

### ドキュメント

- [x] `docs/platform/abi-reference.md` の WIT 型対応表を 16 種全て埋める
- [x] 非対応型 (flags → E0090, resource → phase 2 pending) の診断コードを記載

---

## 実装方針

### 既存の ABI マッピング確認

`crates/ark-wasm/src/component/wit.rs` の現在の型マッピング範囲を確認してから着手する。

### 難度「低」の型から着手

`char`, `tuple`, `enum(WIT)` は scalar または flat representation で実装可能。
ABI 変換ロジックが不要 (または最小限) なため、Phase 1 の起点として適切。

### canonical ABI 未実装型の扱い

`string` / `list<T>` は canonical ABI の lift-lower が未実装 (#029 の残作業)。
これらは fixture を作成した上で `compile-error:` エントリとして登録し、
「非対応 (E0091: component string ABI not yet implemented)」の診断を返すことで
テストを greenにする。実際の動作は canonical ABI 実装後に `component-compile:` に昇格させる。

### flags

`flags` は §12 未解決論点 #4 の通り、v2 では E0090 診断 (`WIT flags: unsupported`) を
返す。fixture は `compile-error:` として登録する。

### resource

`resource` は §5.4 の通り、v2 では i32 handle 方式で実装予定。
実装が間に合わない場合は E0092 診断 (`WIT resource: not yet implemented`) を返す。

---

## 関連ファイル

- `tests/fixtures/component/` — fixture 置き場
- `tests/fixtures/manifest.txt` — `component-compile:` / `compile-error:` エントリ
- `crates/ark-wasm/src/component/wit.rs` — 型マッピング定義
- `crates/ark-wasm/src/component/canonical_abi.rs` — string/list ABI (未実装)
- `docs/platform/abi-reference.md` — WIT 型対応表
- `docs/process/roadmap-v2.md` §8 完了条件 #4

---

## 完了条件

- [x] 11 種の新 fixture が `tests/fixtures/manifest.txt` に登録されている
- [x] `cargo test -p arukellt --test harness` が全 fixture で pass する
  (string/list/resource は `compile-error:` で E009x 診断が正しく出ることを確認)
- [x] `docs/platform/abi-reference.md` の対応表が 16 種全て記載されている
- [x] `scripts/run/verify-harness.sh` の全ゲートが通る

---

## 備考

string/list の canonical ABI 実装は大規模であるため、別 issue として分割することを検討する。
本 issue は「fixture と診断の整備」に集中し、「ABI 変換コードの実装」が必要な場合は
\#039 以降で追跡する。