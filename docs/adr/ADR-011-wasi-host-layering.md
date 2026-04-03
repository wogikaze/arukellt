# ADR-011: host-bound stdlib API は `std::host::*` に隔離する

ステータス: **DECIDED**

決定日: 2026-03-29

---

## 文脈

Arukellt は少なくとも 2 つの実用ターゲットを持つ。

- T1: `wasm32-wasi-p1` — compatibility path
- T3: `wasm32-wasi-p2` — canonical path

従来は `std::io`, `std::fs`, `std::env`, `std::process`, `std::time`, `std::random`
のような名前で host-bound API を提供してきた。しかし実際には、
これらの API は pure stdlib ではなく WASI host capability を経由しないと実現できない。

この状態には 3 つの問題がある。

1. `std::io` という名前が pure/portable な抽象に見える
2. T1/T3 で使える範囲の差が API 名から読めない
3. LLM と人間の両方が `std::*` と `import "wasi:..."` を混同しやすい

また ADR-009 により、Arukellt ソースの `use std::...` と、
WIT / Component 境界の `import "wasi:..."` は別層として扱う方針が既に決まっている。

---

## 決定

**host-bound な標準ライブラリ API は `std::host::*` に置き、`std::*` 直下からは外す。**

### 1. `std::*` は pure または host-agnostic な層

`std::*` 直下には、host access を伴わない API だけを置く。

例:

- `std::core`
- `std::text`
- `std::bytes`
- `std::collections`
- `std::seq`
- `std::path`
- `std::time` (duration arithmetic only)
- `std::random` (deterministic / seeded only)

`std::time` は host clock を読まない。`std::random` は host entropy を読まない。

### 2. `std::host::*` は explicit な host capability 層

host に触る API はすべて `std::host` 配下へ移す。

想定モジュール:

- `std::host::stdio`
- `std::host::fs`
- `std::host::env`
- `std::host::process`
- `std::host::clock`
- `std::host::random`
- `std::host::http`
- `std::host::sockets`

### 3. `std::host::*` と `std::*` の重複 surface は作らない

同じ責務の API を 2 箇所に置かない。

例:

- `stdin/stdout/print/println` は `std::host::stdio` のみ
- `args/var` は `std::host::env` のみ
- `exit/abort` は `std::host::process` のみ
- `monotonic_now` は `std::host::clock` のみ
- nondeterministic random は `std::host::random` のみ

`std::io` のように「抽象 I/O に見えるが実際は host 直結」という module は current API として残さない。

### 4. version 差分は namespace ではなく target support matrix で表現する

`std::host::http` や `std::host::sockets` のような P2-only capability は、
module 名に `p1/p2` を埋め込まず support matrix で表す。

例:

- T1 で `use std::host::http` → compile-time error
- T1 で `use std::host::sockets` → compile-time error
- T3 で `std::host::stdio/fs/env/process/clock/random` → 実行可能

### 5. raw WIT / Component 境界は引き続き `import "wasi:..."` で表現する

`std::host::http` は user-facing facade であり、WIT package identifier そのものではない。

raw interface/world を直接使う場合は、ADR-009 に従い
`import "wasi:http/proxy@..."` のような構文で扱う。

---

## 理由

1. **名前が責務を正しく表す**
   `std::host::fs` は host filesystem だと一目で分かるが、`std::fs` は pure/path-like に誤読されやすい。

2. **T1/T3 の非対称性を自然に表現できる**
   shared host surface と P2-only capability を同じ設計で扱える。

3. **LLM にとって誤用しにくい**
   `std::time` と `std::host::clock`、`std::random` と `std::host::random` の責務が名前で分かれる。

4. **将来 backend が増えても public API を保ちやすい**
   backend 名や WASI version を public namespace に露出しないため、support matrix の更新だけで済む。

5. **ADR-009 と整合する**
   `use std::host::*` は source-level facade、`import "wasi:..."` は component boundary と明確に分けられる。

---

## 結果

- `std::io`, `std::fs`, `std::env`, `std::process`, `std::cli` は current API から外れる
- 旧 import は compile-time error で `std::host::*` への移行先を案内する
- host-touching prelude function (`println`, `print`, `eprintln`, `clock_now`, `random_i32`, `fs_read_file`, `fs_write_file`) は current API から外れる
- pure / host / WIT boundary の三層が名前で分かれる

---

## 例

### Pure stdlib

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::time
use std::random

let elapsed = time::duration_ms(start, end)
let shuffled = random::shuffle_i32(values, 42)
```

### Host facade

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::host::stdio
use std::host::fs
use std::host::clock

let text = fs::read_to_string("input.txt")?
stdio::println(text)
let now = clock::monotonic_now()
```

### P2-only host facade

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::host::http

let response = http::get("https://example.com")?
```

これは T3/T5 系では有効だが、T1 では compile-time error になる。

### Direct WIT boundary

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
import "wasi:http/proxy@0.2.10"
```

これは stdlib import ではなく、Component Model 境界宣言である。

---

## 不採用案

### A. `std::wasi::p1` / `std::wasi::p2`

却下理由:

- version を public API に露出してしまう
- capability ではなく backend 名で考えさせてしまう
- 将来の namespace 爆発を招く

### B. `std::wasi::<capability>`

却下理由:

- user-facing API に backend 名を露出する必要がない
- WASI 以外の host backend を追加したときに不自然
- 利用者が欲しいのは `filesystem` や `http` であって `wasi` ではない

### C. `std::*` に host API を残す

却下理由:

- `std::io` のような名前が pure/portable 抽象に見える
- `std::time` と host clock の責務が混ざる
- LLM が portable subset と host capability を区別しにくい

---

## 関連

- ADR-007: コンパイルターゲット整理
- ADR-009: Import 構文の決定
- `docs/stdlib/std.md`
- `issues/open/077-wasi-p2-http.md`
- `issues/open/139-std-wasi-sockets-p2.md`
