---
Status: open
Created: 2026-06-26
Updated: 2026-10-25
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

`std::io` models `Reader` / `Writer` / `BufReader` / `BufWriter` as
**`Vec<i32>` layout conventions** (`[cursor, b0, ...]` / `[fd_tag, b0, ...]`,
see `std/io/mod.ark`). There is no public `Read` / `Write` trait, so file /
memory / stdio sources cannot be treated uniformly, and a generic
`io::copy<R: Read, W: Write>` is impossible.

Rust's `std::io` is built on `Read` / `Write` / `BufRead` / `Seek` traits,
enabling `io::copy`, `BufReader<File>`, and generic stream composition.

The design is fixed by [ADR-085](../../docs/adr/ADR-085-io-trait-unification.md)
(PROPOSED, 2026-10-25). This issue owns the implementation phases and progress.

## Current state

- `Reader` / `Writer` / `BufReader` / `BufWriter` are `Vec<i32>` aliases with a
  documented byte-layout convention. They are not distinct types.
- `IoError` enum (`UnexpectedEof`, `Other`) is stable and implements `Display`
  and `std::error::Error`.
- The public API is a set of concrete free functions (`reader_from_bytes`,
  `reader_read`, `writer_write`, `buffered_writer`, ...). There is no public
  trait surface.
- The initial #693 implementation (`d4c5f09f6`) added non-`pub` traits, free
  helper functions (`new_memory_buffer`, `read_bytes`, `write_bytes`, ...), and
  a monomorphic `copy_trait` workaround. This is scaffolding, not the target
  surface; ADR-085 replaces its shape.
- `impl Read for Vec<i32>` and `impl Write for Vec<i32>` conflate source and
  sink on the same type. ADR-085 removes these impls in favour of newtypes.
- `std::host::fs` is a path-based whole-file surface
  (`read_to_string` / `write_string` / `write_bytes` and read probes) with
  experimental `fd_seek` / `fd_tell` / `fd_fdstat_get`. There is no `File`
  constructor and no byte-oriented read primitive yet.
- `std::io` is declared stable in `std/manifest.toml`, while
  `src/compiler/lint/deprecated_table.ark` still says `std::io` moved to
  `std::host::stdio`. The abstraction module and the host stdio surface need to
  be separated in docs: keep `std::io` as the abstraction module, and place
  host-backed `File` / stdio implementations under `std::host`.
- The former `std::host::sockets` / `streams` modules were deleted by ADR-054.
  This issue does not reintroduce them and has no socket acceptance criterion.

## Design decisions (ADR-085)

- **Module boundary**: target-independent traits and memory adapters live in
  `std::io`; host-backed `File` lives in `std::host::fs` and implements the
  traits. `std::io` does not import `std::host`.
- **Traits**: `Read`, `Write`, `BufRead: Read`, `Seek` with `self: Self`
  receivers. All methods are required because default methods are outside
  ADR-062. `Read::read(max_bytes) -> Result<Vec<i32>, IoError>` returns the
  bytes actually read (EOF is an empty `Vec`); `Write::write` returns the
  accepted byte count and `flush` is the buffer boundary; `BufRead::read_line`
  returns `Option<String>` (EOF is `None`); `Seek::seek` takes `SeekFrom` and
  returns the new absolute position.
- **Receiver/ownership**: method calls do not move the receiver. Handle state
  is shared by handle identity, matching `Vec::push` / `HashMap::insert`.
  Handle types are not `Copy`; moving one into an adapter transfers ownership.
  A `let mut` binding is not required for handle-internal mutation.
- **Newtypes**: `Reader` (Read + BufRead + Seek) and `Writer` (Write) replace
  the `Vec<i32>` aliases. The layout is private. Conversions:
  `Reader::from_bytes`, `Reader::into_bytes`, `Writer::into_bytes`,
  `Writer::into_reader`.
- **Generic algorithms**: `Io::copy<R: Read, W: Write>`, `Io::read_exact`,
  `Io::read_to_end`, `Io::write_all`. No free `io::copy` (ADR-046).
- **Adapters**: generic `BufReader<R: Read>` and `BufWriter<W: Write>` replace
  the concrete buffered aliases. `BufReader` implements `Seek` when the inner
  handle does.
- **File**: `std::host::fs::File` is a whole-file image handle for the current
  host capability. `File::open` loads a read-only image;
  `File::create` starts a write image; `Write::flush` writes the whole image.
  Streaming fd/resource semantics require a separate ADR.
- **Errors**: the IO traits return the existing concrete `IoError`. #693 does
  not add variants or associated error types; #694 owns the error taxonomy and
  `std::error::Error` boundary.
- **Stability**: all new traits/types/functions start as `experimental`
  (ADR-014). Existing stable free functions remain stable until deprecated with
  `deprecated_by` and a removal window. `std::io`'s incorrect deprecation
  guidance is removed.

## Required work

- [x] Design ADR-085 (PROPOSED, 2026-10-25).
- [ ] **Phase A — trait core and memory newtypes**: define public `Read` /
  `Write` / `BufRead` / `Seek` / `SeekFrom`; add `Reader` / `Writer` newtypes
  and impls; remove `impl Read for Vec<i32>` / `impl Write for Vec<i32>`;
  register `std::io` as a manifest module and the traits as manifest
  `[[types]]` entries (experimental); add positive and negative fixtures on
  both targets; keep the old free functions working.
- [ ] **Phase B — generic algorithms**: add `Io::copy`, `Io::read_exact`,
  `Io::read_to_end`, `Io::write_all`; add the `Io::copy` fixture
  (memory -> memory) and error-propagation fixtures.
  Compiler prerequisite: monomorphization of a generic function with two
  independently dispatched trait-bound parameters must produce valid Wasm.
- [ ] **Phase C — buffered adapters**: add generic `BufReader<R: Read>` and
  `BufWriter<W: Write>`, including `BufRead` and conditional `Seek`; replace
  the concrete buffered implementations behind compatibility wrappers.
  Compiler prerequisite: dispatch of a user-defined trait method through a
  generic field must produce valid Wasm.
- [ ] **Phase D — File**: add the byte-oriented host read primitive and
  `std::host::fs::File` (`open` / `create`, `Read` / `Write` / `Seek`,
  `flush` / `close`); add the file-reader -> memory-writer `Io::copy` fixture;
  record target availability in the manifest.
- [ ] **Phase E — compatibility and deprecation**: turn the old free functions
  into newtype wrappers, add `deprecated_by` / removal metadata, migrate
  fixtures and docs, and remove the stale `std::io`-moved-to-`std::host::stdio`
  guidance. Reconcile the manifest labels of the partial #693 helper free
  functions before removing them.
- [ ] **Phase F — removal**: after the ADR-014 deprecation window, remove the
  legacy free functions and align `std::io` with trait / method / associated
  surface. Sync with #703.
- [ ] Regenerate stdlib docs and manifest, then run the applicable verification
  gates (`verify quick` at merge/CI; `verify lane` during implementation).

## Acceptance

- [ ] `Read` / `Write` / `BufRead` / `Seek` are public traits with the ADR-085
  signatures and receiver model.
- [ ] Memory `Reader` / `Writer` implement the traits; `Vec<i32>` itself no
  longer implements `Read` or `Write`.
- [ ] `std::host::fs::File` implements `Read` / `Write` / `Seek` in its
  documented whole-file mode.
- [ ] A generic `Io::copy` fixture moves bytes between two different source
  types through the traits (file reader -> memory writer, and memory -> memory).
- [ ] `BufReader<R>` / `BufWriter<W>` are generic adapters and the old buffered
  entry points are compatibility wrappers.
- [ ] Existing `std::io` consumers are migrated or documented with a removal
  window; no stable API is removed without the ADR-014 process.
- [ ] Both `wasm32-gc` and `wasm32` compile/run the trait fixtures.
- [ ] `python3 scripts/manager.py verify quick` exits 0 (after the dependent
  phases; this issue remains open until then).

## Dependencies and open questions

- **Compiler: two-bound generic dispatch.** On the current selfhost runtime
  artifact (2026-10-23), a generic function with two differently-bounded type
  parameters that dispatches methods on both produced invalid core Wasm in a
  probe. `Io::copy` requires this; confirm against the current compiler and
  track the gap before Phase B.
- **Compiler: generic-field user-trait dispatch.** A generic adapter struct
  whose impl calls a user-defined trait method on a generic field produced
  invalid core Wasm in the same artifact. `BufReader` / `BufWriter` require
  this; confirm and track before Phase C.
- **Host: byte read.** `__runtime_abi_fs_read_file` returns `String`. `File`'s
  read side needs a byte-oriented host primitive (or an official WASI
  filesystem boundary that returns bytes). This is a Phase D prerequisite.
- **Default methods.** ADR-062 leaves default methods out of scope. This is why
  `Io` exists as a namespace type for `copy` / `read_exact` / `read_to_end` /
  `write_all`, and why `BufRead::read_line` is a required method. Revisit only
  if default methods are adopted.
- **Stability labels.** The partial #693 helper free functions currently carry
  a mix of `stable` and `experimental` manifest labels. Their classification
  must be corrected according to ADR-014 before any removal.
- **Stale module guidance.** `src/compiler/lint/deprecated_table.ark` and
  `docs/compiler/error-codes.md` still describe `std::io` as moved to
  `std::host::stdio`. Remove or rewrite that guidance when `std::io` is
  registered as the canonical abstraction module.

## References

- [ADR-085: `std::io` の `Read` / `Write` / `BufRead` / `Seek` による I/O 統合](../../docs/adr/ADR-085-io-trait-unification.md)
- [ADR-014: 言語仕様と Stdlib API の安定性ラベル](../../docs/adr/ADR-014-stability-labels.md)
- [ADR-044: trait とメソッド構文を言語機能として採択する](../../docs/adr/ADR-044-trait-method-syntax-adopted.md)
- [ADR-046: 公開 free function の根絶](../../docs/adr/ADR-046-free-function-eradication.md)
- [ADR-054: host-linker とリポジトリ内 Rust runtime の退役](../../docs/adr/ADR-054-host-linker-and-rust-runtime-retirement.md)
- [ADR-059: 言語 v2 の所有権・Copy・可変性](../../docs/adr/ADR-059-language-v2-ownership-copy-mutability.md)
- [ADR-062: Language v2 の Trait core と coherence](../../docs/adr/ADR-062-language-v2-trait-core-coherence.md)
- [ADR-064: Error trait の名前空間と静的 source 境界](../../docs/adr/ADR-064-error-trait-namespace-and-source-boundary.md)
- [ADR-078: Language v2 の parametric trait impl](../../docs/adr/ADR-078-language-v2-parametric-trait-impl.md)
- [ADR-079: Language v2 メソッド型パラメータ](../../docs/adr/ADR-079-language-v2-method-level-generics.md)
- [ADR-084: Language v2 の fallible conversion trait と dispatch 形式](../../docs/adr/ADR-084-language-v2-fallible-conversion-dispatch.md)
- `std/io/mod.ark`, `std/host/fs.ark`, `std/manifest.toml`
- Rust `std::io`: <https://doc.rust-lang.org/std/io/index.html>
- Depends on: #688 (trait dispatch), #692 (`From` / `Into` / `AsRef`); error
  boundary from #694.
