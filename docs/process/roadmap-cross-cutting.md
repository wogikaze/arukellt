# v1–v5 横断事項

> このドキュメントは各版をまたぐ依存関係・リスク・意思決定基準・品質保証を定義する。
> 各版の詳細は `roadmap-v{N}.md` を参照すること。

---

## 6.1 依存関係マップ

各版の成果が次版のどの前提になるかを列挙する。

```
v1.GC-native型表現 (struct/array/br_on_cast)
    → v2.canonical-ABI変換 (GC ref ↔ linear memory のリフト/ロワリングに必要)
    → v5.Wasm emitter セルフホスト (GC 型のバイナリ表現を Arukellt で生成)

v1.GcTypeRegistry (Ctx 内の型インデックス管理)
    → v2.WIT型マッピング拡張 (型定義の正規化された表現が前提)
    → v4.エスケープ解析 (型の構造情報が最適化パスに必要)

v1.MirModule.type_table (struct_defs, enum_defs)
    → v2.Component export (型情報から WIT record/variant を生成)
    → v4.インライン化 (fn_sigs が必要)
    → v5.TypeChecker セルフホスト (型テーブルの構造を Arukellt で再現)

v2.Component-export (--emit component の有効化)
    → v3.stdlib-module体系 (モジュールの公開境界が Component export に一致)
    → v5.Driver セルフホスト (--emit component オプションの処理が必要)

v2.WIT-resource (resource handle の実装)
    → v3.ハンドル管理API (resource handle の stdlib ラッパー)

v2.canonical-ABI変換ロジック (canonical_abi.rs)
    → v5.Wasm emitter セルフホスト (ABI 変換を Arukellt で再実装)

v3.HashMap (GC-native, rehash 付き)
    → v5.Resolver セルフホスト (スコープ管理に HashMap が必須)
    → v5.TypeChecker セルフホスト (型変数テーブルに HashMap が必須)

v3.ファイルI/O (fs_read_file, fs_write_file)
    → v5.Driver セルフホスト (ソースファイル読み込みに必須)

v3.コマンドライン引数 (args())
    → v5.CLI セルフホスト (arukellt コマンドの引数処理に必須)

v3.API安定性ルール (Stable/Unstable/Deprecated)
    → v4.ベンチマーク固定 (API が変わるとベンチマーク結果が無効化される)
    → v5.セルフホスト実装 (Stable API を前提として Arukellt 版を書く)

v3.モジュール名前解決 (use std::string)
    → v5.Resolver セルフホスト (モジュール解決ロジックを Arukellt で実装)

v4.MIR最適化パス (passes/ の独立ファイル群)
    → v5.MIR セルフホスト (同等パスを Arukellt で実装。設計が移植可能であることが前提)

v4.--opt-level フラグ
    → v5.Arukellt版コンパイラでの最適化制御 (同一フラグ体系を引き継ぐ)

v4.コンパイル時間目標 (hello.ark 50ms, parser.ark 500ms)
    → v5.Stage1/Stage2のビルド実用性 (Arukellt 版が現実的な時間でビルドできる前提)

v4.言語仕様凍結 (docs/language/spec.md)
    → v5.セルフホスト全体 (仕様変更が起きないことで Arukellt 版と Rust 版の乖離を防ぐ)
```

---

## 6.2 リスク管理

### リスク 1: Wasm GC ランタイム (wasmtime) の成熟度不足

| 項目 | 内容 |
|------|------|
| 発生条件 | wasmtime の Wasm GC 実装にバグや性能問題がある |
| 影響 | T3 の fixture テストが wasmtime のバグで失敗する; binary_tree 1.83x が許容範囲を超える |
| 緩和策 | wasmtime バージョンを `Cargo.toml` で固定。バグ報告は wasmtime issue tracker で追跡。wasmtime バグが修正された新バージョンへの更新時は全 fixture を再検証する |
| 許容基準 | wasmtime の既知バグは `docs/current-state.md` に Known Limitations として記録し、バグ修正後に更新する |

### リスク 2: スコープ肥大化 — トレイト導入が stdlib 設計を複雑化

| 項目 | 内容 |
|------|------|
| 発生条件 | ADR-004 P3 (traits) の評価で「今すぐ必要」と判断し、v3 に前倒しする |
| 影響 | stdlib API 設計が traits 依存になり、v3 の API 安定性ルール策定が遅れる |
| 緩和策 | トレイト導入の判断は ADR-004 補遺に記録する。v3 の完了条件に「traits なしで stdlib が機能すること」を含める。v3 着手時に「traits は v4 後半評価」を明文化する |
| 許容基準 | v3 完了時点で traits の実装が 0 行であること |

### リスク 3: 互換性破壊 — stdlib API 名変更による既存コードの breakage

| 項目 | 内容 |
|------|------|
| 発生条件 | v3 でモノモーフ名 (`Vec_new_i32`) から新 API 名への移行時、deprecation 期間なしに旧 API が除去される |
| 影響 | 既存 346 fixture と user code が全て壊れる |
| 緩和策 | `std/manifest.toml` の `stability` フィールドで管理。旧 API は v3 で Deprecated 化、v4 で除去。除去前に全 fixture を新 API に移行する。`scripts/run/verify-harness.sh` に deprecated API 使用チェックを追加 |
| 許容基準 | v3 リリース時に Deprecated API が 0 の状態の fixture がない (全件移行済み) |

### リスク 4: 性能未達 — GC overhead が許容範囲を超える

| 項目 | 内容 |
|------|------|
| 発生条件 | binary_tree(depth=15) が C 比 3x を超え、v4 の最適化で改善できない |
| 影響 | Arukellt を性能要求のあるユースケースに使えない |
| 緩和策 | v1 完了時点の binary_tree 1.83x を baseline として記録 (ADR-002 参照)。v4 のエスケープ解析 + scalar replacement で 1.5x を目標とする。改善できない場合は 1.83x を「GC オーバーヘッドの現実」として文書化し、T1 (linear memory) の代替ターゲットを提示する |
| 許容基準 | binary_tree の結果は `docs/process/benchmark-results.md` に常に記載。2.5x 超で v4 の perf gate failure とする |

### リスク 5: ドキュメント不足 — 設計意図の欠落によるセルフホスト時の再設計

| 項目 | 内容 |
|------|------|
| 発生条件 | MIR の設計根拠、GcTypeRegistry の型インデックス割り当てルール等が文書化されていない |
| 影響 | v5 の Arukellt 版実装で Rust 版と異なる設計を採用し、fixpoint に到達しない |
| 緩和策 | `docs/compiler/ir-spec.md` を v4 完了時に作成する。ADR に「なぜそう設計したか」を必ず書く。v5 着手前に `docs/language/spec.md` を凍結する |
| 許容基準 | v4 完了時に `docs/compiler/ir-spec.md` が存在し、MIR の全 struct/enum が文書化されていること |

### リスク 6: ブートストラップ失敗 — fixpoint が到達しない

| 項目 | 内容 |
|------|------|
| 発生条件 | Arukellt 版 emitter が非決定的な出力を生成する (関数順序、アドレス等) |
| 影響 | Stage 1 と Stage 2 のバイナリが一致せず、fixpoint 検証が通らない |
| 緩和策 | emitter 設計時に決定性を保証する: 関数インデックスはソース内出現順で固定、HashMap の iteration 順序は挿入順で固定 (または sorted key で iterate)。`ARUKELLT_DUMP_PHASES=emit` でバイナリ生成の中間状態を比較できるようにする |
| 許容基準 | fixpoint 検証が通らない場合は `scripts/run/verify-bootstrap.sh` が詳細な差分を出力する |

---

## 6.3 意思決定基準

「今版でやる」「次版へ送る」「完全に非目標とする」を判断するフレームワーク。

| 判断基準 | 説明 | 例 |
|---------|------|----|
| 基準 1: 今版の完了条件に直接必要か | その機能なしに完了条件が達成できないか | HashMap は v5 セルフホストに必要 → v3 で実装 |
| 基準 2: 次版以降の設計を阻害しない形で遅延可能か | 後回しにしても設計上の変更が不要か | traits は v3 なしでも stdlib が設計できる → v4 以降 |
| 基準 3: 実装コストと価値のバランス | 複雑な実装で得られる価値が小さくないか | 正規表現は uses が限定的 → v3 では非対象 |
| 基準 4: テスト・検証体制が整っているか | fixture や検証スクリプトなしに実装するとリグレッションリスクが高い | GC 型変更は pre-scan/emit の 2 パス確認が必要 |

**「今版でやる」条件**: 基準 1 が yes。  
**「次版へ送る」条件**: 基準 1 が no かつ基準 2 が yes。「次版以降に着手する条件」を明記すること。  
**「完全に非目標とする」条件**: 基準 1, 2 ともに no かつ ADR で非目標を明文化している。「なぜ今版では入れないのか」「何が満たされたら着手するのか」を書くこと。

---

## 6.4 ドキュメント体系

v5 完了時に揃っているべきドキュメントの全リスト:

| ドキュメント | パス | 作成版 |
|------------|------|-------|
| 言語仕様凍結版 | `docs/language/spec.md` | v4 完了時 (v5 着手前) |
| 型システム仕様 | `docs/language/type-system.md` | v3 更新 |
| エラーハンドリング仕様 | `docs/language/error-handling.md` | v2 更新 |
| CoreHIR / MIR 仕様 | `docs/compiler/ir-spec.md` | v4 完了時 |
| コンパイラパイプライン | `docs/compiler/pipeline.md` | v4 更新 |
| エラーコード一覧 | `docs/compiler/error-codes.md` | v3 以降随時 |
| ブートストラップ手順 | `docs/compiler/bootstrap.md` | v5 |
| ABI リファレンス | `docs/platform/abi-reference.md` | v2 |
| Wasm 機能利用状況 | `docs/platform/wasm-features.md` | v2 更新 |
| stdlib リファレンス | `docs/stdlib/reference.md` | v3 |
| ベンチマーク計画・結果 | `benchmarks/README.md` | v4 |
| 移行ガイド v1→v2 | `docs/migration/v1-to-v2.md` | v2 |
| 移行ガイド v2→v3 | `docs/migration/v2-to-v3.md` | v3 |
| 移行ガイド v3→v4 | `docs/migration/v3-to-v4.md` | v4 |
| 移行ガイド v4→v5 | `docs/migration/v4-to-v5.md` | v5 |
| 全 ADR | `docs/adr/ADR-001〜ADR-00X` | 随時 |
| 現状ドキュメント | `docs/current-state.md` | 各版リリース時更新 |
| CHANGELOG | `CHANGELOG.md` | v5 (v1〜v5 分まとめて) |
| セルフホスト stdlib チェックリスト | `docs/process/selfhosting-stdlib-checklist.md` | v3 |

---

## 6.5 品質保証

### テストピラミッド

```
         ┌─────────────────────────────┐
         │  e2e / セルフホスト検証      │  ← v2 (component), v5 (bootstrap)
         │  (少数・重要なシナリオ)       │
         ├─────────────────────────────┤
         │  integration / fixture       │  ← 346+ 件、manifest.txt 駆動
         │  (各言語機能・stdlib の動作)   │
         ├─────────────────────────────┤
         │  unit tests                  │  ← 95+ 件、クレートごと
         │  (MIR, typecheck, resolve 等) │
         └─────────────────────────────┘
```

### スナップショットテスト

- MIR dump: `ARUKELLT_DUMP_PHASES=mir` の出力を `tests/snapshots/mir/` に固定 (v4 で追加)
- diagnostics output: エラーメッセージの期待値を `tests/snapshots/diagnostics/` に固定
- 更新方法: `scripts/run/update-snapshots.sh` で一括更新、diff をコミットに含める

### 性能回帰テスト

- `tests/baselines/perf/` に JSON 形式でベンチマーク結果を保存 (v4 で追加)
- `scripts/run/verify-harness.sh` の perf gate で自動比較: コンパイル時間 +20%, 実行時間 +10%, バイナリサイズ +15% で failure
- 手動更新: `scripts/update-baselines.sh` (意図的な性能変化のみ)

### 再現ビルド検証

- 同一入力 → 同一 `.wasm` (bit-exact) を verify-harness.sh のゲートとして追加 (v4 で実装)
- 非決定的な要素 (HashMap iteration 順序等) は全て deterministic に固定する

### セルフホスト検証

- `scripts/run/verify-bootstrap.sh`: Stage 0 → Stage 1 → Stage 2 → fixpoint 確認
- v5 の verify-harness.sh に組み込む

---

## 6.6 バージョン間整合性チェック

各版完了時に以下を確認する:

**v1 完了時**:
- GC 型表現 (`struct`, `array`, `br_on_cast`) は v2 の canonical ABI 変換を効率的に行えるか?
  → 確認: `crates/ark-wasm/src/component/wit.rs` の既存マッピングと GC-native 型が矛盾しないこと
- `GcTypeRegistry` の型インデックス割り当てが `canonical_abi.rs` の変換ロジックと整合できるか?
  → 確認: ADR-008 の前提調査として実施

**v2 完了時**:
- Component Model 設計は v3 の stdlib モジュール境界と整合するか?
  → 確認: `pub fn` → WIT export の規則が v3 のモジュール設計に矛盾しないこと
- v2 の設計が async 導入 (v5 T5) を阻害しないか?
  → 確認: async WIT 型 (`future<T>`, `stream<T>`) を非対応エラーとして処理しており、後から追加できる設計であること

**v3 完了時**:
- API 安定性ルールは v4 の最適化 (インライン化等) を阻害しないか?
  → 確認: Stable API の関数シグネチャが `passes/inline.rs` の対象に含まれること
- v5 セルフホスト必要 stdlib チェックリストが全件 Stable で揃っているか?
  → 確認: `docs/process/selfhosting-stdlib-checklist.md` の全件チェック

**v4 完了時**:
- 最適化パスは v5 の Arukellt 実装に移植可能な設計か?
  → 確認: `passes/` の各ファイルが Arukellt で書ける程度に複雑すぎないこと (再帰の深さ、データ構造の複雑さを確認)
- v1–v4 の設計判断で、v5 のセルフホストを不可能にする選択はないか?
  → 確認: HashMap iteration 順序の determinism、emitter の決定性、ARM abi の non-FFI な設計

---

## セルフレビュー (横断事項)

1. **v1–v5 の情報量**: v1 (完了報告) v2 (8 セクション) v3 (8 セクション) v4 (8 セクション) v5 (8 セクション) — 均等。
2. **verify-harness.sh との整合**: 各版の完了条件は全て exit code / 数値比較 / ファイル存在確認の形で記述されており、スクリプト化可能。
3. **過去の問題の反映**: bridge mode 型不整合 → v1 の注意点 10.1–10.4。manifest.txt 不整合 → v1 の注意点 10.6。emit 分岐漏れ → v1 の pre-scan 注意点 10.4。
4. **次版受け渡し条件**: 各版の第 9 節に「前提条件」と「渡す成果物」を yes/no 判定可能な形で記載。
5. **ドキュメントパス**: 6.4 節に全ドキュメントをパス付きで一覧化。
6. **スコープ境界**: 各版の第 4 節 (非対象範囲) で「やらないこと」を明文化。第 12 節で「次版以降に着手する条件」を記載。
7. **将来への布石**: v3 HashMap → v5 セルフホスト (依存関係マップに記載)。v4 MIR パス設計 → v5 移植可能設計 (v4 注意点 10.6 に記載)。v1 型表現 → v2 canonical ABI 変換 (依存関係マップに記載)。
8. **仕様肥大化チェック**: v3 でセルフホスト用 stdlib を先取りしていないか → v3 の非対象範囲に「実装は v4 以降」の項目を明記。v1 で Component Model の準備をしすぎていないか → v1 の非対象範囲に `--emit component` hard error 維持を明記。
9. **LLM の逃げ道**: 各版の実装タスクはクレート名・ファイルパス・型名を含む 7 項目以上で記述。「丁寧に設計する」等の抽象表現を使用していない。
10. **ADR 反映**: ADR-001–007 を全て roadmap.md 第 0 章または各版の設計課題に取り込んだ。ADR-008 (v2) 予定を明記。
11. **クレート構成変更**: `crates/ark-mir/src/passes/` (v4) の新規ディレクトリ作成を roadmap-v4.md タスク 1 に記載。
12. **issues 整合**: issues #019–#027 は v1 の骨格として roadmap-v1.md 第 6 節に全件対応表を記載。
