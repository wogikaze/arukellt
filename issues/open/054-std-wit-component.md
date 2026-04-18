# std::wit + std::component: WIT 型、resource handle、canonical ABI

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-04-03
**ID**: 054
**Depends on**: 039, 044, 053
**Track**: stdlib
**Orchestration class**: blocked-by-upstream
**Orchestration upstream**: #39, #44
**Blocks v3 exit**: no (Experimental)

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/054-std-wit-component.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

WIT 型定義 (Type, Record, Variant, Enum, Flags, Resource, Interface, World) と
Component Model のリソースハンドル (Own/Borrow, HandleTable) および
canonical ABI の lift/lower ヘルパーを Arukellt の stdlib として実装する。

## 安定性ラベル

**Experimental** — Component Model spec の進化に追随して変更される。

## 受け入れ条件

### std::wit

```ark
pub enum WitType {
    Bool, U8, U16, U32, U64, S8, S16, S32, S64,
    F32, F64, Char, String,
    List(Box<WitType>),
    Option(Box<WitType>),
    Result(Box<WitType>, Box<WitType>),
    Tuple(Vec<WitType>),
    Record(Vec<(String, WitType)>),
    Variant(Vec<(String, Option<WitType>)>),
    Enum(Vec<String>),
    Flags(Vec<String>),
    Resource(String),
}

pub struct WitFunc { name: String, params: Vec<(String, WitType)>, result: Option<WitType> }
pub struct Interface { name: String, functions: Vec<WitFunc> }
pub struct World { name: String, imports: Vec<Interface>, exports: Vec<Interface> }

pub fn world_new(name: String) -> World
pub fn world_import(w: World, iface: Interface)
pub fn world_export(w: World, iface: Interface)
pub fn print_wit(w: World) -> String
pub fn parse_wit(source: String) -> Result<World, Error>
```

### std::component

```ark
pub struct HandleTable<T> { /* internal */ }
pub fn handle_table_new<T>() -> HandleTable<T>
pub fn handle_new<T>(table: HandleTable<T>, value: T) -> i32
pub fn handle_get<T>(table: HandleTable<T>, handle: i32) -> Option<T>
pub fn handle_drop<T>(table: HandleTable<T>, handle: i32) -> Option<T>

pub fn canonical_lower_string(s: String) -> (i32, i32)  // ptr, len in linear memory
pub fn canonical_lift_string(ptr: i32, len: i32) -> Result<String, Error>
pub fn canonical_lower_list<T>(v: Vec<T>) -> (i32, i32)
pub fn canonical_lift_list<T>(ptr: i32, len: i32) -> Result<Vec<T>, Error>
```

## 実装タスク

1. `std/wit/types.ark`: WitType enum 定義
2. `std/wit/world.ark`: World/Interface 構築関数
3. `std/wit/printer.ark`: WIT テキスト形式の出力 (source 実装)
4. `std/wit/parser.ark`: WIT テキストの基本パーサ (source 実装)
5. `std/component/handle.ark`: HandleTable (SlotMap ベース)
6. `std/component/canonical.ark`: canonical ABI lower/lift (linear memory 経由)

## 検証方法

- fixture: `stdlib_wit/wit_type_basic.ark`, `stdlib_wit/world_build.ark`,
  `stdlib_wit/wit_print.ark`, `stdlib_component/handle_basic.ark`,
  `stdlib_component/canonical_string.ark`, `stdlib_component/canonical_list.ark`

## 完了条件

- WitType enum でコンポーネントの型を表現できる
- World を構築し WIT テキストに印刷できる
- HandleTable で resource handle の管理ができる
- fixture 6 件以上 pass

## 注意点

1. canonical lower/lift は linear memory への書き出し — ADR-008 で定義した制約を厳守すること:
   線形メモリ 1 page (64KB)、offset 256–65535 を canonical ABI スクラッチ領域として使用、
   per-call bump allocator (呼び出しごとにリセット)、最大 65280 bytes/call。
   大きな文字列・リストはこの上限に引っかかるため、`canonical_lower_string`/`canonical_lower_list`
   が上限を超える場合は `Error::MemoryOverflow` を返すこと。
2. WIT parser は完全実装を目指さない — 基本構文のみ、edge case は v4
3. HandleTable は SlotMap (#047) に依存するが、independent に i32 index + Vec でも実装可能

## ドキュメント

- `docs/stdlib/wit-component-reference.md`

## 未解決論点

1. `parse_wit` の完成度 — 完全な WIT parser は大きすぎるため subset parser で始めるか
2. async canonical ABI (future/stream) を v3 で考慮するか (v4 送り推奨)
