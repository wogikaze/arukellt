# v5: セルフホスト

> **状態**: v3完了時点で達成 — v4はスキップ (ADR-027)
> **参照**: [ADR-027: v3完了時点でのセルフホスト完了とv4スキップ](../adr/ADR-027-v3-selfhost-completion-skip-v4.md)

---

## 1. 版の目的

v4 までに確立した最適化済みコンパイラと安定 stdlib の上で、Arukellt コンパイラ自体を Arukellt で再実装する。Rust 実装 (Stage 0) から Arukellt 実装 (Stage 1) を経て、同一ソースから同一バイナリが生成されること (Stage 2 fixpoint) を達成する。

セルフホストの意義:
- 言語仕様の実用性証明: Arukellt で実際のコンパイラが書けることを示す
- 言語機能の充足確認: stdlib と言語機能のギャップを実装を通じて発見・解消する
- 長期的な自立性: Rust 依存を段階的に減らし、Arukellt エコシステムで閉じた開発を可能にする

---

## 2. 到達目標

1. Arukellt で書かれたコンパイラ (`src/compiler/*.ark`) が `arukellt compile` で `.wasm` を生成できる
2. 生成されたコンパイラ (`arukellt-s1.wasm`) が v4 時点の全 fixture test を pass する
3. Stage 1 → Stage 2 の fixpoint が達成される (`arukellt-s1.wasm` と `arukellt-s2.wasm` が byte-exact)
4. stdlib を含む全ソースが Arukellt 版コンパイラでビルドできる
5. Arukellt 版コンパイラのコンパイル時間が Rust 版の 5x 以内であること

---

## 3. 対象範囲

セルフホスト対象のコンポーネント:

| コンポーネント | Phase | 理由 |
|--------------|-------|------|
| Lexer (`ark-lexer`) | Phase 1 | 文字列処理中心、stdlib で十分 |
| Parser (`ark-parser`) | Phase 1 | パターンマッチ + 再帰下降 |
| Driver (`ark-driver`) | Phase 1 | オーケストレーション |
| CLI (`arukellt`) | Phase 1 | 引数解析 + dispatch |
| Resolver (`ark-resolve`) | Phase 2 | HashMap + スコープ管理 |
| TypeChecker (`ark-typecheck`) | Phase 2 | 最も複雑、ユニフィケーション要 |
| HIR (`ark-hir`) | Phase 2 | データ構造変換 |
| MIR (`ark-mir`) + 最適化パス | Phase 2 | 最適化パス含む |
| Wasm Emitter (`ark-wasm`) | Phase 3 | バイナリ出力、最も低レベル |
| LLVM Backend (`ark-llvm`) | **非対象** | LLVM C API 依存、FFI 必須 |

---

## 4. 非対象範囲

- `ark-llvm`: LLVM C API への FFI が必要。Arukellt には FFI 機能がないため非対象。
- WASI P3 / async: T5 スコープ。v5 では同期コンパイラのみ実装する。
- GUI フロントエンド・LSP サーバー (`ark-lsp`): セルフホスト対象外。
- Arukellt 版コンパイラへの新機能追加: v5 は「Rust 版と同等機能の Arukellt 実装」が目標。新機能追加は v6 以降。
- Rust 版コンパイラの削除: Rust 版は参照実装として保持する。削除しない。

---

## 5. 主要設計課題

### 5.1 Phase 1: Lexer + Parser の実装

Lexer は文字列処理の連続。Arukellt の String (GC `(array mut i8)`) で実装可能だが、文字単位アクセス (`array.get`) のパフォーマンスが課題。

Parser は再帰下降。Arukellt には `enum` + `match` があるため AST 表現は可能。ただし、エラー回復 (panic mode recovery) の実装は複雑。v5 Phase 1 では最低限のエラー報告 (最初のエラーで停止) で実装し、エラー回復は Phase 3 で改善する。

### 5.2 TypeChecker のユニフィケーション

型推論のコアはユニフィケーション (union-find アルゴリズム)。実装に必要な要素:
- Union-Find: 配列ベースで実装 (`Vec<i32>` で parent を管理)
- 型変数テーブル: `HashMap<TypeVarId, Type>` (v3 の HashMap が前提)
- 型エラーのスパン情報: `SourceMap` の Arukellt 実装

### 5.3 Wasm バイナリ生成

バイナリ出力は `u8` の Vec を構築して fd_write する。LEB128 エンコード、UTF-8、固定バイトシーケンスが必要。純粋な数値計算と配列操作で実装可能。

### 5.4 ブートストラップ戦略

```
Stage 0: Rust 版 v4 リリースバイナリ (arukellt-rust)
    ↓ compile src/compiler/*.ark
Stage 1: arukellt-s1.wasm
    ↓ compile src/compiler/*.ark (同一ソース)
Stage 2: arukellt-s2.wasm
    ↓ 検証
fixpoint: arukellt-s1.wasm == arukellt-s2.wasm (byte-exact)
```

Stage 1 が全 fixture を pass し、かつ Stage 2 との fixpoint が確認できることで v5 完了。

### 5.5 二重実装期間の管理

Phase 1 完了後: Rust 版 Lexer/Parser を**機能フリーズ** (バグ修正のみ許可)。  
Arukellt 版に切り替え後: Rust 版は参照実装として保持 (削除しない)。  
テスト: 同一入力に対する Rust 版と Arukellt 版の出力一致を差分検証 (`scripts/run/compare-outputs.sh`)。

### 5.6 言語仕様の凍結

v5 着手前に `docs/language/spec.md` の凍結版を作成する。セルフホスト期間中の仕様変更は ADR 必須とし、Rust 版と Arukellt 版の両方への反映を義務付ける。仕様が安定していない状態でセルフホスト実装を開始しない。

---

## 6. 実装タスク

1. **`docs/language/spec.md` の凍結** (v5 着手前に必須)  
   - 型システム、構文、stdlib API の完全仕様を `docs/language/spec.md` に記述。
   - 「凍結」コミット後は仕様変更に ADR が必須となる。

2. **v5 セルフホスト必要 stdlib チェックリストの確認** (`docs/process/selfhosting-stdlib-checklist.md`)  
   - v3 で作成したチェックリストの全件確認。
   - 不足している stdlib 関数があれば v5 着手前に実装する。

3. **Phase 1: Lexer の Arukellt 実装** (`src/compiler/lexer.ark`)  
   - トークン型: `enum Token { Ident(String), Number(i64), Float(f64), Str(String), Punct(String), EOF }`
   - 入力: `String` (ファイル内容)、出力: `Vec<Token>`
   - 文字単位ループ (`for i in 0..len(source)`) + match

4. **Phase 1: Parser の Arukellt 実装** (`src/compiler/parser.ark`)  
   - AST 型: struct + enum で Rust 版 AST と同等の構造
   - 再帰下降パーサー (Pratt parsing for expressions)
   - エラー: 最初のエラーで停止 (Phase 1 では回復なし)

5. **Phase 1: Driver + CLI の Arukellt 実装** (`src/compiler/driver.ark`, `src/compiler/main.ark`)  
   - `args()` でコマンドライン引数取得
   - `fs_read_file()` でソース読み込み
   - 各フェーズを順に呼び出す
   - `exit(code)` で終了コード制御

6. **Phase 2: Resolver + TypeChecker の Arukellt 実装**  
   - `src/compiler/resolver.ark`: スコープ管理 (`HashMap<String, Symbol>` のスタック)
   - `src/compiler/typechecker.ark`: ユニフィケーション + 型推論
   - Union-Find: `Vec<i32>` で実装 (parent 配列)
   - `src/compiler/hir.ark`: HIR データ構造 (struct + enum)
   - `src/compiler/mir.ark`: MIR データ構造 + lowering

7. **Phase 3: Wasm Emitter の Arukellt 実装** (`src/compiler/emitter.ark`)  
   - Wasm バイナリフォーマットを `Vec<i32>` (バイト列) として構築
   - LEB128 エンコード、type section / function section / code section の直接生成
   - `fd_write(1, &bytes, len)` で標準出力に Wasm バイナリを書き出す

8. **ブートストラップ検証スクリプト** (`scripts/run/verify-bootstrap.sh`)  
   - Stage 0 → Stage 1 のコンパイル
   - Stage 1 で全 fixture を実行して pass を確認
   - Stage 1 → Stage 2 のコンパイル
   - `sha256sum` で Stage 1 と Stage 2 の byte-exact 比較
   - 全工程を自動化し exit code で結果を返す

9. **`ARUKELLT_DUMP_PHASES` のデバッグ出力** (Arukellt 版コンパイラ)  
   - Rust 版の `ARUKELLT_DUMP_PHASES` 相当を Arukellt 版にも実装
   - 環境変数を `args()` で読み取り、各フェーズの中間出力を stderr に書く

---

## 7. 検証方法

```bash
# Phase 1 完了確認
arukellt compile src/compiler/lexer.ark src/compiler/parser.ark -o phase1.wasm
wasmtime run phase1.wasm -- tests/fixtures/basic/hello.ark  # AST を出力

# 全コンパイラ (Phase 3 完了後)
arukellt compile src/compiler/*.ark -o arukellt-s1.wasm

# Stage 1 で全 fixture を確認
# (Stage-1 harness runner is planned; today run manually via wasmtime on arukellt-s1.wasm)
cargo test -p arukellt --test harness

# fixpoint 検証
scripts/run/verify-bootstrap.sh

# コンパイル時間 5x 以内の確認
hyperfine 'arukellt-rust compile src/compiler/*.ark' 'wasmtime run arukellt-s1.wasm -- src/compiler/*.ark'
```

---

## 8. 完了条件 (必要条件 A ∧ B ∧ C ∧ D)

| 条件 | 判定方法 |
|------|---------|
| A: Arukellt 版コンパイラが `.wasm` を生成できる | exit code 0 |
| B: `arukellt-s1.wasm` が v4 時点の全 fixture を pass する | 数値確認 (全件) |
| C: `sha256sum arukellt-s1.wasm arukellt-s2.wasm` が一致する (fixpoint) | sha256 比較 |
| D: stdlib を含む全ソースが Arukellt 版コンパイラでビルドできる | exit code 0 |
| コンパイル時間: Arukellt 版が Rust 版の 5x 以内 | hyperfine 計測 |
| `scripts/run/verify-bootstrap.sh` が exit code 0 を返す | スクリプト実行 |
| `docs/language/spec.md` 凍結版が存在する | ファイル確認 |
| `docs/migration/v4-to-v5.md` が存在する | ファイル確認 |

---

## 9. 次版 (v6 以降) への受け渡し

v5 はロードマップ上の最終マイルストーン。v5 完了後の次のステップ候補:

- **v6 候補**: WASI P3 / async-await (T5 ターゲット)
- **v6 候補**: トレイト (ADR-004 P3/P4 完全解禁)
- **v6 候補**: ネストジェネリクス解禁 (ADR-009 判断次第)
- **v6 候補**: LSP サーバー (`ark-lsp`) の完全実装

v5 完了時に揃っているべきドキュメント:

| ドキュメント | パス |
|------------|------|
| 言語仕様凍結版 | `docs/language/spec.md` |
| CoreHIR / MIR 仕様 | `docs/compiler/ir-spec.md` |
| ABI リファレンス | `docs/platform/abi-reference.md` |
| stdlib リファレンス完全版 | `docs/stdlib/reference.md` |
| エラーコード一覧 | `docs/compiler/error-codes.md` |
| ベンチマーク計画・結果 | `benchmarks/README.md` |
| 移行ガイド 4 本 | `docs/migration/v{1-4}-to-v{2-5}.md` |
| 全 ADR | `docs/adr/ADR-001〜ADR-00X` |
| CHANGELOG | `CHANGELOG.md` |

---

## 10. この版で特に気をつけること

1. **仕様未凍結での着手禁止**: `docs/language/spec.md` が凍結されていない状態でセルフホスト実装を開始すると、実装途中の仕様変更で Arukellt 版と Rust 版が乖離し、fixpoint に到達しなくなる。v5 の実装は仕様凍結コミット後に開始する。
2. **stdlib の不足**: セルフホスト実装中に「この機能が stdlib にない」と気づいた場合、それを実装してから続行する。v3 のチェックリスト確認を怠ると、v5 実装中に stdlib 追加が多発し、Rust 版との差が広がる。
3. **fixpoint に到達しない場合の対処**: byte-exact fixpoint が達成されない主な原因は (a) 非決定的な emitter (アドレス、並べ替え)、(b) GC 最適化による差異、(c) 実装バグ。デバッグには `ARUKELLT_DUMP_PHASES=emit` で中間出力を比較する。
4. **Arukellt 版コンパイラのパフォーマンス**: Wasm GC ランタイムの GC pause が TypeChecker の大量アロケーションで問題になる可能性がある。phase 2 (TypeChecker) では arena-style のアロケーション (大きな Vec を事前確保し index で参照) を意識して設計する。
5. **Rust 版のフリーズと分岐**: Phase 1 完了後に Rust 版 Lexer/Parser をフリーズする。フリーズ後のバグ修正は Rust 版と Arukellt 版の両方に同時に適用すること。片方だけ修正すると fixpoint が壊れる。
6. **デバッグ出力の実装**: Arukellt 版コンパイラにデバッグ出力 (`ARUKELLT_DUMP_PHASES`) がないとセルフホスト中のデバッグが不可能。Phase 1 の段階から実装する。
7. **コンパイル時間 5x の厳密な計測方法**: `hyperfine` の warm/cold キャッシュ条件を固定する。Rust 版は native binary、Arukellt 版は `wasmtime run arukellt-s1.wasm --` で比較する。wasmtime の JIT コンパイル時間をコンパイル時間に含めるかを明確にする (含めない: `--precompiled` で AOT コンパイルした wasm を使う)。

---

## 11. この版で必ず残すドキュメント

| ドキュメント | パス | 内容 |
|------------|------|------|
| 言語仕様凍結版 | `docs/language/spec.md` | 型、構文、stdlib 全仕様の凍結 |
| CoreHIR / MIR 仕様 | `docs/compiler/ir-spec.md` | データ構造定義、フェーズ間契約 |
| ブートストラップ手順 | `docs/compiler/bootstrap.md` | Stage 0→1→2 の実行手順 |
| v4→v5 移行ガイド | `docs/migration/v4-to-v5.md` | 変更点、セルフホスト移行への注意 |
| 現状ドキュメント更新 | `docs/current-state.md` | v5 完了状態、fixpoint 達成の記録 |
| CHANGELOG | `CHANGELOG.md` | v1〜v5 のリリースノート |

---

## 12. 未解決論点

1. **TypeChecker のユニフィケーション複雑度**: Arukellt の型推論は「最大 2 型パラメータのモノモーフィゼーション」に制限されているため、フル HM 型推論より単純なはず。ただし、ジェネリック関数の特殊化 (Vec<i32> と Vec<String> の分岐) は union-find で正しく扱う必要がある。
2. **Wasm バイナリ生成の検証**: Arukellt 版 emitter が生成する Wasm が `wasmparser` で valid か検証する仕組みが必要。`fd_write` で標準出力に書き出した後に別コマンドで検証するか、インラインで validation を呼ぶかを判断する。
3. **`ref.func` とアドレスの非決定性**: Wasm の `ref.func` は関数参照だが、バイナリ上は関数インデックスで表現される。同一ソースから同一インデックスが生成されることが fixpoint の前提。関数定義順の非決定性がある場合は、事前ソートで deterministic にする。
4. **エラー回復の実装時期**: Phase 1 の Parser は「最初のエラーで停止」。本格的なエラー回復 (複数エラー同時報告) は Phase 3 以降で実装する。これにより v5 の Arukellt 版コンパイラは Rust 版より少ないエラーメッセージしか出せない可能性がある。これを「Phase 3 での課題」として明記する。
