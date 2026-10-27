# Cookbook: Wasm Binary Construction

Using `std::wasm` types and `std::bytes` to build Wasm binary sections.

> Policy: user-facing samples should avoid direct low-level prelude helper calls.
> Prefer module-local wrappers/facades; if needed, hide prelude details behind local helper functions.
>
> Both `std::wasm` and parts of `std::bytes` are **Experimental** in implementation status.

## Wasm Magic Header

Every Wasm module starts with an 8-byte header: magic number + version.

<!-- skip-doc-check reason="legacy example not fixture-backed" owner="#683" kind="non-runnable" expires="2026-11-30" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::bytes

let buf: Vec<i32> = Vec::new<i32>()
// Magic: \0asm
buf.push_byte(0x00)
buf.push_byte(0x61)  // 'a'
buf.push_byte(0x73)  // 's'
buf.push_byte(0x6D)  // 'm'
// Version: 1
buf.push_u32_le(1)

let header = buf.freeze()
assert_eq(header.len(), 8)
```

## LEB128 Encoding for Section Sizes

Wasm uses LEB128 variable-length encoding for integers in binary format.

<!-- skip-doc-check reason="legacy example not fixture-backed" owner="#683" kind="non-runnable" expires="2026-11-30" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::bytes

let buf: Vec<i32> = Vec::new<i32>()
write_var_u32(buf, 42)     // small value: 1 byte [0x2a]
write_var_u32(buf, 300)    // larger value: 2 bytes [0xac, 0x02]
write_var_u32(buf, 128)    // boundary: 2 bytes [0x80, 0x01]

let encoded = buf.freeze()
assert_eq(encoded.len(), 5)

// Size estimation
let size1 = leb128_u32_size(42)    // 1
let size2 = leb128_u32_size(300)   // 2
let size3 = leb128_u32_size(128)   // 2
```

## Building a Type Section

The type section (id=1) declares function signatures.

<!-- skip-doc-check reason="legacy example not fixture-backed" owner="#683" kind="non-runnable" expires="2026-11-30" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::bytes

fn encode_type_section() -> Vec<i32> {
    // Build section content first to compute size
    let content: Vec<i32> = Vec::new<i32>()

    // Number of types: 1
    write_var_u32(content, 1)

    // Function type tag
    content.push_byte(0x60)

    // Params: (i32, i32)
    write_var_u32(content, 2)   // param count
    content.push_byte(0x7F)  // i32
    content.push_byte(0x7F)  // i32

    // Results: (i32)
    write_var_u32(content, 1)   // result count
    content.push_byte(0x7F)  // i32

    let content_bytes = content.freeze()

    // Build section with header
    let section: Vec<i32> = Vec::new<i32>()
    section.push_byte(0x01)  // section id = Type
    write_var_u32(section, content_bytes.len())
    section.extend_bytes(content_bytes)

    section.freeze()
}
```

## Reading Wasm Bytes with ByteCursor

<!-- skip-doc-check reason="legacy example not fixture-backed" owner="#683" kind="non-runnable" expires="2026-11-30" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::bytes

fn validate_wasm_header(data: Vec<i32>) -> bool {
    let c: Vec<i32> = data.byte_cursor()

    // Read magic number; the read_* sentinel is -1 at EOF.
    let b0 = c.read_u8()
    let b1 = c.read_u8()
    let b2 = c.read_u8()
    let b3 = c.read_u8()

    if b0 != 0x00 { return false }
    if b1 != 0x61 { return false }
    if b2 != 0x73 { return false }
    if b3 != 0x6D { return false }

    // Read version
    let version = c.read_u32_le()
    version == 1
}
```

## WIT Type Inspection

Use `std::wit` to map WIT primitive types.

<!-- skip-doc-check reason="legacy example not fixture-backed" owner="#683" kind="non-runnable" expires="2026-11-30" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::host::stdio
use std::text
use std::wit::types

// List all WIT primitive type ids and names
let mut i = 1
while i <= 13 {
    let wit_ty = types::WitType::from_id(i)
    stdio::println((wit_ty.id()).to_string().concat(": ".concat(wit_ty.name())))
    i = i + 1
}
// 1: bool, 2: u8, ..., 13: string
```

## Component Model Version Check

<!-- skip-doc-check reason="legacy example not fixture-backed" owner="#683" kind="non-runnable" expires="2026-11-30" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::component

let abi = component::Component::canonical_abi_version()
let ver = component::Component::model_version()
println(concat("ABI v", to_string(abi)))  // "ABI v1"
println(concat("CM ", ver))                     // "CM 0.2.0"
```
