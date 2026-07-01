# Syntax v1 Preview

> Features implemented in v1 milestones M4–M8.

---

## Trait Definitions (M4)

```
trait Display {
    fn display(self) -> String
}
```

- Traits define a set of methods that types must implement.
- Only static dispatch is supported (no vtables).

## Impl Blocks (M4)

```
impl Display for Point {
    fn display(self) -> String {
        f"{self.x}, {self.y}"
    }
}

impl Point {
    fn magnitude(self) -> f64 {
        sqrt(f64_from_i32(self.x * self.x + self.y * self.y))
    }
}
```

- `impl Trait for Type` — trait implementation.
- `impl Type` — inherent methods (no trait required).

## Method Call Syntax (M5)

```
let p = Point { x: 3, y: 4 }
p.display()       // desugars to Point__display(p)
p.magnitude()     // desugars to Point__magnitude(p)
```

- `obj.method(args)` desugars to `Type__method(obj, args)`.
- Resolution: inherent methods checked first, then trait impls.

## Operator Overloading (M6)

```
impl Point {
    fn add(self, other: Point) -> Point {
        Point { x: self.x + other.x, y: self.y + other.y }
    }
    fn eq(self, other: Point) -> bool {
        self.x == other.x && self.y == other.y
    }
}

let p3 = p1 + p2   // calls Point__add(p1, p2)
let same = p1 == p2 // calls Point__eq(p1, p2)
```

- Operators dispatch to named `impl` methods: `add`, `sub`, `mul`, `div`, `eq`, `cmp`.
- No trait required in v1; dispatch is by method name.

## Match Guards (M7)

```
match expr {
    pattern if guard_expr => body,
}
```

- Guard expression is evaluated after pattern bindings are established.
- Works with all pattern types: identifier, enum, struct, literal.

## Or-Patterns (M7)

```
match expr {
    Pat1 | Pat2 | Pat3 => body,
}
```

- Multiple patterns share the same arm body.
- Each sub-pattern generates a condition, combined with logical OR.

## Struct Patterns (M7)

```
match point {
    Point { x, y } => ...,
}
```

- Destructure struct fields directly in match arms.
- Field names bind as local variables.

## Struct Field Update (M7)

```
let p2 = Point { x: 10, ..p1 }
```

- Create a new struct copying fields from an existing one.
- Explicitly specified fields override the base.

## Nested Generics (M8)

```
let v: Vec<Vec<i32>> = Vec_new_i32()
let o: Vec<Option<i32>> = Vec_new_i32()
```

- `>>` is now correctly parsed as two closing angle brackets in type contexts.
- Previously emitted E0203; now fully supported.

## Generic Structs (M8)

```
struct Pair<T> {
    first: T,
    second: T,
}
```

- User-defined structs can have type parameters.
- Type erasure at runtime (same as enums).

## Trait Bounds (M8)

```
fn print_item<T: Display>(item: T) {
    // T must implement Display
}
```

- Functions can constrain type parameters with trait bounds.
- Parsed and tracked; enforcement is incremental.
