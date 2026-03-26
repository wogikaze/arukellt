# コンパイルパイプライン

ADR-002 により Wasm GC 前提（設計）、ADR-003 により制限付き monomorphization を採用。

> **現行実装**: 全 6 フェーズが動作。170/175 fixture テスト pass。
> ただし Wasm GC 型ではなく linear memory + bump allocator で実装。
> LLVM IR バックエンド [6b] は未実装。

## パイプライン全体像（v0）

```
ソースファイル (.ark)
    │
    ▼
[1] Lexer / Tokenizer
    トークン列
    │
    ▼
[2] Parser
    AST (Abstract Syntax Tree)
    │
    ▼
[3] Name Resolution
    スコープ解決・インポート解決
    シンボルテーブル付き AST
    │
    ▼
[4] Type Checker
    双方向型推論・exhaustive match チェック
    型注釈付き AST（TAST）
    │
    ▼
[5] MIR Lowering
    中間表現（MIR: Mid-level IR）
    制御フローグラフ（CFG）
    monomorphization
    │
    ▼
[6a] Wasm Emitter          [6b] LLVM IR Emitter
    .wasm / .wat               .ll
    │                          │
    ▼                          ▼
最終出力（.wasm）          native binary
```

---

## [1] Lexer 詳細

### 入出力

- **入力**: UTF-8 ソースファイル
- **出力**: `Vec[Token]`

### Token 定義

```
enum Token {
    // キーワード
    Fn, Struct, Enum, Let, Mut, If, Else, Match, While, Loop,
    Break, Continue, Return, Pub, Import, As, True, False,
    
    // リテラル
    IntLit(i64),
    FloatLit(f64),
    StringLit(String),
    CharLit(char),
    
    // 識別子
    Ident(String),
    
    // 演算子・区切り
    Plus, Minus, Star, Slash, Percent,
    Eq, EqEq, NotEq, Lt, LtEq, Gt, GtEq,
    And, Or, Not,
    Ampersand, Pipe, Caret, Tilde, LShift, RShift,
    Arrow, FatArrow, Question, Colon, ColonColon,
    Comma, Semicolon, Dot,
    LParen, RParen, LBrace, RBrace, LBracket, RBracket,
    
    // 特殊
    Eof,
}

struct TokenWithSpan {
    token: Token,
    span: Span,
}

struct Span {
    file_id: u32,
    start: u32,  // バイトオフセット
    end: u32,
}
```

---

## [2] Parser 詳細

### 入出力

- **入力**: `Vec[Token]`
- **出力**: `Module` (AST)

### AST 定義

```
struct Module {
    imports: Vec[Import],
    items: Vec[Item],
}

enum Item {
    FnDef(FnDef),
    StructDef(StructDef),
    EnumDef(EnumDef),
    // ImplBlock は v1 でサポート（v0 では禁止）
}

struct FnDef {
    name: String,
    type_params: Vec[String],       // ジェネリクス
    params: Vec[(String, Type)],
    return_type: Option[Type],
    body: Block,
    is_pub: bool,
    span: Span,
}

struct StructDef {
    name: String,
    fields: Vec[(String, Type)],
    is_pub: bool,
    span: Span,
}

struct EnumDef {
    name: String,
    type_params: Vec[String],
    variants: Vec[Variant],
    is_pub: bool,
    span: Span,
}

enum Variant {
    Unit(String),                    // Variant
    Tuple(String, Vec[Type]),        // Variant(T1, T2)
    Struct(String, Vec[(String, Type)]),  // Variant { x: T }
}

// 式
enum Expr {
    IntLit(i64),
    FloatLit(f64),
    StringLit(String),
    CharLit(char),
    BoolLit(bool),
    Ident(String),
    Binary(Box[Expr], BinOp, Box[Expr]),
    Unary(UnaryOp, Box[Expr]),
    Call(Box[Expr], Vec[Type], Vec[Expr]),   // func<T>(args) - 型パラメータは明示的
    // MethodCall は v1 でサポート（v0 では関数呼び出しのみ）
    FieldAccess(Box[Expr], String),
    Index(Box[Expr], Box[Expr]),
    If(Box[Expr], Block, Option[Block]),
    Match(Box[Expr], Vec[MatchArm]),
    Block(Block),
    Tuple(Vec[Expr]),
    Array(Vec[Expr]),
    StructInit(String, Vec[(String, Expr)]),
    Return(Option[Box[Expr]]),
    Break(Option[Box[Expr]]),
    Continue,
}

// 文
enum Stmt {
    Let(String, Option[Type], Expr, bool),  // name, type, init, is_mut
    Expr(Expr),
    While(Expr, Block),
    Loop(Block),
}

struct Block {
    stmts: Vec[Stmt],
    expr: Option[Box[Expr]],  // 末尾式
}

struct MatchArm {
    pattern: Pattern,
    guard: Option[Expr],
    body: Expr,
}

enum Pattern {
    Wildcard,
    Ident(String),
    Lit(Expr),
    Tuple(Vec[Pattern]),
    Struct(String, Vec[(String, Pattern)]),
    Enum(String, String, Vec[Pattern]),  // EnumName::Variant(patterns)
    // Or パターンは v1 でサポート
}
```

---

## [3] Name Resolution 詳細

### 入出力

- **入力**: `Module` (AST)
- **出力**: `ResolvedModule` (シンボルテーブル付き AST)

### シンボルテーブル

```
struct SymbolTable {
    scopes: Vec[Scope],
    current_scope: usize,
}

struct Scope {
    symbols: HashMap[String, Symbol],
    parent: Option[usize],
}

enum Symbol {
    Variable { ty: TypeId, is_mut: bool },
    Function { fn_id: FnId },
    Type { type_id: TypeId },
    Module { module_id: ModuleId },
}
```

### 解決処理

1. モジュールのインポートグラフを構築
2. 循環インポートを検出
3. トップレベル宣言を収集
4. 関数本体のスコープを解決
5. 未解決の識別子をエラー報告

---

## [4] Type Checker 詳細

### 入出力

- **入力**: `ResolvedModule`
- **出力**: `TypedModule` (TAST)

### 型表現

```
enum Type {
    // プリミティブ
    I32, I64, F32, F64, Bool, Char, Unit,
    
    // 複合型
    Struct(StructId),
    Enum(EnumId),
    Tuple(Vec[Type]),
    Array(Box[Type], usize),      // 固定長
    Slice(Box[Type]),
    
    // 参照型
    String,
    Vec(Box[Type]),
    
    // ジェネリクス
    TypeVar(u32),                  // 推論中の型変数
    Generic(GenericId, Vec[Type]), // 具象化前
    
    // 特殊
    Never,   // 発散型（return, panic 等）
    Error,   // エラー回復用
}
```

### 型推論アルゴリズム

1. **Synthesis（合成）**: 式から型を導出
2. **Checking（検査）**: 期待される型との一致確認
3. **Unification（単一化）**: 型変数の解決

```
// 疑似コード
fn synthesize(expr: Expr) -> Type {
    match expr {
        Expr::IntLit(_) => Type::I32,
        Expr::Ident(name) => lookup_type(name),
        Expr::Call(func, args) => {
            let func_ty = synthesize(func)
            let (param_tys, ret_ty) = extract_fn_type(func_ty)
            for (arg, param_ty) in zip(args, param_tys) {
                check(arg, param_ty)
            }
            ret_ty
        }
        ...
    }
}

fn check(expr: Expr, expected: Type) {
    let actual = synthesize(expr)
    unify(actual, expected)
}
```

---

## [5] MIR Lowering 詳細

### 入出力

- **入力**: `TypedModule` (TAST)
- **出力**: `MirModule`

### MIR 定義

```
struct MirModule {
    functions: Vec[MirFunction],
    types: Vec[MirType],
}

struct MirFunction {
    name: String,
    params: Vec[MirLocal],
    return_ty: MirType,
    locals: Vec[MirLocal],
    blocks: Vec[BasicBlock],
    entry: BlockId,
}

struct BasicBlock {
    id: BlockId,
    stmts: Vec[MirStmt],
    terminator: Terminator,
}

enum MirStmt {
    Assign(Place, Rvalue),
    Call(Option[Place], FnId, Vec[Operand]),
}

enum Terminator {
    Goto(BlockId),
    If(Operand, BlockId, BlockId),
    Switch(Operand, Vec[(i32, BlockId)], BlockId),
    Return(Option[Operand]),
    Unreachable,
}

enum Operand {
    Copy(Place),
    Constant(Constant),
}

enum Rvalue {
    Use(Operand),
    BinaryOp(BinOp, Operand, Operand),
    UnaryOp(UnaryOp, Operand),
    Aggregate(AggregateKind, Vec[Operand]),
    Ref(Place),
}
```

### Monomorphization

ジェネリック関数を具象型ごとに複製:

```
// identity[i32] と identity[String] は別関数として生成
fn identity_i32(x: i32) -> i32 { x }
fn identity_String(x: String) -> String { x }
```

---

## [6a] Wasm Emitter 詳細

### 入出力

- **入力**: `MirModule`
- **出力**: `.wasm` バイナリ

### GC 型の生成

```wasm
;; struct Point { x: f64, y: f64 }
(type $Point (struct
  (field $x (mut f64))
  (field $y (mut f64))))

;; enum Option[i32]
(type $Option_i32 (struct
  (field $tag i32)
  (field $value i32)))
```

### コード生成パターン

```wasm
;; if-else
(if (result i32)
  (local.get $cond)
  (then (i32.const 1))
  (else (i32.const 0)))

;; while
(block $break
  (loop $continue
    (br_if $break (i32.eqz (local.get $cond)))
    ;; body
    (br $continue)))

;; match (enum)
(block $arm0
  (block $arm1
    (br_table $arm0 $arm1 (struct.get $enum $tag (local.get $e))))
  ;; arm1 body
  (br $end))
;; arm0 body
```

---

## エラーリカバリ方針

パーサ・型チェッカーともに「エラーを1つ見つけたら止まる」にしない。

方針:

- パースエラー: パニックサイトを挿入して続行
- 型エラー: エラー型（`ErrorType`）を注入して続行
- 出力: ユーザーに複数のエラーをまとめて報告する

LLM が生成するコードのデバッグでは「1エラー修正 → 再コンパイル → 次のエラー」のサイクルを速くすることが重要。複数エラーの一括報告はこれに貢献する。

---

## インクリメンタルコンパイル（将来）

v0 では全量コンパイル。将来の粒度候補:

- モジュール単位（ファイル単位）の再コンパイル
- 型チェック結果のキャッシュ

---

## モジュールシステム（詳細）

```
// ファイル構造
src/
  main.ark      // エントリポイント
  math.ark      // モジュール math
  util/
    string.ark  // モジュール util.string

// インポート
import math
import util.string as ustr

// 公開
pub fn my_function() -> i32 { ... }   // pub をつけないと外から見えない
```

- 1ファイル = 1モジュール
- モジュール名はファイルパスから自動決定
- `pub` がないシンボルはモジュール内部のみ

**Import 解決アルゴリズム**:

```
fn resolve_import(name: String, current_file: Path) -> Result[Module, Error] {
    // 1. 標準ライブラリを優先
    if is_stdlib(name) {
        return load_stdlib(name)
    }
    
    // 2. カレントディレクトリの .ark ファイル
    let module_path = current_file.parent() / (name + ".ark")
    if exists(module_path) {
        return parse_and_resolve(module_path)
    }
    
    // 3. 見つからない
    Err(Error::ModuleNotFound(name))
}
```

**制約**:

- 循環 import 検出: DFS で visited セットを管理
- 多重 import: 同じモジュールは1度だけロード（キャッシュ）

---

## shebang サポート

```
#!/usr/bin/env arukellt run
fn main() { ... }
```

`arukellt run hello.ark` での直接実行をサポートする。内部で wasmtime を呼ぶ。

---

## 機能サポートマトリクス（現行実装）

「syntax accepted」と「end-to-end works」は別。以下は現行実装の到達度:

| 機能 | Parsed | Typed | Lowered | Emitted | Runnable |
|------|--------|-------|---------|---------|----------|
| i32 / bool | ✅ | ✅ | ✅ | ✅ | ✅ |
| i64 / f64 | ✅ | ✅ | ✅ | ✅ | ✅ |
| f32 / char | ✅ | ✅ | ⚠️ | ⚠️ | ⚠️ f32→f64, char→i32 |
| String | ✅ | ✅ | ✅ | ✅ | ✅ |
| struct | ✅ | ✅ | ✅ | ✅ | ✅ |
| enum (unit) | ✅ | ✅ | ✅ | ✅ | ✅ |
| enum (payload) | ✅ | ✅ | ✅ | ✅ | ✅ |
| if/else | ✅ | ✅ | ✅ | ✅ | ✅ |
| while / loop | ✅ | ✅ | ✅ | ✅ | ✅ |
| break / continue | ✅ | ✅ | ✅ | ✅ | ✅ |
| match (literals) | ✅ | ✅ | ✅ | ✅ | ✅ |
| match (payload bind) | ✅ | ✅ | ✅ | ✅ | ✅ |
| generic functions | ✅ | ✅ | ✅ (→i32) | ✅ | ✅ |
| Option\<T\> / Result | ✅ | ✅ | ✅ | ✅ | ✅ |
| ? operator | ✅ | ✅ | ✅ | ✅ | ✅ |
| closures | ✅ | ✅ | ✅ | ✅ | ✅ |
| higher-order fns | ✅ | ✅ | ✅ | ✅ | ✅ |
| tuples | ✅ | ✅ | ✅ | ✅ | ✅ |
| Box\<T\> | ✅ | ✅ | ✅ | ✅ | ✅ |
| Vec\<i32\> | ✅ | ✅ | ✅ | ✅ | ✅ |
| modules/import | ✅ | ⚠️ | ⚠️ | ⚠️ | ⚠️ basic only |
| for loops | ✅ | ✅ | ✅ | ✅ | ✅ |
| trait / impl | ✅ | ✅ | ✅ | ✅ | ✅ |
| io/fs | ✅ | ✅ | ✅ | ✅ | ✅ |
