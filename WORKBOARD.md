# WORKBOARD

This file is the shared AI-managed task queue for the repository.
AI updates it; humans primarily read it.
It is the canonical place to park follow-up work, pick the next vertical slice, and record verified completion.

## Operating Rules

- Read this file before substantial planning or implementation work.
- Keep exactly one queue item in `Next`. If it becomes stale, promote the highest-priority unblocked item from `Ready`.
- Add newly discovered work to `Ready` unless a concrete dependency blocks it. Put dependency-gated work in `Blocked`.
- Keep task IDs stable as `WB-###`.
- Keep tasks as small vertical slices with one clear outcome.
- Move an item to `Done` only after the matching verification command or test has been run.
- When a task splits, add a follow-up item instead of mutating the old item beyond recognition.
- Keep `Done` entries concise and newest-first.
- Update this file in the same change when work starts, gets blocked, discovers follow-up tasks, or completes.

## Task Schema

Use this exact field order for every task:

### WB-000
title: Example task title
area: workflow
status: READY
priority: P2
owner: unassigned
depends_on: none
source: where this task came from
done_when:
- concrete verification outcome
notes:
- short context for future agents

Field rules:

- `status`: one of `NEXT`, `READY`, `BLOCKED`, `DONE`
- `priority`: one of `P0`, `P1`, `P2`, `P3`
- `owner`: `unassigned`, `ai`, or a short agent label
- `depends_on`: `none` or one or more `WB-###` identifiers
- `source`: file path, test name, user request, or other concrete origin
- `done_when`: 1 to 3 concrete checks
- `notes`: short bullets; newest note first if there are multiple notes

## Next

### WB-026
title: wasm-wasi の heap object codegen で nested allocation 時の base pointer 破壊を潰す
area: wasm-backend
status: NEXT
priority: P2
owner: unassigned
depends_on: none
source: WB-013 で `emit_iter_step` が nested tuple allocation 後に誤った base pointer を返していた
done_when:
- nested allocation を含む heap-backed emitters が `heap_ptr - size` 再計算ではなく固定 base pointer で返る
- nested payload を持つ構築パスに回帰テストが追加される
notes:
- `emit_iter_step` は scratch local で修正済み; 同じパターンが `emit_construct` などにも潜む可能性がある

## Ready

### WB-027
title: arktc build に `--target wat` を追加し WAT テキストを直接出力できるようにする
area: arktc
status: READY
priority: P2
owner: unassigned
depends_on: none
source: ユーザー要求「target に wat も欲しい」
done_when:
- `arktc build file.ar --target wat` が WAT テキストを stdout に出力する
- `arktc build file.ar --target wat --output out.wat` がファイルに書き出す
- `cargo test -p arktc` が通る新規テストが存在する
notes:
- `emit_wasm` の前に WAT 文字列を生成するステップが既にある; それを `--output` に書くだけでよい
- `--output` 省略時は stdout への印字が自然 (wasm-js/wasm-wasi と違いバイナリでないため)
- `lang-backend-wasm` に `emit_wat(module, target) -> Result<String>` を公開し `arktc` から呼ぶ設計が最小変更
- `wat` / `wasm-js` / `wasm-wasi` と並ぶ第三の target 文字列として CLI に追加する

## Blocked

### WB-008

title: Record a deployed GitHub Pages smoke URL for the docs shell
area: docs/release
status: BLOCKED
priority: P3
owner: unassigned
depends_on: repo-level GitHub Pages configuration
source: docs app shell exists, but deployment settings live outside the workspace
done_when:

- Pages source is configured
- the deployed URL is documented in the repo
- a smoke pass is recorded against the deployed site
notes:
- blocked on repository settings rather than code in this worktree

## Done

### WB-013
title: wasm-wasi でイテレータ (Seq<T> / iter.unfold / take) を実装
area: wasm-backend
status: DONE
priority: P3
owner: ai
depends_on: WB-011, WB-012
source: infinite_iter.ar が `unsupported wasm type: Seq<Int>` で失敗
done_when:
- `infinite_iter.ar` が `arktc build --target wasm-wasi` で成功
- `wasmer` 実行結果が infinite_iter.stdout と一致
notes:
- verified with `cargo fmt`, `cargo test -p lang-backend-wasm`, `cargo test -p arktc --test examples`, `cargo run -p arktc -- build example/infinite_iter.ar --target wasm-wasi --output /tmp/infinite_iter.wasm`, `wasmer /tmp/infinite_iter.wasm`, and `cargo test`
- wasm-wasi now lowers `Seq<i64>` from `iter.unfold` into heap-backed iterator objects, materializes `take` into `List<i64>`, and supports tuple state plus `Next` / `Done` step objects

### WB-014
title: wasm-wasi で WASI ファイル I/O (fs.read_text) を実装
area: wasm-backend
status: DONE
priority: P3
owner: ai
depends_on: WB-012
source: file_read.ar が match 不一致エラーで失敗 (Result<String, ReadError> のペイロード ADT が前提)
done_when:
- `file_read.ar` が `arktc build --target wasm-wasi` で成功
- `wasmer` に hello.txt を渡した実行結果が file_read.stdout と一致
notes:
- verified with `cargo test -p lang-backend-wasm --test build --test examples`, `cargo run -p arktc -- build example/file_read.ar --target wasm-wasi --output /tmp/file_read.wasm`, `wasmer run --dir=. /tmp/file_read.wasm`, and `cargo test`
- wasm-wasi now lowers `fs.read_text` through WASI `path_open` + `fd_read` + `fd_close`, returning heap-backed `Result<String, ReadError>` values that existing match/ADT lowering can consume

### WB-023
title: wasm-wasi で List<i64> 向け map/filter/sum の組み込み lowering を追加
area: wasm-backend
status: DONE
priority: P1
owner: ai
depends_on: WB-021, WB-022
source: `map_filter_sum.ar` が `calls to \`sum\`` で失敗; list collection builtins 未実装
done_when:
- `map_filter_sum.ar` が `arktc build --target wasm-wasi` で成功する
- `wasmer` 実行結果が `map_filter_sum.stdout` と一致する
- backend tests cover `map`, `filter`, `sum` の `List<i64>` happy path
notes:
- verified with `cargo fmt`, `cargo test -p lang-backend-wasm`, `cargo test -p arktc --test examples`, `cargo run -p arktc -- build example/map_filter_sum.ar --target wasm-wasi --output /tmp/map_filter_sum.wasm`, `wasmer /tmp/map_filter_sum.wasm`, and `cargo test`
- wasm-wasi now lowers `List<i64>.map`, `List<i64>.filter`, and `List<i64>.sum` with closure-backed callbacks and linear-memory list buffers

### WB-010
title: wasm-wasi で List リテラルと基本コレクション操作 (map / filter / join / sum) を実装
area: wasm-backend
status: DONE
priority: P1
owner: ai
depends_on: WB-021, WB-022, WB-023, WB-024
source: powers.ar, fizz_buzz.ar, map_filter_sum.ar が `calls to join` / `calls to sum` で失敗
done_when:
- `powers.ar` が `arktc build --target wasm-wasi` で成功し `wasmer` 実行結果が powers.stdout と一致
- `fizz_buzz.ar` と `map_filter_sum.ar` も同様に一致
notes:
- umbrella slice is now closed by `WB-021` through `WB-024`; `powers.ar`, `fizz_buzz.ar`, and `map_filter_sum.ar` all build on wasm-wasi and match their stdout fixtures
- implementation landed as four vertical slices so callback ABI, list runtime, numeric collection ops, and string joining stayed independently verifiable

### WB-012
title: wasm-wasi でペイロード付き ADT (Result / 任意コンストラクタ) を実装
area: wasm-backend
status: DONE
priority: P2
owner: ai
depends_on: WB-009
source: result_error_handling.ar が `ADT payload fields are not yet supported in wasm backend` で失敗
done_when:
- `result_error_handling.ar` が `arktc build --target wasm-wasi` で成功
- `wasmer` 実行結果が result_error_handling.stdout と一致
notes:
- verified with `cargo fmt`, `cargo test`, `cargo run -p arktc -- build example/result_error_handling.ar --target wasm-wasi --output /tmp/result_error_handling.wasm`, and `wasmer /tmp/result_error_handling.wasm`
- wasm-wasi ADTs now lower to heap-backed `{tag, field0, ...}` objects, and match pattern bindings read payload fields by offset

### WB-025
title: arkli — GHCi 風インタラクティブ REPL の新規バイナリを追加する
area: repl
status: DONE
priority: P2
owner: ai
depends_on: none
source: ユーザー指摘「chef run で実行できるが Haskell インタープリタ風の arkli がない」
done_when:
- `cargo run -p arkli` で起動し、式を入力すると評価結果が表示される
- `:load file.ar` でファイルをロードし、その関数を呼び出せる
- `:quit` / `:q` で終了できる
- `cargo test -p arkli` が通る基本的な REPL ループテストが存在する
notes:
- verified with `cargo test -p arkli`, `printf 'let base = 40\nbase + 2\n:q\n' | cargo run -q -p arkli`, and `cargo test`
- added new `arkli` binary with expression evaluation, persistent session `let` bindings, `:load`, `:reload`, `:type`, and `:quit` / `:q`

### WB-024
title: wasm-wasi に heap-backed String 連結を追加し join を実装
area: wasm-backend
status: DONE
priority: P1
owner: ai
depends_on: WB-021, WB-022
source: `powers.ar` / `fizz_buzz.ar` が `calls to \`join\`` で失敗; 現状 String は literal pointer のみ
done_when:
- `List<String>.join(sep)` の wasm-wasi lowering が追加される
- `powers.ar` と `fizz_buzz.ar` が `arktc build --target wasm-wasi` で成功する
- `wasmer` 実行結果が各 `.stdout` fixture と一致する
notes:
- verified with `cargo fmt`, `cargo test`, `cargo run -p arktc -- build example/powers.ar --target wasm-wasi --output /tmp/powers.wasm`, `wasmer /tmp/powers.wasm`, `cargo run -p arktc -- build example/fizz_buzz.ar --target wasm-wasi --output /tmp/fizz_buzz.wasm`, and `wasmer /tmp/fizz_buzz.wasm`
- wasm-wasi strings now use heap-backed writable buffers for `string()` and `join()`, while `console.println` keeps consuming NUL-terminated UTF-8 pointers

### WB-022
title: wasm-wasi で関数参照と unary closure 呼び出しをコレクション組み込みから使えるようにする
area: wasm-backend
status: DONE
priority: P1
owner: ai
depends_on: WB-011
source: `map(fizz_buzz_label)` / `map(n -> ...)` / `filter(n -> ...)` が collection lowering の前提
done_when:
- named function referencesと unary closure values を collection helper 呼び出しに渡せる
- wasm backend tests cover one named-function and one lambda callback path
- 非対応の多引数/高階値ケースは明示的に拒否されたまま
notes:
- verified with `cargo test -p lang-backend-wasm --test build` and `cargo test`
- collection helpers now accept unary named-function references and lambda closures through the shared closure object ABI, while non-unary and higher-order callback cases remain hard errors

### WB-021
title: wasm-wasi に最小 List<i64> ランタイムを追加し list literal / range を lower する
area: wasm-backend
status: DONE
priority: P1
owner: ai
depends_on: WB-009
source: `HighExprKind::List` が wasm backend で即 bail されている; `powers.ar` / `map_filter_sum.ar` の前提
done_when:
- `List<i64>` を線形メモリ上の `{len, items_ptr}` 表現で保持できる
- list literal と `range_inclusive` の wasm-wasi codegen テストが追加される
- 依然未対応の higher-order / string collection 操作は明示的に失敗する
notes:
- verified with `cargo test -p lang-backend-wasm`, `cargo test -p arktc --test examples`, and `cargo test`
- list values now lower to heap-backed `{len, items_ptr}` objects on wasm-wasi
- backend tests cover both int list literals and `range_inclusive`

### WB-011
title: wasm-wasi でクロージャ (Lambda / Fn<A,B>) を実装
area: wasm-backend
status: DONE
priority: P2
owner: ai
depends_on: WB-009
source: closure.ar が `unsupported wasm type: Fn<Int, Int>` で失敗
done_when:
- `closure.ar` が `arktc build --target wasm-wasi` で成功
- `wasmer` 実行結果が closure.stdout と一致
notes:
- verified with `cargo run -p arktc -- build example/closure.ar --target wasm-wasi --output /tmp/closure.wasm` and `wasmer /tmp/closure.wasm`
- wasm backend now lowers unary closures as heap objects `{table_index, captures...}` and applies them via `call_indirect`

### WB-009
title: wasm-wasi で Let バインディングを実装し factorial/fibonacci 以外の単純パイプラインを解放する
area: wasm-backend
status: DONE
priority: P1
owner: ai
depends_on: none
source: HighExprKind::Let が emit_expr で bail! されている; powers.ar 等で必要
done_when:
- `let x = ...; body` 形式の HighExprKind::Let が WASI ターゲットで WAT local として emit される
- `cargo test -p lang-backend-wasm` が全 pass
notes:
- verified with `cargo fmt`, `cargo test -p lang-backend-wasm`, `cargo test -p arktc --test examples`, and `cargo test`
- wasm backend now lowers nested `HighExprKind::Let` bindings into declared locals with `local.set` / `local.get`, including shadow-safe local renaming

### WB-015

title: wasm-wasi で console.println と string() ビルトインを実装し hello_world / factorial / fibonacci を解放
area: wasm-backend
status: DONE
priority: P1
owner: ai
depends_on: none
source: ユーザー要求 "arktc build --target wasm-wasi で予想通りの出力ができるまで開発を進めてください"
done_when:
- `wasmer example/hello_world.wasm` → `Hello, world!`
- `wasmer example/factorial.wasm` → `3628800`
- `wasmer example/fibonacci.wasm` → `55`
notes:
- commit 61b3bd6: fd_write import + $console.println helper + $string helper 実装
- scratch memory 28 bytes (iovec + nwritten + newline + str_buf) を string table 末尾に確保
- matrix.json で 3 例を wasm_wasi_build: true に更新; 全テスト green
