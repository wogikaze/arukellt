見た。これは **Arukellt selfhost compiler + stdlib + docs/issues のリポジトリ**。
今やるべき設計の核心はこれ。

**「ファイル分割」ではなく、「変更理由・データ契約・依存方向・コンテキスト単位」を揃えて、LLM/人間が 1 回の作業で読める閉じた領域を作ること。**

現状の危険箇所はだいたい以下。

| 領域                                       |         現状 |
| ---------------------------------------- | ---------: |
| `src/compiler/component_emitter.ark`     | 約 10,316 行 |
| `src/compiler/mir_lower.ark`             |  約 4,528 行 |
| `src/compiler/emit_intrinsic_string.ark` |  約 3,788 行 |
| `src/compiler/emitter.ark`               |  約 3,569 行 |
| `src/compiler/emit_intrinsic_math.ark`   |  約 3,162 行 |
| `src/compiler/parser.ark`                |  約 2,757 行 |
| `src/compiler/driver.ark`                |  約 2,660 行 |
| `src/compiler/emit_intrinsic_io.ark`     |  約 1,676 行 |
| `src/compiler/typechecker.ark`           |  約 1,605 行 |

さらに重要なのは、`corehir.ark` がまだ本物の中間表現境界ではなく、実質 **AST + TypeCheckResult を MIR lowering に渡す薄いファサード**になっている点。ここが結合度の震源。
あと `driver.ark` の `load_single_module()` は現状 `<base>/<module>.ark` しか読まず、`foo::bar` → `foo/bar.ark` や `foo/bar/mod.ark` の fallback が未実装に見える。既存の `docs/superpowers/specs/2026-04-28-selfhost-module-splitting.md` はまさにこの分割を予定しているが、現コードはまだそこまで行っていない。

---

# 1. まず思想：このプロジェクトでの低結合設計の正体

低結合とは「import 数が少ない」ことではない。
このプロジェクトでは次の 4 つを満たすこと。

1. **上流表現が下流に漏れない**
   MIR lowering が parser の `AstNode` や typechecker の内部 `TypeCheckResult` に依存し続けると、parser/typechecker の変更が MIR/backend に波及する。
   本来は `Parser → Typed AST/CoreHIR builder → CoreHIR → MIR` のように、下流は CoreHIR 契約だけを見る。

2. **変更理由ごとにモジュールを閉じる**
   例：component model の WIT 型追加で `driver.ark` と `component_emitter.ark` と `parser.ark` と `mir_lower.ark` を同時に読む必要があるなら、境界が壊れている。

3. **公開 API を小さくする**
   `pub` は便利だから付けるものではなく、外部に固定する契約。
   各領域の `mod.ark` は「全部 re-export」ではなく「意図した入口だけ」を出す。

4. **LLM 作業単位を設計上の単位にする**
   1 issue / 1 PR / 1 agent context が、最大でも「1〜3 ファイル + 1 契約文書 + 1 fixture 群」で閉じるようにする。
   10k 行ファイルを分けても、変更時に全ファイルを同時に読む必要があるなら、分割ではなく粉砕。

---

# 2. このプロジェクトの理想依存グラフ

今後の依存方向はこう固定する。

```text
CLI / LSP / DAP / Playground
        ↓
Driver / Session / Workspace
        ↓
Source loading / Module graph
        ↓
Lexer → Parser → Resolver → Typechecker
        ↓
CoreHIR
        ↓
MIR
        ↓
Backend plan
        ↓
Wasm core emitter
        ↓
Component wrapper / WIT / ABI adapters
```

逆向き依存は禁止。

具体例：

* `mir_lower` は `parser` を import しない。
* `emitter` は `mir_lower` を import しない。
* `component_emitter` は `driver` の AST 判定ロジックを知らない。
* `analysis` / `lsp` は parser/resolver/typechecker を直接並べるのではなく、`Session` の query API を呼ぶ。
* `mir_ir` は `mir_dump` を import しない。現状 `mir_ir <-> mir_dump` の循環があるので、これは即座に切るべき。

---

# 3. 現状の最大問題を領域別に言う

## A. `corehir.ark` が本当の境界になっていない

現状はこういう形。

```ark
struct CoreHirProgram {
    decls: Vec<AstNode>,
    check_result: TypeCheckResult,
}
```

これは名前は CoreHIR だが、実体は「parser/typechecker の内部物を下流に通す箱」。
低結合にしたいならここを本物の DTO にする。

目標：

```text
parser::AstNode
  ↓
resolver/typechecker
  ↓
corehir::Program
  - modules
  - items
  - bodies
  - exprs
  - patterns
  - resolved call targets
  - typed locals
  - source spans
  - value mode
  - export metadata
  ↓
mir_lower は corehir だけを見る
```

CoreHIR に入る時点で、以下は消えているべき。

* parser の生 tag
* raw method-call syntax
* raw import syntax
* unresolved identifier
* type error
* parser/typechecker 内部構造
* AST child index 前提

CoreHIR に残してよいものは、下流が本当に必要な意味だけ。

---

## B. `driver.ark` が orchestration 以上のことをしすぎ

`driver.ark` は本来「フェーズをつなぐ」だけでよい。
でも現状は以下を抱えている。

* phase orchestration
* file loading
* module loading
* stdlib loading
* target/wasi 判定
* WIT text generation
* component export validation
* component shape detection
* timing output
* output mode branching
* diagnostics conversion
* source span line/column calculation

これは巨大になるのが自然。責務が多すぎる。

分けるなら：

```text
src/compiler/driver/
  mod.ark          // compile_file / compile_source だけ公開
  config.ark       // DriverConfig, target normalization
  result.ark       // CompileResult, phase tags
  session.ark      // Session artifact store
  pipeline.ark     // lex/parse/resolve/check/lower/emit の順序
  outputs.ark      // wasm/wat/wit/component output selection
  timing.ark       // phase timing
```

`driver` が直接 `component_has_*` や `component_fn_is_*` を持つのはやめる。
それは `component/contract.ark` へ移す。

---

## C. `component_emitter.ark` は「生成ロジック」と「具体 fixture adapter」が混ざっている

`component_emitter.ark` は 10k 行超え。
問題は長さだけではなく、以下が混ざっていること。

* component section ID
* WIT primitive encoding
* component writer
* type interner
* canonical ABI helper
* import/export section builder
* WASI stub core module
* 型ごとの adapter core module
* record/enum/tuple/list/option/result/string/f32 などの具体形
* component wrapping entrypoint

分けるなら：

```text
src/compiler/component/
  mod.ark             // emit_component だけ公開
  writer.ark          // ComponentWriter, LEB, section
  constants.ark       // section id, sort id, val type id
  wit_types.ark       // WIT type encoding
  type_interner.ark
  imports.ark
  exports.ark
  core_stub.ark       // WASI stub core module
  abi.ark             // canonical ABI lift/lower rules
  contract.ark        // export surface validation
  adapter_plan.ark    // function signature -> AdapterKind
  adapters/
    string.ark
    list.ark
    option.ark
    result.ark
    tuple.ark
    record.ark
    enum.ark
    numeric.ark
  emit.ark            // plan + writer を組み合わせる
```

ここで重要なのは、`driver` が「この関数は string unary っぽい」みたいな判定をしないこと。
`driver` は `ComponentExportPlan` を要求するだけ。

```text
driver
  → component::build_export_plan(corehir_or_mir_metadata)
  → component::emit_component(core_wasm, mir, plan, target)
```

---

## D. `mir_lower.ark` の `lower_expr` が大きすぎる

`lower_expr` が約 2,378 行。
これは典型的な「全部の言語機能を 1 関数に押し込んだ」状態。

やるべき分割はファイル分割より先に **lowering protocol の定義**。

必要な概念：

```text
LowerCtx
RValue        // 値を返す式
Place         // 代入先
ControlFlow   // break/continue/return/try の経路
BlockCursor   // 現在の MIR block
LoweredExpr   // local id + type + control state
```

分割例：

```text
src/compiler/mir_lower/
  mod.ark
  ctx.ark
  mono.ark
  functions.ark
  stmt.ark
  expr/
    mod.ark
    literals.ark
    locals.ark
    calls.ark
    binary.ark
    control.ark
    aggregate.ark
    match.ark
    try.ark
    closures.ark
  cfg.ark
  reachability.ark
```

ただし分割条件はこれ。

* `expr/calls.ark` が parser の raw call AST を見ない。
* `expr/match.ark` が typechecker 内部型を直接見ない。
* 全部 `CoreHIR` の式種別だけを見る。

つまり、`mir_lower` の分割は **CoreHIR 独立化の後** が本命。

---

## E. `emitter.ark` は section construction と instruction emission が混ざっている

`emitter.ark` の `emit_wasm` は約 2,008 行。
やっていることは多い。

* module header
* type section
* import section
* function section
* memory section
* global section
* export section
* code section
* data section
* start wrapper
* local allocation
* MIR inst dispatch
* intrinsic dispatch
* scratch memory layout
* string constant layout

分けるなら：

```text
src/compiler/wasm/
  mod.ark              // emit_wasm
  writer.ark           // 既存 emit_writer
  opcodes.ark
  sections.ark
  module_layout.ark
  string_data.ark
  scratch_layout.ark
  imports_wasi_p1.ark
  function_types.ark
  function_body.ark
  inst_dispatch.ark
  calls.ark
  validation_plan.ark
  intrinsics/
    mod.ark
    string.ark
    math.ark
    vec.ark
    io.ark
```

`emit_intrinsic_*` は既に分かれているが、まだ「生 Wasm writer を直接叩く巨大関数群」。
次にやるべきは、さらに細かいファイル分割ではなく **共通 bytecode recipe helper** の抽出。

例：

```text
wasm::mem_load_i32(ptr_expr)
wasm::mem_store_i32(addr, value)
wasm::call_import(name)
wasm::write_string_len(ptr)
wasm::loop_i32(init, cond, body)
wasm::copy_bytes(src, dst, len)
```

これで intrinsic は「何をするか」を書き、LEB/opcode の細部を毎回書かない。

---

## F. Parser / Resolver / Typechecker が型定義を重複している

`parser.ark` に AST。
`resolver.ark` に「AST Node duplicated from parser」。
`typechecker.ark` に「Resolve Context duplicated from resolver」。
これは文脈分割ではなく、境界がないための複製。

やるべき構造：

```text
src/compiler/syntax/
  span.ark
  token.ark
  token_kinds.ark
  ast.ark
  ast_kinds.ark
  lexer.ark
  parser.ark

src/compiler/semantic/
  symbols.ark
  symbol_kinds.ark
  types.ark
  type_kinds.ark
  resolve.ark
  check.ark
  result.ark
```

そして：

```text
resolver は syntax::ast を読む
typechecker は syntax::ast + semantic::symbols を読む
mir_lower は syntax::ast を読まない
```

---

# 4. コンテキスト長制約に合わせた「作業粒度」のルール

このプロジェクトでは、LLM/人間向けに次の制限を設けるとよい。

## モジュール制限

| 対象                         |        目安 |   ハード上限 |
| -------------------------- | --------: | ------: |
| 1 ファイル                     | 300〜700 行 | 1,000 行 |
| 1 関数                       |   20〜80 行 |   150 行 |
| 1 module の public function |    3〜12 個 |    20 個 |
| 1 issue で読む主要ファイル          |     1〜3 個 |     5 個 |
| 1 issue で変更する境界            |       1 個 |     2 個 |
| import fan-out             |       3〜8 |      12 |
| 循環依存                       |         0 |       0 |

現状はこの上限を大きく超えている。
特に `component_emitter.ark`, `mir_lower.ark`, `emitter.ark`, `driver.ark` は context overflow を前提にした構造。

## Issue の粒度

悪い issue：

```text
component model を改善する
MIR lowering を整理する
emitter を分割する
```

良い issue：

```text
component::writer を component_emitter.ark から抽出し、emit_component の public API を変えずに ComponentWriter と section emit helper だけ移動する
```

良い issue には必ずこれを入れる。

```text
Scope:
- touch: src/compiler/component_emitter.ark, src/compiler/component/writer.ark
- no behavior change
- public API unchanged: component_emitter::emit_component

Acceptance:
- existing component fixture outputs unchanged
- component_emitter.ark line count decreases
- new writer.ark has no dependency on driver/parser/mir_lower
```

---

# 5. 最優先のリファクタ順序

いきなり `component_emitter.ark` を割ると事故る。
順番が大事。

## Phase 0: 分割可能にする土台

### 0-1. `load_single_module()` を修正する

今の `driver.ark` はローカル module を `<base>/<name>.ark` でしか読んでいない。
まずこれを実装する。

```text
foo::bar::baz
  → <base>/foo/bar/baz.ark
  → <base>/foo/bar/baz/mod.ark
```

さらに単一 segment でも：

```text
emitter
  → emitter.ark
  → emitter/mod.ark
```

これがないと subdirectory 分割が本格的にできない。

### 0-2. `mod.ark` の export 規約を決める

`mod.ark` は何でも re-export する barrel ではない。
各領域の公開入口だけ出す。

例：

```text
component/mod.ark
  pub fn emit_component(...)
  pub fn validate_export_surface(...)
```

内部 helper は出さない。

---

## Phase 1: 循環依存と明白な境界漏れを切る

### 1-1. `mir_ir -> mir_dump` を切る

IR が dump に依存するのは逆。
`mir_dump` が `mir_ir` に依存するだけにする。

```text
Before:
mir_ir -> mir_dump
mir_dump -> mir_ir

After:
mir_ir
  ↑
mir_dump
```

### 1-2. span / diagnostics helper を共通化

`offset_to_line`, `offset_to_column`, diagnostic conversion が driver/analysis などに散っている。
`source_map.ark` または `diagnostic_render.ark` に寄せる。

---

## Phase 2: Driver を痩せさせる

順番：

```text
driver/result.ark
driver/config.ark
driver/module_loader.ark
driver/timing.ark
driver/output.ark
component/contract.ark
```

最初に切るべきは `component_has_*` / `component_fn_is_*` 群。
これは `driver` にあるべきではない。

```text
driver.ark
  compile_source(...)
    if emit_mode == component:
        component::validate_export_surface(...)
```

---

## Phase 3: Component emitter を分割する

安全な順番：

1. constants
2. writer
3. type interner
4. WIT type encoding
5. import/export section
6. WASI stub
7. adapter core module families
8. final `emit_component`

最初の 5 つは振る舞い変更なしで移動できる。
adapter family は fixture 差分が出やすいので後。

---

## Phase 4: Wasm emitter を分割する

順番：

1. `emit_wasm_module` の section ごとの関数化
2. scratch/data/string layout 抽出
3. call dispatch 抽出
4. intrinsic dispatch 抽出
5. instruction dispatch 抽出
6. `emit_wasm` を 100〜200 行の orchestration にする

最終形：

```text
pub fn emit_wasm(mir, target, opt_level) -> Vec<i32> {
    let plan = module_layout::plan(mir, target, opt_level)
    let ctx = emit_context::from_plan(plan)
    sections::emit_all(ctx)
}
```

---

## Phase 5: CoreHIR を本物にする

これは一番価値が高いが、一番危険。
先に driver/emitter/component の外側を整理してからやる。

目標：

```text
src/compiler/corehir/
  mod.ark
  ids.ark
  types.ark
  program.ark
  item.ark
  body.ark
  expr.ark
  pattern.ark
  selection.ark
  value_mode.ark
  source_map.ark
  builder.ark
  validate.ark
```

そして `corehir::build_program` は AST + typecheck result を受け取ってもよいが、下流には AST を出さない。

---

# 6. 低結合化のための具体パターン集

## 6-1. Stable dependency principle

安定したものへ依存する。
不安定なものへ依存しない。

このプロジェクトでは：

```text
parser AST        不安定
typechecker 内部  不安定
CoreHIR contract  安定させるべき
MIR IR            比較的安定
Wasm writer       安定
component ABI     仕様寄りで安定
```

だから `mir_lower` が parser AST に依存するのは危険。
`mir_lower` は安定化した CoreHIR に依存すべき。

---

## 6-2. Ports and Adapters

compiler core と外部世界を分ける。

外部世界：

* filesystem
* env vars
* process exit
* clock
* WASI imports
* component model tools
* LSP JSON-RPC
* DAP
* VS Code extension

内部 core：

* tokenize
* parse
* resolve
* typecheck
* lower
* emit plan

`driver` や `main` は adapter 側。
`corehir`, `mir`, `semantic` は core 側。
core 側が `std::host::fs`, `stdio`, `process`, `clock` を持ちすぎるとテスト不能になる。

目標：

```text
core functions:
  input data -> output data + diagnostics

adapter functions:
  read file / write file / print / exit
```

---

## 6-3. Anti-corruption layer

WIT / Component Model / WASI の外部概念を、compiler 内部へそのまま流さない。

例えば：

```text
WIT primitive id
Component section sort byte
Canonical ABI option byte
```

これらを `driver` や `MIR` が直接知ると汚染される。
`component` module の内側に閉じる。

内部では：

```text
ComponentType::String
ComponentType::List(I32)
ComponentType::Result(I32, String)
```

のような意味型にして、最後に byte encoding へ落とす。

---

## 6-4. Table-driven dispatch

現状の `component_fn_is_*` / `component_has_*` / `emit_*_component` は if 群で増殖しやすい。
言語機能が増えるほど爆発する。

理想はこれ。

```text
SignatureShape
  name
  params
  result
  adapter_kind
```

そして：

```text
detect_signature(fn) -> SignatureShape
plan_adapter(shape) -> AdapterPlan
emit_adapter(plan) -> bytes
```

Arukellt の言語制約で関数ポインタや trait dispatch が弱いなら、完全な table-driven にしなくてもよい。
最低限、判定を 1 箇所、emit を family ごとに分ける。

---

## 6-5. Characterization tests

巨大 refactor の前に、現状の振る舞いを固定するテストを作る。
これは「正しい仕様」ではなく「今と同じであること」を守るテスト。

対象：

* parser AST dump
* resolver scope dump
* typecheck typed functions dump
* MIR dump
* WAT dump
* Wasm bytes size / validation
* component bytes / WIT text
* diagnostics snapshot

特に `component_emitter` と `emitter` は、最初から意味的正しさで検証しようとすると辛い。
まず byte-for-byte / snapshot で現状保持を守る。

---

## 6-6. Contract tests

各境界に「入力契約・出力契約」のテストを置く。

例：

```text
Parser contract:
  source -> ParseResult
  errors contain spans
  no type info

Resolver contract:
  ParseResult + ModuleGraph -> ResolveResult
  all imports either loaded or diagnostic

Typechecker contract:
  AST + ResolveResult -> TypedProgram/CoreHIR input bundle
  no unresolved symbols

CoreHIR contract:
  no Error type
  all calls resolved
  all expressions have type
  all source spans present

MIR contract:
  no parser node dependency
  block ids valid
  locals valid
  no unreachable illegal blocks after prune

Wasm contract:
  produced bytes validate
```

---

## 6-7. Facade / Raw / Adapter の三層

既に stdlib docs にもあるが、compiler 側にも適用できる。

```text
Raw:
  byte writer, opcode, section byte, scratch address

Facade:
  emit string concat
  emit list adapter
  build function type

Adapter:
  WASI fd_write
  component canonical ABI
  LSP JSON-RPC
```

今は Raw が上位へ漏れている。
`emit_intrinsic_string.ark` などが生 opcode を直接並べているため、変更時の認知負荷が高い。

---

# 7. このプロジェクト向けの新ディレクトリ案

完全移行後の形はこう。

```text
src/compiler/
  main.ark

  cli/
    mod.ark
    args.ark
    commands.ark
    help.ark

  driver/
    mod.ark
    config.ark
    result.ark
    session.ark
    pipeline.ark
    output.ark
    timing.ark
    module_loader.ark

  syntax/
    mod.ark
    span.ark
    token.ark
    token_kinds.ark
    lexer.ark
    ast.ark
    ast_kinds.ark
    parser.ark
    print.ark

  semantic/
    mod.ark
    symbols.ark
    symbol_kinds.ark
    resolver.ark
    types.ark
    type_kinds.ark
    typechecker.ark
    mono.ark
    traits.ark
    result.ark

  corehir/
    mod.ark
    ids.ark
    program.ark
    item.ark
    body.ark
    expr.ark
    pattern.ark
    selection.ark
    value_mode.ark
    source_map.ark
    builder.ark
    validate.ark

  mir/
    mod.ark
    ir.ark
    opcodes.ark
    type_info.ark
    builder.ark
    cfg.ark
    ssa.ark
    dominance.ark
    reachability.ark
    lower/
      mod.ark
      ctx.ark
      function.ark
      stmt.ark
      expr.ark
      calls.ark
      aggregate.ark
      control.ark
    dump.ark

  wasm/
    mod.ark
    writer.ark
    opcodes.ark
    sections.ark
    module_layout.ark
    imports.ark
    exports.ark
    data.ark
    scratch.ark
    function_body.ark
    inst/
      mod.ark
      const.ark
      locals.ark
      arith.ark
      control.ark
      convert.ark
      struct.ark
    intrinsics/
      mod.ark
      string.ark
      math.ark
      vec.ark
      io.ark
    wat.ark

  component/
    mod.ark
    constants.ark
    writer.ark
    type_interner.ark
    wit_types.ark
    imports.ark
    exports.ark
    contract.ark
    adapter_plan.ark
    core_stub.ark
    abi.ark
    adapters/
      string.ark
      list.ark
      option.ark
      result.ark
      tuple.ark
      record.ark
      enum.ark
      numeric.ark
    emit.ark

  diagnostics/
    mod.ark
    codes.ark
    span.ark
    render.ark

  analysis/
    mod.ark
    snapshot.ark
    hover.ark
    symbols.ark

  lsp/
    mod.ark
    jsonrpc.ark
    handlers.ark
    state.ark

  dap/
    mod.ark
    protocol.ark
    handlers.ark
```

これは一気にやるものではない。
まず `driver/module_loader`, `component/writer`, `component/constants`, `wasm/sections`, `mir/dump cycle removal` から。

---

# 8. 「分割しても結合度が下がらない」罠

## 罠 1: `mod.ark` が全部公開する

悪い：

```ark
// component/mod.ark
pub use constants
pub use writer
pub use adapters
pub use abi
pub use emit
```

これをやると外部が内部 helper に依存し始める。
低結合ではなく、見た目だけの整理。

良い：

```ark
// component/mod.ark
pub fn emit_component(...)
pub fn validate_export_surface(...)
```

---

## 罠 2: ファイル名だけ綺麗で内部型が全部漏れる

`corehir` を作っても、中身が `AstNode` なら意味がない。
本物の境界は「ファイル」ではなく「データ型」。

---

## 罠 3: helper を共有しすぎる

共通化は万能ではない。
低結合化したい時に最悪なのは `utils.ark` の肥大化。

`utils` に入れてよいもの：

* 純粋な小関数
* 仕様上安定したもの
* どの層にも属さないもの

入れてはいけないもの：

* compiler phase 固有の判断
* AST/MIR/Wasm をまたぐ helper
* component 特有の型判定
* target 判定
* diagnostics policy

---

## 罠 4: 「DRY」にしすぎて境界を壊す

重複には悪い重複と良い重複がある。

悪い重複：

* `AstNode` 定義が resolver/typechecker に重複
* phase tag が driver/analysis に重複
* line/column 計算が複数箇所に重複

許せる重複：

* adapter family ごとの似た emission
* fixture-specific validation
* エラー文言の局所 wrapper

低結合化では、DRY より **変更理由の分離** が優先。

---

# 9. コンパイラ設計としての境界

## Lexer / Parser

責務：

* bytes/string を token / AST にする
* source span を保持する
* syntax error を出す

持ってはいけない責務：

* 型推論
* 名前解決
* import loading
* target 判定
* stdlib manifest 判定
* component model 判定

## Resolver

責務：

* module graph
* scope
* symbol binding
* visibility
* import/export
* unresolved symbol diagnostics

持ってはいけない責務：

* 型決定
* MIR 生成
* Wasm target 判定
* component ABI 判定

## Typechecker

責務：

* type inference
* unification
* generic/mono 情報
* trait bound
* typed symbol info
* call selection

持ってはいけない責務：

* bytecode emission
* component adapter 判定
* filesystem loading
* CLI output

## CoreHIR

責務：

* 下流に渡す typed, resolved, desugared representation
* AST/Typechecker 内部の遮断
* source map
* selection result
* value mode

持ってはいけない責務：

* parser の child index 前提
* raw syntax
* unresolved names
* backend opcode

## MIR

責務：

* backend-independent control/data flow
* locals
* blocks
* operations
* reachability
* SSA / optimization

持ってはいけない責務：

* parser syntax details
* WIT package identifier
* component section byte
* CLI mode

## Wasm emitter

責務：

* MIR → core wasm bytes
* sections
* memory/data layout
* opcodes
* WASI imports

持ってはいけない責務：

* AST 判定
* typechecker の型推論
* component export policy
* CLI parsing

## Component emitter

責務：

* core wasm → component bytes
* WIT types
* canonical ABI
* lift/lower adapters
* component import/export

持ってはいけない責務：

* parser AST traversal
* driver config parsing
* MIR lowering
* general compile pipeline

---

# 10. すぐ切れる具体タスク

## Task 1: module subdirectory support

**目的**: `src/compiler/component/writer.ark` のような分割を可能にする。

変更：

* `driver.ark::load_single_module`
* `::` を `/` に変換
* `<path>.ark` → `<path>/mod.ark` fallback
* loaded name は canonical path にする

Acceptance:

* `use foo::bar` が `foo/bar.ark` を読む
* `use foo` が `foo/mod.ark` を読む
* circular import detection が壊れない
* stdlib loading の挙動は変えない

---

## Task 2: `mir_ir -> mir_dump` cycle removal

**目的**: IR を debug 出力から独立させる。

変更：

* `mir_ir.ark` から `use mir_dump` を削る
* dump/debug helper を `mir_dump.ark` 側に移す
* `mir_ir` は pure data + builder だけにする

Acceptance:

* import graph に cycle がない
* `mir_dump::dump_mir` は動く
* MIR generation output unchanged

---

## Task 3: component constants extraction

**目的**: 低リスクで `component_emitter.ark` を切り始める。

作成：

```text
src/compiler/component/constants.ark
```

移動：

* `COMP_SEC_*`
* `COMP_SORT_*`
* `COMP_EXI_SORT_*`
* `COMP_CANON_*`
* `WIT_VAL_*`
* `WIT_TAG_*`
* `COMP_VAL_TYPE_*`

Acceptance:

* behavior unchanged
* `component_emitter.ark` line count 減少
* constants file は他 compiler phase に依存しない

---

## Task 4: component writer extraction

作成：

```text
src/compiler/component/writer.ark
```

移動：

* `ComponentWriter`
* `ComponentWriter_new`
* `comp_emit_byte`
* `comp_emit_u32_le`
* `comp_emit_leb128_u`
* `comp_emit_bytes`
* `comp_emit_string`
* `comp_writer_size`
* `comp_emit_section`

Acceptance:

* writer は constants 以外に依存しない
* component bytes unchanged

---

## Task 5: driver component contract extraction

作成：

```text
src/compiler/component/contract.ark
```

移動：

* `validate_component_export_surface`
* `component_type_supported_for_func`
* `component_special_signature_supported`
* `component_has_*`
* `component_fn_is_*`

ただしこの段階では中身を大きく変えない。
まず「場所」だけを正す。

Acceptance:

* `driver.ark` から component shape 判定が消える
* `driver` は `component::validate_export_surface(decls)` だけ呼ぶ
* emit behavior unchanged

---

## Task 6: driver result/config extraction

作成：

```text
src/compiler/driver/result.ark
src/compiler/driver/config.ark
```

移動：

* `CompileResult`
* `CompileResult_ok`
* `CompileResult_err`
* phase constants
* `DriverConfig`
* `DriverConfig_default`

Acceptance:

* `compile_source` の public API unchanged
* no behavior change

---

## Task 7: source map helper extraction

作成：

```text
src/compiler/diagnostics/source_map.ark
```

移動：

* `offset_to_line`
* `offset_to_column`
* span conversion helper

Acceptance:

* driver と analysis の重複を削る
* diagnostics output unchanged

---

## Task 8: Wasm section extraction

作成：

```text
src/compiler/wasm/sections.ark
```

移動対象：

* type section emission
* import section emission
* function section emission
* memory/global/export/code/data section emission

最初は `emit_wasm` の中から section 単位で関数を切るだけ。
inst dispatch にはまだ触らない。

Acceptance:

* generated wasm validates
* byte output unchanged or documented allowed difference
* `emit_wasm` が短くなる

---

## Task 9: intrinsic helper API

作成：

```text
src/compiler/wasm/intrinsics/helpers.ark
```

入れる：

* memory load/store helper
* string length helper
* scratch address helper
* loop skeleton helper
* fd_write helper

Acceptance:

* `emit_intrinsic_string.ark` の関数が opcode 列ベタ書きから少し離れる
* scratch layout が一箇所に集まる

---

## Task 10: CoreHIR independent DTO v1

作成：

```text
src/compiler/corehir/program.ark
src/compiler/corehir/expr.ark
src/compiler/corehir/types.ark
src/compiler/corehir/builder.ark
```

最初の v1 は全部を完全移行しなくてよい。
まず function decl / let / call / literal / return だけでもよい。

Acceptance:

* MIR lowering の一部が `AstNode` ではなく `CoreHirExpr` を読む
* CoreHIR validator が最低限動く
* AST child index 依存が減る

---

# 11. LLM-friendly にするためのドキュメント規約

各 module directory に `README.md` か `CONTRACT.md` を置く。

例：

```text
src/compiler/component/CONTRACT.md
```

中身：

```text
責務:
- core wasm + export metadata を component bytes に変換する

入力:
- Vec<i32> core_wasm
- MirModule
- ComponentExportPlan
- target / wasi_version

出力:
- Vec<i32> component bytes

依存してよい:
- wasm writer primitives
- component constants
- MIR public metadata

依存してはいけない:
- parser::AstNode
- typechecker internals
- driver config parsing
- std::host::fs

不変条件:
- WIT type interner は deterministic
- unsupported export shape は validation phase で落とす
- emit phase で parser syntax 判定をしない
```

LLM に投げる時は、この CONTRACT + 該当 1〜3 ファイルだけで作業できる状態にする。

---

# 12. 品質ゲート

今後 CI/verify に入れると効くもの。

## import graph gate

* cycle 禁止
* 上位層への依存禁止
* `mir_*` から `parser` import 禁止
* `wasm/*` から `parser`, `typechecker`, `driver` import 禁止
* `component/*` から `driver` import 禁止
* `corehir/*` から `mir_lower` import 禁止。ただし今の `corehir::lower_to_mir` facade は移行期間だけ許容。

## file size gate

警告：

* 700 行超え

失敗：

* 1,200 行超え
  ただし generated file は除外し、generated marker 必須。

## function size gate

警告：

* 100 行超え

失敗：

* 200 行超え

例外：

* bytecode emission の generated fixture adapter
* ただし generated/adapter marker 必須

## public API gate

* `pub fn` が 20 個超える module はレビュー対象
* `mod.ark` の public API は 10 個以内を目標
* internal helper は private

## generated boundary gate

生成物・手編集物を明確に分ける。

```text
// GENERATED: do not edit by hand
// Source: scripts/gen/...
```

または docs の generated block と同様の規約を code にも入れる。

---

# 13. このプロジェクトで特に使えるソフトウェア工学概念

## Information hiding

「どう実装しているか」を隠し、「何を保証するか」だけ公開する。
Arukellt では `AstNode` の child layout や Wasm opcode 列を外に漏らさないこと。

## High cohesion / Low coupling

同じ変更理由を持つものを集め、違う変更理由を持つものを離す。
`component_emitter.ark` は cohesion が低い。writer, ABI, WIT, adapters, emit entry が混ざっている。

## Stable abstractions

下流が依存するものは安定させる。
CoreHIR/MIR の public contract がそれ。

## Dependency inversion

高レベル policy が低レベル detail に依存しない。
`driver` は byte encoding detail を知らない。
`component` は parser AST を知らない。

## Ports and adapters

compiler core と host I/O / LSP / DAP / WASI / Component ABI を分ける。
testability が上がる。

## Anti-corruption layer

外部仕様を内部全体に漏らさない。
WIT / WASI / Component Model は `component` と `stdlib host adapter` 内に閉じる。

## Strangler fig refactoring

巨大 file を一気に置換しない。
小さい wrapper / extracted module を作り、古い file の public API は保つ。

## Characterization testing

既存挙動を snapshot で固定してから削る。
巨大 emitter refactor では必須。

## Contract testing

境界ごとの入出力保証をテストする。
CoreHIR/MIR/Wasm/component は contract test が効く。

## Table-driven design

if チェーンで増殖する shape 判定を data に寄せる。
component adapters と intrinsic registry に効く。

## Command-query separation

「状態を変える操作」と「情報を見る操作」を分ける。
LSP/analysis は query API にする。compile は command。

## Data transfer object

境界をまたぐ時は内部構造ではなく DTO を渡す。
`CoreHirProgram` は DTO になるべき。

---

# 14. 具体的な「良い境界」の例

## 悪い境界

```text
driver:
  parse source
  load modules
  validate component export
  inspect AST shape
  lower MIR
  emit wasm
  wrap component
```

## 良い境界

```text
driver:
  session = Session_new(config)
  session.load_source(source)
  session.parse()
  session.resolve()
  session.check()
  session.build_corehir()
  session.lower_mir()
  session.emit_output()
```

各 step は artifact を保存する。

```text
Session artifacts:
  source
  tokens
  parse_result
  module_graph
  resolve_result
  type_result
  corehir
  mir
  backend_plan
  output
  diagnostics
```

LSP は同じ artifacts を query する。

```text
LSP hover:
  session.ensure_typechecked(file)
  analysis::hover_at(session.corehir, position)
```

compile も LSP も同じ pipeline を使う。
今の `analysis.ark` は独自に lexer/parser/resolver/typechecker を並べているので、将来ズレる。

---

# 15. 優先順位の結論

最短で効く順に並べる。

1. **module subdirectory resolution を実装**
2. **`mir_ir <-> mir_dump` 循環を切る**
3. **component constants/writer/type interner を抽出**
4. **driver から component contract 判定を追い出す**
5. **driver config/result/timing/module_loader を分離**
6. **wasm emitter の section 単位分割**
7. **intrinsic helper API を作る**
8. **CoreHIR を本物の typed/resolved DTO にする**
9. **MIR lowering を CoreHIR 入力に移行**
10. **analysis/LSP を Session query API に寄せる**

一番重要なのは 8 と 9。
でも最初にやるべきは 1〜5。そうしないと安全に 8/9 へ進めない。

---

# 16. 迷った時の判断基準

この問いで判定する。

```text
この変更をする時、読む必要がある情報はどこまでか？
```

良い状態：

```text
component writer を変える
→ component/writer.ark + component/CONTRACT.md だけ読む
```

悪い状態：

```text
component writer を変える
→ driver.ark, component_emitter.ark, emitter.ark, mir_lower.ark, parser.ark を読む
```

もう一つ。

```text
この module の public API を見れば、何を保証する module か分かるか？
```

分からないなら境界が弱い。

---

# 17. このリポジトリへの一言診断

このプロジェクトは、ドキュメント・ADR・issue 運用はかなり強い。
問題は、実装が「高速に積み上げた selfhost compiler」の典型で、**設計判断がファイル境界にまだ反映されきっていない**こと。

今はおそらく「機能追加はできるが、変更時に巨大文脈を読む必要がある」段階。
次の成長段階は、新機能を足すことではなく、**CoreHIR / MIR / Wasm / Component / Driver の契約を固定し、巨大 file を契約単位に切ること**。

特にこの 3 つをやれば、コンテキスト長制約にかなり強くなる。

```text
1. CoreHIR を AST pass-through から脱却させる
2. Driver を orchestration だけに戻す
3. Component/Wasm emitter を writer・plan・adapter・emit に分ける
```

ZIP内には README が参照している `scripts/` や `tests/` が含まれていなかったので、ここでは静的な構造分析まで。実行検証は未実施。
