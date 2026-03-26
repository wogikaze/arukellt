# v0 → v1 Migration Guide

This guide covers all breaking changes and new features when upgrading from Arukellt v0 to v1.

## Breaking Changes

### 1. `parse_i64` / `parse_f64` Return Type Changed

**v0:** Returns raw value, silently returns `0` / `0.0` on error.

```ark
let n: i64 = parse_i64(String_from("42"))
let f: f64 = parse_f64(String_from("3.14"))
```

**v1:** Returns `Result<i64, String>` / `Result<f64, String>`.

```ark
let r: Result<i64, String> = parse_i64(String_from("42"))
match r {
    Ok(n) => println(i64_to_string(n)),
    Err(e) => println(e),
}

// Or with type inference (no annotation needed):
let r = parse_f64(String_from("3.14"))
match r {
    Ok(f) => println(f64_to_string(f)),
    Err(e) => println(e),
}
```

`parse_i32` was already `Result<i32, String>` in v0 and is unchanged.

### 2. Reserved Keywords

`trait`, `impl`, `for`, `in` are now reserved keywords. If you used any of these as identifiers, rename them.

## New Features (Non-Breaking)

### 3. Trait Definitions & Method Syntax (M4/M5)

```ark
trait Display {
    fn to_string(self) -> String
}

struct Point { x: f64, y: f64 }

impl Display for Point {
    fn to_string(self) -> String {
        concat(f64_to_string(self.x), concat(", ", f64_to_string(self.y)))
    }
}

impl Point {
    fn distance(self, other: Point) -> f64 {
        let dx = self.x - other.x
        let dy = self.y - other.y
        sqrt(dx * dx + dy * dy)
    }
}

// Method call syntax:
let d = p1.distance(p2)   // desugars to Point__distance(p1, p2)
let s = p1.to_string()
```

Dispatch is purely static (no vtable). Old function-call style still works.

### 4. Operator Overloading (M6)

```ark
impl Point {
    fn add(self, other: Point) -> Point {
        Point { x: self.x + other.x, y: self.y + other.y }
    }
    fn eq(self, other: Point) -> bool {
        self.x == other.x && self.y == other.y
    }
}

let p3 = p1 + p2   // desugars to Point__add(p1, p2)
if p1 == p2 { ... } // desugars to Point__eq(p1, p2)
```

Overloadable: `+` (`add`), `-` (`sub`), `*` (`mul`), `/` (`div`), `==` (`eq`), `!=`, `<`, `<=`, `>`, `>=` (via `cmp`).

### 5. Pattern Matching Extensions (M7)

```ark
// Guard patterns
match x {
    n if n > 0 => "positive",
    _ => "non-positive",
}

// Or-patterns
match x {
    1 | 2 | 3 => "small",
    _ => "other",
}

// Struct patterns
match point {
    Point { x, y } => x + y,
}

// Tuple patterns in match
match pair {
    (0, y) => y,
    (x, y) => x + y,
}
```

### 6. Struct Field Update (M7)

```ark
let p2 = Point { x: 10.0, ..p1 }  // inherit y from p1
```

### 7. Nested Generics & User Generic Structs (M8)

```ark
// Now allowed (was forbidden in v0):
let v: Vec<Vec<i32>> = ...
let o: Option<Option<String>> = ...

// User-defined generic structs:
struct Pair<T> { first: T, second: T }
let p: Pair<i32> = Pair { first: 1, second: 2 }
```

## Migration Checklist

- [ ] Update all `parse_i64()` / `parse_f64()` call sites to handle `Result`
- [ ] Rename any identifiers named `trait`, `impl`, `for`, `in`
- [ ] (Optional) Migrate function-style calls to method syntax
- [ ] (Optional) Add operator overloading to custom types
- [ ] (Optional) Use guard / or-pattern / struct pattern in match expressions
