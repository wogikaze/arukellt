use std::fmt;

use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum Type {
    Int,
    Bool,
    String,
    Unit,
    List(Box<Type>),
    Seq(Box<Type>),
    Option(Box<Type>),
    Result(Box<Type>, Box<Type>),
    Fn(Box<Type>, Box<Type>),
    Tuple(Vec<Type>),
    Record(Vec<(String, Type)>),
    Named(String),
    Unknown,
}

impl Type {
    #[must_use]
    pub fn from_name(name: &str) -> Self {
        match name {
            "Int" | "i64" => Self::Int,
            "Bool" => Self::Bool,
            "String" => Self::String,
            "Unit" => Self::Unit,
            other => Self::Named(other.to_owned()),
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int => formatter.write_str("Int"),
            Self::Bool => formatter.write_str("Bool"),
            Self::String => formatter.write_str("String"),
            Self::Unit => formatter.write_str("Unit"),
            Self::List(inner) => write!(formatter, "List<{inner}>"),
            Self::Seq(inner) => write!(formatter, "Seq<{inner}>"),
            Self::Option(inner) => write!(formatter, "Option<{inner}>"),
            Self::Result(ok, err) => write!(formatter, "Result<{ok}, {err}>"),
            Self::Fn(arg, result) => write!(formatter, "Fn<{arg}, {result}>"),
            Self::Tuple(items) => {
                let rendered = items
                    .iter()
                    .map(Self::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(formatter, "({rendered})")
            }
            Self::Record(fields) => {
                let rendered = fields
                    .iter()
                    .map(|(name, ty)| format!("{name}: {ty}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(formatter, "{{{rendered}}}")
            }
            Self::Named(name) => formatter.write_str(name),
            Self::Unknown => formatter.write_str("Unknown"),
        }
    }
}
