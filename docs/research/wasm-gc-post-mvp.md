# WasmGC Post-MVP 機能評価: Arukellt v5 設計調査

**Created**: 2026-03-28
**Issue**: #120
**Status**: Research (v4 では実装しない)

---

## 概要

本ドキュメントは WasmGC Post-MVP 仕様 (`docs/spec/spec-3.0.0/proposals/gc/Post-MVP.md`) に記載された
将来拡張のうち、Arukellt v5 以降に影響する 4 機能を評価する。

| 機能 | Arukellt への関連度 | 推奨アクション |
|------|-------------------|---------------|
| Static fields | 高 | v5 で活用設計 |
| Weak references | 中 | v5 で限定導入 |
| Finalization | 低 | 当面見送り |
| String builtins (stringref) | 高 | v5 で移行検討 |

---

## 1. Static Fields (メタ構造体)

### 仕様の現状

- **ステージ**: Post-MVP 提案 (設計検討段階)
- **概要**: ヒープ値に共有メタ情報 (vtable, タグ等) を付与する仕組み。
  Wasm エンジンが内部的に保持する GC 型ディスクリプタに言語のメタ情報を相乗りさせることで、
  ヒープオブジェクトごとのメタデータ重複を排除する。
- **設計方針**: RTT (Runtime Type Tag) に static data を紐付ける形式が検討されている。
  異なる static data を持つ RTT 同士はキャスト命令では区別不可 (既存の最適化を無効化しない)。

### Arukellt での活用可能性

**関連度: 高**

現在の Arukellt enum 実装はサブタイプ階層 + `br_on_cast` でパターンマッチングを実現している (ADR-002)。
Static fields が利用可能になると:

1. **Enum discriminant の効率化** — 現在は variant ごとに別 struct type を定義しているが、
   static field にタグを格納すれば単一 struct 型 + static tag で表現可能
2. **Trait vtable の実装** — ADR-004 で予定している trait の動的ディスパッチに必要な
   vtable を static field として実現できる。現在は `call_ref` + 関数テーブルで代替中
3. **メモリ効率** — オブジェクトごとのメタデータ word を削減。多数の小オブジェクトで効果大

### 推奨アクション (v5)

- Static fields 仕様が Phase 3 以降に進んだ時点で trait vtable の設計を static fields ベースに移行
- 現在の enum サブタイプ階層は static fields 未使用でも機能するため、フォールバックとして維持
- wasmtime / V8 の実装状況を半期ごとに監視

---

## 2. Weak References (弱参照)

### 仕様の現状

- **ステージ**: Post-MVP 提案 (調査段階、具体的な設計未確定)
- **概要**: 弱参照とファイナライザのためのプリミティブを提供する。
  既存言語間でセマンティクスの差異が大きく、十分に単純かつ効率的なプリミティブセットが
  まだ特定されていない。
- **課題**: 言語ごとに弱参照の挙動が異なる (Java `WeakReference`, Python `weakref`,
  C# `WeakReference<T>` 等)。Wasm レベルで汎用的な抽象を提供する必要がある。

### Arukellt での活用可能性

**関連度: 中**

ADR-002 で以下が明記されている:
- 弱参照 (`Weak<T>`) は「GC の到達可能性に依存」するため当面禁止
- 循環参照グラフのユーザー作成は言語仕様から除外
- wasm32 ターゲットでは RC を使用するため、循環参照は将来 `Weak<T>` で解決予定

Weak references が WasmGC に導入されると:

1. **循環参照の安全な解決** — Observer パターンやキャッシュ等で必要
2. **wasm-gc / wasm32 両ターゲットで統一された `Weak<T>` API** — GC ターゲットは
   Wasm weak ref、wasm32 ターゲットは RC weak ref として lowering
3. **キャッシュ・メモ化の stdlib 実装** — `WeakHashMap<K, V>` 等

### 推奨アクション (v5)

- Wasm weak reference の仕様が確定するまで `Weak<T>` は言語仕様に含めない
- 仕様確定後、`std::weak::Weak<T>` として限定的に導入 (observer, cache ユースケース)
- wasm32 ターゲットへの lowering 設計を同時に行い、API の一貫性を担保

---

## 3. Finalization (ファイナライゼーション)

### 仕様の現状

- **ステージ**: Post-MVP 提案 (Weak references と同セクションで議論)
- **概要**: GC 回収時にユーザー定義のクリーンアップコードを実行する仕組み。
  Weak references と密接に関連し、同時に設計される見込み。
- **課題**: ファイナライザの実行順序・タイミングの保証が言語間で大きく異なる。
  非決定的な実行タイミングはバグの温床であり、仕様での取り扱いが難しい。

### Arukellt での活用可能性

**関連度: 低**

ADR-002 の除外機能リストに「finalizer の実行タイミング保証」が明記されており、
arena ベースの wasm32 ターゲットでは解放タイミングが本質的に不定。

1. **リソース管理** — WASI リソース (file handle, socket) のクリーンアップには有用だが、
   Arukellt では Component Model の `resource` 型と `drop` 関数で対応予定
2. **非決定性のリスク** — ファイナライザの実行タイミングに依存するコードは
   wasm32 ターゲットで動作が変わるため、ターゲット間の互換性を損なう
3. **LLM フレンドリ性** — ファイナライザは暗黙の制御フローを生むため、
   LLM がコードの挙動を予測しにくくなる

### 推奨アクション (v5)

- **Finalization は言語レベルでは導入しない**
- リソースクリーンアップは明示的な `defer` / `with` 構文 (RAII-like) で対応
- Wasm レベルの finalization は stdlib 内部実装でのみ使用を検討 (ユーザーには非公開)

---

## 4. String Builtins Proposal (stringref)

### 仕様の現状

- **ステージ**: 独立提案 (WasmGC Post-MVP とは別の proposal だが密接に関連)
- **概要**: Wasm にネイティブ文字列型 (`stringref`) を導入し、ホスト (JS エンジン等) の
  文字列実装を直接利用可能にする提案。現在は `string builtins` として
  JS API 経由の文字列操作を段階的に導入する approach が進行中。
- **実装状況**:
  - V8: `--experimental-wasm-imported-strings` フラグで実験的サポート
  - Chrome 129+: String builtins が Origin Trial 開始
  - wasmtime: 未実装 (WASI 環境では優先度が低い)

### Arukellt での活用可能性

**関連度: 高**

現在の Arukellt 文字列実装 (ADR-002):
- `(array (mut i8))` — bare GC byte array, UTF-8 エンコーディング
- `array.len` で長さ取得
- 文字列操作は stdlib で手動実装

String builtins が利用可能になると:

1. **ブラウザ環境での性能向上** — JS エンジンの最適化された文字列実装を直接利用。
   文字列比較・検索・連結が大幅に高速化される可能性
2. **JS 相互運用の改善** — JS ↔ Wasm 間の文字列コピーが不要に。
   Component Model の `string` 型との整合性も向上
3. **stdlib の簡素化** — 文字列操作の多くをビルトインに委譲可能

### 留意点

- **WASI / wasmtime 環境では恩恵が限定的** — stringref は主に JS ホスト向け。
  wasmtime では未実装であり、Arukellt の主要ターゲット (WASI p2) では使えない
- **エンコーディングの差異** — stringref は WTF-16 / WTF-8 を想定。
  Arukellt の UTF-8 前提と変換コストが発生する可能性
- **ターゲット分岐** — ブラウザ向け (`stringref` 使用) と WASI 向け (`array (mut i8)` 維持)
  で文字列実装が分岐するリスク

### 推奨アクション (v5)

- ブラウザターゲット追加時に string builtins の採用を検討
- WASI ターゲットでは現在の `(array (mut i8))` を維持
- `std::string::String` の内部実装をターゲット依存で切り替える設計を準備
  (公開 API は統一、内部表現はターゲットで分岐)
- wasmtime での stringref サポート状況を監視

---

## 総合評価マトリクス

| 機能 | 仕様成熟度 | Arukellt 影響度 | 実装コスト | v5 推奨 |
|------|-----------|----------------|-----------|---------|
| Static fields | 低 (設計段階) | 高 (vtable, enum) | 中 | 仕様追従 + 設計準備 |
| Weak references | 低 (調査段階) | 中 (循環参照) | 中 | 仕様確定待ち |
| Finalization | 低 (調査段階) | 低 (RAII で代替) | 高 | 見送り |
| String builtins | 中 (実験実装あり) | 高 (文字列性能) | 中 | ブラウザ向け限定採用 |

## 参照

- `docs/spec/spec-3.0.0/proposals/gc/Post-MVP.md` — WasmGC Post-MVP 仕様
- `docs/spec/spec-3.0.0/OVERVIEW.md` — WasmGC 概要
- ADR-002: GC vs non-GC (メモリモデル決定)
- ADR-004: Trait 戦略
- ADR-008: Component Model ラッピング戦略
- Issue #120: WasmGC Post-MVP プレビュー
