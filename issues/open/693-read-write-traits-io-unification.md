---
Status: open
Created: 2026-06-26
Updated: 2026-06-26
ID: 693
Track: stdlib-api
Depends on: "688, 692"
Orchestration class: blocked-by-upstream
Orchestration upstream: "#688 trait dispatch, #692 From/Into/AsRef"
Blocks v{N}: none
Priority: 2
Source: Stdlib abstraction gap audit 2026-06-26 — Rust parity comparison
---

# 693 — `Read` / `Write` / `BufRead` / `Seek` traits and IO unification

## Summary

`std::io` currently models `Reader` / `Writer` / `BufReader` / `BufWriter` as
**`Vec<i32>` type aliases** with a documented byte-layout convention
(`[cursor, b0, ...]` / `[fd_tag, b0, ...]`, see `std/io/mod.ark:18-30`). There
is no `Read` / `Write` trait, so file / socket / memory / process-pipe sources
cannot be treated uniformly. Generic helpers like `io::copy<R: Read, W:
Write>` are impossible.

Rust's `std::io` is built on `Read` / `Write` / `BufRead` / `Seek` traits,
enabling `io::copy`, `BufReader<File>`, `Chain<R1, R2>`, and generic stream
composition.

## Current state

- `Reader` = `Vec<i32>` alias, `Writer` = `Vec<i32>` alias.
- `IoError` enum (`UnexpectedEof`, `Other`).
- Functions: `reader_from_bytes`, `reader_read_byte`, `writer_write_bytes`,
  `buffered_writer`, etc. — all concrete, none generic.
- `std::host::fs` / `std::host::sockets` / `std::host::streams` are separate
  concrete surfaces with no shared trait.

## Required work

- [ ] Define `trait Read { fn read(self: Read, buf: ...) -> Result<i32, IoError> }`
      and `trait Write { fn write(...) -> Result<i32, IoError> }` in `std::io`.
- [ ] Define `trait BufRead` (extends Read) and `trait Seek`.
- [ ] Implement `impl Read` / `impl Write` for the existing memory buffer
      representation (migrate `Vec<i32>` aliases to newtypes wrapping the
      buffer, or keep layout and add trait impls).
- [ ] Implement `impl Read for std::host::fs::File` and
      `impl Write for std::host::fs::File`.
- [ ] Implement `impl Read` / `impl Write` for `std::host::sockets` /
      `std::host::streams`.
- [ ] Implement `io::copy<R: Read, W: Write>` generically.
- [ ] Implement `BufReader<R: Read>` / `BufWriter<W: Write>` as generic
      adapters (replacing the current concrete `BufReader`/`BufWriter`).
- [ ] Fixtures: generic `copy` between a file reader and a memory writer.
- [ ] Migration plan for existing `Vec<i32>`-alias consumers.
- [ ] Regenerate stdlib docs and manifest.
- [ ] `python3 scripts/manager.py verify quick` exits 0.

## Acceptance

- [ ] `Read` / `Write` / `BufRead` / `Seek` traits defined.
- [ ] At least memory-buffer, file, and socket types implement `Read` or
      `Write` through the trait.
- [ ] A generic `io::copy` fixture moves bytes between two different source
      types via the trait.
- [ ] Existing `std::io` consumers documented or migrated.
- [ ] `python3 scripts/manager.py verify quick` exits 0.

## References

- Depends on: #688 (trait dispatch), #692 (`AsRef`/`Into` for buffer slices)
- `std/io/mod.ark`, `std/host/fs.ark`, `std/host/sockets.ark`,
  `std/host/streams.ark`
- Rust `std::io`: <https://doc.rust-lang.org/std/io/index.html>
