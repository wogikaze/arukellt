# v2: Component Model 完全対応

> **状態**: **完了** (2026-03-28) — ただし jco 対応は upstream 待ち (#037 blocked)

---

## 1. 版の目的

v1 で確立した GC-native T3 バックエンドの上に、Wasm Component Model (canonical ABI) を載せる。`--emit component` フラグを有効化し、WIT 定義による import/export を実用水準で動作させる。外部システム (JavaScript/jco、他言語 Wasm コンポーネント) との相互運用を可能にする。

---

## 2. 到達目標

1. `arukellt compile --emit component foo.ark` が `.component.wasm` を生成する
2. WIT 型 (s32, s64, f64, bool, char, string, list, option, result, record, variant, resource, tuple, flags, enum) が Arukellt 型に正しくマッピングされる
3. wasmtime (Component Model 対応版) で Arukellt component を実行できる
4. jco (JavaScript) で Arukellt component を呼び出せる
5. Calculator sample と Key-Value store sample の 2 つの実用 component が動作する
6. 既存の `--emit wasm` (T3 GC-native) の動作を一切変更しない

---

## 3. 対象範囲

| 対象 | 変更内容 |
|------|---------|
| `crates/ark-wasm/src/component/wit.rs` | WIT 型マッピングの完全定義 |
| `crates/ark-wasm/src/component/canonical_abi.rs` | GC ref ↔ linear memory のリフト/ロワリング |
| `crates/ark-wasm/src/emit/mod.rs` | `EmitKind::Component` の hard error 解除 |
| `crates/ark-driver/src/session.rs` | `compile_wit()` の完全実装 |
| `crates/ark-target/src/lib.rs` | `BackendPlan` に `EmitKind::Component` ルーティング追加 |
| `tests/e2e/` (新規) | Component 相互運用テスト |
| `docs/adr/ADR-008-component-model.md` (新規) | wasm-tools 内製化 vs 外部依存の判断記録 |

---

## 4. 非対象範囲

- async Component (WASI P3): T5 スコープ。v2 では同期 Component のみ。
- `--target t1` (T1): 変更しない。T1 は Component Model 非対応のまま維持。
- `ark-llvm`: 変更しない。
- メソッド構文 (`obj.method()`): v3–v4 スコープ。
- ネストジェネリクス: v3 で評価。v2 では禁止を維持。
- WIT ファイルの自動生成 (WIT first): v2 では Arukellt first (Arukellt ソース → WIT 生成) のみ対象。WIT first (WIT ファイルから Arukellt スタブ生成) は v3 以降で評価。

---

## 5. 主要設計課題

### 5.1 GC ref ↔ canonical ABI の変換コスト

canonical ABI は string を linear memory の utf-8 バイト列として扱う。GC-native の `(ref $string)` (= `(array mut i8)`) から canonical ABI string への変換には:
- GC array の要素を linear memory にコピーする
- 逆方向 (linear memory → GC) は `array.new_data` で対応できる

この変換は component 境界でのみ発生する。component 内部は GC-native のまま。

### 5.2 `wasm-tools component new` の扱い

component wrapping には `wasm-tools` のロジックが必要。選択肢:
- **Option A**: `wasm-tools` の `wasm-component-ld` を外部プロセスとして呼び出す
- **Option B**: `wasm-encoder` + `wasmparser` を使って in-tree で実装する

ADR-008 でどちらを採用するかを判断・記録する。判断基準: in-tree 化のメンテナンスコスト vs 外部ツールの可用性・バージョン管理の複雑さ。

### 5.3 WIT enum と Arukellt enum の差異

WIT の `enum` は variant に payload を持たない純粋な discriminant。Arukellt の `enum` は variant に payload を持てる。対応:
- WIT `enum` → Arukellt の unit variant のみの `enum` (payload なし)
- Arukellt の payload 付き `enum` → WIT の `variant`

### 5.4 resource 型

WIT の `resource` は handle (i32 に近い) として扱われる。Arukellt 側の表現: GC 参照でラップする方式と、`i32` handle + `__resource_drop` hook の方式がある。v2 では後者 (i32 handle) を採用し、GC ラッパーは v3 以降で評価する。

### 5.5 import / export の Arukellt 構文

v2 での設計選択:
- `pub fn` が自動的に WIT export になる (implicit) — 実装が簡単だが粒度制御ができない
- `#[export]` アノテーション (explicit) — 明示的だがパーサー拡張が必要

**判断**: v2 では implicit (`pub fn` → export) で実装し、v3 の API 安定性ルール整備と合わせて explicit アノテーションに移行するかを評価する。この判断を ADR-008 に含める。

---

## 6. 実装タスク

1. **`crates/ark-wasm/src/component/wit.rs` の拡張**  
   - 既存マッピング (s32, s64, f64, bool, char, string, list, option, result, record, variant) に `resource`, `tuple`, `flags`, `enum (WIT)`, `borrow<T>`, `own<T>` を追加。
   - 非対応型 (closure, TypeVar, raw ref) のエラーメッセージを `ark-diagnostics` の診断コード (W5001 等) として定義。

2. **`crates/ark-wasm/src/component/canonical_abi.rs` の実装**  
   - GC `(ref $string)` → canonical ABI string (linear memory utf-8) のリフト関数
   - canonical ABI string → GC `(ref $string)` のロワリング関数 (`array.new_data` を使用)
   - `Vec<T>` ↔ `list<T>` の変換 (GC struct ↔ linear memory list representation)
   - `Result<T, E>` ↔ `result<T, E>` の変換

3. **`crates/ark-wasm/src/emit/mod.rs` の `EmitKind::Component` 解禁**  
   - hard error コードを削除し、`compile_wit()` 経由のルートを有効化。
   - component wrapping ロジックを呼び出す (ADR-008 の判断に従い Option A/B を実装)。

4. **`crates/ark-driver/src/session.rs` の `compile_wit()` 完成**  
   - 現在の stub を完全実装: `Session::compile_wit(src) -> Result<Vec<u8>, Diagnostics>`
   - 出力: `.component.wasm` バイト列

5. **`BackendPlan` の拡張** (`crates/ark-target/src/lib.rs`)  
   - `EmitKind::Component` ルーティングを `BackendPlan::select()` に追加。
   - `--emit component` CLI フラグの有効化 (`crates/arukellt/src/main.rs`)。

6. **e2e テスト追加** (`tests/e2e/`)  
   - Calculator component: WIT import 関数 + Arukellt 計算 + WIT export
   - `wasmtime` CLI で実行する smoke test スクリプト
   - `jco` で JavaScript から呼び出す smoke test

7. **ADR-008 作成** (`docs/adr/ADR-008-component-model.md`)  
   - 判断項目: wasm-tools 内製化 vs 外部依存、`pub fn` implicit export vs explicit アノテーション

8. **`MirModule.type_table` の WIT 情報拡張評価**  
   - WIT 型情報を `type_table` に追加するか、別フィールドとして `MirModule` に持つかを評価。
   - 変更が必要な場合は ADR-009 を記録する。

9. **Component Model 不干渉テスト**  
   - `--emit wasm` (T3 GC-native) の全 346 fixture が v2 変更後も通ることを確認。
   - T1 の全 fixture も通ることを確認。

---

## 7. 検証方法

```bash
# 既存テストの非破壊確認
cargo test --workspace --exclude ark-llvm
cargo test -p arukellt --test harness -- --nocapture

# Component e2e (新規)
scripts/manager.py  # v2 で追加される component smoke test gate が含まれる

# Component 手動検証
arukellt compile --emit component tests/e2e/calculator/main.ark -o calculator.component.wasm
wasmtime run --invoke run calculator.component.wasm

# jco 検証 (Node.js が必要)
npx jco transpile calculator.component.wasm -o ./out
node tests/e2e/calculator/test.mjs
```

---

## 8. 完了条件

| 条件 | 判定方法 | 結果 |
|------|---------|------|
| `arukellt compile --emit component foo.ark` が 0 exit で `.component.wasm` を生成する | コマンド実行 + exit code | ✅ |
| `wasmtime run --invoke run calculator.component.wasm` が期待出力と一致する | 文字列比較 | ✅ (7 ケース pass) |
| `jco` で呼び出しが成功する (jco smoke test が pass) | exit code | ⚠️ jco が Wasm GC 型非対応のため blocked (#037) |
| WIT 型マッピング全 16 種のテスト fixture が pass する | fixture harness | ⚠️ 5/16 実装済み (#038 で残 11 種追跡中) |
| 既存 fixture (T3 GC-native) が引き続き pass する | 数値確認 | ✅ (379 件全件 pass) |
| T1 の全 fixture が引き続き pass する | fixture harness | ✅ |
| `scripts/manager.py` の全ゲートが通る | exit code 0 | ✅ (17/17 — Check 17 は component interop optional) |
| `ADR-008-component-wrapping.md` が `docs/adr/` に存在する | ファイル存在確認 | ✅ |
| `docs/migration/v1-to-v2.md` が存在する | ファイル存在確認 | ✅ |

**判定**: jco (⚠️) と WIT 16 型全種 (⚠️) は次版以降への持ち越しとし、v2 は **完了** とする。
- jco blocked は外部 upstream 起因 → `issues/blocked/037`
- WIT 残 11 型 → `issues/open/038`

---

## 9. 次版 (v3) への受け渡し

v3 が開始できる前提条件:

1. v2 の全完了条件が達成されていること
2. `--emit component` が実用水準で動作し、Calculator / KV store の 2 サンプルが実行できること
3. WIT 型マッピングが全 16 種定義され、テスト済みであること
4. `pub fn` → WIT export の対応規則が確定していること (v3 の stdlib モジュール境界設計の前提)
5. ADR-008 の判断 (wasm-tools 内製/外部) が記録されていること

**v2 → v3 に渡す成果物**:

| 成果物 | パス |
|--------|------|
| WIT 型マッピング完全定義 | `crates/ark-wasm/src/component/wit.rs` |
| canonical ABI 変換ロジック | `crates/ark-wasm/src/component/canonical_abi.rs` |
| Component e2e テスト | `tests/e2e/` |
| Component Model ADR | `docs/adr/ADR-008-component-model.md` |
| 移行ガイド | `docs/migration/v1-to-v2.md` |

---

## 10. この版で特に気をつけること

1. **GC ref ↔ canonical ABI の変換は component 境界のみ**: 内部ロジックは全て GC-native のまま。canonical ABI 変換コードが内部の emit パスに混入しないよう、`component/canonical_abi.rs` をモジュール境界で隔離する。
2. **async Component を v2 に取り込まない**: WASI P3 の `async` component は T5 スコープ。v2 で async に関わる設計決定をした場合、v5 の設計を拘束するリスクがある。async に関わる WIT 型 (`future<T>`, `stream<T>`) は v2 では非対応エラーとする。
3. **WIT enum と Arukellt enum の混同**: WIT `enum` は discriminant only、Arukellt `enum` は payload 付き variant。変換コード内でこの差異を明示的に処理しないと、WIT validate 時に型不整合が起きる。
4. **`resource` の drop semantics**: WIT `resource` には `[resource-drop]` handle があり、drop 時のフックが必要。GC ランタイムの finalization との統合は複雑。v2 では `resource` の生存期間を明示的に管理する (i32 handle + 明示的 drop) に限定し、GC finalization による暗黙 drop は v3 以降で評価する。
5. **jco の Node.js 依存**: CI で jco テストを実行するには Node.js が必要。CI 環境に Node.js がない場合は jco テストをオプショナルにし、`--with-jco` フラグで有効化する。
6. **`verify-harness.sh` の component gate 追加**: v2 の完了ゲートとして `scripts/manager.py` に `arukellt compile --emit component` の smoke test を追加する。このゲートが追加されることで v2 以降のデグレードを検知できる。
7. **Component Model 仕様の安定性**: Wasm Component Model 仕様は現在 Phase 2 (安定化前)。仕様変更による対応コストを考慮し、wasmtime の特定バージョンに固定して実装する。バージョン固定を `Cargo.toml` に明記する。

---

## 11. この版で必ず残すドキュメント

| ドキュメント | パス | 内容 |
|------------|------|------|
| Component Model ADR | `docs/adr/ADR-008-component-model.md` | wasm-tools 内製化 vs 外部依存、export 構文の判断 |
| WIT 型マッピング仕様 | `docs/platform/abi-reference.md` | Arukellt 型 ↔ WIT 型の対応表 |
| Component 利用ガイド | `docs/platform/wasm-features.md` | --emit component の使い方 |
| v1→v2 移行ガイド | `docs/migration/v1-to-v2.md` | API 変更点 (あれば)、component 利用開始手順 |
| 現状ドキュメント更新 | `docs/current-state.md` | v2 完了状態、Component Model 対応状況 |

---

## 12. 未解決論点 → 持ち越し状況

1. **WIT first の扱い**: 未着手。v3 の stdlib モジュール境界確定後に評価。#054 (std::wit + std::component) で追跡。
2. **`borrow<T>` のセマンティクス**: v2 では `own<T>` と同一視して実装済み。所有権セマンティクスの正確な反映は #054 (v3 Experimental) で評価。
3. **cross-language test**: Calculator ↔ wasmtime は手動検証済み (`tests/component-interop/jco/calculator/run.sh`)。Rust component との相互呼び出しは未実施。#038 完了後に評価。
4. **flags 型**: v2 では非対応エラーのまま。`std::wit` (#054) の BitSet/flags 設計と合わせて v3 Experimental として実装予定。
