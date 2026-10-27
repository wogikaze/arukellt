# Cookbook: Collections Usage

Patterns for HashMap, Deque, Vec sorting, and collection algorithms.

> Policy: user-facing samples should avoid direct low-level prelude helper calls.
> Prefer module-local wrappers/facades; if needed, hide prelude details behind local helper functions.

## HashMap — Word Frequency Counter

<!-- skip-doc-check reason="legacy example not fixture-backed" owner="#683" kind="non-runnable" expires="2026-11-30" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let counts = HashMap::new<String, i32>()

let words = Vec_new_String()
push(words, "apple")
push(words, "banana")
push(words, "apple")
push(words, "cherry")
push(words, "banana")
push(words, "apple")

let n = len(words)
let mut i = 0
while i < n {
    let word = get(words, i)
    let current = counts.get(word)
    match current {
        Some(c) => { let _ = counts.insert(word, c + 1) },
        None    => { let _ = counts.insert(word, 1) },
    }
    i = i + 1
}

// Print counts
let keys = counts.keys()
let nk = len(keys)
let mut j = 0
while j < nk {
    let k = get(keys, j)
    let v = unwrap(counts.get(k))
    println(concat(k, concat(": ", to_string(v))))
    j = j + 1
}
// apple: 3, banana: 2, cherry: 1
```

## HashMap — Two-Way Lookup

<!-- skip-doc-check reason="legacy example not fixture-backed" owner="#683" kind="non-runnable" expires="2026-11-30" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let name_to_id = HashMap::new<String, i32>()
let id_to_name = HashMap::new<i32, String>()

fn register(name: String, id: i32) {
    let _ = name_to_id.insert(name, id)
    let _ = id_to_name.insert(id, name)
}

register("alice", 1)
register("bob", 2)

let id = unwrap(name_to_id.get("alice"))
println(to_string(id)) // 1

let name = unwrap(id_to_name.get(2))
println(name) // "bob"
```

## Vec — Sorting and Searching

<!-- skip-doc-check reason="legacy example not fixture-backed" owner="#683" kind="non-runnable" expires="2026-11-30" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::collections::vec
use std::core
use std::core::cmp
use std::core::iter

fn ascending(a: i32, b: i32) -> Ordering {
    a.cmp(b)
}

fn mul_i32(a: i32, b: i32) -> i32 {
    a * b
}

let v = Vec_new_i32()
push(v, 5)  push(v, 2)  push(v, 8)  push(v, 1)  push(v, 3)

v.sort_by(ascending)
// v is now [1, 2, 3, 5, 8]

assert_eq(get(v, 0), 1)
assert_eq(get(v, 4), 8)

// Check membership
assert(v.iter().contains(3))
assert(v.iter().contains(9) == false)

// Sum and product
println(to_string(v.iter().sum()))                  // 19
println(to_string(v.iter().fold(1, mul_i32)))      // 240
```

## Vec — Functional Transforms

<!-- skip-doc-check reason="legacy example not fixture-backed" owner="#683" kind="non-runnable" expires="2026-11-30" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::core::iter

let nums = Vec_new_i32()
push(nums, 1)  push(nums, 2)  push(nums, 3)  push(nums, 4)  push(nums, 5)

// Double each value
let doubled = nums.iter().map(fn(x: i32) -> i32 { x * 2 }).collect()
// [2, 4, 6, 8, 10]

// Keep only even values
let evens = nums.iter().filter(fn(x: i32) -> bool { x % 2 == 0 }).collect()
// [2, 4]

// Sum with fold
let total = nums.iter().fold(0, fn(acc: i32, x: i32) -> i32 { acc + x })
println(to_string(total)) // 15

// Check if any element matches
let has_big = nums.iter().any(fn(x: i32) -> bool { x > 3 })
assert(has_big)
```

## Vec — Stack (LIFO) Pattern

<!-- skip-doc-check reason="legacy example not fixture-backed" owner="#683" kind="non-runnable" expires="2026-11-30" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let stack = Vec_new_i32()
push(stack, 10)
push(stack, 20)
push(stack, 30)

pop(stack)  // removes 30
let top = get(stack, len(stack) - 1)
println(to_string(top)) // 20
```

## Deque — BFS Pattern (v3)

<!-- skip-doc-check reason="legacy example not fixture-backed" owner="#683" kind="non-runnable" expires="2026-11-30" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
// Breadth-first traversal using Deque as a queue
let queue = deque_new_i32()
deque_push_back(queue, 0)  // start node

while deque_is_empty(queue) == false {
    let node = unwrap(deque_pop_front(queue))
    println(concat("visiting: ", to_string(node)))
    // push neighbors...
    if node < 3 {
        deque_push_back(queue, node + 1)
    }
}
```
