# ADR-017: Playground Execution Model and v1/v2 Product Contract

ステータス: **ACCEPTED** — client-side hybrid実行モデル（v1はサーバーサイドexecutorなし、v2はブラウザでcompile+run）
作成日: 2026-03-31（2026-07-10 改訂: ADR-032 を統合）
改訂日: 2026-07-11 — ADR-007/013 整合（`wasm32-gc` + jco パイプライン、freestanding 廃止）
範囲: Playground (web), target roadmap, docs contract

---

## 文脈

Arukellt の web playground は、実装作業の前に具体的な製品契約が必要である。判断を駆動する制約は二つ:

1. **ブラウザ実行は `wasm32-gc` component パイプライン経由。**
   [ADR-007: Targets](ADR-007-targets.md) では、ブラウザ実行は
   `wasm32-gc` component 出力 → `jco transpile` → ESM + JS glue として定義されている。
   旧 `wasm32-freestanding`（T2）相当は廃止済みであり、freestanding 待ちは不要である。

2. **primary ターゲット（`wasm32-gc`、ADR-013）はサーバー実行（wasmtime）向けの正準経路。**
   CI 検証済みの primary はブラウザ内で wasmtime を直接要しない。
   v2 は jco transpile によりブラウザで実行できる。
v1 でサーバー側 executor を出すと運用コスト・悪用面・遅延が増え、
主価値（即時フィードバック）はより軽いクライアント側ツールで得られる。

parser / formatter / diagnostics は WASI 依存のない pure Rust で、
今日 `wasm32-unknown-unknown` にコンパイルでき、ブラウザ安全な Wasm バンドルにできる。

Issue 378 は、下流作業（379, 428）の前にこの判断を強制するために開かれた。

---

## 決定

### 実行モデル: **client-side hybrid**（v1 にサーバー側 executor なし）

| 面 | 実行場所 | Wasm ターゲット | v1? |
|----|----------|-----------------|-----|
| Edit（Monaco/CodeMirror shell） | browser | n/a | ✅ yes |
| Format | browser (Wasm) | `wasm32-unknown-unknown` | ✅ yes |
| Parse | browser (Wasm) | `wasm32-unknown-unknown` | ✅ yes |
| Check / typecheck | browser (Wasm) | `wasm32-unknown-unknown` | ✅ yes |
| Diagnostics（構造化） | browser (Wasm) | `wasm32-unknown-unknown` | ✅ yes |
| Examples（キュレート集合） | static / browser | n/a | ✅ yes |
| Share / permalink | browser + static host | n/a | ✅ yes |
| Full compile（Wasm バイナリ emit） | **v1 ではなし** | — | ❌ v2+ |
| Run（ユーザープログラム実行） | **v1 ではなし** | — | ❌ v2+ |

**v1 でサーバー側 executor を置かない根拠:**

- 即時フィードバック（parse/type エラー、整形）は完全にクライアント側で達成でき、
  「言語を試す」最頻ユースケースをカバーする。
- サーバー側 executor はサンドボックス・レート制限・悪用対策・運用保守を要し、
  v1 目標「言語を探索可能にする」と直交する。
- 完全実行は v2 の `wasm32-gc` component + jco パイプラインに委ねる。
  v1 でサーバー回避策を出すと、v2 着地後に消える保守負担を生む。
- 実行を v2 へ延期すると v1 面が小さく、監査可能で、早く出荷できる。

### v1 スコープ（明示）

> **v1 = edit + format + parse + check + diagnostics + examples + share**

playground v1 完了には上記 6 面がすべて必要。いずれもサーバー executor・wasmtime を要しない。
check/format 等のフロントエンド面は `wasm32-unknown-unknown` バンドルで十分である。

### v1 非目標（明示）

次は v1 の明示的 **範囲外**:

- Wasm バイナリへのフルコンパイル（`--emit component`）
- ユーザープログラムの実行（任意ターゲット）
- サーバー側実行サンドボックス
- Native（`native-*`）実行
- WASI P3 / async runtime
- ブラウザエディタ内の LSP 統合（エディタ shell 作業に付随しうるが v1 ゲートではない）
- 認証セッション、保存プログラム、ユーザーアカウント

### v2 パイプラインと playground ロードマップ

playground v2（ブラウザで compile + run）は `wasm32-gc` component パイプラインに従う。
freestanding（旧 T2）の実装を待つ必要はない（ADR-007 で廃止）。

v1 の playground Wasm バンドルは `wasm32-unknown-unknown`（WASI なし）を対象とし、
コンパイラフロントエンドの既存 pure-Rust クレートで既に支えられる。
v2 の compile + run は selfhost による `wasm32-gc` component 出力と
jco transpile によるブラウザ実行で構成する。

---

## クライアント側サーフェス詳細

次のコンパイラ部品は `wasm32-unknown-unknown` バンドル経由で**完全にブラウザ内**で動く:

| 部品 | Crate(s) | 注記 |
|------|----------|------|
| Lexer | `src/compiler/lexer.ark`（または同等フロント） | WASI 依存なし |
| Parser | `src/compiler/parser.ark`（または同等フロント） | WASI 依存なし |
| Type checker（check-only） | `src/compiler/typechecker.ark` / `ark-driver` check gate | codegen 不要 |
| Formatter | formatter surface | 純変換、WASI なし |
| Diagnostics renderer | `src/compiler/diagnostics.ark` | 構造化出力、WASI なし |

バックエンド（codegen、Wasm emit、wasmtime runner）は v1 ブラウザバンドルに**含めない**。

---

## 帰結

1. **Issue 379**（ブラウザ向け Wasm パッケージ）はフロントのみバンドル向けに
   `wasm32-unknown-unknown` を対象として進めてよい。

2. **Issue 382**（旧 freestanding）は歴史的経緯として残すが、
   freestanding は ADR-007 で廃止済み。playground v2 は freestanding にブロックされない。

3. **Issue 428**（v1 契約 ADR の後続）は本文書を権威ある実行モデル判断として参照してよい。

4. share/permalink は静的ホスティング（または最小の読み取り専用 permalink サービス）を要し、
   コード実行バックエンドは不要。

---

## 検討した代替案

### A: v1 向けサーバー側 executor

ユーザーコードをコンパイル・実行するサンドボックスサーバーを出す。

**却下**: 運用複雑さ・悪用面・遅延が v1 の便益を上回る。
playground の主価値（構文を試し、エラーを即時に見る）は実行を要しない。

### B: `wasm32-gc` compile+run 着地まで v1 を止める

selfhost による `wasm32-gc` component 出力と jco パイプラインを待ち、
ブラウザでコード実行する playground を出す。

**却下**: v2 パイプライン待ちは有用な v1 を無期限に遅らせる。
playground の短期価値は editor + check のフィードバックループにある。

### C: コンパイルのみの v1（Wasm emit、実行なし）

parse/check/diagnostics/format に加え Wasm バイナリ emit を出すが実行はしない。

**却下**: バイナリ emit はフル codegen（`wasm32-gc` component）を要し、
追加作業が大きい。実行できないバイナリ blob を見せる限界効用は低い。
v2 で実行と合わせて再検討できる。

### D: 任意のサーバー側 run 付きハイブリッド

v1 はクライアント側 check/format に加え、任意のサーバー側 run ボタンを出す。

**却下**: 「任意」でもフル運用セットアップが必要で複雑さは減らない。v2 へ延期。

---

## docs / tests / examples との接続

### Docs 接続点

| Doc / page | playground v1 との関係 |
|-----------|------------------------|
| [ADR-007: Targets](ADR-007-targets.md) | `wasm32-gc` + jco パイプラインの読み取り専用参照。playground v1 は**変更しない**。 |
| Language / stdlib docs | エディタ shell（issue 379）が例スニペットから関連 docs へリンクしてよい。v1 起動に新規 doc ページは不要。 |

### Test 接続点

v1 ブラウザ Wasm バンドル（issue 379）は pure-Rust クレートで構成する。
それらの既存 Rust 単体/統合テストが主信号。playground 固有の検証層:

| 層 | 範囲 | 場所 |
|----|------|------|
| Cargo unit tests | バンドルに入る各クレート | `crates/*/tests/` と `#[test]` |
| Harness smoke | `scripts/manager.py --quick` / `--cargo` が通ること。v1 で別ブラウザ試験はゲートにしない | `harness/`, `scripts/` |
| Docs-consistency | `check-docs-consistency.py` が通ること | `scripts/check/check-docs-consistency.py` |
| Browser smoke（v1 ゲート） | Wasm を import して `parse()` を呼ぶ最小 JS/HTML で十分。フル統合は v2 | issue 379 で定義 |

issue 428 クローズに新規テスト基盤は不要。上記が権威ある v1 要件である。

### Examples 接続点

スコープ表の「Examples（キュレート集合）」は次と定義する:

- `std/examples/` または専用 `playground/examples/` に置く**静的・版管理された** `.ark` スニペット集合（正確なパスは issue 379 / editor shell で決定）。
- 各例は harness で **compile-check がクリーン**（parse + typecheck）であること。
  型検査未対応機能を使う例は明示ラベルか v1 から除外。
- 例は stdlib テストから**自動生成しない**。6 つの v1 面を示す手キュレート。
- share/permalink（issue 379）は同じ例ファイルをシードに使う。別コーパスは維持しない。

これにより、別 CI パイプラインなしに例がコンパイラ能力と同期する。

---

## v2: ブラウザ Compile + Run モデル（ADR-032 から統合、2026-07-10）

Playground v2 はブラウザでコンパイル + 実行を行う。TypeScript インタプリタは使わず、
selfhost コンパイラ Wasm をブラウザで実行し、コンパイル結果を jco 経由でブラウザ実行する。

### Two-stage browser pipeline

1. **Compile stage** — selfhost compiler Wasm を Web Worker で実行し、
   in-memory WASI P1 host 経由でコンパイルする
2. **Run stage** — `wasm32-gc` component 出力を `jco transpile` し、
   ESM + JS glue としてブラウザで実行する（ADR-007）

TypeScript 層はプロセスオーケストレーション、仮想ファイル、タイムアウト、
stdio バッファ、診断トランスポート、UI 状態のみを担当する。
Arukellt 言語の実行セマンティクスを TypeScript で再実装してはならない。

### Compile stage

```text
bootstrap/arukellt-selfhost.wasm
  -> docs/playground/assets/arukellt-selfhost.wasm
```

ブラウザ Worker はコンパイラアセットをロードし、コマンドプロセスとして実行:

```text
arukellt compile /work/main.ark --target wasm32-gc --emit component -o /work/out.component.wasm
```

（`--emit core-wasm` + component 組み立て経路も許容する。）

Worker host は argv, env, stdin/stdout/stderr capture, in-memory filesystem,
timeout, size limits を提供する。ネットワーク・ホストファイルシステムへの
アクセスは提供しない。

### Run stage

コンパイル結果の component を `jco transpile` し、ESM + JS glue としてロードする。
jco が WASI P2 imports を JS glue に変換する（ADR-007 改訂に準拠）。

> **注意**: ADR-032 原本では `arukellt_io` import を使用していたが、
> ADR-007 改訂（2026-07）で `arukellt_io` は廃止され、全てのホスト関数は
> WASI P2/P3 imports 経由に統一された。ブラウザ向けは jco transpile が
> WASI imports を JS glue に変換する。

### Non-goals (v2)

- Arukellt 言語インタプリタの TypeScript 実装
- 個別構文機能（`match`, `Result`, `?`, generics, traits）の TypeScript サポート
- Node API, DOM API, fetch, filesystem, network へのユーザープログラムからのアクセス
- `wasm32-gc` component の wasmtime 直接実行（ブラウザ run は jco 経由のみ）

---

## 参照

- `src/compiler/driver.ark` — ターゲット登録（`wasm32-gc` primary）
- [ADR-007](ADR-007-targets.md) — ターゲット分類（`wasm32-gc` + jco ブラウザパイプライン）
- [ADR-013](ADR-013-primary-target.md) — プライマリターゲット（`wasm32-gc`）
- Issue 378 — 本決定
- Issue 379 — Wasm パッケージング（本 ADR に続く）
- Issue 382 — 旧 freestanding（廃止、歴史的経緯）
- Issue 428 — v1 契約の後続（本 ADR を参照）
- Issue 632 — playground コンパイラ Wasm の build/run ループ（v2、旧 ADR-032）
