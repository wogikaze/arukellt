#!/usr/bin/env python3
"""Generate documentation from std/manifest.toml and project metadata.

Generated files (fully generated — do not edit manually):
  - docs/README.md
  - docs/_sidebar.md
  - docs/stdlib/reference.md
  - docs/stdlib/name-index.md
  - docs/stdlib/scoreboard.md
  - docs/stdlib/modules/*.md  (one per stdlib module)
  - docs/stdlib/README.md
  - docs/compiler/README.md
  - docs/language/README.md
  - docs/benchmarks/README.md

Marker-updated files (inline blocks replaced, rest is hand-written):
  - README.md
  - docs/current-state.md

Regenerate: python3 scripts/gen/generate-docs.py
Check:      python3 scripts/gen/generate-docs.py --check
"""
from __future__ import annotations

import argparse
import os
import re
import sys
import subprocess
from collections import Counter, defaultdict
from dataclasses import dataclass
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - Python < 3.11 local compatibility
    import tomli as tomllib

ROOT = Path(__file__).resolve().parent.parent.parent
DOCS = ROOT / "docs"
DATA = DOCS / "data"
PROJECT_STATE = DATA / "project-state.toml"
TARGET_CONTRACT = DOCS / "data" / "target-contract-summary.md"
SECTIONS_FILE = DATA / "sections.toml"
STDLIB_MANIFEST = ROOT / "std" / "manifest.toml"
FIXTURE_MANIFEST = ROOT / "tests" / "fixtures" / "manifest.txt"
FIXTURES_DIR = ROOT / "tests" / "fixtures"
SCOREBOARD_LOW_COVERAGE_THRESHOLD = 50
LANGUAGE_CLASSIFICATIONS = DATA / "language-doc-classifications.toml"
SPEC_MD = ROOT / "docs" / "language" / "spec.md"
MATURITY_MATRIX = ROOT / "docs" / "language" / "maturity-matrix.md"
MONOMORPHIC_DEPRECATION = ROOT / "docs" / "stdlib" / "monomorphic-deprecation.md"

# Lifecycle axes are intentionally distinct. Public stdlib APIs follow ADR-014;
# language features may additionally be specified-but-unimplemented.
PUBLIC_API_STABILITY_LABELS = ("stable", "provisional", "experimental", "deprecated")
LANGUAGE_FEATURE_STABILITY_LABELS = ("stable", "provisional", "experimental", "unimplemented")
STABILITY_LABELS = PUBLIC_API_STABILITY_LABELS
IMPLEMENTATION_STATUSES = ("functional", "limited", "stub", "unreachable")

# ── Manifest schema ──────────────────────────────────────────────────────────
# Enforced by validate_manifest_schema(); see docs/stdlib/generation-schema.md
#
# Page kinds:
#   "prelude"   — entry has `kind` but no `module` (or has `prelude = true`)
#   "module"    — entry has `module` and no `kind` (standard module function)
#   "host_stub" — entry has `kind = "host_stub"` (capability-gated host function)
#   "mixed"     — entry has both `kind` and `module` (e.g., intrinsic_wrapper in std::wasm)
#
# Required on every [[functions]] entry:
FUNCTION_REQUIRED_FIELDS: tuple[str, ...] = ("name", "params", "returns", "stability", "doc_category")

# Valid values for `kind` (when present):
VALID_KIND_VALUES: frozenset[str] = frozenset(
    {"builtin", "intrinsic", "prelude_wrapper", "intrinsic_wrapper", "host_stub"}
)

# Extra required fields per `kind` value:
FUNCTION_KIND_REQUIRED: dict[str, tuple[str, ...]] = {
    "host_stub": ("module", "target"),
}
# ─────────────────────────────────────────────────────────────────────────────

# Regexes for parsing stability annotations from spec.md
_SPEC_SECTION_RE = re.compile(
    r'^## (\d+)\. (.+?)(?:\s+<!--\s*stability:\s*(.*?)\s*-->)?\s*$'
)
_SPEC_SUBSECTION_RE = re.compile(r'^### (\d+\.\d+) (.+)$')
# Matches v1/v2/v3/v4 feature markers in subsection titles
_V_FEATURE_RE = re.compile(r'\(v\d+\w*\)\s*$', re.IGNORECASE)


@dataclass(frozen=True)
class DocEntry:
    rel_path: str
    title: str
    summary: str


STDLIB_MODULE_PAGES = [
    {
        "path": "modules/bytes.md",
        "title": "std::bytes",
        "description": "Source-backed docs for binary data helpers.",
        "modules": ["std::bytes"],
        "overview": {
            "summary": (
                "The `std::bytes` module provides binary data helpers built on `Vec<i32>`. "
                "Each element is treated as a byte in the `0..=255` range. The module covers "
                "buffer creation and manipulation, string-to-bytes conversion, hex encoding/decoding, "
                "LEB128 variable-length encoding, and little-/big-endian integer conversions."
            ),
            "highlights": [
                ("`bytes_from_string(s)` / `string_from_bytes(buf)`", "Convert between UTF-8 strings and byte buffers."),
                ("`hex_encode(buf)` / `hex_decode(s)`", "Hex-encode a buffer or decode a hex string."),
                ("`leb128_encode_u32(n)` / `leb128_encode_i32(n)`", "Variable-length LEB128 encoding."),
                ("`u32_to_le_bytes(n)` / `u32_from_le_bytes(buf)`", "Little-endian u32 ↔ byte conversion."),
                ("`bytes_slice(buf, start, end)`", "Extract a sub-range of bytes as a new buffer."),
                ("`bytes_concat(a, b)`", "Concatenate two byte buffers into a new one."),
            ],
            "typical_usage": """\
```ark
import std::bytes

let buf = bytes_from_string("hello")
let hex = hex_encode(buf)        // "68656c6c6f"
let back = hex_decode(hex)       // [104, 101, 108, 108, 111]
let s = string_from_bytes(back)  // "hello"
```""",
        },
    },
    {
        "path": "modules/collections.md",
        "title": "std::collections family",
        "description": "Source-backed docs for the currently supported collection modules.",
        "modules": [
            "std::collections::compiler",
            "std::collections::hash",
            "std::collections::linear",
            "std::collections::ordered",
        ],
        "overview": {
            "summary": (
                "The `std::collections` family provides the primary collection data structures "
                "for Arukellt programs: hash maps, double-ended queues, min-heap priority queues, "
                "sorted maps, and bitsets. All current containers are monomorphic `i32` "
                "containers backed by `Vec<i32>`, making them compatible with any build target."
            ),
            "highlights": [
                ("`hashmap_new` / `hashmap_get` / `hashmap_set`", "Hash map with open addressing (i32 → i32)."),
                ("`deque_push_back` / `deque_pop_front`", "FIFO / ring-buffer deque."),
                ("`pq_push` / `pq_pop`", "Min-heap priority queue — smallest element first."),
                ("`sorted_map_insert` / `sorted_map_get`", "Sorted-vector map for ordered iteration."),
                ("`bitset_mark` / `bitset_test`", "Compact bit-flag set backed by a `Vec<i32>`."),
            ],
            "typical_usage": """\
```ark
import std::collections::hash

let map = hashmap_new()
hashmap_set(map, 42, 100)
let v = hashmap_get(map, 42)   // 100
let exists = hashmap_contains(map, 42)  // true
```""",
        },
    },
    {
        "path": "modules/component.md",
        "title": "std::component",
        "description": "Source-backed docs for component-model helpers.",
        "modules": ["std::component"],
        "overview": {
            "summary": (
                "The `std::component` module exposes version constants for the WebAssembly "
                "Component Model. The surface is intentionally minimal while the component "
                "model integration is still being designed."
            ),
            "highlights": [
                ("`canonical_abi_version()`", "Returns the canonical ABI version number."),
                ("`component_model_version()`", "Returns the component model version string."),
            ],
            "typical_usage": """\
```ark
import std::component

let abi = canonical_abi_version()       // e.g. 1
let ver = component_model_version()     // e.g. "0.2"
```""",
        },
    },
    {
        "path": "modules/core.md",
        "title": "std::core family",
        "description": "Source-backed docs for ranges, errors, and hashing helpers.",
        "modules": ["std::core", "std::core::error", "std::core::hash"],
        "overview": {
            "summary": (
                "The `std::core` family provides the fundamental building blocks shared across "
                "the standard library: integer ranges (`Range`, `RangeInclusive`), the shared "
                "`Error` type used by fallible stdlib operations, and low-level hash utilities "
                "used internally by collection modules. Import only the sub-module you need — "
                "most application code uses only `std::core` (ranges) and the `Error` type from "
                "`std::core::error`."
            ),
            "highlights": [
                ("`range_new(start, end)`", "Create a half-open range `[start, end)`."),
                ("`range_contains(r, value)`", "Test membership in a range."),
                ("`range_len(r)`", "Length of a half-open range."),
                ("`error_message(e)`", "Convert a stdlib `Error` variant to a human-readable string."),
                ("`hash_i32(n)`", "Hash an `i32` to a stable non-negative integer."),
            ],
            "typical_usage": """\
```ark
import std::core

let r = range_new(0, 10)
if range_contains(r, 5) {
    println("5 is in range")
}
let len = range_len(r)  // 10
```""",
        },
    },
    {
        "path": "modules/csv.md",
        "title": "std::csv",
        "description": "Source-backed docs for CSV parsing helpers.",
        "modules": ["std::csv"],
        "overview": {
            "summary": (
                "The `std::csv` module provides minimal experimental CSV helpers. The current "
                "implementation exposes only line-level splitting and does not yet handle full "
                "RFC 4180 quoting rules."
            ),
            "highlights": [
                ("`csv_split_line(line)`", "Split a comma-separated line into a `Vec<String>` of fields."),
            ],
            "typical_usage": """\
```ark
import std::csv

let fields = csv_split_line("name,age,city")
// fields == ["name", "age", "city"]
```""",
        },
    },
    {
        "path": "modules/fs.md",
        "title": "std::host::fs / std::fs",
        "description": "Source-backed docs for explicit host filesystem operations.",
        "modules": ["std::host::fs", "std::fs"],
        "overview": {
            "summary": (
                "`std::host::fs` is the primary host filesystem module: whole-file reads, string "
                "and byte writes, an `exists` read probe / readable-file check (not path existence), "
                "and experimental fd helpers, all backed "
                "by the current WASI filesystem intrinsics. `std::fs` is a smaller stable-shaped "
                "bridge over the same intrinsics (`read_string` / `write_string` / `exists`) — "
                "useful when you want a compact API, with the understanding that it tracks only a "
                "subset of the evolving `std::host::fs` rollout. Neither module is a complete "
                "filesystem facade (no directory listing, metadata, or streaming I/O in-tree yet). "
                "Pure path manipulation lives in `std::path`; for the full host family overview, "
                "see [io.md](io.md)."
            ),
            "highlights": [
                ("`read_to_string(path)` / `read_string(path)`", "Read a whole file as a UTF-8 string, returning `Result<String, String>`."),
                ("`write_string(path, content)`", "Write or replace a UTF-8 file."),
                ("`write_bytes(path, buf)`", "Write a byte array to a file."),
                (
                    "`exists(path)`",
                    "Read probe: `true` when a full read succeeds; not a general path-existence check.",
                ),
            ],
            "typical_usage": """\
```ark
import std::host::fs

let content = read_to_string("data.txt")
match content {
    Ok(text) => println(text),
    Err(e)   => eprintln("read error: " + e),
}

write_string("output.txt", "hello")
```""",
        },
    },
    {
        "path": "modules/io.md",
        "title": "std::host family",
        "description": "Source-backed docs for explicit host capabilities and adjacent path helpers.",
        "overview_only": True,
        "modules": [
            "std::host::stdio",
            "std::host::fs",
            "std::fs",
            "std::path",
            "std::host::process",
            "std::host::env",
            "std::host::clock",
            "std::host::random",
        ],
        "module_pages": [
            ("std::host::stdio", "../reference.md", "Manifest reference (no dedicated module page yet)"),
            ("std::host::fs", "fs.md", "Host filesystem surface"),
            ("std::fs", "fs.md", "Compatibility fs aliases"),
            ("std::path", "path.md", "Pure path helpers"),
            ("std::host::process", "process.md", "Process control"),
            ("std::host::env", "process.md", "Environment (bundled with process docs)"),
            ("std::host::clock", "time.md", "Pure time helpers; host clock APIs also in reference"),
            ("std::host::random", "random.md", "Deterministic helpers; host entropy APIs in reference"),
        ],
        "overview": {
            "summary": (
                "The `std::host` family exposes all runtime-environment capabilities: standard "
                "I/O, filesystem access, process control, environment variables, wall-clock and "
                "monotonic time, and host-entropy random numbers. These modules are explicitly "
                "host-bound — they depend on WASI capabilities that are not available in pure "
                "freestanding Wasm. `std::path` is included here because it is the pure "
                "companion to `std::host::fs`; path manipulation itself requires no host access."
            ),
            "highlights": [
                ("`std::host::stdio` — `println(s)`", "Write a line to stdout."),
                ("`std::host::stdio` — `eprintln(s)`", "Write a line to stderr."),
                ("`std::host::fs` — `read_to_string(path)`", "Read a whole file as a UTF-8 string."),
                ("`std::host::fs` — `write_string(path, content)`", "Write or replace a UTF-8 file."),
                ("`std::host::env` — `args()`", "Retrieve the CLI argument vector."),
                ("`std::host::env` — `var(name)`", "Look up an environment variable."),
                ("`std::host::clock` — `monotonic_now()`", "High-resolution monotonic timestamp (nanoseconds)."),
                ("`std::host::random` — `random_i32()`", "Host-entropy random integer."),
            ],
            "typical_usage": """\
```ark
import std::host::stdio
import std::host::fs
import std::host::env

let name = var("USER").unwrap_or("world")
println("Hello, " + name + "!")

let content = read_to_string("/etc/hostname")
match content {
    Ok(text) => println(text),
    Err(e)   => eprintln("error: " + e),
}
```""",
        },
    },
    {
        "path": "modules/json.md",
        "title": "std::json",
        "description": "Source-backed docs for the current JSON helpers.",
        "modules": ["std::json"],
        "overview": {
            "summary": (
                "The `std::json` module provides experimental JSON stringify and parse helpers "
                "for primitive types (`i32`, `bool`, `String`). These are building blocks for "
                "constructing or reading JSON fragments; full structured JSON DOM support is "
                "planned for a future release."
            ),
            "highlights": [
                ("`json_stringify_i32(n)` / `json_stringify_bool(b)`", "Serialize a primitive value to a JSON string."),
                ("`json_stringify_string(s)`", "Serialize a string value with JSON escaping."),
                ("`json_parse_i32(s)` / `json_parse_bool(s)`", "Parse a JSON primitive back to a typed value."),
                ("`json_null()`", "Returns the JSON `null` literal."),
            ],
            "typical_usage": """\
```ark
import std::json

let n = json_stringify_i32(42)       // "42"
let b = json_stringify_bool(true)    // "true"
let s = json_stringify_string("hi")  // "\"hi\""
let parsed = json_parse_i32("42")    // 42
```""",
        },
    },
    {
        "path": "modules/path.md",
        "title": "std::path",
        "description": "Source-backed docs for path manipulation helpers.",
        "modules": ["std::path"],
        "overview": {
            "summary": (
                "The `std::path` module provides pure string-based path manipulation helpers. "
                "Paths are represented as `String` values and always use `/` as the separator "
                "to match POSIX and WASI conventions. This module requires no host access; for "
                "file I/O see `std::host::fs` or the [host family overview](io.md)."
            ),
            "highlights": [
                ("`join(base, segment)`", "Join two path segments with a single `/` separator."),
                ("`file_name(path)`", "Returns the final path segment after the last `/`."),
                ("`extension(path)`", "Returns the extension without the leading `.`."),
                ("`parent(path)`", "Returns the parent directory path."),
                ("`is_absolute(path)`", "Returns `true` when the path starts with `/`."),
            ],
            "typical_usage": """\
```ark
import std::path

let p = join("/home/user", "data.txt")  // "/home/user/data.txt"
let name = file_name(p)                 // "data.txt"
let ext  = extension(p)                 // "txt"
let dir  = parent(p)                    // "/home/user"
```""",
        },
    },
    {
        "path": "modules/process.md",
        "title": "std::host::process / std::host::env",
        "description": "Source-backed docs for process control and runtime environment helpers.",
        "modules": ["std::host::process", "std::host::env"],
        "overview": {
            "summary": (
                "This page covers two closely related host modules: `std::host::process` for "
                "process lifecycle control (exit, abort) and `std::host::env` for runtime "
                "environment access (CLI arguments, environment variables). Both are host-bound "
                "and require WASI capabilities. For the full host family overview, see [io.md](io.md)."
            ),
            "highlights": [
                ("`exit(code)`", "Request process termination with the given exit code."),
                ("`abort()`", "Abort execution immediately by panicking."),
                ("`args()`", "Retrieve the CLI argument vector (excluding argv[0])."),
                ("`var(name)`", "Look up an environment variable by name, returning `Option<String>`."),
                ("`has_flag(flag)`", "Check whether a flag is present in the argument vector."),
            ],
            "typical_usage": """\
```ark
import std::host::env
import std::host::process

if has_flag("--help") {
    println("Usage: myapp [options]")
    exit(0)
}

let name = var("USER").unwrap_or("unknown")
println("Running as: " + name)
```""",
        },
    },
    {
        "path": "modules/random.md",
        "title": "std::random",
        "description": "Source-backed docs for pseudo-random utilities.",
        "modules": ["std::random"],
        "overview": {
            "summary": (
                "The `std::random` module provides deterministic pseudo-random helpers that "
                "take an explicit seed, making results reproducible across runs. For "
                "host-entropy (non-deterministic) random numbers, use `std::host::random` "
                "instead (see [host family overview](io.md))."
            ),
            "highlights": [
                ("`seeded_random(seed)`", "Generate a pseudo-random `i32` from a seed value."),
                ("`seeded_range(seed, lo, hi)`", "Generate a pseudo-random value in `[lo, hi)`."),
                ("`shuffle_i32(vec, seed)`", "Return a shuffled copy of a `Vec<i32>`."),
            ],
            "typical_usage": """\
```ark
import std::random

let r = seeded_random(42)           // deterministic i32
let v = seeded_range(42, 0, 100)    // deterministic value in [0, 100)
let shuffled = shuffle_i32(vec, 42) // deterministic shuffle
```""",
        },
    },
    {
        "path": "modules/seq.md",
        "title": "std::seq",
        "description": "Source-backed docs for eager sequence helpers.",
        "modules": ["std::seq"],
        "overview": {
            "summary": (
                "The `std::seq` module provides eager sequence helpers over `Vec<i32>`: "
                "search, aggregation, deduplication, and reversal. Lazy pipelines and "
                "closure-heavy adapters are deferred to a future release."
            ),
            "highlights": [
                ("`binary_search(vec, value)`", "Binary search a sorted vector; returns index or -1."),
                ("`min_i32(vec)` / `max_i32(vec)`", "Find the minimum or maximum element."),
                ("`sum_i32(vec)`", "Sum all elements in a vector."),
                ("`unique(vec)`", "Remove duplicates, preserving first occurrence."),
                ("`seq_reverse(vec)`", "Return a reversed copy of the vector."),
                ("`seq_contains(vec, value)`", "Linear search for membership."),
            ],
            "typical_usage": """\
```ark
import std::seq

let nums = vec![3, 1, 4, 1, 5, 9]
let total = sum_i32(nums)           // 23
let lo    = min_i32(nums)           // 1
let dedup = unique(nums)            // [3, 1, 4, 5, 9]
```""",
        },
    },
    {
        "path": "modules/test.md",
        "title": "std::test",
        "description": "Source-backed docs for assertion and expectation helpers.",
        "modules": ["std::test"],
        "overview": {
            "summary": (
                "The `std::test` module provides typed assertion and expectation helpers for "
                "fixture and test code. It includes equality assertions for all primitive types, "
                "unwrap helpers for `Result` and `Option`, string containment checks, and "
                "snapshot comparison."
            ),
            "highlights": [
                ("`assert_eq_i32(a, b)` / `assert_eq_string(a, b)`", "Assert typed equality; panics with a message on mismatch."),
                ("`assert_true(cond)` / `assert_false(cond)`", "Assert boolean conditions."),
                ("`expect_ok_i32(result)`", "Unwrap a `Result<i32, String>` or panic on `Err`."),
                ("`expect_some_i32(option)`", "Unwrap an `Option<i32>` or panic on `None`."),
                ("`assert_contains(haystack, needle)`", "Assert that a string contains a substring."),
                ("`assert_eq_snapshot(actual, expected)`", "Line-by-line string comparison with diff on failure."),
            ],
            "typical_usage": """\
```ark
import std::test

assert_eq_i32(1 + 1, 2)
assert_eq_string(trim(" hi "), "hi")
assert_true(10 > 5)

let result: Result<i32, String> = Ok(42)
let value = expect_ok_i32(result)  // 42
```""",
        },
    },
    {
        "path": "modules/text.md",
        "title": "std::text",
        "description": "Source-backed docs for string and formatting helpers.",
        "modules": ["std::text"],
        "overview": {
            "summary": (
                "The `std::text` module extends the prelude's built-in string type with "
                "inspection, trimming, searching, transformation, padding, and primitive "
                "formatting helpers. If you need to check whether a string is empty, split it "
                "into lines, remove whitespace, or format a number, start here."
            ),
            "highlights": [
                ("`trim(s)` / `trim_start(s)` / `trim_end(s)`", "Strip ASCII whitespace from both, leading, or trailing end."),
                ("`replace(s, from, to)`", "Replace all non-overlapping occurrences of `from` with `to`."),
                ("`lines(s)`", "Split a string on newlines into a `Vec<String>`."),
                ("`index_of(s, needle)`", "First byte index of `needle`, or -1 when not found."),
                ("`format_i32(n)` / `format_f64(n)`", "Format numeric values as decimal strings."),
                ("`pad_left(s, width, fill)` / `pad_right(s, width, fill)`", "Fixed-width string padding."),
            ],
            "typical_usage": """\
```ark
import std::text

let s = "  hello, world!  "
let trimmed = trim(s)              // "hello, world!"
let parts   = lines("a\\nb\\nc")   // ["a", "b", "c"]
let idx     = index_of(s, "world")  // 9
let label   = pad_right(format_i32(42), 6, " ")  // "42    "
```""",
        },
    },
    {
        "path": "modules/time.md",
        "title": "std::time",
        "description": "Source-backed docs for pure duration helpers.",
        "modules": ["std::time"],
        "overview": {
            "summary": (
                "The `std::time` module provides pure duration arithmetic over caller-supplied "
                "timestamps. It computes elapsed time in milliseconds, microseconds, or "
                "nanoseconds given two `i64` timestamps. Host clock reads live in "
                "`std::host::clock` (see [host family overview](io.md)); this module only "
                "does pure math."
            ),
            "highlights": [
                ("`duration_ms(start, end)`", "Elapsed time in milliseconds."),
                ("`duration_us(start, end)`", "Elapsed time in microseconds."),
                ("`duration_ns(start, end)`", "Elapsed time in nanoseconds (identity: `end - start`)."),
            ],
            "typical_usage": """\
```ark
import std::time
import std::host::clock

let t0 = monotonic_now()
// ... do work ...
let t1 = monotonic_now()
let elapsed = duration_ms(t0, t1)  // milliseconds
```""",
        },
    },
    {
        "path": "modules/toml.md",
        "title": "std::toml",
        "description": "Source-backed docs for the current TOML helpers.",
        "modules": ["std::toml"],
        "overview": {
            "summary": (
                "The `std::toml` module provides minimal experimental helpers for a **bounded "
                "TOML subset** only: blank lines, full-line comments (`# …`), and simple "
                "`key = value` entries (one entry per non-comment line). `toml_parse` returns "
                "`Ok` only for documents that fit that subset; table headers (`[…]`), lines "
                "without `=`, empty keys or values, unclosed quoted values, trailing non-comment "
                "lines without `key = value`, and other malformed or unsupported forms return "
                "`Err(String)`. This is "
                "not full TOML 1.0 compliance."
            ),
            "highlights": [
                ("`toml_parse(doc)`", "Parse a multi-line document in the supported subset; `Err` on unsupported or malformed lines."),
                ("`toml_parse_line(line)`", "Legacy single-line filter: pass through `key = value` lines; blank or `#` comment lines become `\"\"`."),
            ],
            "typical_usage": """\
```ark
import std::toml

let value = toml_parse_line("name = \\"arukellt\\"")
// value == "arukellt"
```""",
        },
    },
    {
        "path": "modules/wasm.md",
        "title": "std::wasm",
        "description": "Source-backed docs for WebAssembly helpers.",
        "modules": ["std::wasm"],
        "overview": {
            "summary": (
                "The `std::wasm` module exposes WebAssembly binary-format constants and low-level "
                "memory operations. It provides the magic bytes, version bytes, section IDs, "
                "value-type constants, and bulk memory helpers (`memory_copy`, `memory_fill`) "
                "needed to build or inspect Wasm modules programmatically."
            ),
            "highlights": [
                ("`wasm_magic()` / `wasm_version()`", "The 4-byte Wasm magic number and version bytes."),
                ("`section_type()` … `section_data()`", "Section ID constants for all standard Wasm sections."),
                ("`valtype_i32()` … `valtype_f64()`", "Value-type constants matching the Wasm binary encoding."),
                ("`memory_copy(dst, src, len)`", "Bulk memory copy (like `memory.copy`)."),
                ("`memory_fill(dst, val, len)`", "Bulk memory fill (like `memory.fill`)."),
            ],
            "typical_usage": """\
```ark
import std::wasm

let magic = wasm_magic()    // [0x00, 0x61, 0x73, 0x6d]
let ver   = wasm_version()  // [0x01, 0x00, 0x00, 0x00]
let type_section = section_type()  // 1
let i32_type = valtype_i32()       // 0x7f
```""",
        },
    },
    {
        "path": "modules/wit.md",
        "title": "std::wit",
        "description": "Source-backed docs for WIT helpers.",
        "modules": ["std::wit"],
        "overview": {
            "summary": (
                "The `std::wit` module exposes WebAssembly Interface Types (WIT) primitive types "
                "as a typed enum plus explicit integer conversions for interop and introspection. "
                "These are building blocks for component-model tooling and WIT-aware utilities."
            ),
            "highlights": [
                ("`WitType` + `wit_type_bool()` … `wit_type_string()`", "Typed constructors for each WIT primitive type."),
                ("`wit_type_id(ty)` / `wit_type_from_id(id)`", "Explicit conversions between `WitType` and raw numeric IDs."),
                ("`wit_type_name(ty)`", "Map a `WitType` value back to its human-readable name."),
            ],
            "typical_usage": """\
```ark
import std::wit

let t = wit_type_u32()
let id = wit_type_id(t)       // 4
let name = wit_type_name(t)   // "u32"
```""",
        },
    },
    {
        "path": "modules/http.md",
        "title": "std::host::http",
        "description": "Source-backed docs for HTTP client operations.",
        "modules": ["std::host::http"],
        "overview": {
            "summary": (
                "The `std::host::http` module defines HTTP/1.1 client helpers "
                "(provisional). It is **not user-reachable** on the current "
                "selfhost compile path — host bindings are tracked by "
                "[#446](../../../issues/done/446-std-host-http-implementation.md) and "
                "native WASI P2 HTTP by "
                "[#077](../../../issues/done/077-wasi-p2-http.md). "
                "When implemented, only plain `http://` URLs are supported — "
                "**HTTPS is not available**."
            ),
            "highlights": [
                ("`request(method, url, body)`", "Send an HTTP request with an explicit method, URL, and body."),
                ("`get(url)`", "Send an HTTP GET request and return the response body as a string."),
            ],
            "typical_usage": """\
```ark
import std::host::http

let body = http::get("http://example.com")
match body {
    Ok(s)  => println(s),
    Err(e) => eprintln("error: " + e),
}
```""",
        },
    },
    {
        "path": "modules/sockets.md",
        "title": "std::host::sockets",
        "description": "Source-backed docs for TCP socket operations.",
        "modules": ["std::host::sockets"],
        "overview": {
            "summary": (
                "The `std::host::sockets` module defines TCP socket helpers "
                "(provisional). It is **not user-reachable** on the current "
                "selfhost compile path — host bindings are tracked by "
                "[#447](../../../issues/done/447-std-host-sockets-implementation.md) and "
                "native WASI P2 sockets by "
                "[#139](../../../issues/done/139-std-wasi-sockets-p2.md). "
                "Importing this module on `wasm32` (legacy alias `wasm32-wasi-p1`) emits E0500."
            ),
            "highlights": [
                ("`connect(host, port)`", "Open a TCP connection; returns `Ok(fd)` or `Err(message)`."),
            ],
            "typical_usage": """\
```ark
import std::host::sockets

let sock = sockets::connect("localhost", 8080)
match sock {
    Ok(fd)  => println("Connected: " + i32_to_string(fd)),
    Err(e)  => eprintln("Connection failed: " + e),
}
```""",
        },
    },
]

HOST_MODULE_SOURCE_DOC_OVERRIDES: dict[str, list[str]] = {
    "std::host::http": [
        "Host HTTP client helpers (provisional). **Not user-reachable** on the",
        "current selfhost compile path — see",
        "[Capability surface](../../platform/target-runtime-and-surfaces.md#capability-surface) and issues #446 / #077.",
        "",
        "When implemented, only plaintext HTTP/1.1 over TCP is in scope;",
        "**HTTPS is not supported**.",
    ],
    "std::host::sockets": [
        "Host TCP socket helpers (provisional). **Not user-reachable** on the",
        "current selfhost compile path — see",
        "[Capability surface](../../platform/target-runtime-and-surfaces.md#capability-surface) and issues #447 / #139.",
        "",
        "Importing this module on `wasm32` (legacy alias `wasm32-wasi-p1`) emits E0500.",
    ],
    "std::host::udp": [
        "Host UDP datagram helpers (provisional). **Not user-reachable** on the",
        "current selfhost compile path — see",
        "[Capability surface](../../platform/target-runtime-and-surfaces.md#capability-surface) and issues #447 / #139.",
        "",
        "Importing this module on `wasm32` (legacy alias `wasm32-wasi-p1`) emits E0500.",
    ],
}

STDLIB_ALIAS_PAGES = [
    {
        "path": "core.md",
        "title": "std/core — generated index",
        "description": "Legacy landing page for the current core-related stdlib docs.",
        "primary_target": {"path": "modules/core.md", "label": "std::core family"},
        "links": [
            {"path": "modules/core.md", "label": "Core family docs", "notes": "Source-backed core, error, and hash modules."},
            {"path": "reference.md", "label": "Manifest reference", "notes": "Complete manifest-backed public API."},
            {"path": "cookbook.md", "label": "Cookbook", "notes": "Current-first usage examples."},
        ],
    },
    {
        "path": "io.md",
        "title": "std/io — generated index",
        "description": "Legacy landing page for the current host-capability and path stdlib docs.",
        "primary_target": {"path": "modules/io.md", "label": "std::host family"},
        "links": [
            {"path": "modules/io.md", "label": "Host family docs", "notes": "Explicit host stdio/fs/env/process/clock/random modules plus path helpers."},
            {"path": "modules/fs.md", "label": "Host filesystem", "notes": "Manifest-backed file read/write surface."},
            {"path": "modules/path.md", "label": "Path helpers", "notes": "Path manipulation helpers."},
            {"path": "modules/process.md", "label": "Process/env", "notes": "Process control and runtime environment helpers."},
            {"path": "modules/time.md", "label": "Pure time helpers", "notes": "Duration arithmetic without host clock access."},
            {"path": "modules/random.md", "label": "Deterministic random helpers", "notes": "Seeded helpers without host entropy."},
            {"path": "reference.md", "label": "Manifest reference", "notes": "Complete manifest-backed public API."},
        ],
    },
]

SOURCE_SECTION_RE = re.compile(r"//\s*---\s*(.*?)\s*---\s*$")
SOURCE_ITEM_PATTERNS = (
    ("fn", re.compile(r"pub fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(")),
    ("struct", re.compile(r"pub struct\s+([A-Za-z_][A-Za-z0-9_]*)\b")),
    ("enum", re.compile(r"pub enum\s+([A-Za-z_][A-Za-z0-9_]*)\b")),
)


@dataclass(frozen=True)
class StdlibSourceItem:
    name: str
    kind: str
    docs: list[str]
    section: str | None
    order: int


@dataclass(frozen=True)
class StdlibSourceModule:
    module: str
    source_path: Path
    docs: list[str]
    items: list[StdlibSourceItem]


def load_toml(path: Path) -> dict:
    return tomllib.loads(path.read_text(encoding="utf-8"))


def validate_manifest_schema(manifest: dict) -> list[str]:
    """Validate every [[functions]] entry against the manifest schema.

    Returns a list of error strings (empty if all entries are valid).
    Rules enforced:
      - Every entry must have: name, params, returns, stability, doc_category
      - `stability` must be one of STABILITY_LABELS
      - `kind`, when present, must be one of VALID_KIND_VALUES
      - `target`, when present, must be a list
      - `kind = "host_stub"` entries must also have `module` and `target`
    """
    violations: list[str] = []
    functions = manifest.get("functions", [])
    deprecation_policy = manifest.get("deprecation_policy", {})
    if not deprecation_policy.get("default_since"):
        violations.append("deprecation_policy: missing default_since")
    if not deprecation_policy.get("default_remove_in"):
        violations.append("deprecation_policy: missing default_remove_in")
    if int(deprecation_policy.get("minimum_complete_releases", 0)) < 1:
        violations.append("deprecation_policy: minimum_complete_releases must be >= 1")

    for entry in functions:
        fn_name = entry.get("name", "<unnamed>")
        label = f"function '{fn_name}'"

        # 1. Required fields on every entry
        for field in FUNCTION_REQUIRED_FIELDS:
            if field not in entry:
                violations.append(f"{label}: missing required field '{field}'")

        # 2. stability must be a known label
        stability = entry.get("stability")
        if stability is not None and stability not in STABILITY_LABELS:
            violations.append(
                f"{label}: invalid stability '{stability}'; "
                f"must be one of {list(STABILITY_LABELS)}"
            )

        implementation_status = entry.get("implementation_status")
        if implementation_status is not None and implementation_status not in IMPLEMENTATION_STATUSES:
            violations.append(
                f"{label}: invalid implementation_status '{implementation_status}'; "
                f"must be one of {list(IMPLEMENTATION_STATUSES)}"
            )

        deprecated_by = entry.get("deprecated_by")
        prose_deprecated = "@deprecated" in str(entry.get("doc", "")).lower()
        if (deprecated_by or prose_deprecated) and stability != "deprecated":
            violations.append(f"{label}: deprecation metadata/prose requires stability='deprecated'")
        if stability == "deprecated" and not deprecated_by:
            violations.append(f"{label}: stability='deprecated' requires deprecated_by")

        # 3. kind must be a known value when present
        kind = entry.get("kind")
        if kind is not None and kind not in VALID_KIND_VALUES:
            violations.append(
                f"{label}: invalid kind '{kind}'; "
                f"must be one of {sorted(VALID_KIND_VALUES)}"
            )

        # 4. target must be a list when present
        target = entry.get("target")
        if target is not None and not isinstance(target, list):
            violations.append(
                f"{label}: 'target' must be a list of strings, got {type(target).__name__}"
            )

        # 5. kind-specific required fields
        if kind in FUNCTION_KIND_REQUIRED:
            for field in FUNCTION_KIND_REQUIRED[kind]:
                if field not in entry:
                    violations.append(
                        f"{label}: kind='{kind}' requires field '{field}'"
                    )

    return violations


def fixture_count() -> int:
    """Count non-comment entries in tests/fixtures/manifest.txt.

    Prefer ``state['verification']['fixture_manifest_count']`` for published
    docs numbers. This helper is for freshness / drift checks only.
    """
    return sum(
        1
        for line in FIXTURE_MANIFEST.read_text(encoding="utf-8").splitlines()
        if line.strip() and not line.strip().startswith("#")
    )


def format_fixture_harness(verification: dict, *, with_manifest: bool = True) -> str:
    """Harness outcomes (observed snapshot) vs registry size (distinct units)."""
    passed = verification.get("fixture_passed", 0)
    failed = verification.get("fixture_failures", 0)
    skipped = verification.get("fixture_skipped", 0)
    observed = verification.get("fixture_harness_observed")
    if observed is None:
        observed = int(passed) + int(failed) + int(skipped)
    base = (
        f"{passed} passed, {failed} failed, {skipped} skipped "
        f"(observed harness: {observed})"
    )
    if not with_manifest:
        return base
    manifest_count = verification.get("fixture_manifest_count")
    if manifest_count is None:
        return base
    return f"{base}; registry: {manifest_count} manifest entries"


def format_component_emit_summary(state: dict | None = None) -> str:
    """Multi-axis component availability — never flatten to available true/false."""
    path = DOCS / "data" / "component-availability.toml"
    if path.is_file():
        data = load_toml(path)
        meta = data.get("meta", {})
        target = meta.get("target")
        if not target and state:
            target = state.get("targets", {}).get("component_target", "wasm32-gc")
        surfaces = data.get("surfaces", [])
        axes = "; ".join(f"`{s['id']}`=`{s['status']}`" for s in surfaces)
        note = meta.get("implementation_note", "")
        suffix = f" — {note}" if note else ""
        return f"multi-axis on `{target}`: {axes}{suffix}"
    if state:
        targets = state.get("targets", {})
        return (
            f"on `{targets.get('component_target')}` "
            f"({targets.get('component_note', 'see docs/data/component-availability.toml')})"
        )
    return "see docs/data/component-availability.toml"


def fixture_manifest_count_from_state(state: dict, fallback: int | None = None) -> int:
    """Published fixture total must come from project-state.toml, not a re-count."""
    count = state.get("verification", {}).get("fixture_manifest_count")
    if count is not None:
        return int(count)
    if fallback is not None:
        return int(fallback)
    return fixture_count()


def escape_table(text: str) -> str:
    stripped = re.sub(r"\[([^\]]+)\]\([^)]+\)", r"\1", text)
    stripped = re.sub(r"`([^`]+)`", r"\1", stripped)
    stripped = stripped.replace("**", "").replace("*", "")
    stripped = re.sub(r"\s+", " ", stripped).strip()
    return stripped.replace("|", r"\|")


def extract_doc_entry(path: Path, base_dir: Path) -> DocEntry:
    lines = path.read_text(encoding="utf-8").splitlines()
    title = path.stem
    summary = ""
    in_code = False
    for line in lines:
        stripped = line.strip()
        if stripped.startswith("# ") and title == path.stem:
            title = stripped[2:].strip()
            continue
        if stripped.startswith("```"):
            in_code = not in_code
            continue
        if in_code or not stripped:
            continue
        if stripped.startswith("<!--") or stripped == "---":
            continue
        if stripped.startswith("> This file is generated by"):
            continue
        if stripped.startswith("#") or stripped.startswith("|"):
            continue
        if stripped.startswith("- ") or stripped.startswith("* "):
            continue
        if re.match(r"\d+\.\s", stripped):
            continue
        if stripped.startswith(">"):
            stripped = stripped.lstrip(">").strip()
        summary = escape_table(stripped)
        break
    if not summary:
        summary = "See the document for details."
    return DocEntry(
        rel_path=path.relative_to(base_dir).as_posix(),
        title=title,
        summary=summary,
    )


def _git_tracked_files(directory: Path) -> set[Path]:
    """Return the set of git-tracked files under *directory*."""
    import subprocess
    try:
        result = subprocess.run(
            ["git", "ls-files", "--full-name", str(directory.relative_to(ROOT))],
            capture_output=True, text=True, cwd=ROOT, check=True,
        )
        return {ROOT / line for line in result.stdout.splitlines() if line}
    except Exception:
        return set()


def collect_markdown_entries(section_dir: Path) -> list[DocEntry]:
    tracked = _git_tracked_files(section_dir)
    entries: list[DocEntry] = []
    for path in sorted(section_dir.rglob("*.md")):
        if path.name == "README.md":
            continue
        if tracked and path.resolve() not in tracked:
            continue
        entries.append(extract_doc_entry(path, section_dir))
    return entries


def humanize_slug(value: str) -> str:
    return value.replace("-", " ").replace("_", " ").title()


def collect_examples(state: dict) -> list[dict]:
    baseline_cases = {Path(case).name for case in state["perf"]["baseline_cases"]}
    entries: list[dict] = []
    for path in sorted((DOCS / "examples").glob("*.ark")):
        expected_path = path.with_suffix(".expected")
        entries.append(
            {
                "file": path.name,
                "title": humanize_slug(path.stem),
                "expected": "yes" if expected_path.exists() else "no",
                "baseline": "yes" if path.name in baseline_cases else "no",
                "run": f"`target/release/arukellt run docs/examples/{path.name}`",
            }
        )
    return entries


def collect_sample_files() -> list[str]:
    sample_dir = DOCS / "sample"
    return [path.name for path in sorted(sample_dir.iterdir()) if path.is_file()]


def load_stdlib_manifest() -> dict:
    return load_toml(STDLIB_MANIFEST)


def stdlib_stats(manifest: dict) -> dict:
    types = manifest.get("types", [])
    values = manifest.get("values", [])
    functions = manifest.get("functions", [])
    public_functions = [entry for entry in functions if not entry["name"].startswith("__intrinsic_")]
    prelude_functions = [entry for entry in public_functions if entry.get("prelude")]
    category_counts = Counter(entry.get("doc_category", "misc") for entry in public_functions)
    return {
        "types": types,
        "values": values,
        "functions": functions,
        "public_functions": public_functions,
        "prelude_functions": prelude_functions,
        "category_counts": category_counts,
    }


def rel_link(from_path: Path, to_path: Path) -> str:
    return Path(os.path.relpath(to_path, from_path.parent)).as_posix()


def module_source_path(module_name: str) -> Path:
    parts = module_name.split("::")[1:]
    candidate = ROOT / "std" / Path(*parts)
    mod_path = candidate / "mod.ark"
    file_path = candidate.with_suffix(".ark")
    if mod_path.exists():
        return mod_path
    if file_path.exists():
        return file_path
    raise FileNotFoundError(f"no std source file found for {module_name}")


def extract_stdlib_source_module(module_name: str) -> StdlibSourceModule:
    source_path = module_source_path(module_name)
    lines = source_path.read_text(encoding="utf-8").splitlines()
    module_docs: list[str] = []
    items: list[StdlibSourceItem] = []
    pending_docs: list[str] = []
    current_section: str | None = None
    collecting_module_docs = True
    item_order = 0

    for raw_line in lines:
        stripped = raw_line.strip()
        if collecting_module_docs:
            if stripped.startswith("//!"):
                module_docs.append(stripped[3:].lstrip())
                continue
            if not stripped:
                if module_docs:
                    module_docs.append("")
                continue
            collecting_module_docs = False

        if stripped.startswith("///"):
            pending_docs.append(stripped[3:].lstrip())
            continue

        section_match = SOURCE_SECTION_RE.fullmatch(stripped)
        if section_match:
            current_section = section_match.group(1).strip()
            pending_docs = []
            continue

        matched_item = None
        for kind, pattern in SOURCE_ITEM_PATTERNS:
            match = pattern.search(stripped)
            if match:
                matched_item = (kind, match.group(1))
                break
        if matched_item is not None:
            item_order += 1
            kind, name = matched_item
            items.append(
                StdlibSourceItem(
                    name=name,
                    kind=kind,
                    docs=pending_docs.copy(),
                    section=current_section,
                    order=item_order,
                )
            )
            pending_docs = []
            continue

        if not stripped or not stripped.startswith("//"):
            pending_docs = []

    while module_docs and not module_docs[-1]:
        module_docs.pop()
    return StdlibSourceModule(module=module_name, source_path=source_path, docs=module_docs, items=items)


def collect_stdlib_source_modules() -> dict[str, StdlibSourceModule]:
    modules = {
        module
        for page in STDLIB_MODULE_PAGES
        for module in page["modules"]
    }
    return {module: extract_stdlib_source_module(module) for module in sorted(modules)}


def source_doc_summary(lines: list[str]) -> str:
    for line in lines:
        stripped = line.strip()
        if stripped:
            return escape_table(stripped)
    return "-"


def render_source_doc_block(lines: list[str], fallback: str) -> list[str]:
    if not lines:
        return [fallback]
    rendered: list[str] = []
    for line in lines:
        match = re.match(r"^(#+)(\s+.*)$", line)
        if match:
            hashes, rest = match.groups()
            rendered.append("#" * min(len(hashes) + 1, 6) + rest)
        else:
            rendered.append(line)
    return rendered


def format_stability_counts(entries: list[dict]) -> str:
    counts = Counter(entry.get("stability", "unknown") for entry in entries)
    return ", ".join(f"{name} {count}" for name, count in sorted(counts.items()))


def format_host_module_badges(
    module_name: str,
    functions: list[dict],
    manifest_modules: dict[str, dict],
) -> list[str]:
    """Return badge lines for a host module section.

    Returns an empty list for non-host modules.
    For ``std::host::*`` modules, returns a blockquote badge line showing:
      - Target constraint (from manifest module metadata or function targets)
      - Implementation status (implemented vs stub, derived from ``kind``)
    """
    if not module_name.startswith("std::host::"):
        return []

    # Availability is per-symbol. A module with different symbol coverage must
    # not be flattened to a single target badge.
    t3_only = _availability_t3_only(functions)
    unbacked = _availability_unbacked(functions)
    availability_shapes = {
        (fn.get("availability", {}).get("t1"), fn.get("availability", {}).get("t3"))
        for fn in functions
    }
    if len(availability_shapes) > 1:
        availability = "⚠️ **Availability:** mixed — see individual symbols"
    elif unbacked:
        availability = "⚠️ **Availability:** not user-reachable"
    elif t3_only:
        availability = "🎯 **Availability:** `wasm32-gc` only"
    else:
        availability = "🎯 **Availability:** `wasm32` and `wasm32-gc`"

    # Determine implementation status from function ``kind`` fields and
    # explicit ``implementation_status`` annotations.
    stub_count = sum(1 for fn in functions if fn.get("kind") == "host_stub")
    unreachable_count = sum(1 for fn in functions if _function_semantic_status(fn) == "unreachable")
    limited_count = sum(1 for fn in functions if _function_semantic_status(fn) not in ("functional", "unreachable"))
    total = len(functions) if functions else 0
    if unbacked or (total > 0 and unreachable_count == total):
        status = "⚠️ **Status:** not user-reachable on selfhost path"
    elif total > 0 and stub_count == total:
        status = "⚠️ **Status:** stub — not yet implemented"
    elif unreachable_count > 0 and unreachable_count == total:
        status = "⚠️ **Status:** not user-reachable on selfhost path"
    elif stub_count > 0 or unreachable_count > 0:
        mixed_parts = []
        if stub_count > 0:
            mixed_parts.append(f"{stub_count}/{total} stub")
        if unreachable_count > 0:
            mixed_parts.append(f"{unreachable_count}/{total} unreachable")
        status = f"⚠️ **Status:** mixed — {', '.join(mixed_parts)}"
    elif limited_count > 0:
        status = f"⚠️ **Status:** partial — {limited_count}/{total} APIs have limited or placeholder semantics"
    else:
        status = "✅ **Status:** implemented"

    return [
        "",
        f"> {availability} · {status}",
        "",
    ]


def _function_semantic_status(entry: dict) -> str:
    """Classify manifest-described runtime semantics without equating presence with function."""
    explicit = entry.get("implementation_status")
    if explicit:
        return explicit
    if entry.get("kind") == "host_stub":
        return "stub"
    evidence = " ".join(
        str(value) for value in (
            entry.get("doc", ""),
            entry.get("errors", ""),
            entry.get("availability", {}).get("note", ""),
        )
    ).lower()
    limited_markers = (
        "always false",
        "always returns err",
        "always err",
        "honest stub",
        "honest rejection",
        "not yet supported",
        "not yet exposed",
        "does not distinguish file types",
    )
    return "limited" if any(marker in evidence for marker in limited_markers) else "functional"


def join_pipeline(parts: list[str]) -> str:
    return " -> ".join(parts)


def render_target_table(state: dict) -> str:
    rows = [
        "| Target | Support Tier | Implementation | Contract Stability | Run | Notes |",
        "|--------|--------------|----------------|--------------------|-----|-------|",
    ]
    for profile in state.get("target_profiles", []):
        run = "Yes" if profile.get("run_supported") else "No"
        rows.append(
            "| `{}` | {} | {} | {} | {} | {} |".format(
                profile["id"],
                profile["support_tier"],
                profile["implementation_state"],
                profile["contract_stability"],
                run,
                escape_table(profile.get("role", "")),
            )
        )
    return "\n".join(rows)


def render_host_profile_table(state: dict) -> str:
    rows = [
        "| Host profile | Targets | Planned | Support Tier | Implementation | Contract Stability | Notes |",
        "|--------------|---------|---------|--------------|----------------|--------------------|-------|",
    ]
    for profile in state.get("host_profiles", []):
        targets = ", ".join(f"`{t}`" for t in profile.get("supported_targets", []))
        planned = ", ".join(f"`{t}`" for t in profile.get("planned_targets", [])) or "—"
        rows.append(
            "| `{}` | {} | {} | {} | {} | {} | {} |".format(
                profile["id"],
                targets,
                planned,
                profile["support_tier"],
                profile["implementation_state"],
                profile["contract_stability"],
                escape_table(profile.get("role", "")),
            )
        )
    return "\n".join(rows)


def write_target_contract_summary(state: dict, check: bool, stale: list[Path]) -> None:
    """Generate docs/data/target-contract-summary.md from project-state.toml (SSOT)."""
    lines = [
        "# Target / host contract summary",
        "",
        "> **Generated** from `docs/data/project-state.toml` by `scripts/gen/generate-docs.py`.",
        "> Do not hand-edit the tables below. Edit `project-state.toml` instead.",
        ">",
        "> Axes:",
        "> - **Support Tier**: primary | supported | scaffold | not-started (ADR-007/013)",
        "> - **Implementation**: complete | partial | scaffold | unimplemented",
        "> - **Contract Stability**: stable | provisional | experimental (never `unimplemented`)",
        ">",
        "> Host profiles are separate from language targets.",
        "",
        "<!-- BEGIN GENERATED:CURRENT_STATE_TARGET_SUMMARY_SOURCE -->",
        render_target_table(state),
        "<!-- END GENERATED:CURRENT_STATE_TARGET_SUMMARY_SOURCE -->",
        "",
        "<!-- BEGIN GENERATED:HOST_PROFILE_SUMMARY_SOURCE -->",
        render_host_profile_table(state),
        "<!-- END GENERATED:HOST_PROFILE_SUMMARY_SOURCE -->",
        "",
    ]
    write_file(TARGET_CONTRACT, "\n".join(lines), check, stale)


def render_current_state_targets(state: dict) -> str:
    return "\n".join(
        [
            "## Targets",
            "",
            render_target_table(state),
            "",
            "### Host profiles",
            "",
            render_host_profile_table(state),
        ]
    )


def _parse_markdown_table_row(line: str) -> list[str]:
    stripped = line.strip()
    if not stripped.startswith("|") or not stripped.endswith("|"):
        raise ValueError(f"invalid markdown table row: {line}")
    return [cell.strip() for cell in stripped[1:-1].split("|")]


def render_current_state_updated(state: dict) -> str:
    updated = state["project"]["updated"]
    cmd = state["project"].get("verification_command", "python3 scripts/manager.py verify quick")
    verification = state.get("verification", {})
    source_commit = verification.get("source_commit", verification.get("last_verified_commit", "not-recorded"))
    generated_at = verification.get("last_verified_at", updated)
    blockers = verification.get("blockers", [])
    failures = sum(int(blocker.get("affected_count", 1)) for blocker in blockers if blocker.get("category") == "fixture")
    check_gap = sum(int(blocker.get("affected_count", 1)) for blocker in blockers if blocker.get("category") == "verification")
    ready = not blockers
    readiness = "READY" if ready else "NOT READY"
    blocking = []
    if failures:
        blocking.append(f"{failures} fixture failure(s)")
    if check_gap:
        blocking.append(f"{check_gap} verification check failure(s)")
    full_only = [blocker for blocker in blockers if blocker.get("category") not in {"fixture", "verification"}]
    if full_only:
        blocking.append(f"{len(full_only)} additional full-verification blocker group(s)")
    blocking_line = ", ".join(blocking) if blocking else "none"
    return "\n".join(
        [
            f"> Updated: {updated}.",
            f"> Generated-At: {generated_at}",
            f"> Source-Commit: `{source_commit}`",
            f"> Verification-Command: `{cmd}`",
            f"> Release-Readiness: **{readiness}**",
            f"> Blocking: {blocking_line}",
        ]
    )


def render_current_state_test_health(state: dict, fixture_total: int) -> str:
    verification = state["verification"]
    manifest_count = fixture_manifest_count_from_state(state, fixture_total)
    observed = verification.get("fixture_harness_observed")
    if observed is None:
        observed = (
            int(verification.get("fixture_passed", 0))
            + int(verification.get("fixture_failures", 0))
            + int(verification.get("fixture_skipped", 0))
        )
    remainder = verification.get("fixture_not_in_last_harness_snapshot")
    if remainder is None and verification.get("fixture_manifest_count") is not None:
        remainder = int(verification["fixture_manifest_count"]) - int(observed)
    accounting = verification.get("fixture_accounting_note", "")
    lines = [
        "## Test Health",
        "",
        f"- Unit tests: {verification['unit_tests_note']}",
        f"- Fixture harness (observed snapshot): {format_fixture_harness(verification, with_manifest=False)}",
        f"- Fixture registry: {manifest_count} manifest entries (distinct unit from harness outcomes)",
    ]
    if remainder is not None:
        lines.append(
            f"- Not in last harness snapshot: {remainder} registry entries "
            f"(not proof they fail)"
        )
    if accounting:
        lines.append(f"- Accounting note: {accounting}")
    lines.extend(
        [
            "- Wasm validation is a hard error (W0004)",
            f"- Verification entry point: `{state['project']['verification_command']}` — "
            f"**{verification['checks_passed']}/{verification['checks_total']} checks pass**",
        ]
    )
    blockers = verification.get("blockers", [])
    source_commit = verification.get("source_commit", "")
    lines.extend(
        [
            "",
            "### Active blockers",
            "",
            "This table is generated from structured blocker records. Counts above must equal these rows.",
            "",
            "| ID | Scope | Category | Affected | Failure summary | Command | Owner | Issue | First seen | Last verified |",
            "|----|-------|----------|---------:|-----------------|---------|-------|-------|------------|---------------|",
        ]
    )
    if blockers:
        for blocker in blockers:
            last_vc = blocker["last_verified_commit"]
            stale_tag = " ⏰STALE" if (source_commit and last_vc and last_vc != source_commit) else ""
            lines.append(
                "| `{id}` | `{scope}` | `{category}` | {count} | {summary} | `{command}` | {owner} | {issue} | `{first}` | `{last}`{stale} |".format(
                    id=blocker["id"],
                    category=blocker["category"],
                    scope=blocker["scope"],
                    count=blocker.get("affected_count", 1),
                    summary=escape_table(blocker["summary"]),
                    command=blocker["command"],
                    owner=blocker["owner"],
                    issue=blocker["issue"],
                    first=blocker["first_seen_commit"],
                    last=last_vc,
                    stale=stale_tag,
                )
            )
    else:
        lines.append("| — | — | — | 0 | No active blockers in the recorded verification run. | — | — | — | — | — |")
    return "\n".join(lines)


def render_current_state_perf(state: dict) -> str:
    perf = state["perf"]
    lines = [
        "## Baseline and Perf Gates",
        "",
        "- Baselines are materialized under `tests/baselines/`",
        "- Compile-time baseline cases:",
    ]
    lines.extend(f"  - `{case}`" for case in perf["baseline_cases"])
    lines.extend(
        [
            "- Current thresholds:",
            f"  - `arukellt check`: median compile time regression must stay within {perf['check_budget_pct']}%",
            f"  - `arukellt compile`: median compile time regression must stay within {perf['compile_budget_pct']}%",
            f"- {perf['heavy_note']}",
        ]
    )
    return "\n".join(lines)


def render_current_state_diagnostics(state: dict) -> str:
    lines = [
        "## Diagnostics and Validation",
        "",
        "- Canonical diagnostics registry lives in `src/compiler/diagnostics.ark`",
        "- Diagnostics are tracked by code, severity, and phase origin",
    ]
    for diagnostic in state["diagnostics"]:
        lines.append(
            f"- `{diagnostic['code']}`: {diagnostic['summary']} ({diagnostic['severity']}, `{diagnostic['phase']}`)"
        )
    lines.append("- Structured diagnostic snapshots are available for tests/docs via `ARUKELLT_DUMP_DIAGNOSTICS=1`")
    return "\n".join(lines)


def render_readme_status(state: dict, fixture_total: int, manifest_stats: dict) -> str:
    verification = state["verification"]
    targets = state["targets"]
    harness_str = format_fixture_harness(verification)
    return "\n".join(
        [
            "## Status",
            "",
            f"- Updated: {state['project']['updated']}",
            f"- CLI default target: `{targets['cli_default']}`",
            f"- Canonical target: `{targets['canonical']}`",
            f"- Component/WIT target: `{targets['component_target']}`",
            f"- Unit tests: {verification['unit_tests_note']}",
            f"- Fixture harness: {harness_str}",
            f"- Verification: `{state['project']['verification_command']}` — {verification['checks_passed']}/{verification['checks_total']} checks pass",
            f"- Stdlib manifest-backed public API: {len(manifest_stats['public_functions'])} functions",
        ]
    )


def render_root_docs_readme(sections: list[dict], state: dict, fixture_total: int, manifest_stats: dict) -> str:
    _harness = format_fixture_harness(state["verification"])
    lines = [
        "# Arukellt Documentation",
        "",
        "> This file is generated by `python3 scripts/gen/generate-docs.py`.",
        f"> Source of truth: current behavior is [`current-state.md`](current-state.md); structured state lives in [`data/project-state.toml`](data/project-state.toml) and [`../std/manifest.toml`](../std/manifest.toml).",
        "",
        "## Current Snapshot",
        "",
        f"- Updated: {state['project']['updated']}",
        f"- CLI default target: `{state['targets']['cli_default']}`",
        f"- Canonical target: `{state['targets']['canonical']}`",
        f"- Component emit: {format_component_emit_summary(state)} ([axes](data/component-availability.md))",
        f"- Fixture harness: {_harness}",
        f"- Verification: `{state['project']['verification_command']}` — {state['verification']['checks_passed']}/{state['verification']['checks_total']} checks pass",
        f"- Stdlib manifest-backed public API: {len(manifest_stats['public_functions'])} functions",
        "",
        "- [Current state](current-state.md)",
        "- [Quickstart](quickstart.md)",
        "- [コンパイラ](compiler/README.md)",
        "- [標準ライブラリ](stdlib/README.md)",
        "- [Contributing](contributing.md)",
        "",
        "## Root reference docs",
        "",
        "| Doc | Role |",
        "|-----|------|",
        "| [cli-reference.md](cli-reference.md) | CLI surface reference |",
        "| [cli-startup-contract.md](cli-startup-contract.md) | CLI startup / env contract |",
        "| [ark-toml.md](ark-toml.md) | `ark.toml` schema |",
        "| [test-strategy.md](test-strategy.md) | Test category strategy |",
        "| [release-criteria.md](release-criteria.md) | Release guarantee tiers |",
        "| [release-checklist.md](release-checklist.md) | Executable release checklist |",
        "| [retention-policy.md](retention-policy.md) | Docs retention / archive zones |",
        "| [debug-support.md](debug-support.md) | Debug / DAP support status |",
        "| [capability-surface.md](capability-surface.md) | Host capability reachability matrix |",
        "| [data/cli-surface.md](data/cli-surface.md) | Generated CLI surface from cli-surface.toml |",
        "| [data/bootstrap-contract.md](data/bootstrap-contract.md) | Generated bootstrap contract (ADR-029) |",
        "| [data/release-guarantees.md](data/release-guarantees.md) | Generated release guarantees matrix |",
        "| [data/component-availability.md](data/component-availability.md) | Component availability axes |",
        "| [data/target-contract-summary.md](data/target-contract-summary.md) | Generated target contract summary |",
        "| [directory-ownership.md](directory-ownership.md) | Directory ownership map |",
        "| [release/README.md](release/README.md) | Release criteria + checklist entry |",
        "| [governance/document-ownership.md](governance/document-ownership.md) | Shared ownership schema |",
        "",
        "> [overview.html](overview.html) is an **archived** visual map (pre-canonical target rename).",
        "> Do not treat it as current behavior — use [current-state.md](current-state.md).",
    ]
    category_labels = {
        "current": "Current Docs",
        "supporting": "Supporting Docs",
        "archive": "Archive / History",
    }
    for category in ("current", "supporting", "archive"):
        category_sections = [section for section in sections if section["category"] == category]
        if not category_sections:
            continue
        lines.extend(["", f"## {category_labels[category]}", "", "| Section | Entry | Notes |", "|---------|-------|-------|"])
        for section in category_sections:
            lines.append(
                "| {} | [{}]({}/README.md) | {} |".format(
                    section["title"],
                    section["dir"],
                    section["dir"],
                    escape_table(section["description"]),
                )
            )
    lines.extend([
        "",
        "## Generated vs Hand-Written Files",
        "",
        "Files marked *generated* are fully produced by `python3 scripts/gen/generate-docs.py`.",
        "Files marked *marker-updated* have inline `<!-- GENERATED:xxx -->` blocks replaced but are otherwise hand-written.",
        "Do not edit generated files manually — changes will be overwritten on the next regeneration.",
        "",
        "| File | Status |",
        "|------|--------|",
        "| `docs/README.md` | generated |",
        "| `docs/_sidebar.md` | generated |",
        "| `docs/stdlib/reference.md` | generated |",
        "| `docs/stdlib/name-index.md` | generated |",
        "| `docs/stdlib/modules/*.md` | generated |",
        "| `docs/compiler/README.md` | generated |",
        "| `docs/language/README.md` | generated |",
        "| `docs/stdlib/README.md` | generated |",
        "| `README.md` (repo root) | marker-updated |",
        "| `docs/current-state.md` | marker-updated |",
        "| `docs/data/target-contract-summary.md` | generated |",
        "| section `*/README.md` under `docs/` | generated |",
        "| `docs/history/benchmarks/benchmark-results.md` | historical (benchmark runner) |",
        "| `docs/data/project-state.toml` | hand-written SSOT (input) |",
        "| `docs/data/sections.toml` | hand-written SSOT (input) |",
        "| other hand-written `docs/**/*.md` | hand-written |",
    ])
    return "\n".join(lines) + "\n"


def render_sidebar(sections: list[dict]) -> str:
    category_labels = {
        "current": "Current Docs",
        "supporting": "Supporting Docs",
        "archive": "Archive / History",
    }
    lines = [
        "<!-- generated by scripts/gen/generate-docs.py — do not edit manually -->",
        "- **Arukellt**",
        "  - [ホーム](#/)",
        "  - [Docs Overview](#/README)",
        "  - [Current state](#/current-state)",
        "  - [クイックスタート](#/quickstart)",
        "  - [Contributing](#/contributing)",
    ]
    for category in ("current", "supporting", "archive"):
        category_sections = [section for section in sections if section["category"] == category]
        if not category_sections:
            continue
        lines.extend(["", f"- **{category_labels[category]}**"])
        if category == "archive":
            lines.append("  - [見取り図 HTML（アーカイブ）](#/overview.html)")
        for section in category_sections:
            lines.append(f"  - [{section['title']}](#/{section['dir']}/README)")
            if section["dir"] == "playground":
                lines.append("    - [▶ Try Playground](#/playground/index.html)")
    return "\n".join(lines) + "\n"


def render_generic_section_readme(section: dict, entries: list[DocEntry], snapshot_lines: list[str]) -> str:
    lines = [
        f"# {section['title']}",
        "",
        "> This file is generated by `python3 scripts/gen/generate-docs.py`.",
        section["description"],
        "",
        "## Current Snapshot",
        "",
    ]
    lines.extend(snapshot_lines or ["- Current source of truth: [../current-state.md](../current-state.md)"])
    is_archive = section.get("snapshot") == "archive"
    if section.get("dir") == "rfcs":
        lines.extend(
            ["", "## Documents", "", "| File | Title | Status | Summary |", "|------|-------|--------|---------|"]
        )
        for entry in entries:
            status = _rfc_status_for_entry(entry)
            lines.append(
                f"| [{entry.rel_path}]({entry.rel_path}) | {escape_table(entry.title)} | {status} | {entry.summary} |"
            )
    elif is_archive:
        lines.extend(["", "## Documents", "", "| File | Title | Label | Summary |", "|------|-------|-------|---------|"])
        for entry in entries:
            lines.append(f"| [{entry.rel_path}]({entry.rel_path}) | {escape_table(entry.title)} | Archive | {entry.summary} |")
    else:
        lines.extend(["", "## Documents", "", "| File | Title | Summary |", "|------|-------|---------|"])
        for entry in entries:
            lines.append(f"| [{entry.rel_path}]({entry.rel_path}) | {escape_table(entry.title)} | {entry.summary} |")
    return "\n".join(lines) + "\n"


_ADR_STATUS_RE = re.compile(
    r"(?:ステータス|\*\*Status\*\*|Status)\s*[:：]\s*\*?\*?([A-Za-z]+)",
    re.IGNORECASE,
)
_RFC_STATUS_RE = re.compile(
    r"(?:ステータス|\*\*Status\*\*|Status)\s*[:：]\s*\*?\*?([A-Za-z/_-]+)",
    re.IGNORECASE,
)
_ADR_STATUS_SECTIONS = (
    ("ACCEPTED", "採択"),
    ("PROPOSED", "提案"),
    ("DEFERRED", "保留"),
    ("REJECTED", "却下"),
    ("SUPERSEDED", "後継済み"),
)


def _adr_status_for_entry(entry: DocEntry) -> str:
    path = DOCS / "adr" / entry.rel_path
    if not path.is_file():
        return "UNKNOWN"
    for line in path.read_text(encoding="utf-8").splitlines()[:20]:
        m = _ADR_STATUS_RE.search(line)
        if m:
            return m.group(1).upper()
    return "UNKNOWN"


def _rfc_status_for_entry(entry: DocEntry) -> str:
    """RFC document status (DRAFT/ACCEPTED/...), not section Archive category."""
    path = DOCS / "rfcs" / entry.rel_path
    if not path.is_file():
        return "UNKNOWN"
    for line in path.read_text(encoding="utf-8").splitlines()[:30]:
        if "ステータス" not in line and "Status" not in line and "status" not in line:
            continue
        for token in ("DRAFT", "ACCEPTED", "SUPERSEDED", "ARCHIVED", "REJECTED"):
            if token.lower() in line.lower():
                return token
        # Japanese prose statuses used by older RFCs
        if "運用メモ" in line:
            return "NOTE"
        if "仕様草案" in line:
            return "DRAFT"
        m = _RFC_STATUS_RE.search(line)
        if m:
            return m.group(1).upper().strip("*")
    return "UNKNOWN"


def render_adr_readme(section: dict, entries: list[DocEntry], snapshot_lines: list[str]) -> str:
    """ADR index grouped by status (ADR-000); no uniform Archive label."""
    lines = [
        f"# {section['title']}",
        "",
        "> This file is generated by `python3 scripts/gen/generate-docs.py`.",
        section["description"],
        "",
        "これらの文書は設計判断の記録であり、現行挙動の正本ではない。",
        "現行挙動は [../current-state.md](../current-state.md) を参照。",
        "",
        "ステータスライフサイクル: [ADR-000-process.md](ADR-000-process.md)",
        "（`PROPOSED` → `ACCEPTED` → `SUPERSEDED`。補助: `REJECTED` / `DEFERRED`）。",
        "",
        "台帳検査: `python3 scripts/check/check-adrs.py`。",
    ]

    by_status: dict[str, list[DocEntry]] = {name: [] for name, _ in _ADR_STATUS_SECTIONS}
    by_status["OTHER"] = []
    for entry in entries:
        status = _adr_status_for_entry(entry)
        if status in by_status:
            by_status[status].append(entry)
        else:
            by_status["OTHER"].append(entry)

    for status, heading in _ADR_STATUS_SECTIONS:
        group = by_status[status]
        if not group:
            continue
        lines.extend(
            [
                "",
                f"## {heading}",
                "",
                "| ファイル | タイトル | 要約 |",
                "|----------|----------|------|",
            ]
        )
        for entry in group:
            lines.append(
                f"| [{entry.rel_path}]({entry.rel_path}) | {escape_table(entry.title)} | {entry.summary} |"
            )

    if by_status["OTHER"]:
        lines.extend(
            [
                "",
                "## その他",
                "",
                "| ファイル | タイトル | 要約 |",
                "|----------|----------|------|",
            ]
        )
        for entry in by_status["OTHER"]:
            lines.append(
                f"| [{entry.rel_path}]({entry.rel_path}) | {escape_table(entry.title)} | {entry.summary} |"
            )

    return "\n".join(lines) + "\n"


def parse_spec_stability_sections() -> list[dict]:
    """Parse docs/language/spec.md and extract section/subsection stability data.

    Returns a list of dicts with keys: id, name, stability, notes.
    Stability is one of: stable, provisional, experimental, unimplemented.
    Subsections marked (v1)/(v2)/etc. are treated as provisional.
    """
    if not SPEC_MD.exists():
        return []

    lines = SPEC_MD.read_text(encoding="utf-8").splitlines()
    sections: list[dict] = []
    current_section_stability: str = "stable"

    for line in lines:
        # Top-level section: ## N. Name <!-- stability: LABEL -->
        m = _SPEC_SECTION_RE.match(line)
        if m:
            sec_id = m.group(1)
            sec_name = m.group(2).strip()
            raw_stability = (m.group(3) or "").strip().lower()
            if raw_stability in STABILITY_LABELS:
                stability = raw_stability
            else:
                # "see individual entries" or empty — default to stable
                stability = "stable"
            current_section_stability = stability
            notes: str = ""
            if raw_stability == "see individual entries":
                notes = "See subsections for individual stability"
            sections.append({"id": sec_id, "name": sec_name, "stability": stability, "notes": notes})
            continue

        # Subsection: ### N.M Name
        ms = _SPEC_SUBSECTION_RE.match(line)
        if ms:
            sub_id = ms.group(1)
            sub_name = ms.group(2).strip()
            # Version-gated features (v1/v2/...) are provisional
            if _V_FEATURE_RE.search(sub_name):
                stability = "provisional"
                sub_notes = "version-gated feature — interface may change before stable exit"
            else:
                stability = current_section_stability
                sub_notes = ""
            sections.append({"id": sub_id, "name": sub_name, "stability": stability, "notes": sub_notes})

    return sections


def render_maturity_matrix(sections: list[dict]) -> str:
    """Render the feature maturity matrix as a Markdown file."""
    from collections import Counter as _Counter
    stability_counts = _Counter(s["stability"] for s in sections)

    lines = [
        "# Feature Maturity Matrix",
        "",
        "> **Normative**: This document defines the authoritative behavior of Arukellt as implemented.",
        "> Content accuracy is manually reviewed unless a row links to a specific fixture or check ID.",
        "> For current verified state, see [../current-state.md](../current-state.md).",
        ">",
        "> This file is generated by `python3 scripts/gen/generate-docs.py`. Do not edit manually.",
        "> Source of truth: `docs/data/language-doc-classifications.toml` `[[features]]` section.",
        ">",
        "> **Update workflow:** edit `[[features]]` in the TOML → run `python3 scripts/gen/generate-docs.py` → commit.",
        "",
        "## Stability Labels",
        "",
        "| Label | Meaning |",
        "|-------|---------|",
        "| **stable** | Feature is finalized. Breaking changes require a new major version. |",
        "| **provisional** | Feature is implemented and tested, but the interface may change before stabilization. |",
        "| **experimental** | Feature exists in the codebase but is not tested or guaranteed on every build. |",
        "| **unimplemented** | Feature is specified but not yet implemented. |",
        "",
        "## Summary",
        "",
        "| Stability | Count |",
        "|-----------|-------|",
    ]
    for label in LANGUAGE_FEATURE_STABILITY_LABELS:
        lines.append(f"| {label} | {stability_counts.get(label, 0)} |")

    lines.extend([
        "",
        "## Feature Classification",
        "",
        "| § | Feature | Stability | Notes |",
        "|---|---------|-----------|-------|",
    ])
    for s in sections:
        notes_cell = escape_table(s["notes"]) if s["notes"] else "—"
        # Bold non-stable labels for visibility
        if s["stability"] in ("provisional", "experimental", "unimplemented"):
            stability_cell = f"**{s['stability']}**"
        else:
            stability_cell = s["stability"]
        lines.append(f"| {s['id']} | {escape_table(s['name'])} | {stability_cell} | {notes_cell} |")

    return "\n".join(lines) + "\n"


def load_language_classifications() -> list[dict]:
    """Load language doc classifications from docs/data/language-doc-classifications.toml."""
    if not LANGUAGE_CLASSIFICATIONS.exists():
        return []
    with open(LANGUAGE_CLASSIFICATIONS, "rb") as f:
        data = tomllib.load(f)
    return data.get("docs", [])


def load_feature_classifications() -> list[dict]:
    """Load feature maturity data from docs/data/language-doc-classifications.toml.

    Returns a list of dicts with keys: id, name, stability, notes.
    This is the authoritative source for the feature maturity matrix.
    """
    if not LANGUAGE_CLASSIFICATIONS.exists():
        return []
    with open(LANGUAGE_CLASSIFICATIONS, "rb") as f:
        data = tomllib.load(f)
    features = data.get("features", [])
    # Normalise: ensure every feature has a notes field
    for f in features:
        f.setdefault("notes", "")
    return features


def render_language_readme(
    section: dict,
    entries: list[DocEntry],
    snapshot_lines: list[str],
    classifications: list[dict],
) -> str:
    lines = [
        f"# {section['title']}",
        "",
        "> This file is generated by `python3 scripts/gen/generate-docs.py`.",
        section["description"],
        "",
        "## Current Snapshot",
        "",
    ]
    lines.extend(snapshot_lines or ["- Current source of truth: [../current-state.md](../current-state.md)"])

    # Reading Paths — purpose-driven paths for different user personas
    lines.extend([
        "",
        "## Reading Paths",
        "",
        "Choose the path that fits your goal.",
        "",
        "### 🟢 Quick Start",
        "",
        "> **Want to write Arukellt code right away?** Read the guide and you're off.",
        "",
        "| Step | Document | What You'll Learn |",
        "|------|----------|-------------------|",
        "| 1 | [guide.md](guide.md) | **Start here.** Practical walkthrough — variables, functions, structs, enums, control flow |",
        "| 2 | [error-handling.md](error-handling.md) | Result, Option, and the `?` operator — essential for real programs |",
        "",
        "### 📘 Deep Dive",
        "",
        "> **New to Arukellt and want a thorough understanding?** Follow every step.",
        "",
        "| Step | Document | What You'll Learn |",
        "|------|----------|-------------------|",
        "| 1 | [guide.md](guide.md) | **Start here.** Complete language walkthrough (14 sections) |",
        "| 2 | [type-system.md](type-system.md) | Types, generics, type inference, and trait-like behavior |",
        "| 3 | [error-handling.md](error-handling.md) | Result, Option, error propagation, and recovery patterns |",
        "| 4 | [memory-model.md](memory-model.md) | GC-native ownership, value semantics, and lifetime model |",
        "| 5 | [syntax.md](syntax.md) | Complete syntax reference for all implemented constructs |",
        "| 6 | [spec.md](spec.md) | Full normative specification for definitive answers |",
        "",
        "### 🔍 Reference Lookup",
        "",
        "> **Already know the language?** Jump straight to the topic you need.",
        "",
        "| Topic | Document | Use When… |",
        "|-------|----------|-----------|",
        "| Specification | [spec.md](spec.md) | You need the authoritative, normative answer |",
        "| Syntax | [syntax.md](syntax.md) | You want the exact syntax for a construct |",
        "| Types | [type-system.md](type-system.md) | You need type rules, generics, or inference details |",
        "| Errors | [error-handling.md](error-handling.md) | You're working with Result, Option, or `?` |",
        "| Memory | [memory-model.md](memory-model.md) | You need ownership, GC, or value-semantics rules |",
        "| Formatter | [formatter.md](formatter.md) | You need canonical layout rules for fmt / LSP formatting |",
        "| Stability | [maturity-matrix.md](maturity-matrix.md) | You want to know if a feature is stable, provisional, or experimental |",
        "",
        "### 🔮 Language Evolution",
        "",
        "> **Tracking what's changing?** These documents cover planned and in-progress work.",
        "",
        "| Document | Purpose |",
        "|----------|---------|",
        "| [maturity-matrix.md](maturity-matrix.md) | Feature stability classification — see what's stable vs. experimental |",
        "| [spec.md](spec.md) | Stability labels per section mark which parts of the spec are provisional |",
        "| [../history/language/syntax-v1-preview.md](../history/language/syntax-v1-preview.md) | Retired v1 syntax memo (landed items now live in spec/syntax) |",
    ])

    # Interactive playground cross-link
    lines.extend([
        "",
        "### 🎮 Playground",
        "",
        "<!-- issue-466 ✅ entrypoint live; issue-467 ✅ docs route wired; issue-635 ✅ format + tokenize highlight in browser UI -->",
        "> **[▶ Try the Playground](../playground/index.html)** — parse, format, and edit Arukellt code in your browser (with diagnostics and syntax highlighting).",
        "",
        "The playground editor shell supports parse + diagnostics, a **Format** toolbar action, and",
        "tokenize-driven syntax highlighting via the browser-native TypeScript engine.",
        "See the [Playground docs](../playground/README.md) for architecture details, design policies, and remaining work.",
    ])

    # Classification table (ADR-018)
    if classifications:
        class_by_file = {c["file"]: c for c in classifications}
        lines.extend([
            "",
            "## Classification (ADR-018)",
            "",
            "Each document is classified as **normative**, **explanatory**, or **transitional**.",
            "See [../adr/ADR-018-language-docs-classification.md](../adr/ADR-018-language-docs-classification.md) for definitions and banner templates.",
            "",
            "| File | Class | Note |",
            "|------|-------|------|",
        ])
        # Emit rows in canonical order (entries order, fall back to classifications order)
        covered: set[str] = set()
        for entry in entries:
            fname = entry.rel_path
            c = class_by_file.get(fname)
            if c:
                lines.append(f"| [{fname}]({fname}) | {c['class']} | {escape_table(c['note'])} |")
                covered.add(fname)
            else:
                lines.append(f"| [{fname}]({fname}) | — | (unclassified — add entry to language-doc-classifications.toml) |")
        # Any classifications not matched to a current entry (future-proofing)
        for c in classifications:
            if c["file"] not in covered:
                lines.append(f"| {c['file']} | {c['class']} | {escape_table(c['note'])} |")

    if classifications:
        class_by_file_label = {c["file"]: c["class"] for c in classifications}
        lines.extend(["", "## Documents", "", "| File | Title | Class | Summary |", "|------|-------|-------|---------|"])
        for entry in entries:
            cls = class_by_file_label.get(entry.rel_path, "—")
            lines.append(f"| [{entry.rel_path}]({entry.rel_path}) | {escape_table(entry.title)} | {cls} | {entry.summary} |")
    else:
        lines.extend(["", "## Documents", "", "| File | Title | Summary |", "|------|-------|---------|"])
        for entry in entries:
            lines.append(f"| [{entry.rel_path}]({entry.rel_path}) | {escape_table(entry.title)} | {entry.summary} |")

    # Link-Check Coverage Policy (ADR-019)
    lines.extend([
        "",
        "## Link-Check Coverage Policy (ADR-019)",
        "",
        "Link-check coverage for docs is governed by",
        "[ADR-019](../adr/ADR-019-anchor-permalink-policy.md). Key rules:",
        "",
        "- `scripts/check/check-links.sh` validates internal file references (`path/to/file.md`).",
        "- `scripts/check/check-anchor-fragments.py` validates anchor fragments (`file.md#section-id`, `#section-id`) and runs in `verify quick`.",
        "- External URLs (`https://...`) are out of scope.",
    ])

    # Link to the generated maturity matrix
    lines.extend([
        "",
        "## Feature Maturity",
        "",
        "See [maturity-matrix.md](maturity-matrix.md) for a full classification of all language features",
        "(stable / provisional / experimental / unimplemented).",
        "",
        "**Source of truth:** `docs/data/language-doc-classifications.toml` `[[features]]` section.",
        "",
        "**Update workflow:**",
        "",
        "1. Edit `[[features]]` entries in `docs/data/language-doc-classifications.toml`",
        "2. Run `python3 scripts/gen/generate-docs.py`",
        "3. Commit the TOML and regenerated `maturity-matrix.md` together",
        "",
        "CI will fail if the TOML changes without regenerating the matrix.",
    ])
    return "\n".join(lines) + "\n"


def render_playground_readme(
    section: dict,
    entries: list[DocEntry],
    snapshot_lines: list[str],
) -> str:
    lines = [
        f"# {section['title']}",
        "",
        "> This file is generated by `python3 scripts/gen/generate-docs.py`.",
        f"> {section['description']}",
        "",
        "## Current Snapshot",
        "",
    ]
    lines.extend(snapshot_lines or ["- Current source of truth: [../current-state.md](../current-state.md)"])

    lines.extend([
        "",
        "## What is in this directory today?",
        "",
        "This section documents the browser-side playground work in the repository:",
        "the browser-native engine, editor/diagnostics/share/example components, design/operations documents,",
        "and the live browser entrypoint at [`playground/index.html`](index.html).",
        "",
        "**Build outputs in this folder:** `dist/` (TypeScript emit)",
        "is produced by `playground`'s `npm run build:app`; it is **not** tracked in git.",
        "`.github/workflows/pages.yml` builds it before uploading `./docs` to GitHub Pages.",
        "",
        "### Current repo-proved surfaces",
        "",
        "| Surface | Repo proof today | Notes |",
        "|---------|------------------|-------|",
        "| Parse | ✅ | `playground/src/engine.ts` exports `parseSource(source)` |",
        "| Format | ✅ | `playground/src/engine.ts` exports `formatSource(source)` |",
        "| Tokenize | ✅ | `playground/src/engine.ts` exports `tokenizeSource(source)` |",
        "| Editor components | ✅ | `playground/src/**` contains editor / diagnostics UI building blocks |",
        "| Share helpers | ✅ | `playground/src/share.ts` provides fragment helpers |",
        "| Curated examples catalog | ✅ | `playground/src/examples.ts` contains example metadata |",
        "| Browser entrypoint | ✅ | [`docs/playground/index.html`](index.html) — parse + diagnostics, **Format** button, tokenize-driven highlighting (#466, #635) |",
        "| Docs route to live playground | ✅ | [`playground/index.html`](index.html) — linked from docs site navigation (issue 467) |",
        "| Publish / deploy path | ✅ | `.github/workflows/pages.yml` builds and deploys to GitHub Pages (issue 468) |",
        "| Privacy / telemetry guardrail | ✅ | `playground/src/telemetry.ts` — `TELEMETRY_DISABLED=true`; `reportError`/`reportWasmLoadError`/`reportCompilerPanic` log locally only. Policy: [`privacy-telemetry-policy.md`](privacy-telemetry-policy.md) (issue 438) |",
        "| Type-checking (compiler-backed engine) | ✅ | [#472](../../issues/done/472-playground-type-checker-product-claim.md) — `playground/src/engine.ts` and `playground.ts` expose compiler-backed `typecheck()` |",
        "<!-- target-state: rows below are not yet repo-proved in the browser entrypoint -->",
        "| Type-checking in browser index.html parse path | ❌ repo-proof missing | `index.html` does not pass compiler wasm to `createPlayground`; parse stays parse-only until wired (Build/Run uses a separate compiler client) |",
        "",
        "### Architecture status",
        "",
        "The current browser-side engine is the TypeScript playground engine plus",
        "TypeScript UI components. The browser entrypoint `docs/playground/index.html` provides an",
        "editor shell with parse + diagnostics, a Format toolbar action, and tokenize-driven syntax",
        "highlighting. The docs site navigation links to it. See",
        "[ADR-017](../adr/ADR-017-playground-execution-model.md) for the intended execution model and",
        "[issues/done/465-playground-false-done-audit-and-status-rollback.md](../../issues/done/465-playground-false-done-audit-and-status-rollback.md)",
        "for the current audit status.",
        "",
        "### Learning the Language",
        "",
        "Use the language docs to learn Arukellt now. The playground documents in this section describe",
        "the browser-side implementation work and remaining product-proof gaps.",
        "",
        "| Resource | Purpose |",
        "|----------|---------|",
        "| [Language Guide](../language/guide.md) | Practical walkthrough — variables, functions, structs, enums, control flow |",
        "| [Error Handling](../language/error-handling.md) | Result, Option, and the `?` operator |",
        "| [Type System](../language/type-system.md) | Types, generics, type inference |",
        "| [Syntax Reference](../language/syntax.md) | Complete syntax for all implemented constructs |",
        "| [Standard Library](../stdlib/README.md) | Available functions and modules |",
        "",
        "## Architecture & Policies",
        "",
    ])

    # Document table from existing playground docs
    if entries:
        lines.extend([
            "| Document | Summary |",
            "|----------|---------|",
        ])
        for entry in entries:
            lines.append(f"| [{entry.rel_path}]({entry.rel_path}) | {entry.summary} |")
    else:
        lines.append("_(No supporting documents yet.)_")

    lines.extend([
        "",
        "## Related ADRs",
        "",
        "| ADR | Topic |",
        "|-----|-------|",
        "| [ADR-017](../adr/ADR-017-playground-execution-model.md) | Execution model and v1/v2 product contract |",
        "| [ADR-021](../adr/ADR-021-playground-share-url-format.md) | Share URL format (fragment-based) |",
        "| [ADR-022](../adr/ADR-022-playground-deployment-and-caching.md) | Deployment strategy and asset caching |",
    ])

    return "\n".join(lines) + "\n"


def render_stdlib_readme(
    section: dict,
    entries: list[DocEntry],
    state: dict,
    manifest_stats: dict,
    source_modules: dict[str, StdlibSourceModule],
) -> str:
    types = ", ".join(f"`{entry['name']}`" for entry in manifest_stats["types"] if entry.get("prelude"))
    values = ", ".join(f"`{entry['name']}`" for entry in manifest_stats["values"] if entry.get("prelude"))
    category_summary = ", ".join(
        f"`{name}` {count}" for name, count in sorted(manifest_stats["category_counts"].items())
    )
    lines = [
        f"# {section['title']}",
        "",
        "> This file is generated by `python3 scripts/gen/generate-docs.py`.",
        section["description"],
        "",
        "## Quick Navigation",
        "",
        "| Resource | Description |",
        "|----------|-------------|",
        "| [**cookbook.md**](cookbook.md) | Hands-on usage recipes with fixture links — **start here** for working examples |",
        "| [**reference.md**](reference.md) | Complete manifest-backed API reference (all functions, types, stability tiers) |",
        "| [migration-guidance.md](migration-guidance.md) | Deprecated API migration paths and replacement patterns |",
        "| [stability-policy.md](stability-policy.md) | What stable / provisional / experimental mean for your code |",
        "",
        "## Reading Guide",
        "",
        "**New to the stdlib?** Follow the family groups below in order — each family",
        "is numbered to give you a recommended reading sequence from fundamentals to",
        "specialised modules. Start with *Core & Collections*, then *Text & Data Formats*.",
        "",
        "**Looking up a specific API?** Jump straight to [reference.md](reference.md)",
        "or the [name-index.md](name-index.md) for an alphabetical function lookup.",
        "",
        "**Migrating deprecated code?** See [migration-guidance.md](migration-guidance.md)",
        "and [monomorphic-deprecation.md](monomorphic-deprecation.md).",
        "",
        "---",
        "",
        "## Module Families",
        "",
        "Each module page has a curated **Overview** (when/how to use it) plus exhaustive",
        "generated reference tables. Read the modules within each family in the numbered order.",
        "",
        "### 1 · Core & Collections",
        "",
        "Foundational types, error handling, and data structures.",
        "",
        "| # | Module | Description |",
        "|---|--------|-------------|",
        "| 1 | [std::core](modules/core.md) | Ranges, errors, and hashing — start here for foundational primitives. |",
        "| 2 | [std::collections](modules/collections.md) | Hash maps, deques, priority queues, sorted maps, bitsets. |",
        "| 3 | [std::seq](modules/seq.md) | Eager sequence search, aggregation, and deduplication over collections. |",
        "",
        "### 2 · Text & Data Formats",
        "",
        "String processing, binary data, and structured-format parsing.",
        "",
        "| # | Module | Description |",
        "|---|--------|-------------|",
        "| 1 | [std::text](modules/text.md) | String inspection, trimming, searching, and formatting. |",
        "| 2 | [std::bytes](modules/bytes.md) | Binary data buffers, hex encoding, LEB128, endian conversion. |",
        "| 3 | [std::json](modules/json.md) | JSON stringify and parse helpers (experimental). |",
        "| 4 | [std::csv](modules/csv.md) | CSV line splitting (experimental). |",
        "| 5 | [std::toml](modules/toml.md) | TOML key=value parsing (experimental). |",
        "",
        "### 3 · I/O & Host Capabilities",
        "",
        "Host-dependent capabilities — these require a Wasm host that provides the corresponding imports.",
        "",
        "| # | Module | Description |",
        "|---|--------|-------------|",
        "| 1 | [std::host family](modules/io.md) | Overview of all host I/O capabilities — **read first** for the big picture. |",
        "| 2 | [std::host::fs](modules/fs.md) | Host filesystem read/write operations. |",
        "| 3 | [std::path](modules/path.md) | Pure path manipulation (no host required). |",
        "| 4 | [std::host::process / env](modules/process.md) | Process control and environment variable access. |",
        "",
        "### 4 · Algorithms & Utilities",
        "",
        "Deterministic helpers for randomness, time, and testing.",
        "",
        "| # | Module | Description |",
        "|---|--------|-------------|",
        "| 1 | [std::test](modules/test.md) | Typed assertion and expectation helpers — useful from day one. |",
        "| 2 | [std::random](modules/random.md) | Deterministic seeded pseudo-random helpers. |",
        "| 3 | [std::time](modules/time.md) | Pure duration arithmetic over timestamps. |",
        "",
        "### 5 · WebAssembly & Component Model",
        "",
        "Low-level Wasm introspection and component-model constants.",
        "",
        "| # | Module | Description |",
        "|---|--------|-------------|",
        "| 1 | [std::wasm](modules/wasm.md) | Wasm binary-format constants and memory operations. |",
        "| 2 | [std::wit](modules/wit.md) | WIT primitive type constants and introspection. |",
        "| 3 | [std::component](modules/component.md) | Component model version constants. |",
        "",
        "---",
        "",
        "## Documentation Layers",
        "",
        "| Layer | Where | What it covers |",
        "|-------|-------|----------------|",
        "| **Curated overview** | `## Overview` in each module page | *When* and *how* to use a module — highlights, constraints, usage patterns. |",
        "| **Generated reference** | Rest of module pages + [reference.md](reference.md) | Exhaustive API tables from `std/manifest.toml`. Auto-regenerated. |",
        "",
        f"All {len([p for p in STDLIB_MODULE_PAGES if 'overview' in p])} module pages carry curated overviews.",
        "",
        "## Current Snapshot",
        "",
        f"- Manifest-backed public functions: **{len(manifest_stats['public_functions'])}**",
        f"- Prelude wrappers: {len(manifest_stats['prelude_functions'])}",
        f"- Prelude types: {types}",
        f"- Prelude values: {values}",
        f"- Categories: {category_summary}",
        f"- Source-backed modules: {len(source_modules)}",
        "- Source of truth: [../current-state.md](../current-state.md), [`../../std/manifest.toml`](../../std/manifest.toml), `std/*.ark` source files",
        "",
        "## Additional Documents",
        "",
    ]
    # Categorize entries for the documents table
    module_entries: list[DocEntry] = []
    guide_entries: list[DocEntry] = []
    policy_entries: list[DocEntry] = []
    legacy_entries: list[DocEntry] = []
    # Paths already prominently linked above — skip from the table
    skip_paths = {"cookbook.md", "reference.md", "migration-guidance.md", "std.md", "stability-policy.md"}
    legacy_paths = {"core.md", "io.md"}
    policy_keywords = {"policy", "deprecation", "dedup", "migration", "scoreboard"}
    for entry in entries:
        if entry.rel_path in skip_paths:
            continue
        if entry.rel_path in legacy_paths:
            legacy_entries.append(entry)
        elif entry.rel_path.startswith("modules/"):
            module_entries.append(entry)
        elif any(kw in entry.rel_path for kw in policy_keywords):
            policy_entries.append(entry)
        else:
            guide_entries.append(entry)
    if guide_entries:
        lines.extend([
            "### Guides & Schemas",
            "",
            "| File | Summary |",
            "|------|---------|",
        ])
        for entry in guide_entries:
            lines.append(f"| [{entry.rel_path}]({entry.rel_path}) | {entry.summary} |")
        lines.append("")
    if policy_entries:
        lines.extend([
            "### Policies & Deprecation",
            "",
            "| File | Summary |",
            "|------|---------|",
        ])
        for entry in policy_entries:
            summary = entry.summary
            if entry.rel_path == "scoreboard.md":
                summary = (
                    "Generated from `std/manifest.toml` and fixture coverage — "
                    "do not edit manually."
                )
            lines.append(f"| [{entry.rel_path}]({entry.rel_path}) | {summary} |")
        lines.append("")
    if module_entries:
        lines.extend([
            "### Module Pages",
            "",
            "| File | Summary |",
            "|------|---------|",
        ])
        for entry in module_entries:
            lines.append(f"| [{entry.rel_path}]({entry.rel_path}) | {entry.summary} |")
        lines.append("")
    if legacy_entries:
        lines.extend([
            "### Legacy Index Pages",
            "",
            "*These are superseded by the module pages above but kept for backward-compatible links.*",
            "",
            "| File | Summary |",
            "|------|---------|",
        ])
        for entry in legacy_entries:
            lines.append(f"| [{entry.rel_path}]({entry.rel_path}) | {entry.summary} |")
        lines.append("")
    return "\n".join(lines) + "\n"


def render_curated_overview_section(overview: dict, *, overview_only: bool = False) -> list[str]:
    """Render a curated overview section for a module family page.

    The overview dict should contain:
      summary           - prose description of the module family
      highlights        - list of (api_pattern, description) tuples
      target_constraints - target restriction note (string)
      typical_usage     - fenced code block as a string
    """
    if overview_only:
        vs_ref = (
            "> **Overview vs Reference:** This page is a family overview with cross-links. "
            "Per-module API tables live on the linked module pages and in "
            "[`reference.md`](../reference.md)."
        )
    else:
        vs_ref = (
            "> **Overview vs Reference:** This section is curated prose — it explains when and "
            "how to use this module family. The sections below are exhaustive generated reference "
            "tables sourced directly from `std/manifest.toml` and source doc comments."
        )
    lines: list[str] = [
        "",
        "## Overview",
        "",
        vs_ref,
        "",
        overview["summary"],
    ]

    if overview.get("highlights"):
        lines.extend(["", "**Recommended API highlights:**", ""])
        lines.extend(["| API | Purpose |", "|-----|---------|"])
        for api, purpose in overview["highlights"]:
            lines.append(f"| {api} | {purpose} |")

    if overview.get("target_constraints"):
        lines.extend(["", f"**Target constraints:** {overview['target_constraints']}"])

    if overview.get("typical_usage"):
        lines.extend(["", "**Typical usage:**", "", overview["typical_usage"]])

    lines.extend(["", "---", ""])
    return lines


def render_stdlib_module_page(
    page: dict,
    manifest_functions_by_module: dict[str, list[dict]],
    source_modules: dict[str, StdlibSourceModule],
    manifest_modules: dict[str, dict],
) -> str:
    output_path = DOCS / "stdlib" / page["path"]
    lines = [
        f"# {page['title']}",
        "",
        f"> This file is generated by `python3 scripts/gen/generate-docs.py` from source doc comments and [`{rel_link(output_path, STDLIB_MANIFEST)}`]({rel_link(output_path, STDLIB_MANIFEST)}).",
        page["description"],
    ]

    if page.get("overview"):
        # Compute target_constraints dynamically from manifest data and use it
        # to override (or confirm) the hardcoded value in the page dict.
        all_page_funcs: list[dict] = []
        for mod in page["modules"]:
            all_page_funcs.extend(manifest_functions_by_module.get(mod, []))
        computed_constraints = build_target_constraints(
            page["modules"], all_page_funcs
        )
        # Build a copy of the overview with the dynamically-derived constraint
        overview_with_computed = dict(page["overview"])
        overview_with_computed["target_constraints"] = computed_constraints
        lines.extend(
            render_curated_overview_section(
                overview_with_computed,
                overview_only=bool(page.get("overview_only")),
            )
        )

    if page.get("overview_only"):
        lines.extend(
            [
                "## Module pages",
                "",
                "| Module | Detail | Notes |",
                "|--------|--------|-------|",
            ]
        )
        for module_name, rel_page, notes in page.get("module_pages", []):
            lines.append(f"| `{module_name}` | [{rel_page}]({rel_page}) | {notes} |")
        lines.extend(
            [
                "",
                "Also see [capability-surface.md](../../capability-surface.md) for reachability axes "
                "and [reference.md](../reference.md) for the full manifest-backed API index.",
                "",
            ]
        )
        return "\n".join(lines) + "\n"

    for module_name in page["modules"]:
        source_module = source_modules[module_name]
        functions = manifest_functions_by_module.get(module_name, [])
        items_by_name = {item.name: item for item in source_module.items}
        source_link = rel_link(output_path, source_module.source_path)
        lines.extend(
            [
                "",
                f"## Module `{module_name}`",
                "",
                f"- Source: [`{source_link}`]({source_link})",
                f"- Manifest-backed functions: {len(functions)}",
                f"- Stability: {format_stability_counts(functions) if functions else 'n/a'}",
                "",
            ]
        )

        # Add host-capability badges for std::host::* modules
        badge_lines = format_host_module_badges(module_name, functions, manifest_modules)
        if badge_lines:
            lines.extend(badge_lines)

        doc_lines = HOST_MODULE_SOURCE_DOC_OVERRIDES.get(
            module_name, source_module.docs
        )
        first_content = next((i for i, line in enumerate(doc_lines) if line.strip()), None)
        if first_content is not None and doc_lines[first_content].lstrip().startswith("# "):
            doc_lines = doc_lines[:first_content] + doc_lines[first_content + 1 :]
        lines.extend(
            render_source_doc_block(
                doc_lines,
                "_No module doc comment yet. Add `//!` comments in the source file to describe this module._",
            )
        )

        type_items = [item for item in source_module.items if item.kind != "fn"]
        if type_items:
            lines.extend(
                [
                    "",
                    f"### `{module_name}` — Public Types",
                    "",
                    "| Name | Kind | Summary |",
                    "|------|------|---------|",
                ]
            )
            for item in type_items:
                lines.append(
                    f"| `{item.name}` | `{item.kind}` | {source_doc_summary(item.docs)} |"
                )

        if not functions:
            lines.extend(["", "_No manifest-backed functions in this module._"])
            continue

        ordered_functions = sorted(
            functions,
            key=lambda entry: (
                items_by_name.get(entry["name"]).order if entry["name"] in items_by_name else 10_000,
                entry["name"],
            ),
        )
        grouped: dict[str, list[dict]] = defaultdict(list)
        section_order: list[str] = []
        for entry in ordered_functions:
            section = items_by_name.get(entry["name"]).section if entry["name"] in items_by_name else None
            label = section or "Public API"
            if label not in grouped:
                section_order.append(label)
            grouped[label].append(entry)

        for section_name in section_order:
            scoped_section_name = f"`{module_name}` — {section_name}"
            lines.extend(
                [
                    "",
                    f"### {scoped_section_name}",
                    "",
                    "| Name | Signature | Stability | Implementation | Summary |",
                    "|------|-----------|-----------|----------------|---------|",
                ]
            )
            for entry in grouped[section_name]:
                item = items_by_name.get(entry["name"])
                deprecated = _is_deprecated(entry)
                name_display = f"~~`{entry['name']}`~~" if deprecated else f"`{entry['name']}`"
                dep_inline = _format_deprecated_inline(entry) if deprecated else ""
                semantic_status = _function_semantic_status(entry)
                if semantic_status == "stub":
                    fn_status = "⚠️ stub"
                elif semantic_status == "limited":
                    fn_status = "⚠️ limited semantics"
                elif semantic_status == "unreachable":
                    fn_status = "⛔ unreachable"
                else:
                    fn_status = "✅ functional"
                lines.append(
                    "| {name}{dep} | `{signature}` | `{stability}` | {status} | {summary} |".format(
                        name=name_display,
                        dep=dep_inline,
                        signature=format_signature(entry.get("params", []), entry.get("returns", "()")),
                        stability=entry.get("stability", "unknown"),
                        status=fn_status,
                        summary=source_doc_summary(item.docs if item else []),
                    )
                )

            # After the table, emit per-function detail blocks (manifest doc + errors + examples)
            for entry in grouped[section_name]:
                fn_doc = entry.get("doc")
                fn_errors = entry.get("errors")
                fn_examples = entry.get("examples", [])
                fn_avail = entry.get("availability")
                if not (fn_doc or fn_errors or fn_examples or fn_avail):
                    continue
                lines.extend(["", f"#### `{module_name}::{entry['name']}`"])
                if fn_doc:
                    lines.extend(["", fn_doc])
                if fn_avail:
                    avail_parts: list[str] = []
                    if fn_avail.get("t1") is False:
                        avail_parts.append("⚠️ Not available on `wasm32`")
                    if fn_avail.get("t3") is False:
                        avail_parts.append("⚠️ Not available on `wasm32-gc`")
                    if fn_avail.get("note"):
                        avail_parts.append(fn_avail["note"])
                    if avail_parts:
                        lines.extend(["", "**Availability:** " + " — ".join(avail_parts)])
                if fn_errors:
                    lines.extend(["", f"**Errors:** {fn_errors}"])
                for ex in fn_examples:
                    code = ex.get("code", "").strip()
                    if not code:
                        continue
                    desc = ex.get("description", "")
                    lines.append("")
                    if desc:
                        lines.append(f"_Example — {desc}:_")
                        lines.append("")
                    lines.extend(["```ark", code, "```"])
                    if ex.get("output"):
                        lines.extend(["", f"Expected output: `{ex['output']}`"])

        # Deprecation cross-link: if any function in this module is deprecated,
        # add a link to migration-guidance.md at the end of the module section.
        module_deprecated = [
            entry for entry in functions if _is_deprecated(entry)
        ]
        if module_deprecated:
            migration_link = rel_link(output_path, DOCS / "stdlib" / "migration-guidance.md")
            lines.extend(
                [
                    "",
                    f"> ⚠️ **{len(module_deprecated)} deprecated API(s)** in this module. "
                    f"See [{migration_link}]({migration_link}) for replacement examples and migration steps.",
                ]
            )

    return "\n".join(lines) + "\n"


def render_stdlib_alias_page(page: dict) -> str:
    output_path = DOCS / "stdlib" / page["path"]
    target = page["primary_target"]
    target_rel = rel_link(output_path, DOCS / "stdlib" / target["path"])
    lines = [
        f"# {page['title']}",
        "",
        "> This file is generated by `python3 scripts/gen/generate-docs.py`.",
        f"> ⚠️ **Archived page.** This legacy index has been superseded by "
        f"[{target['label']}]({target_rel}). "
        f"Please update your bookmarks and links to point there instead.",
        "",
        page["description"],
        "",
        "## Current Docs",
        "",
        "| File | Notes |",
        "|------|-------|",
    ]
    for link in page["links"]:
        rel = rel_link(output_path, DOCS / "stdlib" / link["path"])
        lines.append(f"| [{link['label']}]({rel}) | {escape_table(link['notes'])} |")
    return "\n".join(lines) + "\n"


def render_examples_readme(section: dict, examples: list[dict], state: dict) -> str:
    baseline_cases = {Path(case).name for case in state["perf"]["baseline_cases"]}
    lines = [
        f"# {section['title']}",
        "",
        "> This file is generated by `python3 scripts/gen/generate-docs.py`.",
        section["description"],
        "",
        "## Current Snapshot",
        "",
        f"- Executable examples: {len(examples)}",
        f"- `.expected` pairs present: {sum(1 for entry in examples if entry['expected'] == 'yes')}",
        f"- Baseline-tracked examples: {sum(1 for entry in examples if entry['file'] in baseline_cases)}",
        "- These files serve as both documentation and runnable integration examples.",
        "",
        "## Run",
        "",
        "```bash",
        "target/release/arukellt run docs/examples/hello.ark",
        "```",
        "",
        "## Examples",
        "",
        "| File | Description | Expected | Baseline |",
        "|------|-------------|----------|----------|",
    ]
    for entry in examples:
        lines.append(
            f"| [{entry['file']}]({entry['file']}) | {entry['title']} | {entry['expected']} | {entry['baseline']} |"
        )
    return "\n".join(lines) + "\n"


def render_sample_readme(section: dict, files: list[str]) -> str:
    lines = [
        f"# {section['title']}",
        "",
        "> This file is generated by `python3 scripts/gen/generate-docs.py`.",
        section["description"],
        "",
        "## Current Snapshot",
        "",
        "- Current source of truth for runnable behavior remains [../current-state.md](../current-state.md).",
        "- This directory contains implementation-oriented sample files rather than narrative docs.",
        "",
        "## Files",
        "",
        "| File | Notes |",
        "|------|-------|",
    ]
    for filename in files:
        lines.append(f"| [{filename}]({filename}) | Sample artifact |")
    return "\n".join(lines) + "\n"


def render_archive_snapshot() -> list[str]:
    return [
        "- These documents are historical or design references, not the current behavior contract.",
        "- Current source of truth: [../current-state.md](../current-state.md).",
    ]


def section_snapshot(section: dict, state: dict, fixture_total: int, manifest_stats: dict, examples: list[dict]) -> list[str]:
    snapshot = section["snapshot"]
    manifest_count = fixture_manifest_count_from_state(state, fixture_total)
    if snapshot == "compiler":
        return [
            f"- Current path: `{join_pipeline(state['pipeline']['current'])}`",
            f"- Refactor target: `{join_pipeline(state['pipeline']['refactor_target'])}`",
            f"- Shared orchestration entry point: `{state['pipeline']['session_entry']}`",
            f"- Dump phases: `{', '.join(state['pipeline']['dump_phases'])}`",
        ]
    if snapshot == "language":
        v = state.get("verification", {})
        lines = [
            "- Current user-visible behavior is described by [../current-state.md](../current-state.md).",
            f"- Fixture registry: {manifest_count} manifest entries (`docs/data/project-state.toml`).",
            f"- Last observed harness: {v.get('fixture_harness_observed', 'n/a')} outcomes "
            f"(passed: {v.get('fixture_passed', 'n/a')}, failed: {v.get('fixture_failures', 'n/a')}, "
            f"skipped: {v.get('fixture_skipped', 'n/a')}).",
            f"- Not included in last snapshot: {v.get('fixture_not_in_last_harness_snapshot', 'n/a')} "
            f"registry entries (not proof they fail).",
            f"- Canonical target for current docs: `{state['targets']['canonical']}`",
        ]
        return lines
    if snapshot == "platform":
        return [
            f"- CLI default target: `{state['targets']['cli_default']}`",
            f"- Canonical target: `{state['targets']['canonical']}`",
            f"- Component emit: {format_component_emit_summary(state)} ([axes](../data/component-availability.md))",
            "- Backend validation failure (`W0004`) is a hard error.",
        ]
    if snapshot == "process":
        return [
            f"- Verification command: `{state['project']['verification_command']}`",
            f"- Current verification gate: {state['verification']['checks_passed']}/{state['verification']['checks_total']} checks pass",
            f"- Fixture registry: {manifest_count} manifest entries (`docs/data/project-state.toml`)",
            f"- Fixture harness (observed): {format_fixture_harness(state['verification'], with_manifest=False)}",
            "- Generated docs pull state from `docs/data/project-state.toml`, structured TOMLs under `docs/data/`, `std/manifest.toml`, and fixture manifests.",
        ]
    if snapshot == "stdlib":
        return [
            f"- Manifest-backed public functions: {len(manifest_stats['public_functions'])}",
            f"- Prelude wrappers: {len(manifest_stats['prelude_functions'])}",
            f"- Prelude types: {', '.join(entry['name'] for entry in manifest_stats['types'] if entry.get('prelude'))}",
            f"- Prelude values: {', '.join(entry['name'] for entry in manifest_stats['values'] if entry.get('prelude'))}",
        ]
    if snapshot == "examples":
        return [
            f"- Executable examples: {len(examples)}",
            f"- `.expected` coverage: {sum(1 for entry in examples if entry['expected'] == 'yes')}/{len(examples)}",
            "- Baseline-tracked examples are shared with perf gates.",
        ]
    if snapshot == "playground":
        return [
            "- ADR-017 defines a client-side browser execution model for playground work.",
            "- Current repo proof: `playground/src/engine.ts` exports parse, format, tokenize, typecheck, and version functions.",
            "- Current repo proof: `playground/src/**` contains editor / diagnostics / share / examples components.",
            "- Current repo proof: `docs/playground/index.html` provides parse + diagnostics, a Format toolbar action, and tokenize-driven highlighting.",
            "- Current repo proof: `docs/_sidebar.md` links to the playground page.",
            "- Current repo proof: `.github/workflows/pages.yml` builds playground JS into `docs/playground/dist/` and deploys to GitHub Pages (those outputs are not committed).",
            "- Compiler-backed typecheck surface: closed in [#472](../../issues/done/472-playground-type-checker-product-claim.md); `index.html` parse path remains parse-only until compiler wasm is wired to `createPlayground`.",
        ]
    if snapshot == "migration":
        return [
            f"- CLI default target remains `{state['targets']['cli_default']}`.",
            f"- Canonical path for current docs is `{state['targets']['canonical']}`.",
            f"- Component emit: {format_component_emit_summary(state)} ([axes](../data/component-availability.md)).",
        ]
    if snapshot == "sample":
        return [
            "- This directory is intentionally code-first.",
            "- Use the sample files as reference artifacts, not as the current behavior contract.",
        ]
    if snapshot == "archive":
        return render_archive_snapshot()
    return ["- Current source of truth: [../current-state.md](../current-state.md)"]


def format_signature(params: list[str], returns: str) -> str:
    joined = ", ".join(params)
    return f"({joined}) -> {returns}" if params else f"() -> {returns}"


# ── Target-constraint helpers ────────────────────────────────────────────────


def _canonicalize_target_name(name: str) -> str:
    """Map legacy CLI/manifest target aliases to ADR-007 canonical names."""
    aliases = {
        "wasm32-wasi-p1": "wasm32",
        "wasm32-wasi": "wasm32",
        "wasm32-wasi-p2": "wasm32-gc",
        "wasm-gc": "wasm32-gc",
        "wasm-gc-wasi-p2": "wasm32-gc",
    }
    return aliases.get(name, name)


def _availability_t3_only(funcs: list[dict]) -> bool:
    """Return True when all functions with ``availability`` data have ``t1 = false``.

    This detects modules whose entire public surface is restricted to the T3
    (wasm32-wasi-p2 / component model) tier.  Functions with no ``availability``
    key are ignored (they do not contribute to the verdict either way).
    """
    avail_entries = [f["availability"] for f in funcs if f.get("availability")]
    if not avail_entries:
        return False
    return all(not a.get("t1", True) for a in avail_entries)


def _availability_unbacked(funcs: list[dict]) -> bool:
    avail_entries = [f["availability"] for f in funcs if f.get("availability")]
    if not avail_entries:
        return False
    return all(
        not a.get("t1", True) and not a.get("t3", True) for a in avail_entries
    )


def build_target_constraints(page_modules: list[str], funcs: list[dict]) -> str:
    """Derive a human-readable target-constraint string from manifest data.

    Reads ``availability`` (t1/t3 tier flags) and ``target`` (platform list) from
    each manifest function entry.  The ``availability`` field is the primary signal:
    when all functions with availability data carry ``t1 = false``, the module is
    T3-only.  The ``target`` list is used as a fallback for explicit platform
    constraints that are not expressed through availability tiers.

    Args:
        page_modules: Module names on this doc page (used for filesystem runtime notes).
        funcs: List of manifest function dicts for the module(s) being described.

    Returns:
        A Markdown-friendly constraint string such as:
        - ``"All targets (`wasm32` + `wasm32-gc`). No host capability required."``
        - ``"⚠ **`wasm32-gc` only** — WASI P2 / component host profile required."``
        - ``"Targets: wasm32, wasm32-gc."``
    """
    fs_on_page = any(m in ("std::host::fs", "std::fs") for m in page_modules)
    host_dependent = any(m.startswith("std::host::") for m in page_modules)
    explicit_permission = (
        "`--dir` (or equivalent) for file I/O"
        if fs_on_page
        else "none beyond providing the documented host profile"
    )
    axes = (
        f"Host dependency: {'yes' if host_dependent else 'no'}. "
        f"Explicit runtime permission: {explicit_permission}."
    )

    # Primary signal: availability.t1 / availability.t3 tier flags from manifest.
    if _availability_unbacked(funcs):
        return (
            "⚠ **Not user-reachable** on the current selfhost compile/run path. "
            "See [Capability surface](../../platform/target-runtime-and-surfaces.md#capability-surface). "
            + axes
        )
    availability_shapes = {
        (f.get("availability", {}).get("t1"), f.get("availability", {}).get("t3"))
        for f in funcs if f.get("availability")
    }
    if len(availability_shapes) > 1:
        return "Target/profile availability: mixed — see individual symbols. " + axes
    if _availability_t3_only(funcs):
        base = "⚠ **`wasm32-gc` only** — WASI P2 / component host profile required."
        return base + " " + axes

    # Fallback: collect explicit ``target`` lists from function entries.
    targets: set[str] = set()
    for f in funcs:
        t = f.get("target", [])
        if t:
            targets.update(t)
    if not targets or targets in ({"wasm32-wasi-p1", "wasm32-wasi-p2"}, {"wasm32", "wasm32-gc"}):
        base = "Target availability: `wasm32` and `wasm32-gc`."
        return base + " " + axes
    if targets in ({"wasm32-wasi-p2"}, {"wasm32-gc"}):
        base = "⚠ **`wasm32-gc` only** — WASI P2 / component host profile required."
        return base + " " + axes
    canon = sorted({_canonicalize_target_name(x) for x in targets})
    base = f"Targets: {', '.join(canon)}."
    return base + " " + axes


# ── Deprecation helpers ──────────────────────────────────────────────────────


def _is_deprecated(entry: dict) -> bool:
    """Return True if a manifest function entry is deprecated.

    A function is deprecated if it has ``deprecated_by`` set or
    ``stability == "deprecated"``.
    """
    return bool(entry.get("deprecated_by")) or entry.get("stability") == "deprecated"


def _format_deprecated_inline(entry: dict) -> str:
    """Return an inline deprecation annotation for a table row.

    Format:  `` ⚠️ Deprecated → `replacement` ``
    If no ``deprecated_by`` is specified, omits the arrow/replacement.
    """
    replacement = entry.get("deprecated_by")
    if replacement:
        return f" ⚠️ Deprecated → `{replacement}`"
    return " ⚠️ Deprecated"


def _format_deprecated_badge_block(entry: dict, migration_link: str) -> str:
    """Return a standalone deprecation badge line for module page detail views.

    Used when rendering per-function deprecation notices outside of tables.
    """
    replacement = entry.get("deprecated_by")
    if replacement:
        return (
            f"> ⚠️ **Deprecated** — use `{replacement}` instead. "
            f"See [{migration_link}]({migration_link}) for migration examples."
        )
    return (
        f"> ⚠️ **Deprecated** — "
        f"see [{migration_link}]({migration_link}) for migration guidance."
    )


def _render_reference_function_row(entry: dict) -> str:
    """Render a single function table row for reference.md."""
    intrinsic = f"`{entry['intrinsic']}`" if entry.get("intrinsic") else "-"
    module_name = f"`{entry['module']}`" if entry.get("module") else "`prelude`"
    is_deprecated = _is_deprecated(entry)
    name_display = f"~~`{entry['name']}`~~" if is_deprecated else f"`{entry['name']}`"
    deprecated_note = _format_deprecated_inline(entry) if is_deprecated else ""
    # Add target/capability annotation for host functions
    kind_display = entry.get("kind", "builtin")
    target_list = entry.get("target", [])
    if kind_display == "host_stub":
        kind_display = "host_stub ⚠️"
    if target_list:
        kind_display += f" ({', '.join(target_list)})"
    # Include manifest doc text (truncated for table legibility)
    doc_text = entry.get("doc", "") or ""
    doc_cell = escape_table(doc_text[:100] + ("…" if len(doc_text) > 100 else "")) if doc_text else "-"
    implementation = _function_semantic_status(entry)
    return (
        "| {name}{deprecated} | `{signature}` | {module_name} | `{stability}` | `{implementation}` | `{kind}` | {prelude} | {intrinsic} | {doc} |".format(
            name=name_display,
            deprecated=deprecated_note,
            signature=format_signature(entry.get("params", []), entry.get("returns", "()")),
            module_name=module_name,
            stability=entry.get("stability", "unknown"),
            implementation=implementation,
            kind=kind_display,
            prelude="yes" if entry.get("prelude") else "no",
            intrinsic=intrinsic,
            doc=doc_cell,
        )
    )


def _render_reference_function_details(entry: dict) -> list[str]:
    """Render a detail block for a function that has errors or examples in the manifest.

    Emitted after the category table so users can see full descriptions and code samples.
    Only generates output when the entry carries ``errors`` or ``examples`` fields.
    """
    name = entry["name"]
    module = entry.get("module", "prelude")
    errors = entry.get("errors")
    examples = entry.get("examples", [])

    if not errors and not examples:
        return []

    lines: list[str] = ["", f"### `{name}` — `{module}`"]
    if errors:
        lines.extend(["", f"**Errors:** {errors}"])
    for ex in examples:
        code = ex.get("code", "").strip()
        if not code:
            continue
        desc = ex.get("description", "")
        expected = ex.get("output", "")
        lines.append("")
        if desc:
            lines.append(f"_Example — {desc}:_")
            lines.append("")
        lines.extend(["```ark", code, "```"])
        if expected:
            lines.extend([f"", f"Expected output: `{expected}`"])
    return lines


_REFERENCE_TABLE_HEADER = [
    "| Name | Signature | Module | Stability | Implementation | Kind | Prelude | Intrinsic | Description |",
    "|------|-----------|--------|-----------|----------------|------|---------|-----------|-------------|",
]

# Stability tiers rendered as sections, in display order.
_STABILITY_TIER_ORDER = ["stable", "provisional", "experimental"]

_STABILITY_TIER_LABELS = {
    "stable": "Stable",
    "provisional": "Provisional",
    "experimental": "Experimental",
}

_STABILITY_TIER_DESCRIPTIONS = {
    "stable": "Backward-compatible within a major version. Safe for production use.",
    "provisional": "API is usable but may change in minor versions based on feedback.",
    "experimental": "API may change without notice. Functionality is available but not finalized.",
}


def render_deprecation_table(manifest: dict) -> str:
    """Generate the lifecycle table directly from manifest deprecation fields."""
    entries = sorted(
        (entry for entry in manifest.get("functions", []) if entry.get("stability") == "deprecated"),
        key=lambda entry: (entry.get("module", "prelude"), entry["name"]),
    )
    policy = manifest.get("deprecation_policy", {})
    lines = [
        "# Deprecated API Migration Table",
        "",
        "> Generated from `std/manifest.toml` by `scripts/gen/generate-docs.py`.",
        "> Lifecycle state and replacement are never maintained separately here.",
        "",
        "Deprecated APIs remain callable for the policy window in",
        "[stability-policy.md](stability-policy.md). Monomorphic compatibility",
        "helpers are included alongside any other deprecated public entry.",
        "",
        "| API | Module | Stability | Replacement | Deprecated since | Earliest removal | Reason |",
        "|-----|--------|-----------|-------------|------------------|------------------|--------|",
    ]
    for entry in entries:
        lines.append(
            "| `{name}` | `{module}` | `{stability}` | `{replacement}` | `{since}` | `{remove}` | {reason} |".format(
                name=entry["name"],
                module=entry.get("module", "prelude"),
                stability=entry["stability"],
                replacement=entry["deprecated_by"],
                since=entry.get("deprecated_since", policy["default_since"]),
                remove=entry.get("remove_in", policy["default_remove_in"]),
                reason=escape_table(entry.get("deprecated_reason", entry.get("doc", "Superseded API"))),
            )
        )
    lines.extend(["", f"Total deprecated public entries: **{len(entries)}**.", ""])
    return "\n".join(lines)


def render_stdlib_reference(manifest: dict) -> str:
    types = manifest.get("types", [])
    values = manifest.get("values", [])
    functions = [entry for entry in manifest.get("functions", []) if not entry["name"].startswith("__intrinsic_")]
    grouped: dict[str, list[dict]] = defaultdict(list)
    for entry in functions:
        grouped[entry.get("doc_category", "misc")].append(entry)

    # Compute stability tier counts for the overview
    stability_counts: dict[str, int] = Counter(
        entry.get("stability", "unknown") for entry in functions
    )

    lines = [
        "# stdlib API Reference",
        "",
        "> This file is generated by `python3 scripts/gen/generate-docs.py` from [`../../std/manifest.toml`](../../std/manifest.toml).",
        "> It reflects the manifest-backed declared public API surface, not roadmap-only or archived design notes.",
        "> Some APIs may be `unreachable`, `limited`, or `stub` — check the Implementation column.",
        "",
        "## Stability Overview",
        "",
        "| Tier | Count | Description |",
        "|------|-------|-------------|",
    ]
    for tier in _STABILITY_TIER_ORDER:
        count = stability_counts.get(tier, 0)
        desc = _STABILITY_TIER_DESCRIPTIONS[tier]
        lines.append(f"| `{tier}` | {count} | {desc} |")
    dep_count = stability_counts.get("deprecated", 0)
    if dep_count:
        lines.append(f"| [deprecated](#deprecated-apis) | {dep_count} | Superseded — see migration guidance. |")

    lines.extend([
        "",
        "## Prelude Types",
        "",
        "| Name | Generic Params | Prelude |",
        "|------|----------------|---------|",
    ])
    for entry in types:
        generic = ", ".join(entry.get("generic_params", [])) or "-"
        lines.append(
            f"| `{entry['name']}` | {generic} | {'yes' if entry.get('prelude') else 'no'} |"
        )

    lines.extend(["", "## Prelude Values", "", "| Name | Prelude |", "|------|---------|"])
    for entry in values:
        lines.append(f"| `{entry['name']}` | {'yes' if entry.get('prelude') else 'no'} |")

    # ── Per-category sections (existing layout) ──────────────────────────
    for category in sorted(grouped):
        sorted_entries = sorted(grouped[category], key=lambda item: item["name"])
        lines.extend(["", f"## {humanize_slug(category)}", ""] + _REFERENCE_TABLE_HEADER)
        for entry in sorted_entries:
            lines.append(_render_reference_function_row(entry))
        # After the table, emit per-function detail blocks (errors + examples)
        for entry in sorted_entries:
            lines.extend(_render_reference_function_details(entry))

    # Deprecation summary: collect deprecated entries and link to migration guide
    deprecated_entries = [
        entry for entry in functions if _is_deprecated(entry)
    ]
    if deprecated_entries:
        lines.extend(
            [
                "",
                "## Deprecated APIs",
                "",
                f"> ⚠️ **{len(deprecated_entries)} API(s) are deprecated.** "
                "See [Migration Guidance](migration-guidance.md) for replacement examples and migration steps.",
                "",
                "| Deprecated | Replacement | Migration Guide |",
                "|------------|-------------|-----------------|",
            ]
        )
        for entry in sorted(deprecated_entries, key=lambda e: e["name"]):
            replacement = f"`{entry['deprecated_by']}`" if entry.get("deprecated_by") else "_see docs_"
            lines.append(
                f"| ~~`{entry['name']}`~~ | {replacement} | [migration-guidance.md](migration-guidance.md) |"
            )

    return "\n".join(lines) + "\n"


def render_name_index(manifest: dict) -> str:
    """Generate a canonical/alias/historical name search index.

    Maps every public function name (including deprecated historical names)
    to its current canonical replacement, module, stability, and doc links.
    Source of truth: std/manifest.toml.
    """
    functions = [
        entry for entry in manifest.get("functions", [])
        if not entry["name"].startswith("__intrinsic_")
    ]

    # ── Canonical entries (all non-deprecated public functions) ───────────
    canonical_entries: list[dict] = []
    # ── Historical entries (deprecated names pointing to replacements) ────
    historical_entries: list[dict] = []

    for entry in functions:
        name = entry["name"]
        module = entry.get("module", "prelude")
        stability = entry.get("stability", "unknown")
        category = entry.get("doc_category", "misc")
        deprecated_by = entry.get("deprecated_by")
        is_dep = _is_deprecated(entry)

        if is_dep:
            historical_entries.append({
                "old_name": name,
                "replacement": deprecated_by or "—",
                "module": module,
                "category": category,
            })
        else:
            canonical_entries.append({
                "name": name,
                "module": module,
                "stability": stability,
                "category": category,
            })

    # Sort both lists alphabetically
    canonical_entries.sort(key=lambda e: e["name"].lower())
    historical_entries.sort(key=lambda e: e["old_name"].lower())

    total_canonical = len(canonical_entries)
    total_historical = len(historical_entries)

    lines = [
        "# stdlib Name Index",
        "",
        "> This file is generated by `python3 scripts/gen/generate-docs.py` from "
        "[`../../std/manifest.toml`](../../std/manifest.toml).",
        "> Do not edit manually — changes will be overwritten on the next regeneration.",
        "",
        "Use this index to look up any stdlib function name — including old, "
        "deprecated, or historical names — and find the current canonical replacement.",
        "",
        f"- **Canonical names:** {total_canonical}",
        f"- **Historical/deprecated names:** {total_historical}",
        f"- **Total entries:** {total_canonical + total_historical}",
        "",
        "Related:",
        "- [reference.md](reference.md) — full manifest-backed API reference",
        "- [migration-guidance.md](migration-guidance.md) — migration steps for deprecated APIs",
        "",
        "---",
        "",
        "## Canonical Names",
        "",
        "Current public API names, sorted alphabetically.",
        "",
        "| Name | Module | Stability | Category |",
        "|------|--------|-----------|----------|",
    ]

    for entry in canonical_entries:
        lines.append(
            f"| `{entry['name']}` | `{entry['module']}` "
            f"| `{entry['stability']}` | {humanize_slug(entry['category'])} |"
        )

    lines.extend([
        "",
        "---",
        "",
        "## Historical / Deprecated Names",
        "",
        "Old or deprecated names that have been superseded. "
        "Each entry links to its canonical replacement and the migration guide.",
        "",
    ])

    if historical_entries:
        lines.extend([
            "| Old Name | Replacement | Category | Migration Guide |",
            "|----------|-------------|----------|-----------------|",
        ])
        for entry in historical_entries:
            replacement_display = (
                f"`{entry['replacement']}`" if entry["replacement"] != "—" else "—"
            )
            lines.append(
                f"| ~~`{entry['old_name']}`~~ | {replacement_display} "
                f"| {humanize_slug(entry['category'])} "
                f"| [migration-guidance.md](migration-guidance.md) |"
            )
    else:
        lines.append("_No deprecated names at this time._")

    lines.extend([
        "",
        "---",
        "",
        "## Combined Alphabetical Index",
        "",
        "All names (canonical and historical) in a single alphabetical listing "
        "for quick lookup.",
        "",
        "| Name | Status | Module | Replacement / Notes |",
        "|------|--------|--------|---------------------|",
    ])

    # Build combined list for unified alphabetical lookup
    combined: list[tuple[str, str, str, str]] = []
    for entry in canonical_entries:
        sort_key = entry["name"].lower()
        combined.append((
            sort_key,
            f"`{entry['name']}`",
            f"✅ `{entry['stability']}`",
            f"`{entry['module']}`",
            f"{humanize_slug(entry['category'])}",
        ))
    for entry in historical_entries:
        sort_key = entry["old_name"].lower()
        replacement_display = (
            f"→ `{entry['replacement']}`" if entry["replacement"] != "—" else "— _see docs_"
        )
        combined.append((
            sort_key,
            f"~~`{entry['old_name']}`~~",
            "⚠️ deprecated",
            f"`{entry['module']}`",
            f"{replacement_display} · [migration guide](migration-guidance.md)",
        ))

    combined.sort(key=lambda t: t[0])
    for _, name_col, status_col, module_col, notes_col in combined:
        lines.append(f"| {name_col} | {status_col} | {module_col} | {notes_col} |")

    return "\n".join(lines) + "\n"


def _scoreboard_family(entry: dict) -> str:
    """Return the module family key for a manifest function entry."""
    module = entry.get("module")
    if module:
        return module
    if entry.get("prelude"):
        return "prelude"
    return "prelude"


def _scoreboard_has_host_dep(entry: dict) -> bool:
    """Return True when a function depends on host imports."""
    if entry.get("kind") == "host_stub":
        return True
    module = entry.get("module", "")
    return module.startswith("std::host::")


def _scoreboard_stability_bucket(entry: dict) -> str:
    """Return the exact manifest stability tier used by the scoreboard."""
    if _is_deprecated(entry):
        return "deprecated"
    stability = entry.get("stability", "stable")
    if stability == "stable":
        return "stable"
    if stability in {"experimental", "provisional"}:
        return stability
    if stability == "deprecated":
        return "deprecated"
    return "experimental"


def _load_fixture_source_text() -> str:
    """Load concatenated fixture sources for word-boundary coverage scans."""
    if not FIXTURES_DIR.exists():
        return ""
    chunks: list[str] = []
    for path in sorted(FIXTURES_DIR.rglob("*.ark")):
        try:
            chunks.append(path.read_text(encoding="utf-8"))
        except (OSError, UnicodeDecodeError):
            continue
    return "\n".join(chunks)


def _function_in_fixtures(name: str, fixture_text: str) -> bool:
    """Return True when *name* appears as a whole identifier in fixture sources."""
    if not fixture_text:
        return False
    return bool(re.search(r"\b" + re.escape(name) + r"\b", fixture_text))


def _format_fixture_coverage(covered: int, total: int) -> str:
    """Format fixture coverage as ``covered/total (pct%)``."""
    if total == 0:
        return "0/0 (—)"
    pct = round(100 * covered / total)
    return f"{covered}/{total} ({pct}%)"


def render_scoreboard(manifest: dict) -> str:
    """Generate the stdlib module maturity scoreboard from manifest + fixtures."""
    functions = [
        entry for entry in manifest.get("functions", [])
        if not entry["name"].startswith("__intrinsic_")
    ]
    fixture_text = _load_fixture_source_text()

    families: dict[str, list[dict]] = defaultdict(list)
    for entry in functions:
        families[_scoreboard_family(entry)].append(entry)

    def family_sort_key(name: str) -> tuple[int, str]:
        return (0 if name == "prelude" else 1, name)

    lines = [
        "# Stdlib Module Maturity Scoreboard",
        "",
        "> This file is generated by `python3 scripts/gen/generate-docs.py` from "
        "[`../../std/manifest.toml`](../../std/manifest.toml).",
        "> Do not edit manually — changes will be overwritten on the next regeneration.",
        "",
        "| Family | APIs | Stable | Provisional | Experimental | Deprecated | Fixture Coverage | Host Dep |",
        "|--------|------|--------|-------------|--------------|------------|------------------|----------|",
    ]

    totals = {
        "apis": 0,
        "stable": 0,
        "provisional": 0,
        "experimental": 0,
        "deprecated": 0,
        "fixture_covered": 0,
        "fixture_total": 0,
    }
    low_coverage: list[str] = []

    for family_name in sorted(families.keys(), key=family_sort_key):
        entries = families[family_name]
        apis = len(entries)
        stable = sum(1 for entry in entries if _scoreboard_stability_bucket(entry) == "stable")
        provisional = sum(
            1 for entry in entries if _scoreboard_stability_bucket(entry) == "provisional"
        )
        experimental = sum(
            1 for entry in entries if _scoreboard_stability_bucket(entry) == "experimental"
        )
        deprecated = sum(1 for entry in entries if _scoreboard_stability_bucket(entry) == "deprecated")
        host_dep = "yes" if any(_scoreboard_has_host_dep(entry) for entry in entries) else "no"

        non_deprecated = [entry for entry in entries if not _is_deprecated(entry)]
        fixture_total = len(non_deprecated)
        fixture_covered = sum(
            1 for entry in non_deprecated
            if _function_in_fixtures(entry["name"], fixture_text)
        )
        coverage_display = _format_fixture_coverage(fixture_covered, fixture_total)
        if fixture_total > 0:
            coverage_pct = round(100 * fixture_covered / fixture_total)
            if coverage_pct < SCOREBOARD_LOW_COVERAGE_THRESHOLD:
                coverage_display = f"**{coverage_display}**"
                low_coverage.append(f"`{family_name}` ({coverage_pct}%)")

        totals["apis"] += apis
        totals["stable"] += stable
        totals["provisional"] += provisional
        totals["experimental"] += experimental
        totals["deprecated"] += deprecated
        totals["fixture_covered"] += fixture_covered
        totals["fixture_total"] += fixture_total

        lines.append(
            f"| `{family_name}` | {apis} | {stable} | {provisional} | {experimental} | {deprecated} "
            f"| {coverage_display} | {host_dep} |"
        )

    total_coverage = _format_fixture_coverage(totals["fixture_covered"], totals["fixture_total"])
    lines.extend([
        "",
        f"**Total**: {totals['apis']} APIs, {totals['stable']} stable, "
        f"{totals['provisional']} provisional, {totals['experimental']} experimental, "
        f"{totals['deprecated']} deprecated, "
        f"{total_coverage} fixture coverage",
        "",
        "## Legend",
        "",
        "- **APIs**: Total function count in the family",
        "- **Stable**: Functions with `stability = \"stable\"` (or default)",
        "- **Provisional**: Functions with `stability = \"provisional\"`",
        "- **Experimental**: Functions with `stability = \"experimental\"`",
        "- **Deprecated**: Functions with `deprecated_by` set or `stability = \"deprecated\"`",
        "- **Fixture Coverage**: How many non-deprecated functions appear in at least one test fixture",
        "- **Host Dep**: Whether the family contains host-imported functions "
        "(`kind = \"host_stub\"` or module under `std::host::`)",
    ])

    if low_coverage:
        lines.extend([
            "",
            "## Low Coverage Families",
            "",
            "Families below 50% fixture coverage (denominator excludes deprecated APIs):",
            "",
            ", ".join(low_coverage) + ".",
        ])

    return "\n".join(lines) + "\n"


def replace_generated_block(text: str, marker: str, content: str) -> str:
    start = f"<!-- BEGIN GENERATED:{marker} -->"
    end = f"<!-- END GENERATED:{marker} -->"
    pattern = re.compile(re.escape(start) + r".*?" + re.escape(end), re.DOTALL)
    replacement = f"{start}\n{content.rstrip()}\n{end}"
    if not pattern.search(text):
        raise ValueError(f"missing marker {marker}")
    return pattern.sub(replacement, text, count=1)


def ensure_parent(path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)


def _collapse_blank_lines(text: str) -> str:
    """Collapse 3+ consecutive newlines to 2 (one blank line) — satisfies MD012."""
    import re
    return re.sub(r"\n{3,}", "\n\n", text)


def write_file(path: Path, desired: str, check: bool, stale: list[Path]) -> None:
    ensure_parent(path)
    normalized = _collapse_blank_lines(desired).rstrip() + "\n"
    current = path.read_text(encoding="utf-8") if path.exists() else None
    if current == normalized:
        return
    if check:
        stale.append(path)
        return
    path.write_text(normalized, encoding="utf-8")


def apply_marker_updates(path: Path, replacements: dict[str, str], check: bool, stale: list[Path]) -> None:
    text = path.read_text(encoding="utf-8")
    updated = text
    for marker, content in replacements.items():
        updated = replace_generated_block(updated, marker, content)
    write_file(path, updated, check, stale)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true", help="Fail if generated docs are out of date.")
    args = parser.parse_args()

    state = load_toml(PROJECT_STATE)
    sections = load_toml(SECTIONS_FILE)["sections"]
    manifest = load_stdlib_manifest()

    actual_fixture_total = fixture_count()
    stated_fixture_total = state.get("verification", {}).get("fixture_manifest_count")
    if stated_fixture_total is not None and int(stated_fixture_total) != actual_fixture_total:
        print(
            "project-state.toml fixture_manifest_count drift: "
            f"stated={stated_fixture_total} manifest.txt={actual_fixture_total}",
            file=sys.stderr,
        )
        return 1

    verification = state.get("verification", {})
    passed = int(verification.get("fixture_passed", 0) or 0)
    failed = int(verification.get("fixture_failures", 0) or 0)
    skipped = int(verification.get("fixture_skipped", 0) or 0)
    observed_sum = passed + failed + skipped
    blockers = verification.get("blockers", [])
    required_blocker_fields = {
        "id", "scope", "category", "summary", "command", "owner", "issue",
        "first_seen_commit", "last_verified_commit",
    }
    blocker_errors: list[str] = []
    seen_blocker_ids: set[str] = set()
    for blocker in blockers:
        missing = sorted(required_blocker_fields - blocker.keys())
        if missing:
            blocker_errors.append(f"{blocker.get('id', '<unnamed>')}: missing {missing}")
        if blocker.get("id") in seen_blocker_ids:
            blocker_errors.append(f"duplicate blocker id {blocker.get('id')}")
        seen_blocker_ids.add(blocker.get("id"))
        if blocker.get("category") not in {"fixture", "verification", "target-contract", "component-interop", "bootstrap"}:
            blocker_errors.append(f"{blocker.get('id')}: invalid category")
        if blocker.get("scope") not in {"quick", "full"}:
            blocker_errors.append(f"{blocker.get('id')}: invalid scope")
    fixture_blockers = sum(int(blocker.get("affected_count", 1)) for blocker in blockers if blocker.get("category") == "fixture")
    verification_blockers = sum(int(blocker.get("affected_count", 1)) for blocker in blockers if blocker.get("category") == "verification")
    check_gap = int(verification.get("checks_total", 0)) - int(verification.get("checks_passed", 0))
    if fixture_blockers != failed:
        blocker_errors.append(f"fixture_failures={failed} but {fixture_blockers} fixture blocker rows")
    if verification_blockers != check_gap:
        blocker_errors.append(f"verification check gap={check_gap} but {verification_blockers} verification blocker rows")
    if blocker_errors:
        print("project-state.toml blocker contract FAILED:", file=sys.stderr)
        for error in blocker_errors:
            print(f"  ✗ {error}", file=sys.stderr)
        return 1
    stated_observed = verification.get("fixture_harness_observed")
    if stated_observed is not None and int(stated_observed) != observed_sum:
        print(
            "project-state.toml fixture harness accounting drift: "
            f"passed+failed+skipped={observed_sum} fixture_harness_observed={stated_observed}",
            file=sys.stderr,
        )
        return 1
    stated_remainder = verification.get("fixture_not_in_last_harness_snapshot")
    if (
        stated_fixture_total is not None
        and stated_observed is not None
        and stated_remainder is not None
        and int(stated_remainder) != int(stated_fixture_total) - int(stated_observed)
    ):
        print(
            "project-state.toml fixture remainder drift: "
            f"manifest-observed={int(stated_fixture_total) - int(stated_observed)} "
            f"fixture_not_in_last_harness_snapshot={stated_remainder}",
            file=sys.stderr,
        )
        return 1

    # Schema validation: always run, fail fast before any doc generation
    schema_errors = validate_manifest_schema(manifest)
    if schema_errors:
        print("stdlib manifest schema validation FAILED:", file=sys.stderr)
        for err in schema_errors:
            print(f"  ✗ {err}", file=sys.stderr)
        print(
            f"\n{len(schema_errors)} violation(s) found in std/manifest.toml. "
            "Fix the entries above to match the schema in docs/stdlib/generation-schema.md",
            file=sys.stderr,
        )
        return 1

    manifest_stats = stdlib_stats(manifest)
    manifest_functions_by_module: dict[str, list[dict]] = defaultdict(list)
    for entry in manifest_stats["public_functions"]:
        module_name = entry.get("module")
        if module_name:
            manifest_functions_by_module[module_name].append(entry)
    manifest_modules_by_name: dict[str, dict] = {
        mod["name"]: mod for mod in manifest.get("modules", []) if "name" in mod
    }
    source_modules = collect_stdlib_source_modules()
    examples = collect_examples(state)
    fixture_total = fixture_count()
    stale: list[Path] = []

    write_target_contract_summary(state, args.check, stale)

    # Phase-2 structured state views (#770)
    structured_script = ROOT / "scripts" / "gen" / "generate-structured-state-docs.py"
    structured_cmd = [sys.executable, str(structured_script)]
    if args.check:
        structured_cmd.append("--check")
    structured_result = subprocess.run(
        structured_cmd,
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if structured_result.returncode != 0:
        if structured_result.stderr:
            print(structured_result.stderr, file=sys.stderr, end="")
        if structured_result.stdout:
            print(structured_result.stdout, file=sys.stderr, end="")
        if args.check:
            stale.append(DOCS / "capability-surface.md")
        else:
            print("structured state doc generation failed", file=sys.stderr)
            return 1

    apply_marker_updates(
        ROOT / "README.md",
        {
            "README_STATUS": render_readme_status(state, fixture_total, manifest_stats),
        },
        args.check,
        stale,
    )
    apply_marker_updates(
        DOCS / "current-state.md",
        {
            "CURRENT_STATE_UPDATED": render_current_state_updated(state),
            "CURRENT_STATE_TARGETS": render_current_state_targets(state),
            "CURRENT_STATE_TEST_HEALTH": render_current_state_test_health(state, fixture_total),
            "CURRENT_STATE_PERF": render_current_state_perf(state),
            "CURRENT_STATE_DIAGNOSTICS": render_current_state_diagnostics(state),
        },
        args.check,
        stale,
    )

    write_file(DOCS / "README.md", render_root_docs_readme(sections, state, fixture_total, manifest_stats), args.check, stale)
    write_file(DOCS / "_sidebar.md", render_sidebar(sections), args.check, stale)
    write_file(DOCS / "stdlib" / "reference.md", render_stdlib_reference(manifest), args.check, stale)
    write_file(DOCS / "stdlib" / "name-index.md", render_name_index(manifest), args.check, stale)
    write_file(DOCS / "stdlib" / "scoreboard.md", render_scoreboard(manifest), args.check, stale)
    write_file(MONOMORPHIC_DEPRECATION, render_deprecation_table(manifest), args.check, stale)
    for page in STDLIB_MODULE_PAGES:
        write_file(
            DOCS / "stdlib" / page["path"],
            render_stdlib_module_page(page, manifest_functions_by_module, source_modules, manifest_modules_by_name),
            args.check,
            stale,
        )
    for page in STDLIB_ALIAS_PAGES:
        write_file(DOCS / "stdlib" / page["path"], render_stdlib_alias_page(page), args.check, stale)

    classifications = load_language_classifications()
    classified = {c.get("file") for c in classifications if c.get("file")}
    lang_files = {
        p.name
        for p in (DOCS / "language").glob("*.md")
        if p.name != "README.md"
    }
    unclassified = sorted(lang_files - classified)
    if unclassified:
        print(
            "language doc classification incomplete — add entries to "
            "docs/data/language-doc-classifications.toml:",
            file=sys.stderr,
        )
        for name in unclassified:
            print(f"  ✗ {name}", file=sys.stderr)
        return 1

    for section in sections:
        section_dir = DOCS / section["dir"]
        entries = collect_markdown_entries(section_dir)
        if section["dir"] == "stdlib":
            content = render_stdlib_readme(section, entries, state, manifest_stats, source_modules)
        elif section["dir"] == "examples":
            content = render_examples_readme(section, examples, state)
        elif section["dir"] == "sample":
            content = render_sample_readme(section, collect_sample_files())
        elif section["dir"] == "language":
            content = render_language_readme(
                section,
                entries,
                section_snapshot(section, state, fixture_total, manifest_stats, examples),
                classifications,
            )
        elif section["dir"] == "playground":
            content = render_playground_readme(
                section,
                entries,
                section_snapshot(section, state, fixture_total, manifest_stats, examples),
            )
        elif section["dir"] == "adr":
            content = render_adr_readme(
                section,
                entries,
                section_snapshot(section, state, fixture_total, manifest_stats, examples),
            )
        else:
            content = render_generic_section_readme(
                section,
                entries,
                section_snapshot(section, state, fixture_total, manifest_stats, examples),
            )
        write_file(section_dir / "README.md", content, args.check, stale)

    # Generate the feature maturity matrix from TOML feature classifications
    toml_features = load_feature_classifications()
    if not toml_features:
        # Fallback: parse spec.md if TOML has no [[features]] yet
        toml_features = parse_spec_stability_sections()
    write_file(MATURITY_MATRIX, render_maturity_matrix(toml_features), args.check, stale)

    if stale:
        for path in stale:
            print(path.relative_to(ROOT), file=sys.stderr)
        print("generated docs are out of date; run `python3 scripts/gen/generate-docs.py`", file=sys.stderr)
        return 1

    print("generated docs are up to date")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
