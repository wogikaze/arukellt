//! Extended constant expression builders for the Wasm GC emitter.
//!
//! The WebAssembly Extended Const proposal allows arithmetic operations
//! (`i32.add`, `i32.sub`, `i32.mul`, and their `i64` counterparts) inside
//! constant expressions used for global initializers, data-segment offsets,
//! and element-segment offsets.
//!
//! This module provides typed helpers that wrap `wasm_encoder::ConstExpr` to
//! make extended-const usage explicit and safe.

use wasm_encoder::ConstExpr;

/// Build an `i32.add` constant expression: `(i32.add (i32.const a) (i32.const b))`.
pub fn i32_add(a: i32, b: i32) -> ConstExpr {
    ConstExpr::i32_const(a)
        .with_i32_const(b)
        .with_i32_add()
}

/// Build an `i32.sub` constant expression: `(i32.sub (i32.const a) (i32.const b))`.
pub fn i32_sub(a: i32, b: i32) -> ConstExpr {
    ConstExpr::i32_const(a)
        .with_i32_const(b)
        .with_i32_sub()
}

/// Build an `i32.mul` constant expression: `(i32.mul (i32.const a) (i32.const b))`.
pub fn i32_mul(a: i32, b: i32) -> ConstExpr {
    ConstExpr::i32_const(a)
        .with_i32_const(b)
        .with_i32_mul()
}

/// Build an `i64.add` constant expression: `(i64.add (i64.const a) (i64.const b))`.
pub fn i64_add(a: i64, b: i64) -> ConstExpr {
    ConstExpr::i64_const(a)
        .with_i64_const(b)
        .with_i64_add()
}

/// Build an `i64.sub` constant expression: `(i64.sub (i64.const a) (i64.const b))`.
pub fn i64_sub(a: i64, b: i64) -> ConstExpr {
    ConstExpr::i64_const(a)
        .with_i64_const(b)
        .with_i64_sub()
}

/// Build an `i64.mul` constant expression: `(i64.mul (i64.const a) (i64.const b))`.
pub fn i64_mul(a: i64, b: i64) -> ConstExpr {
    ConstExpr::i64_const(a)
        .with_i64_const(b)
        .with_i64_mul()
}

/// Build a heap-pointer initializer.
///
/// At `opt_level >= 2`, emits an extended const expression that makes the
/// layout semantics explicit:
/// ```wasm
/// (i32.add (i32.const DATA_START) (i32.const data_size))
/// ```
///
/// At lower opt levels, emits a plain `i32.const` with the precomputed offset.
pub fn heap_ptr_init(data_start: u32, data_offset: u32, opt_level: u8) -> ConstExpr {
    let data_size = data_offset.wrapping_sub(data_start);
    if opt_level >= 2 && data_size > 0 {
        i32_add(data_start as i32, data_size as i32)
    } else {
        ConstExpr::i32_const(data_offset as i32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_encoder::Encode;

    fn encode(expr: &ConstExpr) -> Vec<u8> {
        let mut buf = Vec::new();
        expr.encode(&mut buf);
        buf
    }

    #[test]
    fn i32_add_encoding() {
        let expr = i32_add(10, 20);
        let bytes = encode(&expr);
        // i32.const 10, i32.const 20, i32.add, end
        assert_eq!(bytes[0], 0x41); // i32.const opcode
        assert!(bytes.contains(&0x6a)); // i32.add opcode
        assert_eq!(*bytes.last().unwrap(), 0x0b); // end
    }

    #[test]
    fn i32_sub_encoding() {
        let expr = i32_sub(30, 10);
        let bytes = encode(&expr);
        assert_eq!(bytes[0], 0x41); // i32.const
        assert!(bytes.contains(&0x6b)); // i32.sub
    }

    #[test]
    fn i32_mul_encoding() {
        let expr = i32_mul(5, 6);
        let bytes = encode(&expr);
        assert_eq!(bytes[0], 0x41); // i32.const
        assert!(bytes.contains(&0x6c)); // i32.mul
    }

    #[test]
    fn i64_add_encoding() {
        let expr = i64_add(100, 200);
        let bytes = encode(&expr);
        assert_eq!(bytes[0], 0x42); // i64.const opcode
        assert!(bytes.contains(&0x7c)); // i64.add opcode
    }

    #[test]
    fn heap_ptr_init_plain_at_low_opt() {
        let expr = heap_ptr_init(256, 300, 0);
        let bytes = encode(&expr);
        // Should be a single i32.const 300
        assert_eq!(bytes[0], 0x41); // i32.const
        assert!(!bytes.contains(&0x6a)); // no i32.add
    }

    #[test]
    fn heap_ptr_init_extended_at_opt2() {
        let expr = heap_ptr_init(256, 300, 2);
        let bytes = encode(&expr);
        // Should contain i32.add
        assert!(bytes.contains(&0x6a)); // i32.add
    }

    #[test]
    fn heap_ptr_init_no_data_stays_plain() {
        // When data_size == 0 (no data segments), use plain const even at opt2
        let expr = heap_ptr_init(256, 256, 2);
        let bytes = encode(&expr);
        assert!(!bytes.contains(&0x6a)); // no i32.add
    }
}
