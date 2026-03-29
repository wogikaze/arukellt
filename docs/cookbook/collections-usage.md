# Cookbook: Collections Usage

Patterns for HashMap, Deque, Vec sorting, and collection algorithms.

## HashMap — Word Frequency Counter

```ark
let counts = HashMap_new_String_i32()

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
    let current = hashmap_get(counts, word)
    match current {
        Some(c) => hashmap_insert(counts, word, c + 1),
        None    => hashmap_insert(counts, word, 1),
    }
    i = i + 1
}

// Print counts
let keys = hashmap_keys(counts)
let nk = len(keys)
let mut j = 0
while j < nk {
    let k = get(keys, j)
    let v = unwrap(hashmap_get(counts, k))
    println(concat(k, concat(": ", to_string(v))))
    j = j + 1
}
// apple: 3, banana: 2, cherry: 1
```

## HashMap — Two-Way Lookup

```ark
let name_to_id = HashMap_new_String_i32()
let id_to_name = HashMap_new_i32_String()

fn register(name: String, id: i32) {
    hashmap_insert(name_to_id, name, id)
    hashmap_insert(id_to_name, id, name)
}

register("alice", 1)
register("bob", 2)

let id = unwrap(hashmap_get(name_to_id, "alice"))
println(to_string(id)) // 1

let name = unwrap(hashmap_get(id_to_name, 2))
println(name) // "bob"
```

## Vec — Sorting and Searching

```ark
let v = Vec_new_i32()
push(v, 5)  push(v, 2)  push(v, 8)  push(v, 1)  push(v, 3)

sort_i32(v)
// v is now [1, 2, 3, 5, 8]

assert_eq(get(v, 0), 1)
assert_eq(get(v, 4), 8)

// Check membership
assert(contains_i32(v, 3))
assert(contains_i32(v, 9) == false)

// Sum and product
println(to_string(sum_i32(v)))     // 19
println(to_string(product_i32(v))) // 240
```

## Vec — Functional Transforms

```ark
let nums = Vec_new_i32()
push(nums, 1)  push(nums, 2)  push(nums, 3)  push(nums, 4)  push(nums, 5)

// Double each value
let doubled = map_i32_i32(nums, fn(x: i32) -> i32 { x * 2 })
// [2, 4, 6, 8, 10]

// Keep only even values
let evens = filter_i32(nums, fn(x: i32) -> bool { x % 2 == 0 })
// [2, 4]

// Sum with fold
let total = fold_i32_i32(nums, 0, fn(acc: i32, x: i32) -> i32 { acc + x })
println(to_string(total)) // 15

// Check if any element matches
let has_big = any_i32(nums, fn(x: i32) -> bool { x > 3 })
assert(has_big)
```

## Vec — Stack (LIFO) Pattern

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
