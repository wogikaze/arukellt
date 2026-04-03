# harness.rs: ARUKELLT_BIN 環境変数でバイナリパスを上書きできるようにする

**Status**: open
**Created**: 2026-04-03
**Updated**: 2026-04-03
**ID**: 482
**Depends on**: 328
**Track**: selfhost-retirement
**Blocks v1 exit**: no

---

## Decomposed from 330

Issue 330 (`fixture-harness-selfhost-compat`) は:
1. harness.rs が `ARUKELLT_BIN` env var を読む (実装変更)
2. selfhost で pass/fail するフィクスチャリストの生成 (出力形式)
3. regression 追跡と CI artifact 記録 (CI 配線)

の 3 層を混ぜている。この issue は **harness implementation layer のみ** を担当する。

**現在の false-done**: issue 330 の acceptance criterion 1
「`ARUKELLT_BIN=path/to/selfhost cargo test -p arukellt --test harness` で
selfhost compiler が使われる」は、`crates/arukellt/tests/harness.rs` が
`current_exe()` を使っておりこの env var を無視するため、**未達成**。

Downstream: #330 (CI artifact / regression tracking) — この issue 完了後に着手

---

## Summary

`crates/arukellt/tests/harness.rs` の `arukellt_binary()` 関数を修正し、
`ARUKELLT_BIN` 環境変数が設定されている場合はその値をバイナリパスとして使い、
設定されていない場合は現在の `current_exe()` を使うようにする。

これにより、`ARUKELLT_BIN=/path/to/selfhost.wasm cargo test -p arukellt --test harness`
でフィクスチャテストを selfhost binary に対して実行できるようになる。

## Why this is a separate issue

「harness が環境変数を読む」という実装変化は、CI artifact や regression 追跡とは独立して
Rust ユニットテストで検証できる。実装変更なしに CI から env var を渡しても意味がない。
実装が先、CI 配線が後。この順序を issue 分離で強制する。

## Visibility

internal-only (CI / 開発者が使う env var; ユーザーが直接触る surface ではない)

## Primary paths

- `crates/arukellt/tests/harness.rs` — `arukellt_binary()` 関数 (line 97-113)

## Allowed adjacent paths

- `scripts/run/verify-harness.sh` — ARUKELLT_BIN を渡す既存箇所の確認

## Non-goals

- selfhost バイナリの fixture pass/fail リスト生成 (#330)
- regression 追跡の実装 (#330)
- CI artifact への記録 (#330)
- selfhost バイナリ自体のビルド (Issue 459 範囲)

## Acceptance

1. `crates/arukellt/tests/harness.rs` の `arukellt_binary()` が
   `std::env::var("ARUKELLT_BIN")` を試み、設定されていれば `PathBuf::from(val)` を返す
2. `ARUKELLT_BIN` が未設定の場合は従来通り `current_exe()` ベースのパスを使う
   (regression なし)
3. `ARUKELLT_BIN=/nonexistent cargo test -p arukellt --test harness -- --list`
   が binary not found 系エラーを出す (env var が読まれていることの証拠)
4. `cargo test -p arukellt --test harness` (env var なし) が従来通り全テスト pass

## Required verification

- `grep "ARUKELLT_BIN" crates/arukellt/tests/harness.rs` が 1 件以上ヒットする
- `cargo test -p arukellt --test harness` が exit 0 (regression なし)

## Close gate

- `harness.rs` の `arukellt_binary()` に `ARUKELLT_BIN` env var の読み取りコードがある (grep)
- env var なしでの既存テストが全て pass する
- **fixture pass/fail リストや CI artifact の実装はこの issue の close 条件ではない** (#330 担当)

## Evidence to cite when closing

- `crates/arukellt/tests/harness.rs` の `arukellt_binary()` 修正後のコード (行番号)
- `cargo test -p arukellt --test harness` の pass 結果

## False-done risk if merged incorrectly

- env var を読むコードが追加されたが、実際に使われていない (コードパスが dead)
  → acceptance 3 の「nonexistent パス指定でエラーが出る」テストが必須
- CI で `ARUKELLT_BIN` を渡しても regression 追跡なしで「selfhost compat done」と言う
  → CI artifact / regression 追跡は #330 が担当; この issue では扱わない
