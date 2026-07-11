# ADR-029 — セルフホストネイティブ検証契約

ステータス: **ACCEPTED**
日付: 2026-04-22
Issue: [#585](../../issues/done/585-selfhost-native-verification-contract.md)
解除する依存: #583, #560, #561, #562, #563, #564（Phase 5 Rust 退役）
関連: ADR-024（selfhost MIR）

## 背景

`scripts/selfhost/checks.py` に実装された 4 つの正規セルフホストゲートは、歴史的にレガシー Rust コンパイラバイナリ（`target/debug/arukellt`）をすべてのパリティ比較の**信頼できるベース**として使っていた:

| ゲート | Pre-585 ベースライン | 比較 |
|------|------------------|------------|
| `selfhost fixpoint` | Rust バイナリが `src/compiler/main.ark` をコンパイル → `s1.wasm` | `sha256(s2) == sha256(s3)`（s2/s3 はセルフホスト生成） |
| `selfhost fixture-parity` | Rust バイナリが各フィクスチャをコンパイル | `wasmtime` で Rust 出力とセルフホスト出力を実行（文字列等価） |
| `selfhost diag-parity` | Rust バイナリ `check fixture.ark` が正規診断を生成 | セルフホスト `check` 出力が同じ `.diag` パターンを含むこと |
| `selfhost parity --cli` | Rust バイナリ `--version`/`--help`/終了コード | セルフホスト `--version`/`--help`/非ゼロ終了コードがバイト等価 |

この契約は Phase 5 Rust 退役チェーン（#583, #560–#564）をブロックする。`_find_arukellt()` が `target/debug/arukellt` を見つけられないとゲートはハードフェイルする。Rust バイナリが信頼ベースである限り、Rust crates は削除できない。

セルフホストコンパイラは #559 で fixpoint に到達した。自身の wasm 出力（`s2 == s3`）から自己ブートストラップし、`scripts/run/arukellt-selfhost.sh` はすでにセルフホスト wasm をユーザー向けデフォルトとして実行する。再現可能な由来を持つ**バイト固定**セルフホストアーティファクトを記録すれば、挙動カバレッジを失わずに検証の信頼ベースを Rust バイナリから移せる。

## 決定

レガシー Rust ベースライン契約を、単一のコミット済みピン留め参照 wasm に固定した**セルフホストネイティブ検証契約**に置き換える。

### 信頼ベース: `bootstrap/arukellt-selfhost.wasm`

`bootstrap/arukellt-selfhost.wasm` にコミットされた単一の wasm ファイルが、すべてのセルフホストゲートの信頼ベースである。リポジトリ全体の `*.wasm` `.gitignore` は明示 allow-list で本ファイルを除外する。由来、sha256、サイズ、再現レシピは `bootstrap/PROVENANCE.md` に記載する。更新は明示的（`chore(bootstrap): refresh pinned selfhost wasm`）で、自動ではない。導入するすべての挙動ドリフトを列挙する必要がある。

アーティファクトは現状 524 KiB — 採用する 10 MiB 上限のソフトサイズ予算を十分下回る。将来の更新が予算に近づく場合は更新コミットメッセージで明記する。

### 再定義されたゲートセマンティクス

すべてのゲートは `wasmtime` 下でセルフホストコンパイラを実行する。いずれも `target/{debug,release}/arukellt` を読んだりシェルアウトしたりしない。`cargo build` も不要。

#### 1. `selfhost fixpoint` — ピン留めからのブートストラップ + Stage-3 fixpoint

```text
pinned (bootstrap/arukellt-selfhost.wasm)  ──▶  s2.wasm
s2.wasm  ──▶  s3.wasm
require: sha256(s2) == sha256(s3)
```

Pre-585 契約から維持されるもの:
- 古典的ブートストラップ fixpoint 定義は不変（`sha256(s2) == sha256(s3)`）。
- セルフホストコンパイラの再現性を壊すドリフト（例: 非決定的 codegen）は依然ゲート失敗。

変わるもの:
- **Stage 0 はピン留め wasm であり Rust コンパイラではない。** Rust バイナリは参照されない。
- **以前のベースラインとの Stage-1 バイト等価は `fixpoint` の一部としてはもはやアサートしない** — 歴史的に `s1`（Rust）と `s2`（セルフホスト）は別エンコーディングだったため、意味のあるバイト固定は `s2 == s3` のみだった。

#### 2. `selfhost fixture-parity` — ピン留め対現行の実行パリティ

```text
for each fixture in tests/fixtures/manifest.txt (run:):
    out_pinned   = wasmtime(pinned, "compile", fixture)
    out_current  = wasmtime(current_selfhost, "compile", fixture)
    run both wasms; require execution stdout/stderr/exit equal
```

`current_selfhost` は `run_fixpoint` が生成する Stage-2 wasm（またはピン留め wasm + `src/compiler/main.ark` からオンデマンド再ビルド）。

維持されるもの:
- `run:` フィクスチャコーパス全体の挙動カバレッジ（≥ 350 フィクスチャ）。実行出力等価は pre-585 契約と同じ比較。
- `FIXTURE_PARITY_SKIP` allow-list はそのまま — 既知のセルフホスト専用 emitter 不足は追跡され、黙って落とされない。
- `pass_count >= 10` 下限は維持（issue ファイル要件）。

変わるもの:
- ベースラインはピン留めセルフホストであり Rust コンパイラではない。ゲートは**ピン留めベースラインと現行ソースツリー間の挙動ドリフト**を検出し、セルフホスト対 Rust ドリフトではない。
- `src/compiler/**` が不変なら `current_selfhost == pinned` がバイト等価でゲートは自明にパス（意図的 — 回帰検出器であり正しさのオラクルではない）。
- `src/compiler/**` が意図的にドリフトし挙動保存ならゲートはパス。挙動変更ドリフトなら、回帰修正か、`bootstrap/PROVENANCE.md` 更新ポリシーに従ったピン留め wasm 更新とドリフト一覧が必要まで失敗。

これは issue ファイルに列挙された 2 案のうち、挙動保存に最も強い選択（セルフホストのみ決定性はピン留め対現行パリティの厳密部分集合。後者は意図的だが未文書化の意味ドリフトも検出する）。

#### 3. `selfhost diag-parity` — 純セルフホスト診断スナップショット

```text
for each fixture in tests/fixtures/manifest.txt (diag:):
    out = wasmtime(current_selfhost, "check", fixture)
    pattern = (fixture[:-4] + ".selfhost.diag")  if exists  else  (fixture[:-4] + ".diag")
    require: pattern in out
```

維持されるもの:
- コミット済み `.diag` / `.selfhost.diag` ゴールデンが契約 — pre-585 セルフホストと同じファイル。
- `DIAG_PARITY_SKIP` セットはそのまま。
- `pass_count >= 10` 下限は維持。

変わるもの:
- ゲートは Rust バイナリの診断出力をクロスチェックしない。Pre-585 で Rust が「pattern not found; test may be stale」としたフィクスチャは `skip` になった。新契約では `.diag` ゴールデンがセルフホスト出力と一致しないフィクスチャは `FAIL`（または追跡 issue 付きで `DIAG_PARITY_SKIP` 追加）。長期的には**より強い**アサーションだが、移行時点の live `pass_count` は同じ（≈ 11）。以前パスしていたフィクスチャはすべて依然パス。

#### 4. `selfhost parity --cli` — 純セルフホスト CLI スナップショット

```text
require: wasmtime(current_selfhost, "--version") == tests/snapshots/selfhost/cli-version.txt
require: wasmtime(current_selfhost, "--help")    == tests/snapshots/selfhost/cli-help.txt
require: wasmtime(current_selfhost, "foobar_unknown_cmd").returncode != 0
require: for cmd in {compile, check, run}: wasmtime(current_selfhost, cmd).returncode != 0
```

維持されるもの:
- 未知コマンド / 引数なしでの非ゼロ終了アサーションは不変。
- `--version` / `--help` テキスト形式は実装間比較ではなく追跡スナップショット。

変わるもの:
- ゴールデンは `tests/snapshots/selfhost/` 配下。意図的 CLI テキスト変更は同一コミットでゴールデン更新が必要。

### 明示的にスコープ外のもの

- Rust crate（`crates/**`）の削除 — それは #560–#564。
- `scripts/run/arukellt-selfhost.sh` から `ARUKELLT_USE_RUST=1` オプトインの削除 — それは #583。
- セルフホストソース（`src/compiler/**`）やフィクスチャ（`tests/fixtures/**`）の変更。
- `scripts/manager.py` CLI 表面の変更 — `selfhost {fixpoint,fixture-parity,diag-parity,parity}` は維持。

## 採用時のベースラインゲート件数（コミット `662c3f58`）

| ゲート | 結果 | 備考 |
|------|--------|-------|
| `selfhost fixpoint` | PASS | s2 sha256 = `c16e32ef…0cc`（ピン留め `3a035037…f2c` からビルド） |
| `selfhost fixture-parity` | PASS | 321 PASS, 0 FAIL, 41 SKIP（16 selfhost wasm-trap, 23 selfhost-compile timeout under wasmtime, 2 explicit `FIXTURE_PARITY_SKIP`) |
| `selfhost diag-parity` | PASS | 12 PASS, 22 SKIP, 0 FAIL |
| `selfhost parity --cli` | PASS | 6 PASS, 0 FAIL |

この 4 行は `cargo clean` と `target/debug/arukellt` 削除後の新規クローンで再現可能（採用時に検証 — #585 クローズノート参照）。

## 結果

### 肯定的

- Phase 5 Rust 退役（#560–#564）のブロックが解除。`target/debug/arukellt` は検証依存ではなくなる。
- 新規クローンは `wasmtime` と `python3` のみでセルフホストコンパイラを検証可能 — Rust ツールチェーン、`cargo build` 不要。
- 信頼ベースはバイト固定で git SHA から再現可能。
- 意図的ソース変更とピン留めベースラインのドリフトは黙った回帰ではなくゲート失敗として表面化。

### 否定的 / 受け入れたトレードオフ

- ピン留め wasm の更新が保守儀式になる。緩和: `bootstrap/PROVENANCE.md` に更新ポリシーを文書化。
- リポジトリに 524 KiB のバイナリアーティファクトを載せる。緩和: 小さい、更新頻度は低い（意図的意味変更時のみ）。代替（CI 時に取得する事前ビルド blob）はネットワーク/可用性依存を導入し、より悪いと判断。
- `fixture-parity` は独立実装とのクロスチェックをしなくなる。緩和: 以前の Rust 対セルフホスト比較は Phase 5 でいずれ退役。より強いオラクル（例: 仕様由来フィクスチャと期待出力ゴールデン）は別途追跡で本スコープ外。

## 検討した代替案

1. **セルフホストのみ決定性**（各フィクスチャを 2 回コンパイルし wasm がビット同一）。却下: ピン留め対現行パリティより弱く、意図的だが未文書化の意味ドリフトを検出しない。
2. **ネットワーク取得ピン留め wasm**（バイナリをコミットせずリリースアセットからダウンロード）。却下: 新規クローン検証に可用性依存。
3. **コミット時にフィクスチャ出力ゴールデンを生成**し現行セルフホスト実行と比較。本スライスでは却下（はるかに大きなフィクスチャ出力コーパスと `tests/fixtures/**` 変更が必要でここでは FORBIDDEN）。フォローアップ issue に延期。

## 検証

#585 受け入れどおり:

```bash
rm -f target/debug/arukellt
python3 scripts/manager.py selfhost fixpoint        # PASS
python3 scripts/manager.py selfhost fixture-parity  # PASS
python3 scripts/manager.py selfhost diag-parity     # PASS
python3 scripts/manager.py selfhost parity --mode --cli  # PASS
```

`cargo build --workspace --exclude ark-llvm` ビルドは依然成功 — 本スライスは Rust crate を削除しない。
