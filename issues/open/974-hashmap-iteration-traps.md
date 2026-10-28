---
Status: open
Created: 2026-10-28
Updated: 2026-10-28
ID: 974
Track: stdlib-api
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 1
Source: "ABC481 arukellt port review (2026-10-28, HEAD 2436c92b0) + HashMap probe"
Related: "#966, #940, #888, #895, #801, #973"
---

# 974 — `HashMap` runtime traps: `for k in map` with `String` keys traps, and unsupported pairs fail typechecking

## Summary

The ABC481 port records that `HashMap` traps at runtime and hand-rolls an
open-addressing table for C. On the current build the basic surface is narrower than
that claim but still broken:

1. **Iteration traps.** `for k in m` on `HashMap<String, i32>` traps with
   `wasm trap: wasm 'unreachable' instruction executed` inside
   `HashMap__into_iter__String__i32`, on both `wasm32` and `wasm32-gc`. The same
   program with `HashMap<i32, i32>` works.
2. **Unsupported pairs hide behind a scalar mismatch.** `HashMap<String, String>`
   and `HashMap<i32, String>` fail with
   `error[E0200|typecheck]: type mismatch: i32 vs String` instead of a diagnostic
   that names the unsupported specialization.
3. **Basic String-key operations work** and contradict the blanket README claim:
   500 `insert`/`get`/`contains_key`/`remove`/`len` calls on
   `HashMap<String, i32>` pass on both targets, as do 1000 inserts on
   `HashMap<i32, i32>` plus `for k in m` for the i32 pair.

So the tracked problem is the iteration/rewrite gap for String-keyed maps
(#966/#940 family), not the whole HashMap surface.

## Repro A — iteration trap

```ark
use std::host::stdio
use std::prelude::String_from

fn main() {
    let m = HashMap::new<String, i32>()
    m.insert(String_from("a"), 1)
    let mut t: i32 = 0
    for k in m {
        t = t + 1
    }
    stdio::println(t.to_string())
}
```

```
$ scripts/run/arukellt-selfhost.sh compile --target wasm32 --emit wasm -o /tmp/hm.wasm /tmp/hm.ark
$ wasmtime run --dir=. /tmp/hm.wasm
Error: ... wasm trap: wasm `unreachable` instruction executed
  0: ... !HashMap__into_iter__String__i32
```

`wasm32-gc` traps in the same function. The generic `IntoIterator` body snapshots
`keys()` and wraps a `VecIter<K>`; `keys()` alone works (returns `Vec<K>`), so the
failure is in the iteration rewrite/monomorphization.

## Repro B — unsupported pair

```ark
fn main() {
    let m = HashMap::new<String, String>()
    m.insert("x", "1")
}
```

`error[E0200|typecheck]: type mismatch: i32 vs String`

## Acceptance

- [ ] `for k in HashMap<String, _>` iterates (or is rejected) consistently on both
      targets.
- [ ] Unsupported `K`/`V` pairs produce an "unsupported HashMap specialization"
      diagnostic naming the pair, not a scalar type mismatch.
- [ ] Fixture covers insert/get/remove/len and iteration for `HashMap<i32, i32>` and
      `HashMap<String, i32>` on both targets.
- [ ] The port README's "HashMap is unusable" note is rescoped to what is actually
      broken.

## Notes

- `keys()` works: `let ks: Vec<i32> = m.keys()` prints `2` on `wasm32`.
- Related: #966 (same-named method dispatch), #940 (bare type-name identity),
  #888/#895 (HashMap entry/enumeration and intrinsic family), #801 (HashMap GC is
  still gated on Vec/Enum GC work).
- Probe files: `tmp/issueprobe/hash_str.ark` (iteration) and
  `tmp/issueprobe/hash_ss.ark` (unsupported pair), gitignored.
