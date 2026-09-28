---
Status: open
Created: 2026-09-28
Updated: 2026-09-28
ID: 901
Parent: 875
Track: language-v2
Depends on: none
Related: "ADR-058, ADR-082, RFC-011, #898, #899, #852"
Orchestration class: design-ready
Orchestration upstream: 875
Blocks v{N}: language-v2
Priority: 2
Source: "#899 implementation residual — or-pattern arm with enum-variant alternatives emits invalid core Wasm"
---

# 901 — variant alternative を含む or-pattern arm が invalid Wasm を生成

## Scope

literal alternative の or-pattern(`1 | 2 =>`)は emit できるが、
enum variant alternative を含む or-pattern arm
(`Shape::Circle(r) | Shape::Square =>`)を持つ match が
compile には成功するものの **invalid core Wasm** を生成する
（GC emit 経路）。pattern 照合と payload bind の OR-fold が
variant 形状に対応していないと考えられる。

## Evidence

最小プローブ（catchall 併用でも再現）:

```ark
enum Shape { Circle(i32), Square }

fn mk() -> Shape {
    Shape::Square
}

fn main() {
    let r = match mk() {
        Shape::Circle(r) | Shape::Square => "hit",
        _ => "other",
    }
    stdio::println(r)
}
```

- wasm32-gc emit component で `compile` 成功 →
  `compiler produced invalid core Wasm`。
- catchall 併用で pre-existing に到達可能だったが、ADR-082 以降は
  variant or-alternative が finite 型の coverage を満たす経路でも
  到達する。
- literal or-pattern(`1 | 2` + catchall)は両 backend で正常動作
  （match_extensions/or_arm_edge, or_pattern fixture）。

## Acceptance

- [ ] variant alternative を含む or-pattern arm が compile・validate・
      実行でき、or の各 alternative の照合と payload bind が正しい。
- [ ] wasm32-gc と wasm32 linear で同じ意味論・同じ観測結果になる。
- [ ] 対応しない場合は compile diagnostic で拒否する
      （invalid Wasm を生成しない）。

## Required evidence

- implementation commit SHA
- positive fixture（variant or-alternative arm）の両 backend 実行結果
- 既存 or-pattern fixture の継続 PASS
