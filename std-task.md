# stdlib migration task board

Rust 実装中心の stdlib を、Arukellt 実装中心の `std/*.ark` へ移していくための実行順タスクボード。

方針:

- 優先順位は付けない
- ただし **実行順** は固定する
- 各タスクは、関連コードと関連ドキュメントへ必ずリンクする
- docs 更新は最後にまとめて行うのではなく、実装 reality が固まった段階で反映対象を洗い出す

---

## 0. 前提リンク

### 実装の現状

- `docs/process/v0-status.md`
- `docs/stdlib/README.md`
- `std/prelude.ark`
- `crates/arukellt/src/main.rs`
- `crates/ark-resolve/src/resolve.rs`
- `crates/ark-typecheck/src/checker.rs`
- `crates/ark-mir/src/lower.rs`
- `crates/ark-wasm/src/emit.rs`
- `crates/ark-stdlib/src/lib.rs`
- `tests/fixtures/modules/`

### 設計・計画

- `docs/stdlib/README.md`
- `docs/stdlib/core.md`
- `docs/stdlib/io.md`
- `docs/process/v0-status.md`
- `docs/process/v0-scope.md`
- `docs/compiler/pipeline.md`
- `/home/wogikaze/.claude/plans/serialized-crafting-music.md`

---

## 1. source-backed prelude の runtime 破綻を直す

### 目的

`std/prelude.ark` を通した wrapper 呼び出しが `check` だけでなく `run` でも通るようにする。

### 対象コード

- `std/prelude.ark`
- `crates/arukellt/src/main.rs`
- `crates/ark-resolve/src/resolve.rs`
- `crates/ark-typecheck/src/checker.rs`
- `crates/ark-mir/src/lower.rs`
- `crates/ark-wasm/src/emit.rs`
- `tests/fixtures/modules/import_basic/main.ark`
- `tests/fixtures/modules/module_alias/main.ark`

### 関連ドキュメント

- `docs/process/v0-status.md`
- `docs/compiler/pipeline.md`
- `docs/stdlib/README.md`

### TODO

- [ ] `import_basic` が `arukellt run` で動くようにする
- [ ] `module_alias` が `arukellt run` で動くようにする
- [ ] wrapper 経由 call の Wasm stack balance 崩れを修正する
- [ ] user function call / builtin call / intrinsic call の codegen 規則を整理する
- [ ] `println`, `print`, `eprintln`, `String_from`, `eq` の wrapper 経路を end-to-end で安定化する

---

## 2. module loading を「実験実装」から「使える実装」へ固める

### 目的

単一 file compile ではなく、entry + imports + std module を安定して読む program/module graph を成立させる。

### 対象コード

- `crates/arukellt/src/main.rs`
- `crates/ark-resolve/src/resolve.rs`
- `crates/ark-resolve/src/scope.rs`
- `tests/fixtures/modules/circular/`
- `tests/fixtures/modules/pub_private/`

### 関連ドキュメント

- `docs/compiler/pipeline.md`
- `docs/language/syntax.md`
- `docs/process/v0-status.md`

### TODO

- [ ] import 解決規則を `local module` と `std module` で明確化する
- [ ] circular import の診断を実装する
- [ ] pub/private の可視性ルールを module import に反映する
- [ ] alias import を resolver/typechecker/lowering で整合させる
- [ ] module graph の source of truth を 1 箇所にまとめる

---

## 3. public stdlib 名と intrinsic 名の境界を固定する

### 目的

backend が public stdlib 名を直接 special-case しない形へ寄せる。

### 対象コード

- `std/prelude.ark`
- `crates/ark-resolve/src/resolve.rs`
- `crates/ark-typecheck/src/checker.rs`
- `crates/ark-mir/src/lower.rs`
- `crates/ark-wasm/src/emit.rs`

### 関連ドキュメント

- `docs/stdlib/README.md`
- `docs/process/v0-status.md`
- `/home/wogikaze/.claude/plans/serialized-crafting-music.md`

### TODO

- [ ] compiler が知る intrinsic 名一覧を固定する
- [ ] public wrapper 名一覧を `std/prelude.ark` 側へ寄せる
- [ ] resolver の prelude 注入から public wrapper 名を外す
- [ ] typechecker が public 名ではなく intrinsic 名を builtin として持つよう整理する
- [ ] emitter の special-case 対象を intrinsic 名中心へ揃える

---

## 4. std/prelude.ark を実ファイルの source of truth にする

### 目的

現在の合成 prelude 相当を、実際の `std/prelude.ark` 読み込みへ寄せる。

### 対象コード

- `std/prelude.ark`
- `crates/ark-resolve/src/resolve.rs`
- `crates/arukellt/src/main.rs`

### 関連ドキュメント

- `docs/stdlib/README.md`
- `docs/process/v0-status.md`
- `docs/compiler/pipeline.md`

### TODO

- [ ] `std/prelude.ark` を filesystem から読む経路に統一する
- [ ] resolver 内の synthetic prelude を段階的に削る
- [ ] 実際にロードされた prelude が public API を供給することを fixture で確認する
- [ ] `Prelude names injected` という現状を docs から更新できる状態にする

---

## 5. String 系 stdlib を Arukellt 側へ移す

### 目的

現在 runnable な String 公開面と、その直近の helper を `std/collections/string.ark` へ寄せる。

### 対象コード

- `std/prelude.ark`
- `std/collections/string.ark`（新規または拡張）
- `crates/ark-typecheck/src/checker.rs`
- `crates/ark-mir/src/lower.rs`
- `crates/ark-wasm/src/emit.rs`

### 関連ドキュメント

- `docs/stdlib/README.md`
- `docs/stdlib/core.md`
- `docs/process/v0-status.md`
- `docs/process/parser-ark-evaluation.md`

### TODO

- [ ] `String_from` と `eq` の source wrapper を prelude から string module へ整理する
- [ ] `concat` を runnable にするか、未実装として境界を明示する
- [ ] `slice` / `split` / `join` のどこまで source 化できるか切り出す
- [ ] parser/self-hosting 候補に必要な String utility を棚卸しする
- [ ] String API の public surface と intrinsic/runtime boundary を分ける

---

## 6. Option / Result の payload foundation を実装する

### 目的

`Option<T>` / `Result<T, E>` を source stdlib に移す前提として、payload variant を end-to-end で扱えるようにする。

### 対象コード

- `crates/ark-typecheck/src/checker.rs`
- `crates/ark-mir/src/lower.rs`
- `crates/ark-mir/src/mir.rs`
- `crates/ark-wasm/src/emit.rs`
- `std/core/option.ark`
- `std/core/result.ark`

### 関連ドキュメント

- `docs/process/v0-status.md`
- `docs/stdlib/README.md`
- `docs/stdlib/core.md`

### TODO

- [ ] tuple enum payload lowering を実装する
- [ ] `Some(x)` / `Ok(x)` / `Err(x)` の end-to-end 動作を安定化する
- [ ] payload binding を伴う `match` を runnable にする
- [ ] `Option` / `Result` helper の最低限 (`is_some`, `is_none`, `unwrap_or`) を source 化可能にする
- [ ] `?` 演算子対応の前提条件を揃える

---

## 7. Vec runtime と Vec stdlib を実装する

### 目的

`Vec<T>` を「名前だけある状態」から、少なくとも v0 必須操作が動く状態にする。

### 対象コード

- `std/collections/vec.ark`
- `crates/ark-typecheck/src/checker.rs`
- `crates/ark-mir/src/lower.rs`
- `crates/ark-wasm/src/emit.rs`
- `crates/ark-stdlib/src/lib.rs`

### 関連ドキュメント

- `docs/stdlib/README.md`
- `docs/stdlib/core.md`
- `docs/process/v0-status.md`
- `docs/quickstart.md`

### TODO

- [ ] Vec の runtime representation を固定する
- [ ] `Vec_new_*`, `push`, `pop`, `get`, `set`, `len` を runnable にする
- [ ] `Vec<String>` と `Vec<i32>` の動作境界を明確にする
- [ ] `get_unchecked` の扱いを決める
- [ ] `quickstart` の Vec 章が動く状態に近づける

---

## 8. Prelude 再設計を行う

### 目的

prelude へ何を自動 import するかを、「設計」ではなく「現実装」に合わせて固定する。

### 対象コード

- `std/prelude.ark`
- `crates/ark-resolve/src/resolve.rs`
- `crates/ark-typecheck/src/checker.rs`

### 関連ドキュメント

- `docs/stdlib/README.md`
- `docs/process/v0-status.md`
- `docs/process/v0-scope.md`
- `docs/FREEZE-v0-READY.md`

### TODO

- [ ] 現実に prelude へ置くべき最小集合を決める
- [ ] type names / constructors / helper functions の境界を整理する
- [ ] auto-import と explicit import の役割を docs に反映できる形にする
- [ ] future modules (`collections`, `io`) との責務分離を定義する

---

## 9. I/O stdlib の source 化準備をする

### 目的

`io/fs`, `io/clock`, `io/random` を source 化する前に、runtime boundary と capability API を整理する。

### 対象コード

- `std/io/fs.ark`
- `std/io/clock.ark`
- `std/io/random.ark`
- `crates/ark-wasm/src/emit.rs`
- `crates/arukellt/src/main.rs`

### 関連ドキュメント

- `docs/stdlib/io.md`
- `docs/stdlib/README.md`
- `docs/process/v0-status.md`
- `docs/platform/wasi-resource-model.md`

### TODO

- [ ] capability-based I/O を intrinsic/runtime/import のどこに置くか決める
- [ ] `fs_read_file`, `fs_write_file` などの公開面を source module へ移す前提を作る
- [ ] `clock` / `random` の低レベル境界を整理する
- [ ] `main(caps: Capabilities)` 系 API を実装 reality と揃える

---

## 10. 高階関数・closure 依存 API を後続で解放する

### 目的

`map/filter/fold` など、いま docs 先行の API を source stdlib 化できる前提を作る。

### 対象コード

- `crates/ark-typecheck/src/checker.rs`
- `crates/ark-mir/src/lower.rs`
- `crates/ark-wasm/src/emit.rs`
- `std/collections/vec.ark`
- `std/core/option.ark`
- `std/core/result.ark`

### 関連ドキュメント

- `docs/process/v0-status.md`
- `docs/stdlib/README.md`
- `docs/stdlib/core.md`
- `docs/quickstart.md`

### TODO

- [ ] closure typing を実装する
- [ ] higher-order call lowering を実装する
- [ ] `map/filter/fold` を source stdlib で書けるようにする
- [ ] `unwrap_or_else` や `and_then` のような API を後続で有効化する

---

## 11. 影響範囲テストを本物のタスクにする

### 目的

「通った気がする」ではなく、source stdlib migration の影響範囲を実際に継続検証できる状態にする。

### 対象コード

- `tests/harness.rs`
- `tests/fixtures/modules/`
- `tests/fixtures/`
- `scripts/verify-harness.sh`
- `.github/workflows/ci.yml`

### 関連ドキュメント

- `docs/process/v0-status.md`
- `docs/process/v0-scope.md`
- `README.md`

### TODO

- [ ] `tests/harness.rs` の compile/run 比較 TODO を実装する
- [ ] module import 系 fixture を追加する
- [ ] source-backed prelude fixture を追加する
- [ ] stdlib migration の影響を CI で継続確認できるようにする
- [ ] `scripts/verify-harness.sh` と実テスト内容の意味を一致させる

---

## 12. docs を実装 reality に合わせて整備する

### 目的

source stdlib migration が一定段階まで進んだ後、docs と実装を再同期する。

### 対象ドキュメント

- `docs/process/v0-status.md`
- `docs/stdlib/README.md`
- `docs/stdlib/core.md`
- `docs/stdlib/io.md`
- `docs/quickstart.md`
- `docs/process/v0-scope.md`
- `docs/integrity.md`
- `README.md`

### 関連コード

- `std/prelude.ark`
- `std/core/*.ark`
- `std/collections/*.ark`
- `std/io/*.ark`
- `crates/ark-resolve/src/resolve.rs`
- `crates/ark-typecheck/src/checker.rs`
- `crates/ark-wasm/src/emit.rs`

### TODO

- [ ] `docs/process/v0-status.md` を最新 stage に更新する
- [ ] `docs/stdlib/README.md` に「Rust builtin から source stdlib へ移行中」の reality を反映する
- [ ] `docs/quickstart.md` のサンプルを runnable 範囲へ寄せる
- [ ] `docs/process/v0-scope.md` の ✅ を実装 reality に合わせる
- [ ] `docs/integrity.md` の該当項目を消し込めるようにする

---

## 実行順まとめ

1. source-backed prelude の runtime 破綻を直す
2. module loading を「使える実装」へ固める
3. public stdlib 名と intrinsic 名の境界を固定する
4. `std/prelude.ark` を実ファイルの source of truth にする
5. String 系 stdlib を Arukellt 側へ移す
6. Option / Result の payload foundation を実装する
7. Vec runtime と Vec stdlib を実装する
8. Prelude 再設計を行う
9. I/O stdlib の source 化準備をする
10. 高階関数・closure 依存 API を後続で解放する
11. 影響範囲テストを本物のタスクにする
12. docs を実装 reality に合わせて整備する
