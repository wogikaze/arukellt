//! Type inference logic: type resolution and compatibility.

use ark_parser::ast;

use crate::types::Type;

use super::TypeChecker;

impl TypeChecker {
    /// Resolve a type expression to a Type.
    pub(crate) fn suffix_to_type(&self, suffix: &str) -> Type {
        match suffix {
            "i32" => Type::I32,
            "i64" => Type::I64,
            "f32" => Type::F32,
            "f64" => Type::F64,
            "u8" => Type::U8,
            "u16" => Type::U16,
            "u32" => Type::U32,
            "u64" => Type::U64,
            "i8" => Type::I8,
            "i16" => Type::I16,
            _ => Type::Error,
        }
    }

    pub fn resolve_type_expr(&self, ty: &ast::TypeExpr) -> Type {
        match ty {
            ast::TypeExpr::Named { name, .. } => match name.as_str() {
                "i32" => Type::I32,
                "i64" => Type::I64,
                "f32" => Type::F32,
                "f64" => Type::F64,
                "u8" => Type::U8,
                "u16" => Type::U16,
                "u32" => Type::U32,
                "u64" => Type::U64,
                "i8" => Type::I8,
                "i16" => Type::I16,
                "bool" => Type::Bool,
                "char" => Type::Char,
                "String" => Type::String,
                _ => {
                    if let Some(info) = self.struct_defs.get(name) {
                        Type::Struct(info.type_id)
                    } else if let Some(info) = self.enum_defs.get(name) {
                        Type::Enum(info.type_id)
                    } else {
                        Type::Error
                    }
                }
            },
            ast::TypeExpr::Generic { name, args, .. } => {
                let resolved_args: Vec<Type> =
                    args.iter().map(|a| self.resolve_type_expr(a)).collect();
                match name.as_str() {
                    "Vec" if resolved_args.len() == 1 => {
                        Type::Vec(Box::new(resolved_args[0].clone()))
                    }
                    "Option" if resolved_args.len() == 1 => {
                        Type::Option(Box::new(resolved_args[0].clone()))
                    }
                    "Result" if resolved_args.len() == 2 => Type::Result(
                        Box::new(resolved_args[0].clone()),
                        Box::new(resolved_args[1].clone()),
                    ),
                    "Box" if resolved_args.len() == 1 => Type::I32, // Box is a pointer
                    _ => Type::Error,
                }
            }
            ast::TypeExpr::Tuple(types, _) => {
                Type::Tuple(types.iter().map(|t| self.resolve_type_expr(t)).collect())
            }
            ast::TypeExpr::Array { elem, size, .. } => {
                Type::Array(Box::new(self.resolve_type_expr(elem)), *size)
            }
            ast::TypeExpr::Slice { elem, .. } => {
                Type::Slice(Box::new(self.resolve_type_expr(elem)))
            }
            ast::TypeExpr::Function { params, ret, .. } => Type::Function {
                params: params.iter().map(|p| self.resolve_type_expr(p)).collect(),
                ret: Box::new(self.resolve_type_expr(ret)),
            },
            ast::TypeExpr::Unit(_) => Type::Unit,
            ast::TypeExpr::Qualified { .. } => {
                // e.g., io.Capabilities — treated as opaque for now
                Type::Error
            }
        }
    }

    /// Get the user-visible name of a type (e.g., "AppError" instead of "enum#3").
    pub(crate) fn type_name(&self, ty: &Type) -> String {
        match ty {
            Type::Struct(id) => self
                .struct_defs
                .iter()
                .find(|(_, info)| info.type_id == *id)
                .map(|(name, _)| name.clone())
                .unwrap_or_else(|| format!("{}", ty)),
            Type::Enum(id) => self
                .enum_defs
                .iter()
                .find(|(_, info)| info.type_id == *id)
                .map(|(name, _)| name.clone())
                .unwrap_or_else(|| format!("{}", ty)),
            _ => format!("{}", ty),
        }
    }

    pub(crate) fn types_compatible(&self, a: &Type, b: &Type) -> bool {
        if *a == Type::Error || *b == Type::Error || *a == Type::Never || *b == Type::Never {
            return true;
        }
        if a == b {
            return true;
        }
        // Generic enum types (Option<T>, Result<T,E>) are represented as
        // Option/Result/Enum variants. Constructors produce I32 (pointer).
        // Structs are also heap pointers (i32). Be lenient for these compound types.
        if matches!(
            a,
            Type::Enum(_) | Type::Option(_) | Type::Result(_, _) | Type::Vec(_) | Type::Struct(_)
        ) || matches!(
            b,
            Type::Enum(_) | Type::Option(_) | Type::Result(_, _) | Type::Vec(_) | Type::Struct(_)
        ) {
            return true;
        }
        false
    }
}
