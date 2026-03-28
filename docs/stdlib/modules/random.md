# std::random

**Stability**: Stable
**Module**: `std::random`

## Overview

Pseudo-random number generation. Provides both WASI-backed true random (`random_i32`) and a deterministic
xorshift32 PRNG (`seeded_random`). **Not cryptographically secure** — do not use for security-sensitive operations.

## Functions

### `random_i32() -> i32`

Returns a random 32-bit integer from the WASI random source.

**Example:**
```ark
use std::random

let n = random::random_i32()
println(i32_to_string(n))
```

### `random_i32_range(lo: i32, hi: i32) -> i32`

Returns a random integer in the range `[lo, hi)`. Uses WASI random source.

**Example:**
```ark
use std::random

let dice = random::random_i32_range(1, 7)
println(i32_to_string(dice)) // 1..6
```

### `random_bool() -> bool`

Returns a random boolean value.

**Example:**
```ark
use std::random

let coin = random::random_bool()
println(if coin { "heads" } else { "tails" })
```

### `seeded_random(seed_val: i32) -> i32`

Deterministic xorshift32 PRNG. Given the same seed, always produces the same result.
Seed of `0` is replaced with `42` internally.

**Example:**
```ark
use std::random

let r = random::seeded_random(12345)
println(i32_to_string(r))
```

### `seeded_range(seed_val: i32, lo: i32, hi: i32) -> i32`

Deterministic random integer in `[lo, hi)` using xorshift32.

**Example:**
```ark
use std::random

let val = random::seeded_range(42, 0, 100)
println(i32_to_string(val))
```

### `shuffle_i32(v: Vec<i32>) -> Vec<i32>`

Returns a new `Vec<i32>` with elements randomly shuffled using Fisher-Yates algorithm.

**Example:**
```ark
use std::random

let v = Vec_new_i32()
push(v, 1)  push(v, 2)  push(v, 3)  push(v, 4)
let shuffled = random::shuffle_i32(v)
// shuffled is a random permutation of [1, 2, 3, 4]
```
