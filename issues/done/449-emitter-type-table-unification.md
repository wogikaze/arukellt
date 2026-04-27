---
Status: done
Created: 2026-04-02
Updated: 2026-04-03
ID: 449
Track: compiler
Depends on: none
Orchestration class: implementation-ready
---
# emitter type_table 一本化: T1/T3 両 emitter から checker fallback を除去する
**Blocks v1 exit**: yes
**Priority**: 5

---

## Closed by audit — 2026-04-03

**Reason**: All acceptance criteria verified by repo evidence.

**Evidence**: tests/fixtures/regression/type_table/ fixtures, validate_type_table_consistency() in ark-mir

**Action**: Moved from `issues/open/` → `issues/done/` by false-done audit (confirmed truly-done).

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/449-emitter-type-table-unification.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

`crates/ark-wasm/src/emit/t1/mod.rs` と `crates/ark-wasm/src/emit/t3/mod.rs` に残っている `TODO(MIR-01)` を解消し、型レイアウト情報（struct layouts、enum payload types）を `mir.type_table` 単一ソースから読むようにする。checker fallback を除去することで型情報の source of truth を一本化し、MIR validation を強化する。

---

## 現状の問題

### T1 emitter (`crates/ark-wasm/src/emit/t1/mod.rs`)

- **行 219**: `// TODO(MIR-01): remove checker fallback — enum_payload_types should come from type_table only`
  - 現状: `enum_payload_types: mir.type_table.enum_defs.clone()` が既に type_table から読んでいるが、コメントが残っている。コメントの意味は「以前は checker からフォールバックしていたが今は type_table からのみ」であることを確認する必要がある。実際にコードが type_table を使っているなら TODO コメントを削除し、fallback パスがないことを確認する。
- **行 910**: `// TODO(MIR-01): remove checker fallback — read layouts from type_table only`
  - 同様に確認が必要。

### T3 emitter (`crates/ark-wasm/src/emit/t3/mod.rs`)

- **行 910**: `// TODO(MIR-01): remove checker fallback — read layouts from type_table only`
  - T3 emitter の struct_layouts は `mir.type_table.struct_defs.clone()` から既に読んでいる（確認済み: `let struct_layouts: HashMap<String, Vec<(String, String)>> = mir.type_table.struct_defs.clone()`）が、checker fallback パスが残っているか確認する。

---

## 矛盾と前提

T1 行 219 の実コードを確認すると `enum_payload_types: mir.type_table.enum_defs.clone()` が入っており、既に type_table から読んでいる。しかし行 219 の直前に `// TODO(MIR-01)` が残っている。2 つの解釈がある。

1. **解釈 A**: type_table への移行は完了しているが TODO コメントが残っている → コメント削除 + fallback パスがないことの確認テストを追加する。
2. **解釈 B**: type_table へ読み替えた部分とまだ checker 由来データに頼っている部分が混在している → 混在箇所を特定して全面 type_table 化する。

**採用方針**: 本 issue では両解釈をカバーする。まず全 TODO 箇所の前後コードを精査し、checker 由来データが残っていれば除去し、残っていなければ TODO コメントのみ削除する。どちらの場合も MIR validation テストを追加する。

---

## 詳細実装内容

### Step 1: T1 emitter の全 TODO(MIR-01) 箇所の精査

`crates/ark-wasm/src/emit/t1/mod.rs` を開き、以下を確認する。

1. **行 219 の前後** (`enum_payload_types` 設定箇所):
   - `EmitCtx` の `enum_payload_types` フィールドが `mir.type_table.enum_defs` から設定されているか確認。
   - `EmitCtx` の他のフィールド（`struct_layouts` 等）が type_table 以外のソースから設定されていないか確認。
   - checker 由来のデータが `EmitCtx` コンストラクタ外で注入されていないか確認。
   - 確認後、TODO コメントを削除する（fallback パスがなければ）。

2. **行 910 の前後** (`read layouts from type_table only` 箇所):
   - 型レイアウト読み取りのコード（struct field type の解決など）で checker のデータを参照している箇所を特定。
   - checker fallback を削除し、存在しない場合は type_table から `panic!` または `unreachable!` で失敗させる（fallback でサイレントに進まないようにする）。
   - 削除後、既存のフィールドアクセスで type_table に存在しない型が来た場合に panic ではなく `Err` を返すか、or_else で診断を emit するかを決める。**採用方針**: `DiagnosticSink` に error を追加し、続行不能な場合は空の Wasm バイナリを返す（既存のエラーハンドリングパターンに倣う）。

### Step 2: T3 emitter の全 TODO(MIR-01) 箇所の精査

`crates/ark-wasm/src/emit/t3/mod.rs` の行 910 前後で同様の作業を行う。

T3 emitter は GC-native であるため、struct レイアウトの重要性が高い。特に以下を確認する。

- `struct.new` / `struct.get` / `struct.set` を emit する際に type_table の struct_defs を参照しているか。
- enum の `br_on_cast` dispatch で enum_defs を type_table から読んでいるか。
- 型パラメータを含む generic struct / enum の展開が type_table 由来か。

T3 固有の確認箇所:
- `fn_ret_type_names` の構築が `mir.type_table.fn_sigs` から行われているか確認（行 910 以降のコードから確認済みの部分は維持）。
- `struct_layouts` が `mir.type_table.struct_defs.clone()` から構築されていることを確認（既に確認済みだが明示的に TODO コメントを削除）。

### Step 3: MIR validation 強化 (`crates/ark-mir/src/validate.rs` または `crates/ark-mir/src/`)

checker fallback を除去した後、type_table に存在しない型への参照が MIR 生成段階で早期に検出されるよう validation を強化する。

- `MirModule::type_table` に存在しない struct/enum 名が `MirExpr` に出現した場合に validate エラーを追加する。
- 既存の `validate` 関数（`crates/ark-hir/src/validate.rs` にあるパターン）に倣い、新しい validation pass を追加する。

追加する validation:
1. `type_table` に登録されている全 struct/enum 名を収集する。
2. MIR の全式・全文を走査し、struct/enum 名が (1) のセットに含まれることを確認する。
3. 不整合があれば `DiagnosticCode::W0004` 相当の内部エラーを emit する（新しいコードを割り当てるか、既存の internal error コードを使うか検討）。

### Step 4: 型レイアウト由来の退行テスト追加

以下の fixture を `tests/fixtures/regression/type_table/` 以下に追加する。

| fixture | 内容 | 期待値 |
|---|---|---|
| `struct_field_access.ark` | struct フィールドアクセス（全フィールド型） | 正常 compile + 期待出力 |
| `enum_match_variants.ark` | enum の全 variant match | 正常 compile + 期待出力 |
| `nested_struct.ark` | struct のフィールドが別の struct | 正常 compile + 期待出力 |
| `generic_struct.ark` | 型パラメータ付き struct | 正常 compile + 期待出力（T3 のみ） |

これらは既存の struct/enum fixture と重複する可能性があるので、重複する場合は新規追加不要。ただし **T1 と T3 の両方で実行して type_table 由来の挙動が一致することを確認**すること。

### Step 5: TODO コメントの除去

TODO(MIR-01) を含む全コメントを削除する。削除後、各 TODO が解消された根拠を commit message に記載する（「emit/t1: remove MIR-01 fallback — enum_payload_types now exclusively from type_table」等）。

---

## 依存関係

- Issue 445/446/447/448 とは独立して進行可能。
- MIR / emitter を変更するため、他の emitter 変更 issue と同じ PR に含める場合はコンフリクトに注意する。

---

## 影響範囲

- `crates/ark-wasm/src/emit/t1/mod.rs`
- `crates/ark-wasm/src/emit/t3/mod.rs`
- `crates/ark-mir/src/validate.rs`（または validation モジュール）
- `tests/fixtures/regression/type_table/`（新規）
- `docs/current-state.md`（MIR validation 強化を Diagnostics セクションに追記する場合）

---

## 後方互換性・移行影響

- type_table から正しく読んでいる場合はコメント削除のみで動作変化なし。
- checker fallback が実際に動いていた場合: checker 由来データと type_table データが食い違う型でエラーが出るようになる。これは潜在的バグの早期検出であり、意図的な変化。既存の全 fixture が pass し続けることで regression がないことを確認する。

---

## 今回の範囲外（明確な非対象）

- type_table のスキーマ変更・拡張
- checker の削除
- T2/T4 emitter への同様の変更（T2 は unimplemented、T4 は別クレート）
- MIR 最適化パスの追加

---

## 完了条件

- [x] T1 / T3 emitter に `TODO(MIR-01)` コメントが 0 件
- [x] checker 由来のフォールバックパスが T1/T3 emitter から削除されている（または元から存在しないことが確認されている）
- [x] MIR validation が type_table 整合性チェックを含む
- [x] `tests/fixtures/regression/type_table/` の fixture が T1/T3 で pass
- [x] `bash scripts/run/verify-harness.sh` が 13/13 pass（全 592 fixture が pass のまま）

---

## 必要なテスト

1. 既存の全 struct/enum fixture（T1/T3 両方）が引き続き pass すること — regression gate
2. 新規 `regression/type_table/` fixture 群（Step 4）
3. 型が type_table に存在しないケース（人工的に作成）が validation エラーを出すことの unit test

---

## 実装時の注意点

- T1 の `EmitCtx.enum_payload_types` は `mir.type_table.enum_defs.clone()` から既に設定されているかもしれない。その場合は TODO コメントを削除するだけで十分であるが、型が一致することを unit test で確認する。
- 行番号はコードの編集で変わる可能性があるため、TODO を grep で特定してから作業すること: `grep -n "TODO(MIR-01)" crates/ark-wasm/src/emit/t1/mod.rs crates/ark-wasm/src/emit/t3/mod.rs`
- 型レイアウト情報は struct 名 → `Vec<(field_name, type_name)>` のマップ。type_name が文字列として入っている場合、T3 の GC type 生成で正しくマッピングされるか確認する（特に generic 型の展開）。
- T3 では `fn_ret_type_names` ビルドの後半に checker fallback の名残がある可能性がある（Step 2 の精査で特定）。