//! Standard library for Arukellt.
//!
//! Provides built-in function signatures and Wasm implementations
//! for the prelude, Vec, String, Option, Result, and I/O modules.

/// Standard library function descriptor.
#[derive(Debug, Clone)]
pub struct StdlibFn {
    pub name: String,
    pub module: &'static str,
    pub wasi_import: Option<(&'static str, &'static str)>,
}

/// Get all stdlib function descriptors.
pub fn stdlib_functions() -> Vec<StdlibFn> {
    vec![
        // Print functions
        StdlibFn { name: "println".into(), module: "prelude", wasi_import: None },
        StdlibFn { name: "print".into(), module: "prelude", wasi_import: None },
        StdlibFn { name: "eprintln".into(), module: "prelude", wasi_import: None },

        // Conversion
        StdlibFn { name: "i32_to_string".into(), module: "prelude", wasi_import: None },
        StdlibFn { name: "i64_to_string".into(), module: "prelude", wasi_import: None },
        StdlibFn { name: "f64_to_string".into(), module: "prelude", wasi_import: None },
        StdlibFn { name: "parse_i32".into(), module: "prelude", wasi_import: None },
        StdlibFn { name: "parse_i64".into(), module: "prelude", wasi_import: None },
        StdlibFn { name: "parse_f64".into(), module: "prelude", wasi_import: None },

        // Vec constructors
        StdlibFn { name: "Vec_new_i32".into(), module: "vec", wasi_import: None },
        StdlibFn { name: "Vec_new_i64".into(), module: "vec", wasi_import: None },
        StdlibFn { name: "Vec_new_f64".into(), module: "vec", wasi_import: None },
        StdlibFn { name: "Vec_new_String".into(), module: "vec", wasi_import: None },

        // Math
        StdlibFn { name: "sqrt".into(), module: "prelude", wasi_import: None },
        StdlibFn { name: "abs".into(), module: "prelude", wasi_import: None },
        StdlibFn { name: "min".into(), module: "prelude", wasi_import: None },
        StdlibFn { name: "max".into(), module: "prelude", wasi_import: None },
    ]
}
