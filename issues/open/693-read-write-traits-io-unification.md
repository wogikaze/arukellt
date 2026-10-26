---
Status: open
Created: 2026-06-26
Updated: 2026-10-27
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
(ACCEPTED, 2026-10-25). This issue owns the implementation phases and progress.

## Current state

- Phases A–C are implemented: `Read` / `Write` / `BufRead` / `Seek` /
  `SeekFrom`, the memory `Reader` / `Writer` newtypes, the receiver-less
  `Io` algorithms, and the generic `BufReader<R: Read>` / `BufWriter<W:
  Write>` adapters are public. Phase D's read side is also implemented below.
- Phases E and F are implemented: the `reader_*` read methods, `copy*`, the
  legacy memory-buffer helpers, and the experimental `file_*` fd lane are
  removed with E0100 replacement hints. The retained compatibility family
  (`reader_from_bytes`, `buf_*`, and the tagged `writer_*` entry points) carries
  `deprecated` metadata, W0009 coverage, and recorded deletion conditions.
- `Reader` / `Writer` are distinct newtypes with private layouts. `BufReader`
  and `BufWriter` are the canonical generic adapters; the legacy `buf_*`
  entry points remain deprecated `Vec<i32>`-layout wrappers.
- `IoError` enum (`UnexpectedEof`, `Other`) is stable and implements `Display`
  and `std::error::Error`.
- The initial #693 scaffolding (non-`pub` traits, `new_memory_buffer` /
  `read_bytes` / `write_bytes` helpers, and the monomorphic `copy_trait`
  workaround) is no longer the public shape. The helpers remain as legacy
  compatibility functions until Phase E; `copy_trait` stays scaffolding and is
  not part of the target surface.
- `impl Read for Vec<i32>` and `impl Write for Vec<i32>` were removed in
  Phase A, so source and sink can no longer be conflated on the same type.
- `std::host::fs` is a path-based whole-file surface
  (`read_to_string` / `read_bytes` / `write_string` / `write_bytes` and read
  probes) with experimental `fd_seek` / `fd_tell` / `fd_fdstat_get`.
  `read_bytes` preserves NUL and invalid UTF-8, so it is the byte-oriented
  primitive behind `File::open`.
- `std::host::fs::File` is the Phase D read side: `File::open` loads a
  read-only whole-file image, `Read` / `Seek` operate on that image, and
  `close` rejects later operations. `File::create`, `Write`, and `flush`
  (the whole-file write side) remain open under this phase.
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

- [x] Design and accept ADR-085 (ACCEPTED, 2026-10-25).
- [x] **Phase A — trait core and memory newtypes**: define public `Read` /
  `Write` / `BufRead` / `Seek` / `SeekFrom`; add `Reader` / `Writer` newtypes
  and impls; remove `impl Read for Vec<i32>` / `impl Write for Vec<i32>`;
  register `std::io` as a manifest module and the traits as manifest
  `[[types]]` entries (experimental); add positive and negative fixtures on
  both targets; keep the old free functions working. (Landed at `7d1e0700c`.)
- [x] **Phase B — generic algorithms**: add `Io::copy`, `Io::read_exact`,
  `Io::read_to_end`, `Io::write_all`, plus the `Io::read_to_string` UTF-8
  convenience; add the `Io::copy` fixture (memory -> memory) and
  error-propagation fixtures. Both targets compile and run the Phase B
  fixtures under `tests/fixtures/stdlib_io_traits/`.
- [x] **Phase C — buffered adapters**: add generic `BufReader<R: Read>` and
  `BufWriter<W: Write>`, including `BufRead` and conditional `Seek`; the
  legacy `buf_reader_*` / `buf_writer_*` entry points keep their `Vec<i32>`
  layout contracts until Phase E. Field-receiver dispatch is verified on both
  targets.
- [ ] **Phase D — File**: add the byte-oriented host read primitive and
  `std::host::fs::File` (`open` / `create`, `Read` / `Write` / `Seek`,
  `flush` / `close`); add the file-reader -> memory-writer `Io::copy` fixture;
  record target availability in the manifest. Status: read side landed and
  runtime-green (`fs::read_bytes`, `File::open`, `Read`, `Seek`, `close`, the
  file roundtrip / EOF fixtures on `wasm32-gc` memory32+P2 and `wasm32`);
  `File::create`, `Write`, and `flush` remain open.
- [x] **Phase E — compatibility and deprecation**: the retained free functions
  carry `deprecated_by` / `deprecated_since` / `remove_in`, W0009 is live for
  the qualified calls, the stale `std::io`-moved-to-`std::host::stdio`
  guidance is gone, and the fixtures / migration guide record the new surface.
  The 15-entry window is 0.1.0 → 1.0.0 (ADR-014, shortened under ADR-087).
- [x] **Phase F — removal**: the fully-replaced free functions were retired
  under the ADR-087 0.x exception (W0009 + migration guide + changelog), with
  E0100 replacement hints; `std::io` now exposes the trait / method /
  associated surface, with the recorded compatibility residuals below.
- [ ] Regenerate stdlib docs and manifest, then run the applicable verification
  gates (`verify quick` at merge/CI; `verify lane` during implementation).

## Acceptance

- [x] `Read` / `Write` / `BufRead` / `Seek` are public traits with the ADR-085
  signatures and receiver model.
- [x] Memory `Reader` / `Writer` implement the traits; `Vec<i32>` itself no
  longer implements `Read` or `Write`.
- [ ] `std::host::fs::File` implements `Read` / `Write` / `Seek` in its
  documented whole-file mode.
- [x] A generic `Io::copy` fixture moves bytes between two different source
  types through the traits (file reader -> memory writer, and memory -> memory).
- [x] `BufReader<R>` / `BufWriter<W>` are generic adapters and the old buffered
  entry points are deprecated compatibility wrappers with a removal window.
- [x] Existing `std::io` consumers are migrated or documented with a removal
  window; the `stable` removals follow the ADR-087 exception (W0009 +
  migration guide + changelog).
- [x] Both `wasm32-gc` and `wasm32` compile/run the trait fixtures.
- [ ] `python3 scripts/manager.py verify quick` exits 0 (after the dependent
  phases; this issue remains open until then).

## Dependencies and open questions

- **Compiler: two-bound generic dispatch — resolved 2026-10-26.**
  `Io::copy<Reader, Writer>` and the other two-bound helpers compile, validate,
  and run on both `wasm32-gc` and `wasm32`. Unused generic impl method templates
  are now pruned: their type-variable bodies lowered as scalar placeholders and
  failed GC validation (`copy_alias` exposed this through the unused `Io::copy`
  template).
- **Compiler: generic-field user-trait dispatch — resolved 2026-10-26.**
  `BufReader<R>` / `BufWriter<W>` dispatch `self.inner.read(...)` /
  `self.inner.write(...)` on a generic field; the adapters validate and run on
  both targets.
- **Compiler: impl-method Result payloads — resolved 2026-10-26.**
  `corehir/return_type_name.ark` kept a bare `Result` return spelling for impl
  methods, so `match File::open(...)` was rejected by MIR lowering as an
  untyped Option/Result scrutinee. Method returns now encode
  `result:<ok>:<err>` with the same spelling free functions use; the Phase D
  fixture exercises the Ok/Err binds without fixture-side annotations.
- **Compiler: generic match-payload instantiation — open (2026-10-26).**
  A payload bound from `match` keeps the enum declaration's type variable
  (`Result<T, E>` / `MyEnum<T>::Ok`), so passing it to a generic callee
  (`print_read<T: Read>`, `Io::copy<R, W>`) cannot infer a concrete type
  argument. The call then binds the unspecialized template body, whose
  `unreachable` stub was the Phase D runtime trap. A general fix in match
  binding (substituting the scrutinee type arguments) regressed 121
  generic/iterator fixtures with `E0200 i32 vs struct`; it was reverted. The
  Phase D fixtures bind the handle and byte image with explicit types, which
  mints the `File` / `Writer` mono instances without changing inference for
  other matches. The compiler-side inference remains an open residual.
- **Compiler: GC local layout placeholder for inline match payloads — open
  (2026-10-26).** `get_unchecked` on an inline `Result<Vec<i32>, _>` payload
  lowered `array.get` (i32) into a temp declared as the size-class structref
  placeholder (the 0x1813 / 0x2991 validation mismatch); memory64 reported the
  same mismatch as i64. `file_read_eof` pins the byte image with an explicit
  `Vec<i32>` binding so its byte / EOF / seek / close contract stays checked on
  both targets.
- **Host: byte read — resolved for the read side 2026-10-26.**
  `__runtime_abi_fs_read_file` returns `String`, but the emitted value is the
  raw byte image (no UTF-8 validation), so `fs::read_bytes` exposes it as
  `Vec<i32>` through `text::to_utf8_bytes`. The Phase D fixture reads NUL,
  0x80, 0xFF, and a valid UTF-8 sequence to keep the byte contract checked. A
  future descriptor/resource streaming API is still a separate ADR.
- **Default methods.** ADR-062 leaves default methods out of scope. This is why
  `Io` exists as a namespace type for `copy` / `read_exact` / `read_to_end` /
  `write_all`, and why `BufRead::read_line` is a required method. Revisit only
  if default methods are adopted.
- **Stability labels — resolved 2026-10-27.** The retained compatibility
  family is `deprecated` with `deprecated_since = "0.1.0"` and
  `remove_in = "1.0.0"`; the removed entries are gone. The 15 retained names
  are `reader_from_bytes`, the seven `buf_*` entry points, and the seven tagged
  `writer_*` entry points.
- **Stale module guidance — resolved 2026-10-27.** `deprecated_table.ark` no
  longer warns that `std::io` moved to `std::host::stdio`, and
  `docs/compiler/error-codes.md` uses the `std::time` → `std::host::clock`
  example. `std::io` is the canonical abstraction module.
- **Residual: generic-adapter dispatch.** Migrating the `buf_*` fixtures to
  `BufReader` / `BufWriter` still exposes the generic-impl dispatch trap
  (`BufReader::read_line` lowers to `unreachable` when the mono instance is not
  instantiated through a preceding `read` call). The retained `buf_*` wrappers
  stay deprecated until the compiler lane fixes it.
- **Residual: host stdio handles.** `stdout()` / `stderr()` are still tagged
  `Vec<i32>` handles, so the `writer_*` family stays deprecated until
  `std::host::stdio` exposes typed handles with `Write` impls.
- **Residual: File write side.** `File::create`, `Write`, and `flush` remain
  open; the fd-lane `file_*` helpers were retired in favour of the whole-file
  `File` contract.

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
