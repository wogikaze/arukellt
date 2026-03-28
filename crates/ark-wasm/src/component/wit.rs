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
}

/// Errors during WIT generation.
#[derive(Debug)]
pub enum WitError {
    /// Type cannot be exported via WIT (e.g., closure, raw ref)
    NonExportableType { type_name: String, reason: String },
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

    // Type definitions inside world block
    for record in &world.records {
        writeln!(out, "    record {} {{", to_kebab_case(&record.name)).unwrap();
        for (i, (name, ty)) in record.fields.iter().enumerate() {
            let comma = if i < record.fields.len() - 1 { "," } else { "" };
            writeln!(out, "        {}: {}{}", to_kebab_case(name), ty.to_wit(), comma).unwrap();
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
                    writeln!(out, "        {}({}){}", to_kebab_case(name), ty.to_wit(), comma).unwrap();
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

/// Convert PascalCase/snake_case to kebab-case for WIT identifiers.
fn to_kebab_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch == '_' {
            result.push('-');
        } else if ch.is_uppercase() {
            if i > 0 {
                result.push('-');
            }
            result.push(ch.to_lowercase().next().unwrap());
        } else {
            result.push(ch);
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
}
