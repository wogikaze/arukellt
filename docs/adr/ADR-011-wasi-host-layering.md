# ADR-011: WASI ホスト API は version ではなく capability で名前付けする

ステータス: **DECIDED**

決定日: 2026-03-29

---

## 文脈

Arukellt は現在、少なくとも 2 つの実用ターゲットを持つ。

- T1: `wasm32-wasi-p1` — compatibility path
- T3: `wasm32-wasi-p2` — canonical path

これまでは「同じソースコードが T1/T3 の両方でだいたい動く」ことを強く意識してきた。
しかし今後 `wasi:http`, `wasi:sockets`, native Preview 2 component imports のような
P2 以降にしか存在しない host capability を扱うと、
その前提を stdlib 全体にまで拡張するのは無理がある。

このとき設計候補として次の 2 つがある。

1. `std::wasi::p1::*`, `std::wasi::p2::*` のように version で分ける
2. `std::wasi::cli`, `std::wasi::filesystem`, `std::wasi::http` のように capability で分ける

また ADR-009 により、Arukellt ソースの `use std::...` と、
Component Model / WIT 境界の `import "wasi:..."` は別層として扱う方針が既に決まっている。

---

## 決定

**WASI ホスト API は `std::wasi::<capability>` で提供し、`std::wasi::p1` / `std::wasi::p2` のような version namespace は作らない。**

### 1. `std::*` は portable stdlib 層

`std::io`, `std::fs`, `std::path`, `std::env`, `std::process`, `std::time`, `std::random`
のようなモジュールは、可能な限り target-neutral な surface を提供する。

これらは T1/T3 の両方で意味を持つ範囲を優先し、backend 差分は実装側で吸収する。

### 2. `std::wasi::<capability>` は target-gated host API 層

WASI に強く結び付いた host capability は `std::wasi` 配下に置く。

想定モジュール:

- `std::wasi::cli`
- `std::wasi::filesystem`
- `std::wasi::clocks`
- `std::wasi::random`
- `std::wasi::http`
- `std::wasi::sockets`

### 3. version 差分は module 名ではなく target support matrix で表現する

たとえば `std::wasi::http` は T3/T5 系ターゲット専用であり、T1 では使えない。
一方 `std::wasi::cli` や `std::wasi::filesystem` は、
意味が揃う限り T1/P1 と T3/P2 に別 backend 実装を持ってよい。

### 4. WIT / Component 境界は引き続き `import "wasi:..."` で表現する

`std::wasi::http` は Arukellt から見た convenience wrapper / facade であり、
WIT package identifier そのものではない。

WIT interface や world を直接参照する場合は ADR-009 に従い、
`import "wasi:http/proxy@..."` のような Layer C 構文で扱う。

### 5. 未対応 capability は compile-time error にする

ある module が target に対応していない場合、
ランタイム no-op や暗黙 fallback ではなく compile-time error とする。

例:

- T1 で `use std::wasi::http` → error
- T1 で `use std::wasi::sockets` → error

---

## 理由

1. **利用者が version ではなく能力で考えられる**
   `http` を使いたいのに `p1/p2/p3` を先に意識させるのは API として不親切。

2. **将来の p3 移行で namespace 爆発を防げる**
   `std::wasi::p3::*` を増やすより、support matrix を更新する方が単純。

3. **T1/T3 の共有可能部分だけを共有できる**
   `cli`, `filesystem`, `clocks`, `random` は共通 surface を持ちうるが、
   `http` は T1 では共有不可能である。capability 切りならそれを自然に表現できる。

4. **ADR-009 と整合する**
   `use std::...` の source-level module と
   `import "wasi:..."` の WIT boundary を混同しない。

5. **LLM にとって誤用しにくい**
   `std::wasi::http` は host capability module、
   `import "wasi:http/..."` は interface boundary、
   `std::io` は portable stdlib という三層が名前で分かれる。

---

## 結果

- 「T1/T3 完全互換」を stdlib 全面では前提にしない
- 「portable subset の互換」を維持対象にする
- target-specific API は explicit に分離する
- `wasi:http` / `wasi:sockets` のような将来機能を、
  T1 互換の都合で stdlib 設計から外す必要がなくなる

---

## 例

### Portable stdlib

```ark
use std::fs
use std::path

let config = fs::read_to_string(path::join(".", "config.txt"))
```

これは可能な限り T1/T3 両方で意味を揃える。

### Target-gated host API

```ark
use std::wasi::http

let res = http::send(request)?
```

これは T3/T5 系では有効だが、T1 では compile-time error になる。

### Direct WIT boundary

```ark
import "wasi:http/proxy@0.2.10"
```

これは stdlib import ではなく、Component Model 境界宣言である。

---

## 不採用案

### A. `std::wasi::p1` / `std::wasi::p2` / `std::wasi::p3`

却下理由:

- capability より version を露出してしまう
- p3 導入時に module tree が膨張する
- `cli` / `filesystem` のような共通面が二重化しやすい

### B. `std::http` / `std::sockets` をそのまま追加

却下理由:

- portable stdlib と target-gated host API の境界が曖昧になる
- T1 での可用性を誤解しやすい

---

## 関連

- ADR-007: コンパイルターゲット整理
- ADR-009: Import 構文の決定
- `docs/stdlib/std.md`
- `issues/open/074-wasi-p2-native-component.md`
- `issues/open/077-wasi-p2-http.md`
