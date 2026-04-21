# Cookbook: Wasm Binary Construction

Using `std::wasm` types and `std::bytes` to build Wasm binary sections.

> Policy: user-facing samples should avoid direct low-level prelude helper calls.
> Prefer module-local wrappers/facades; if needed, hide prelude details behind local helper functions.
>
> Both `std::wasm` and parts of `std::bytes` are **Experimental** in implementation status.

## Wasm Magic Header

Every Wasm module starts with an 8-byte header: magic number + version.

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::bytes

let buf = buf_new()
// Magic: \0asm
buf_push_u8(buf, 0x00)
buf_push_u8(buf, 0x61)  // 'a'
buf_push_u8(buf, 0x73)  // 's'
buf_push_u8(buf, 0x6D)  // 'm'
// Version: 1
buf_push_u32_le(buf, 1)

let header = buf_freeze(buf)
assert_eq(bytes_len(header), 8)
```

## LEB128 Encoding for Section Sizes

Wasm uses LEB128 variable-length encoding for integers in binary format.

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::bytes

let buf = buf_new()
write_var_u32(buf, 42)     // small value: 1 byte [0x2a]
write_var_u32(buf, 300)    // larger value: 2 bytes [0xac, 0x02]
write_var_u32(buf, 128)    // boundary: 2 bytes [0x80, 0x01]

let encoded = buf_freeze(buf)
assert_eq(bytes_len(encoded), 5)

// Size estimation
let size1 = leb128_u32_size(42)    // 1
let size2 = leb128_u32_size(300)   // 2
let size3 = leb128_u32_size(128)   // 2
```

## Building a Type Section

The type section (id=1) declares function signatures.

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::bytes

fn encode_type_section() -> Bytes {
    // Build section content first to compute size
    let content = buf_new()

    // Number of types: 1
    write_var_u32(content, 1)

    // Function type tag
    buf_push_u8(content, 0x60)

    // Params: (i32, i32)
    write_var_u32(content, 2)   // param count
    buf_push_u8(content, 0x7F)  // i32
    buf_push_u8(content, 0x7F)  // i32

    // Results: (i32)
    write_var_u32(content, 1)   // result count
    buf_push_u8(content, 0x7F)  // i32

    let content_bytes = buf_freeze(content)

    // Build section with header
    let section = buf_new()
    buf_push_u8(section, 0x01)  // section id = Type
    write_var_u32(section, bytes_len(content_bytes))
    buf_extend(section, content_bytes)

    buf_freeze(section)
}
```

## Reading Wasm Bytes with ByteCursor

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::bytes

fn validate_wasm_header(data: Bytes) -> bool {
    let c = cursor_new(data)

    // Read magic number
    let b0 = unwrap(read_u8(c))
    let b1 = unwrap(read_u8(c))
    let b2 = unwrap(read_u8(c))
    let b3 = unwrap(read_u8(c))

    if b0 != 0x00 { return false }
    if b1 != 0x61 { return false }
    if b2 != 0x73 { return false }
    if b3 != 0x6D { return false }

    // Read version
    let version = unwrap(read_u32_le(c))
    version == 1
}
```

## WIT Type Inspection

Use `std::wit` to map WIT primitive types.

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::wit

// List all WIT primitive types
let types = Vec_new_i32()
let mut i = 1
while i <= 13 {
    push(types, i)
    i = i + 1
}

let n = len(types)
let mut j = 0
while j < n {
    let ty = get(types, j)
    let wit_ty = wit::wit_type_from_id(ty)
    let name = wit::wit_type_name(wit_ty)
    println(concat(to_string(wit::wit_type_id(wit_ty)), concat(": ", name)))
    j = j + 1
}
// 1: bool, 2: u8, ..., 13: string
```

## Component Model Version Check

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::component

let abi = component::canonical_abi_version()
let ver = component::component_model_version()
println(concat("ABI v", to_string(abi)))  // "ABI v1"
println(concat("CM ", ver))                     // "CM 0.2.0"
```
