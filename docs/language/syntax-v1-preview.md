# v1 構文メモ

> **Status: Transitional** — This document describes planned v1 syntax changes.
> For current behavior, see [../current-state.md](../current-state.md).
> For the normative specification, see [spec.md](spec.md).

このページは、v0 の関数呼び出し中心スタイルに対して、
このブランチで入っている **v1 系の追加構文** をざっくり把握するためのメモです。

## 既に入っている主な項目

- `for` ループ
- 文字列補間
- `trait`
- `impl`
- メソッド構文
- 演算子オーバーロード
- match guard
- or-pattern
- struct pattern
- match での tuple pattern
- nested generics
- trait bounds

## 例

### for

```ark
for i in 0..10 {
    println(to_string(i))
}
```

```ark
for item in values(v) {
    println(to_string(item))
}
```

### 文字列補間

```ark
let name = String_from("world")
let msg = f"Hello, {name}!"
println(msg)
```

### trait / impl / method

```ark
trait Display {
    fn to_string(self) -> String
}

struct Point { x: i32, y: i32 }

impl Point {
    fn sum(self) -> i32 {
        self.x + self.y
    }
}
```

## 注意

- 「v1 preview」という名前ですが、実際には **既に実装済みの項目を多く含みます**
- ただし、細部や制限は feature ごとに差があるので、最終判断は `current-state.md` と fixture を優先してください
- v0 互換の書き方としては、依然として関数呼び出し形式が一番安定です

## 関連

- [syntax.md](syntax.md)
- [type-system.md](type-system.md)
- [../current-state.md](../current-state.md)
