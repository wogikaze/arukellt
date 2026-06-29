//! Type representation for Arukellt.

use std::fmt;

/// Unique type identifier for struct/enum definitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeId(pub u32);

/// The type representation used during type checking.
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    // Primitives
    I32,
    I64,
    F32,
    F64,
    Bool,
    Char,
    Unit,

    // Extended scalar types (v3 #040)
    // Unsigned: Wasm has no native unsigned types — these use i32/i64 with masking
    U8,  // i32 with 0xFF mask
    U16, // i32 with 0xFFFF mask
    U32, // i32 (unsigned div/rem/cmp)
    U64, // i64 (unsigned div/rem/cmp)
    // Signed narrow: i32 with sign-extension
    I8,  // i32 with sign-extend from 8-bit
    I16, // i32 with sign-extend from 16-bit

    // String (built-in reference type)
    String,

    // Compound
    Struct(TypeId),
    Enum(TypeId),
    Tuple(Vec<Type>),
    Array(Box<Type>, u64),        // [T; N]
    Slice(Box<Type>),             // [T]
    Vec(Box<Type>),               // Vec<T>
    Option(Box<Type>),            // Option<T>
    Result(Box<Type>, Box<Type>), // Result<T, E>

    // Function type
    Function { params: Vec<Type>, ret: Box<Type> },

    // Inference
    TypeVar(u32), // unresolved type variable

    // Special
    Never, // diverging (return, panic, break)
    Error, // error recovery sentinel

    // Polymorphic (generic type param erased to anyref at Wasm level)
    Any,
}

impl Type {
    /// Is this a numeric type?
    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            Type::I32
                | Type::I64
                | Type::F32
                | Type::F64
                | Type::U8
                | Type::U16
                | Type::U32
                | Type::U64
                | Type::I8
                | Type::I16
        )
    }

    /// Is this an integer type?
    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            Type::I32
                | Type::I64
                | Type::U8
                | Type::U16
                | Type::U32
                | Type::U64
                | Type::I8
                | Type::I16
        )
    }

    /// Is this an unsigned integer type?
    pub fn is_unsigned(&self) -> bool {
        matches!(self, Type::U8 | Type::U16 | Type::U32 | Type::U64)
    }

    /// Is this a float type?
    pub fn is_float(&self) -> bool {
        matches!(self, Type::F32 | Type::F64)
    }

    /// Is this a reference type (GC-managed)?
    pub fn is_reference(&self) -> bool {
        matches!(
            self,
            Type::String
                | Type::Struct(_)
                | Type::Enum(_)
                | Type::Vec(_)
                | Type::Slice(_)
                | Type::Option(_)
                | Type::Result(_, _)
                | Type::Function { .. }
        )
    }

    /// Is this a value type (copied on assignment)?
    pub fn is_value(&self) -> bool {
        matches!(
            self,
            Type::I32
                | Type::I64
                | Type::F32
                | Type::F64
                | Type::U8
                | Type::U16
                | Type::U32
                | Type::U64
                | Type::I8
                | Type::I16
                | Type::Bool
                | Type::Char
                | Type::Unit
                | Type::Tuple(_)
                | Type::Array(_, _)
        )
    }

    /// Returns the Wasm-level width of this scalar type.
    /// U8/U16/U32/I8/I16 → I32 (all narrow to i32 in Wasm)
    /// U64 → I64
    /// F32/F64 → themselves
    pub fn wasm_width(&self) -> Type {
        match self {
            Type::U8 | Type::U16 | Type::U32 | Type::I8 | Type::I16 => Type::I32,
            Type::U64 => Type::I64,
            _ => self.clone(),
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::I32 => write!(f, "i32"),
            Type::I64 => write!(f, "i64"),
            Type::F32 => write!(f, "f32"),
            Type::F64 => write!(f, "f64"),
            Type::U8 => write!(f, "u8"),
            Type::U16 => write!(f, "u16"),
            Type::U32 => write!(f, "u32"),
            Type::U64 => write!(f, "u64"),
            Type::I8 => write!(f, "i8"),
            Type::I16 => write!(f, "i16"),
            Type::Bool => write!(f, "bool"),
            Type::Char => write!(f, "char"),
            Type::Unit => write!(f, "()"),
            Type::String => write!(f, "String"),
            Type::Struct(id) => write!(f, "struct#{}", id.0),
            Type::Enum(id) => write!(f, "enum#{}", id.0),
            Type::Tuple(types) => {
                write!(f, "(")?;
                for (i, t) in types.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", t)?;
                }
                write!(f, ")")
            }
            Type::Array(elem, size) => write!(f, "[{}; {}]", elem, size),
            Type::Slice(elem) => write!(f, "[{}]", elem),
            Type::Vec(elem) => write!(f, "Vec<{}>", elem),
            Type::Option(inner) => write!(f, "Option<{}>", inner),
            Type::Result(ok, err) => write!(f, "Result<{}, {}>", ok, err),
            Type::Function { params, ret } => {
                write!(f, "fn(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, ") -> {}", ret)
            }
            Type::TypeVar(id) => write!(f, "?T{}", id),
            Type::Never => write!(f, "!"),
            Type::Error => write!(f, "<error>"),
            Type::Any => write!(f, "any"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_display() {
        assert_eq!(Type::I32.to_string(), "i32");
        assert_eq!(Type::Vec(Box::new(Type::I32)).to_string(), "Vec<i32>");
        assert_eq!(
            Type::Result(Box::new(Type::String), Box::new(Type::I32)).to_string(),
            "Result<String, i32>"
        );
    }

    #[test]
    fn test_type_categories() {
        assert!(Type::I32.is_numeric());
        assert!(Type::I32.is_integer());
        assert!(Type::F64.is_float());
        assert!(Type::String.is_reference());
        assert!(Type::I32.is_value());
        assert!(!Type::String.is_value());
        // Extended scalar types
        assert!(Type::U8.is_numeric());
        assert!(Type::U8.is_integer());
        assert!(Type::U8.is_unsigned());
        assert!(Type::U32.is_unsigned());
        assert!(Type::U64.is_unsigned());
        assert!(!Type::I32.is_unsigned());
        assert!(Type::I8.is_integer());
        assert!(Type::I16.is_integer());
        assert!(Type::U8.is_value());
        assert!(Type::U64.is_value());
        assert_eq!(Type::U8.wasm_width(), Type::I32);
        assert_eq!(Type::U16.wasm_width(), Type::I32);
        assert_eq!(Type::U32.wasm_width(), Type::I32);
        assert_eq!(Type::U64.wasm_width(), Type::I64);
        assert_eq!(Type::I8.wasm_width(), Type::I32);
        assert_eq!(Type::I16.wasm_width(), Type::I32);
    }
}
