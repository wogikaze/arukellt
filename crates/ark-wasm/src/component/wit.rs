//! WIT text generation from public function signatures.

use std::fmt::Write;

/// A simplified public function descriptor for WIT generation.
#[derive(Debug, Clone)]
pub struct WitFunction {
    pub name: String,
    pub params: Vec<(String, WitType)>,
    pub result: Option<WitType>,
}

/// WIT type mapping from Arukellt types.
#[derive(Debug, Clone, PartialEq)]
pub enum WitType {
    U8,
    U16,
    U32,
    U64,
    S8,
    S16,
    S32,
    S64,
    F32,
    F64,
    Bool,
    Char,
    StringType,
    List(Box<WitType>),
    Option(Box<WitType>),
    Result {
        ok: Option<Box<WitType>>,
        err: Option<Box<WitType>>,
    },
    Record(String),
    Enum(String),
    Variant(String),
    Tuple(Vec<WitType>),
    /// Resource type reference (name only)
    Resource(String),
    /// Owned handle: `own<T>`
    Own(Box<WitType>),
    /// Borrowed handle: `borrow<T>`
    Borrow(Box<WitType>),
}

/// A named record definition for WIT generation.
#[derive(Debug, Clone)]
pub struct WitRecord {
    pub name: String,
    pub fields: Vec<(String, WitType)>,
}

/// A named enum definition for WIT generation (unit variants only).
#[derive(Debug, Clone)]
pub struct WitEnum {
    pub name: String,
    pub variants: Vec<String>,
}

/// A named variant definition (with payloads).
#[derive(Debug, Clone)]
pub struct WitVariant {
    pub name: String,
    pub cases: Vec<(String, Option<WitType>)>,
}

/// Specification for a standard WASI world.
#[derive(Debug, Clone)]
pub struct WitWorldSpec {
    /// World name to use (e.g., "command", "proxy")
    pub world_name: String,
    /// `use` import directives (e.g., "wasi:cli/stdin@0.2.0")
    pub use_imports: Vec<String>,
    /// `use` export directives (e.g., "wasi:http/incoming-handler@0.2.0")
    pub use_exports: Vec<String>,
    /// Required exports the user code must provide (interface path → function name).
    /// E.g., ("wasi:cli/run/run", "run")
    pub required_exports: Vec<(String, String)>,
}

/// Parse a `--world` spec string into a `WitWorldSpec`.
///
/// Supported values: `wasi:cli/command`, `wasi:http/proxy`.
/// Returns `None` for unrecognized specs.
pub fn parse_world_spec(spec: &str) -> Option<WitWorldSpec> {
    match spec {
        "wasi:cli/command" => Some(WitWorldSpec {
            world_name: "command".to_string(),
            use_imports: vec![
                "wasi:cli/stdin@0.2.0".to_string(),
                "wasi:cli/stdout@0.2.0".to_string(),
                "wasi:clocks/wall-clock@0.2.0".to_string(),
            ],
            use_exports: vec![],
            required_exports: vec![("wasi:cli/run/run".to_string(), "run".to_string())],
        }),
        "wasi:http/proxy" => Some(WitWorldSpec {
            world_name: "proxy".to_string(),
            use_imports: vec!["wasi:http/types@0.2.0".to_string()],
            use_exports: vec!["wasi:http/incoming-handler@0.2.0".to_string()],
            required_exports: vec![],
        }),
        _ => None,
    }
}

/// Public API surface for WIT generation.
#[derive(Debug, Clone, Default)]
pub struct WitWorld {
    pub name: String,
    /// Exported functions
    pub functions: Vec<WitFunction>,
    /// Imported functions (from WIT host interfaces)
    pub imports: Vec<WitFunction>,
    pub records: Vec<WitRecord>,
    pub enums: Vec<WitEnum>,
    pub variants: Vec<WitVariant>,
    /// Resource type names
    pub resources: Vec<String>,
    /// Optional standard world spec (controls `use` directives)
    pub world_spec: Option<WitWorldSpec>,
}

/// Errors during WIT generation.
#[derive(Debug)]
pub enum WitError {
    /// Type cannot be exported via WIT (e.g., closure, raw ref)
    NonExportableType { type_name: String, reason: String },
    /// Unknown `--world` spec string
    UnknownWorld { spec: String },
    /// Required export missing for the specified world
    MissingWorldExport { world: String, required: String },
}

impl std::fmt::Display for WitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WitError::NonExportableType { type_name, reason } => {
                write!(
                    f,
                    "type `{}` cannot be exported via WIT: {}",
                    type_name, reason
                )
            }
            WitError::UnknownWorld { spec } => {
                write!(
                    f,
                    "unknown world `{}` (supported: wasi:cli/command, wasi:http/proxy)",
                    spec
                )
            }
            WitError::MissingWorldExport { world, required } => {
                write!(
                    f,
                    "world `{}` requires export `{}`, but no matching function found",
                    world, required
                )
            }
        }
    }
}

impl WitType {
    /// Convert to WIT type string.
    pub fn to_wit(&self) -> String {
        match self {
            WitType::U8 => "u8".to_string(),
            WitType::U16 => "u16".to_string(),
            WitType::U32 => "u32".to_string(),
            WitType::U64 => "u64".to_string(),
            WitType::S8 => "s8".to_string(),
            WitType::S16 => "s16".to_string(),
            WitType::S32 => "s32".to_string(),
            WitType::S64 => "s64".to_string(),
            WitType::F32 => "f32".to_string(),
            WitType::F64 => "f64".to_string(),
            WitType::Bool => "bool".to_string(),
            WitType::Char => "char".to_string(),
            WitType::StringType => "string".to_string(),
            WitType::List(inner) => format!("list<{}>", inner.to_wit()),
            WitType::Option(inner) => format!("option<{}>", inner.to_wit()),
            WitType::Result { ok, err } => match (ok, err) {
                (Some(ok), Some(err)) => format!("result<{}, {}>", ok.to_wit(), err.to_wit()),
                (Some(ok), None) => format!("result<{}>", ok.to_wit()),
                (None, Some(err)) => format!("result<_, {}>", err.to_wit()),
                (None, None) => "result".to_string(),
            },
            WitType::Record(name) | WitType::Enum(name) | WitType::Variant(name) => {
                to_kebab_case(name)
            }
            WitType::Resource(name) => to_kebab_case(name),
            WitType::Own(inner) => format!("own<{}>", inner.to_wit()),
            WitType::Borrow(inner) => format!("borrow<{}>", inner.to_wit()),
            WitType::Tuple(elems) => {
                let parts: Vec<_> = elems.iter().map(|e| e.to_wit()).collect();
                format!("tuple<{}>", parts.join(", "))
            }
        }
    }
}

/// Generate WIT text from a world descriptor.
pub fn generate_wit(world: &WitWorld) -> Result<String, WitError> {
    let mut out = String::new();

    writeln!(out, "package arukellt:app;").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "world {} {{", to_kebab_case(&world.name)).unwrap();

    // Emit `use` import directives from world spec
    if let Some(ref spec) = world.world_spec {
        for use_import in &spec.use_imports {
            writeln!(out, "    import {};", use_import).unwrap();
        }
        for use_export in &spec.use_exports {
            writeln!(out, "    export {};", use_export).unwrap();
        }
        if !spec.use_imports.is_empty() || !spec.use_exports.is_empty() {
            writeln!(out).unwrap();
        }
    }

    // Type definitions inside world block
    for record in &world.records {
        writeln!(out, "    record {} {{", to_kebab_case(&record.name)).unwrap();
        for (i, (name, ty)) in record.fields.iter().enumerate() {
            let comma = if i < record.fields.len() - 1 { "," } else { "" };
            writeln!(
                out,
                "        {}: {}{}",
                to_kebab_case(name),
                ty.to_wit(),
                comma
            )
            .unwrap();
        }
        writeln!(out, "    }}").unwrap();
        writeln!(out).unwrap();
    }

    for en in &world.enums {
        writeln!(out, "    enum {} {{", to_kebab_case(&en.name)).unwrap();
        for (i, variant) in en.variants.iter().enumerate() {
            let comma = if i < en.variants.len() - 1 { "," } else { "" };
            writeln!(out, "        {}{}", to_kebab_case(variant), comma).unwrap();
        }
        writeln!(out, "    }}").unwrap();
        writeln!(out).unwrap();
    }

    for var in &world.variants {
        writeln!(out, "    variant {} {{", to_kebab_case(&var.name)).unwrap();
        for (i, (name, payload)) in var.cases.iter().enumerate() {
            let comma = if i < var.cases.len() - 1 { "," } else { "" };
            match payload {
                Some(ty) => {
                    writeln!(
                        out,
                        "        {}({}){}",
                        to_kebab_case(name),
                        ty.to_wit(),
                        comma
                    )
                    .unwrap();
                }
                None => writeln!(out, "        {}{}", to_kebab_case(name), comma).unwrap(),
            }
        }
        writeln!(out, "    }}").unwrap();
        writeln!(out).unwrap();
    }

    // Resource declarations
    for res_name in &world.resources {
        writeln!(out, "    resource {};", to_kebab_case(res_name)).unwrap();
    }
    if !world.resources.is_empty() {
        writeln!(out).unwrap();
    }

    // Import declarations
    for func in &world.imports {
        let params: Vec<String> = func
            .params
            .iter()
            .map(|(n, t)| format!("{}: {}", to_kebab_case(n), t.to_wit()))
            .collect();
        match &func.result {
            Some(ret) => {
                writeln!(
                    out,
                    "    import {}: func({}) -> {};",
                    to_kebab_case(&func.name),
                    params.join(", "),
                    ret.to_wit()
                )
                .unwrap();
            }
            None => {
                writeln!(
                    out,
                    "    import {}: func({});",
                    to_kebab_case(&func.name),
                    params.join(", ")
                )
                .unwrap();
            }
        }
    }

    // Export declarations
    for func in &world.functions {
        let params: Vec<String> = func
            .params
            .iter()
            .map(|(n, t)| format!("{}: {}", to_kebab_case(n), t.to_wit()))
            .collect();
        match &func.result {
            Some(ret) => {
                writeln!(
                    out,
                    "    export {}: func({}) -> {};",
                    to_kebab_case(&func.name),
                    params.join(", "),
                    ret.to_wit()
                )
                .unwrap();
            }
            None => {
                writeln!(
                    out,
                    "    export {}: func({});",
                    to_kebab_case(&func.name),
                    params.join(", ")
                )
                .unwrap();
            }
        }
    }
    writeln!(out, "}}").unwrap();

    Ok(out)
}

/// Convert PascalCase/snake_case/camelCase to kebab-case for WIT identifiers.
///
/// Rules:
/// - `_` → `-` (leading underscores are dropped)
/// - uppercase letter after a non-separator character → insert `-` before it
/// - no double dashes: an uppercase letter immediately following `_` does not
///   generate an extra `-` (e.g., `snake_Upper` → `snake-upper`)
fn to_kebab_case(s: &str) -> String {
    let mut result = String::new();
    let mut last_was_sep = false;
    for (i, ch) in s.chars().enumerate() {
        if ch == '_' {
            // Drop leading underscores; otherwise emit a single dash.
            if !result.is_empty() {
                result.push('-');
                last_was_sep = true;
            }
        } else if ch.is_uppercase() {
            // Insert a dash before uppercase unless this is the first character
            // or the previous character was already a separator.
            if i > 0 && !last_was_sep {
                result.push('-');
            }
            result.push(ch.to_lowercase().next().unwrap());
            last_was_sep = false;
        } else {
            result.push(ch);
            last_was_sep = false;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_wit_generation() {
        let world = WitWorld {
            name: "hello".to_string(),
            functions: vec![WitFunction {
                name: "greet".to_string(),
                params: vec![("name".to_string(), WitType::StringType)],
                result: Some(WitType::StringType),
            }],
            imports: vec![],
            records: vec![],
            enums: vec![],
            variants: vec![],
            resources: vec![],
            world_spec: None,
        };
        let wit = generate_wit(&world).unwrap();
        assert!(wit.contains("package arukellt:app;"));
        assert!(wit.contains("export greet: func(name: string) -> string;"));
    }

    #[test]
    fn test_wit_type_mapping() {
        assert_eq!(WitType::S32.to_wit(), "s32");
        assert_eq!(WitType::S64.to_wit(), "s64");
        assert_eq!(WitType::F32.to_wit(), "f32");
        assert_eq!(WitType::F64.to_wit(), "f64");
        assert_eq!(WitType::Bool.to_wit(), "bool");
        assert_eq!(WitType::Char.to_wit(), "char");
        assert_eq!(WitType::StringType.to_wit(), "string");
        assert_eq!(WitType::List(Box::new(WitType::S32)).to_wit(), "list<s32>");
        assert_eq!(
            WitType::Option(Box::new(WitType::StringType)).to_wit(),
            "option<string>"
        );
    }

    #[test]
    fn test_record_wit_generation() {
        let world = WitWorld {
            name: "shapes".to_string(),
            functions: vec![],
            imports: vec![],
            records: vec![WitRecord {
                name: "Point".to_string(),
                fields: vec![
                    ("x".to_string(), WitType::F64),
                    ("y".to_string(), WitType::F64),
                ],
            }],
            enums: vec![],
            variants: vec![],
            resources: vec![],
            world_spec: None,
        };
        let wit = generate_wit(&world).unwrap();
        assert!(wit.contains("record point {"));
        assert!(wit.contains("x: f64"));
    }

    #[test]
    fn test_kebab_case() {
        assert_eq!(to_kebab_case("hello_world"), "hello-world");
        assert_eq!(to_kebab_case("MyStruct"), "my-struct");
        assert_eq!(to_kebab_case("simple"), "simple");
    }

    #[test]
    fn test_kebab_case_consistency() {
        // camelCase
        assert_eq!(to_kebab_case("camelCase"), "camel-case");
        assert_eq!(to_kebab_case("myFunction"), "my-function");
        // snake_case
        assert_eq!(to_kebab_case("safe_div"), "safe-div");
        assert_eq!(to_kebab_case("maybe_double"), "maybe-double");
        // snake_UpperCase mix — must not produce double dash
        assert_eq!(to_kebab_case("snake_Upper"), "snake-upper");
        assert_eq!(to_kebab_case("my_Point"), "my-point");
        // leading underscore — stripped
        assert_eq!(to_kebab_case("_internal"), "internal");
        // already kebab
        assert_eq!(to_kebab_case("distance-sq"), "distance-sq");
    }

    #[test]
    fn test_result_type() {
        let rt = WitType::Result {
            ok: Some(Box::new(WitType::S32)),
            err: Some(Box::new(WitType::StringType)),
        };
        assert_eq!(rt.to_wit(), "result<s32, string>");
    }

    #[test]
    fn test_resource_wit_generation() {
        let world = WitWorld {
            name: "storage".to_string(),
            functions: vec![
                WitFunction {
                    name: "open".to_string(),
                    params: vec![("path".to_string(), WitType::StringType)],
                    result: Some(WitType::Own(Box::new(WitType::Resource(
                        "file".to_string(),
                    )))),
                },
                WitFunction {
                    name: "read".to_string(),
                    params: vec![(
                        "f".to_string(),
                        WitType::Borrow(Box::new(WitType::Resource("file".to_string()))),
                    )],
                    result: Some(WitType::StringType),
                },
            ],
            imports: vec![],
            records: vec![],
            enums: vec![],
            variants: vec![],
            resources: vec!["file".to_string()],
            world_spec: None,
        };
        let wit = generate_wit(&world).unwrap();
        assert!(wit.contains("resource file;"));
        assert!(wit.contains("own<file>"));
        assert!(wit.contains("borrow<file>"));
    }

    #[test]
    fn test_own_borrow_wit_types() {
        assert_eq!(
            WitType::Own(Box::new(WitType::Resource("conn".to_string()))).to_wit(),
            "own<conn>"
        );
        assert_eq!(
            WitType::Borrow(Box::new(WitType::Resource("conn".to_string()))).to_wit(),
            "borrow<conn>"
        );
    }

    #[test]
    fn test_parse_world_spec_cli_command() {
        let spec = parse_world_spec("wasi:cli/command").unwrap();
        assert_eq!(spec.world_name, "command");
        assert!(spec.use_imports.iter().any(|s| s.contains("stdin")));
        assert!(spec.use_imports.iter().any(|s| s.contains("stdout")));
        assert_eq!(spec.required_exports.len(), 1);
        assert_eq!(spec.required_exports[0].1, "run");
    }

    #[test]
    fn test_parse_world_spec_http_proxy() {
        let spec = parse_world_spec("wasi:http/proxy").unwrap();
        assert_eq!(spec.world_name, "proxy");
        assert!(spec.use_imports.iter().any(|s| s.contains("http/types")));
        assert!(
            spec.use_exports
                .iter()
                .any(|s| s.contains("incoming-handler"))
        );
    }

    #[test]
    fn test_parse_world_spec_unknown() {
        assert!(parse_world_spec("wasi:unknown/world").is_none());
    }

    #[test]
    fn test_generate_wit_with_world_spec() {
        let spec = parse_world_spec("wasi:cli/command").unwrap();
        let world = WitWorld {
            name: spec.world_name.clone(),
            functions: vec![WitFunction {
                name: "run".to_string(),
                params: vec![],
                result: None,
            }],
            imports: vec![],
            records: vec![],
            enums: vec![],
            variants: vec![],
            resources: vec![],
            world_spec: Some(spec),
        };
        let wit = generate_wit(&world).unwrap();
        assert!(wit.contains("world command {"));
        assert!(wit.contains("import wasi:cli/stdin@0.2.0;"));
        assert!(wit.contains("import wasi:cli/stdout@0.2.0;"));
        assert!(wit.contains("import wasi:clocks/wall-clock@0.2.0;"));
        assert!(wit.contains("export run: func();"));
    }

    #[test]
    fn test_generate_wit_http_proxy_spec() {
        let spec = parse_world_spec("wasi:http/proxy").unwrap();
        let world = WitWorld {
            name: spec.world_name.clone(),
            functions: vec![],
            imports: vec![],
            records: vec![],
            enums: vec![],
            variants: vec![],
            resources: vec![],
            world_spec: Some(spec),
        };
        let wit = generate_wit(&world).unwrap();
        assert!(wit.contains("world proxy {"));
        assert!(wit.contains("import wasi:http/types@0.2.0;"));
        assert!(wit.contains("export wasi:http/incoming-handler@0.2.0;"));
    }

    // ── WIT accuracy tests ────────────────────────────────────────────────────
    // Each test constructs the WitWorld that matches a known fixture source file
    // and compares the output byte-for-byte against the corresponding
    // `.expected.wit` file checked into tests/fixtures/component/.

    #[test]
    fn test_wit_accuracy_export_add() {
        let expected = include_str!("../../../../tests/fixtures/component/export_add.expected.wit");
        let world = WitWorld {
            name: "export_add".to_string(),
            functions: vec![WitFunction {
                name: "add".to_string(),
                params: vec![
                    ("a".to_string(), WitType::S32),
                    ("b".to_string(), WitType::S32),
                ],
                result: Some(WitType::S32),
            }],
            imports: vec![],
            records: vec![],
            enums: vec![],
            variants: vec![],
            resources: vec![],
            world_spec: None,
        };
        let got = generate_wit(&world).unwrap();
        assert_eq!(
            got, expected,
            "export_add WIT output does not match expected"
        );
    }

    #[test]
    fn test_wit_accuracy_export_record() {
        let expected =
            include_str!("../../../../tests/fixtures/component/export_record.expected.wit");
        let world = WitWorld {
            name: "export_record".to_string(),
            functions: vec![WitFunction {
                name: "distance_sq".to_string(),
                params: vec![("p".to_string(), WitType::Record("Point".to_string()))],
                result: Some(WitType::S32),
            }],
            imports: vec![],
            records: vec![WitRecord {
                name: "Point".to_string(),
                fields: vec![
                    ("x".to_string(), WitType::S32),
                    ("y".to_string(), WitType::S32),
                ],
            }],
            enums: vec![],
            variants: vec![],
            resources: vec![],
            world_spec: None,
        };
        let got = generate_wit(&world).unwrap();
        assert_eq!(
            got, expected,
            "export_record WIT output does not match expected"
        );
    }

    #[test]
    fn test_wit_accuracy_export_variant() {
        let expected =
            include_str!("../../../../tests/fixtures/component/export_variant.expected.wit");
        let world = WitWorld {
            name: "export_variant".to_string(),
            functions: vec![WitFunction {
                name: "area".to_string(),
                params: vec![("s".to_string(), WitType::Variant("Shape".to_string()))],
                result: Some(WitType::F64),
            }],
            imports: vec![],
            records: vec![],
            enums: vec![],
            variants: vec![WitVariant {
                name: "Shape".to_string(),
                cases: vec![
                    ("Circle".to_string(), Some(WitType::F64)),
                    ("Square".to_string(), Some(WitType::F64)),
                ],
            }],
            resources: vec![],
            world_spec: None,
        };
        let got = generate_wit(&world).unwrap();
        assert_eq!(
            got, expected,
            "export_variant WIT output does not match expected"
        );
    }

    #[test]
    fn test_wit_accuracy_export_enum() {
        let expected =
            include_str!("../../../../tests/fixtures/component/export_enum_wit.expected.wit");
        let world = WitWorld {
            name: "export_enum_wit".to_string(),
            functions: vec![WitFunction {
                name: "color_code".to_string(),
                params: vec![("c".to_string(), WitType::Enum("Color".to_string()))],
                result: Some(WitType::S32),
            }],
            imports: vec![],
            records: vec![],
            enums: vec![WitEnum {
                name: "Color".to_string(),
                variants: vec!["red".to_string(), "green".to_string(), "blue".to_string()],
            }],
            variants: vec![],
            resources: vec![],
            world_spec: None,
        };
        let got = generate_wit(&world).unwrap();
        assert_eq!(
            got, expected,
            "export_enum_wit WIT output does not match expected"
        );
    }

    #[test]
    fn test_wit_accuracy_multi_export() {
        let expected =
            include_str!("../../../../tests/fixtures/component/multi_export.expected.wit");
        let world = WitWorld {
            name: "multi_export".to_string(),
            functions: vec![
                WitFunction {
                    name: "add".to_string(),
                    params: vec![
                        ("a".to_string(), WitType::S32),
                        ("b".to_string(), WitType::S32),
                    ],
                    result: Some(WitType::S32),
                },
                WitFunction {
                    name: "multiply".to_string(),
                    params: vec![
                        ("a".to_string(), WitType::S32),
                        ("b".to_string(), WitType::S32),
                    ],
                    result: Some(WitType::S32),
                },
                WitFunction {
                    name: "negate".to_string(),
                    params: vec![("x".to_string(), WitType::S32)],
                    result: Some(WitType::S32),
                },
            ],
            imports: vec![],
            records: vec![],
            enums: vec![],
            variants: vec![],
            resources: vec![],
            world_spec: None,
        };
        let got = generate_wit(&world).unwrap();
        assert_eq!(
            got, expected,
            "multi_export WIT output does not match expected"
        );
    }

    #[test]
    fn test_wit_accuracy_export_string() {
        let expected =
            include_str!("../../../../tests/fixtures/component/export_string.expected.wit");
        let world = WitWorld {
            name: "export_string".to_string(),
            functions: vec![WitFunction {
                name: "greet".to_string(),
                params: vec![("name".to_string(), WitType::StringType)],
                result: Some(WitType::StringType),
            }],
            imports: vec![],
            records: vec![],
            enums: vec![],
            variants: vec![],
            resources: vec![],
            world_spec: None,
        };
        let got = generate_wit(&world).unwrap();
        assert_eq!(
            got, expected,
            "export_string WIT output does not match expected"
        );
    }

    #[test]
    fn test_wit_accuracy_export_option() {
        let expected =
            include_str!("../../../../tests/fixtures/component/export_option.expected.wit");
        let world = WitWorld {
            name: "export_option".to_string(),
            functions: vec![WitFunction {
                name: "maybe_double".to_string(),
                params: vec![("n".to_string(), WitType::S32)],
                result: Some(WitType::Option(Box::new(WitType::S32))),
            }],
            imports: vec![],
            records: vec![],
            enums: vec![],
            variants: vec![],
            resources: vec![],
            world_spec: None,
        };
        let got = generate_wit(&world).unwrap();
        assert_eq!(
            got, expected,
            "export_option WIT output does not match expected"
        );
    }

    #[test]
    fn test_wit_accuracy_export_result() {
        let expected =
            include_str!("../../../../tests/fixtures/component/export_result.expected.wit");
        let world = WitWorld {
            name: "export_result".to_string(),
            functions: vec![WitFunction {
                name: "safe_div".to_string(),
                params: vec![
                    ("a".to_string(), WitType::S32),
                    ("b".to_string(), WitType::S32),
                ],
                result: Some(WitType::Result {
                    ok: Some(Box::new(WitType::S32)),
                    err: Some(Box::new(WitType::StringType)),
                }),
            }],
            imports: vec![],
            records: vec![],
            enums: vec![],
            variants: vec![],
            resources: vec![],
            world_spec: None,
        };
        let got = generate_wit(&world).unwrap();
        assert_eq!(
            got, expected,
            "export_result WIT output does not match expected"
        );
    }

    #[test]
    fn test_wit_accuracy_export_tuple() {
        let expected =
            include_str!("../../../../tests/fixtures/component/export_tuple.expected.wit");
        let world = WitWorld {
            name: "export_tuple".to_string(),
            functions: vec![WitFunction {
                name: "swap".to_string(),
                params: vec![
                    ("a".to_string(), WitType::S32),
                    ("b".to_string(), WitType::S32),
                ],
                result: Some(WitType::Tuple(vec![WitType::S32, WitType::S32])),
            }],
            imports: vec![],
            records: vec![],
            enums: vec![],
            variants: vec![],
            resources: vec![],
            world_spec: None,
        };
        let got = generate_wit(&world).unwrap();
        assert_eq!(
            got, expected,
            "export_tuple WIT output does not match expected"
        );
    }

    /// Validate that generate_wit produces WIT that parses without error.
    /// Uses the internal wit_parse module (which skips world blocks but validates
    /// that the surrounding package header and overall structure are accepted).
    #[test]
    fn test_generated_wit_is_parseable() {
        use super::super::wit_parse::parse_wit;

        let worlds = vec![
            WitWorld {
                name: "option_world".to_string(),
                functions: vec![WitFunction {
                    name: "maybe_double".to_string(),
                    params: vec![("n".to_string(), WitType::S32)],
                    result: Some(WitType::Option(Box::new(WitType::S32))),
                }],
                imports: vec![],
                records: vec![],
                enums: vec![],
                variants: vec![],
                resources: vec![],
                world_spec: None,
            },
            WitWorld {
                name: "result_world".to_string(),
                functions: vec![WitFunction {
                    name: "safe_div".to_string(),
                    params: vec![
                        ("a".to_string(), WitType::S32),
                        ("b".to_string(), WitType::S32),
                    ],
                    result: Some(WitType::Result {
                        ok: Some(Box::new(WitType::S32)),
                        err: Some(Box::new(WitType::StringType)),
                    }),
                }],
                imports: vec![],
                records: vec![],
                enums: vec![],
                variants: vec![],
                resources: vec![],
                world_spec: None,
            },
            WitWorld {
                name: "record_world".to_string(),
                functions: vec![WitFunction {
                    name: "distance_sq".to_string(),
                    params: vec![("p".to_string(), WitType::Record("Point".to_string()))],
                    result: Some(WitType::S32),
                }],
                imports: vec![],
                records: vec![WitRecord {
                    name: "Point".to_string(),
                    fields: vec![
                        ("x".to_string(), WitType::S32),
                        ("y".to_string(), WitType::S32),
                    ],
                }],
                enums: vec![],
                variants: vec![],
                resources: vec![],
                world_spec: None,
            },
            WitWorld {
                name: "variant_world".to_string(),
                functions: vec![WitFunction {
                    name: "area".to_string(),
                    params: vec![("s".to_string(), WitType::Variant("Shape".to_string()))],
                    result: Some(WitType::F64),
                }],
                imports: vec![],
                records: vec![],
                enums: vec![],
                variants: vec![WitVariant {
                    name: "Shape".to_string(),
                    cases: vec![
                        ("Circle".to_string(), Some(WitType::F64)),
                        ("Square".to_string(), Some(WitType::F64)),
                    ],
                }],
                resources: vec![],
                world_spec: None,
            },
            WitWorld {
                name: "tuple_world".to_string(),
                functions: vec![WitFunction {
                    name: "swap".to_string(),
                    params: vec![
                        ("a".to_string(), WitType::S32),
                        ("b".to_string(), WitType::S32),
                    ],
                    result: Some(WitType::Tuple(vec![WitType::S32, WitType::S32])),
                }],
                imports: vec![],
                records: vec![],
                enums: vec![],
                variants: vec![],
                resources: vec![],
                world_spec: None,
            },
        ];

        for world in &worlds {
            let wit_text = generate_wit(world).unwrap();
            let parse_result = parse_wit(&wit_text);
            assert!(
                parse_result.is_ok(),
                "generated WIT for world '{}' failed to parse: {:?}\n---\n{}",
                world.name,
                parse_result.err(),
                wit_text
            );
        }
    }

    // ── WitError display tests ────────────────────────────────────────────────

    #[test]
    fn test_wit_error_non_exportable_display() {
        let err = WitError::NonExportableType {
            type_name: "MyFunc".to_string(),
            reason: "function types cannot be exported via WIT".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("MyFunc"));
        assert!(msg.contains("cannot be exported via WIT"));
    }

    #[test]
    fn test_wit_error_unknown_world_display() {
        let err = WitError::UnknownWorld {
            spec: "wasi:unknown/world".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("wasi:unknown/world"));
    }

    #[test]
    fn test_wit_accuracy_world_command() {
        let expected = include_str!(
            "../../../../tests/fixtures/component/world_command.expected.wit"
        );
        let spec = parse_world_spec("wasi:cli/command").unwrap();
        let world = WitWorld {
            name: spec.world_name.clone(),
            functions: vec![WitFunction {
                name: "run".to_string(),
                params: vec![],
                result: None,
            }],
            imports: vec![],
            records: vec![],
            enums: vec![],
            variants: vec![],
            resources: vec![],
            world_spec: Some(spec),
        };
        let got = generate_wit(&world).unwrap();
        assert_eq!(
            got, expected,
            "world_command WIT output does not match expected"
        );
    }
}
