---
Status: open
Created: 2026-07-25
Updated: 2026-07-25
ID: 836
Track: compiler-internal
Depends on: 832
Related: ADR-033, #726, #832, #835
Orchestration class: implementation-ready
Orchestration upstream: 832
Blocks v{N}: none
Priority: 2
Source: post-#832 / #833 follow-up — isolate Vec&lt;Vec&gt; vs flat Vec gaps
---

# `get` on `Vec&lt;Vec&lt;T&gt;&gt;` nullability validate-fail

## Summary

#832 F で `Vec&lt;Vec&lt;i32&gt;&gt;` + `get_unchecked` は閉じた。
本 issue は **flat `Vec` と nested `Vec&lt;Vec&gt;` を切り分けた結果**、残る穴を追跡する。

切り分け結論（2026-07-25, s2）:

| 形 | 結果 |
|----|------|
| `Vec&lt;i32&gt;` + `get` → `Option&lt;i32&gt;` | OK |
| `Vec&lt;String&gt;` + `get` → `Option&lt;String&gt;` | OK |
| `Vec&lt;Vec&lt;i32&gt;&gt;` + `get_unchecked` | OK（既存 `vec_vec_i32`） |
| `Vec&lt;Vec&lt;i32&gt;&gt;` outer `get_unchecked` → inner `get` | OK |
| **`Vec&lt;Vec&lt;i32&gt;&gt;` outer `get` → `Option&lt;Vec&lt;i32&gt;&gt;`** | **VALIDATE_FAIL** |
| `Vec&lt;Vec&lt;String&gt;&gt;` + `get_unchecked` | OK |
| `Vec&lt;Vec&lt;fn&gt;&gt;` + `get_unchecked` / call | OK |
| `Vec&lt;Option&lt;Vec&lt;i32&gt;&gt;&gt;` + `get_unchecked` | OK |

失敗メッセージ:

```
type mismatch: expected (ref null $type), found (ref $type)
```

**flat `Vec` の `get` は健全。壊れるのは nested outer の `get` → `Option&lt;Vec&lt;_&gt;&gt;` 経路だけ。**

## Reproduction

```ark
use std::host::stdio

fn main() {
    let mut outer: Vec<Vec<i32>> = Vec_new_i32()
    let mut inner: Vec<i32> = Vec_new_i32()
    push(inner, 9)
    push(outer, inner)
    match get(outer, 0) {
        Some(row) => stdio::println(get_unchecked(row, 0).to_string()),
        None => stdio::println("none"),
    }
}
```

## Root cause hypothesis

`get` が返す `Option&lt;Vec&lt;T&gt;&gt;` の Some payload は nullable enum field。
outer `Vec&lt;Vec&gt;` 要素配列から取った **non-null vec ref** を、nullable 期待の
`struct.set` / local へ渡すときに nullability が食い違う（#835 と同系統の
open-enum / nullable field 問題の nested 版）。

## Workstreams

1. RED fixture draft: `tests/fixtures/collections/vec_vec_i32_get.ark`
   （**manifest 未登録** — validate-fail のまま t3-run に載せない）
2. WAT で mismatch 地点（Some payload set vs match extract）
3. owner 層修正（vec get emit / Option&lt;Vec&gt; payload VT / locals cast）
4. 対称: `Vec&lt;Vec&lt;String&gt;&gt;` outer `get` も緑化
5. 緑化後に `run:` / `t3-compile:` / `t3-run:` 登録 + `verify lane --gate t3`

## Non-goals

- `#697` Vec API 拡張（windows/chunks 等）
- flat `Vec&lt;T&gt;` の `get` 契約変更
- `get_unchecked` 経路（既に緑）

## Acceptance

- [ ] outer `get(outer, i)` → `Option&lt;Vec&lt;i32&gt;&gt;` match が validate + hosted run
- [ ] 既存 `vec_vec_i32` / flat `get` fixtures が緑
- [ ] fixture を `run:` / `t3-compile:` / `t3-run:` 登録
- [ ] `python3 scripts/manager.py verify lane --gate t3`

## Related

- #832 nested container / funcref matrix（F は get_unchecked のみ）
- #835 Option mut open-enum cast（近縁）
- #726 GC ref 型推論
