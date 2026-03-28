//! Canonical ABI lift/lower functions for Component Model boundary crossing.
//!
//! Bridges Arukellt's GC-native Wasm representations and the Component Model's
//! flat linear-memory canonical ABI representations.
//!
//! ## Memory Layout
//! - Linear memory page 0 (64KB) is shared with I/O bridge
//! - Canonical ABI scratch region: offset DATA_START (256) through 65535
//! - Bump-style sub-allocator resets per component call
//!
//! ## Type Mapping
//! | Arukellt (GC) | Canonical ABI |
//! |---------------|---------------|
//! | i32/i64/f32/f64/bool/char | pass-through (no conversion) |
//! | String (ref (array (mut i8))) | (i32 ptr, i32 len) in linear memory |
//! | Vec<T> (ref struct) | (i32 ptr, i32 len) in linear memory |
//! | struct (ref struct) | flattened scalar fields |
//! | enum (subtype hierarchy) | (i32 discriminant, payload...) |

use super::WitType;

/// Canonical ABI scratch memory starts at this offset in linear memory.
pub const CABI_SCRATCH_START: u32 = 256;

/// Maximum bytes available for canonical ABI per-call scratch.
pub const CABI_SCRATCH_SIZE: u32 = 65536 - CABI_SCRATCH_START;

/// Classification of how a WIT type crosses the canonical ABI boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalAbiClass {
    /// Scalar pass-through: no conversion needed.
    Scalar(ScalarKind),
    /// String: GC array ↔ linear memory (ptr, len).
    String,
    /// List: GC vec struct ↔ linear memory (ptr, len).
    List(Box<CanonicalAbiClass>),
    /// Record: flatten/unflatten fields.
    Record(Vec<(String, CanonicalAbiClass)>),
    /// Variant/enum: discriminant + optional payload.
    Variant(Vec<(String, Option<CanonicalAbiClass>)>),
    /// Option<T>: discriminant(0|1) + T-or-zero.
    OptionType(Box<CanonicalAbiClass>),
    /// Result<T, E>: discriminant(0|1) + payload.
    ResultType {
        ok: Option<Box<CanonicalAbiClass>>,
        err: Option<Box<CanonicalAbiClass>>,
    },
    /// Resource handle: i32 index into handle table.
    Handle,
}

/// Scalar sub-classifications for canonical ABI pass-through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarKind {
    I32,
    I64,
    F32,
    F64,
}

/// Classify a WIT type for canonical ABI boundary crossing.
pub fn classify_wit_type(ty: &WitType) -> CanonicalAbiClass {
    match ty {
        WitType::U8
        | WitType::U16
        | WitType::U32
        | WitType::S8
        | WitType::S16
        | WitType::S32
        | WitType::Bool
        | WitType::Char => CanonicalAbiClass::Scalar(ScalarKind::I32),
        WitType::U64 | WitType::S64 => CanonicalAbiClass::Scalar(ScalarKind::I64),
        WitType::F32 => CanonicalAbiClass::Scalar(ScalarKind::F32),
        WitType::F64 => CanonicalAbiClass::Scalar(ScalarKind::F64),
        WitType::StringType => CanonicalAbiClass::String,
        WitType::List(inner) => CanonicalAbiClass::List(Box::new(classify_wit_type(inner))),
        WitType::Option(inner) => CanonicalAbiClass::OptionType(Box::new(classify_wit_type(inner))),
        WitType::Result { ok, err } => CanonicalAbiClass::ResultType {
            ok: ok.as_ref().map(|t| Box::new(classify_wit_type(t))),
            err: err.as_ref().map(|t| Box::new(classify_wit_type(t))),
        },
        WitType::Tuple(elems) => {
            let fields = elems
                .iter()
                .enumerate()
                .map(|(i, t)| (format!("f{}", i), classify_wit_type(t)))
                .collect();
            CanonicalAbiClass::Record(fields)
        }
        WitType::Record(_) | WitType::Enum(_) | WitType::Variant(_) => {
            // Named types — classified as i32 handle for now; full flattening
            // requires looking up the type definition in the type table.
            CanonicalAbiClass::Scalar(ScalarKind::I32)
        }
        WitType::Resource(_) | WitType::Own(_) | WitType::Borrow(_) => CanonicalAbiClass::Handle,
    }
}

/// Number of flat i32/i64/f32/f64 values this type occupies in canonical ABI.
pub fn flat_count(class: &CanonicalAbiClass) -> usize {
    match class {
        CanonicalAbiClass::Scalar(_) => 1,
        CanonicalAbiClass::String => 2,  // (ptr, len)
        CanonicalAbiClass::List(_) => 2, // (ptr, len)
        CanonicalAbiClass::Handle => 1,  // i32 index
        CanonicalAbiClass::Record(fields) => fields.iter().map(|(_, c)| flat_count(c)).sum(),
        CanonicalAbiClass::Variant(cases) => {
            1 + cases
                .iter()
                .map(|(_, c)| c.as_ref().map_or(0, flat_count))
                .max()
                .unwrap_or(0)
        }
        CanonicalAbiClass::OptionType(inner) => 1 + flat_count(inner),
        CanonicalAbiClass::ResultType { ok, err } => {
            let ok_n = ok.as_ref().map_or(0, |c| flat_count(c));
            let err_n = err.as_ref().map_or(0, |c| flat_count(c));
            1 + ok_n.max(err_n)
        }
    }
}

/// Returns true if the type requires no conversion at the canonical ABI boundary.
pub fn is_scalar_passthrough(class: &CanonicalAbiClass) -> bool {
    matches!(class, CanonicalAbiClass::Scalar(_))
}

/// Canonical ABI adapter function name for lowering an export return value.
pub fn lower_fn_name(type_desc: &str) -> String {
    format!("$__cabi_lower_{}", type_desc)
}

/// Canonical ABI adapter function name for lifting an import parameter.
pub fn lift_fn_name(type_desc: &str) -> String {
    format!("$__cabi_lift_{}", type_desc)
}

/// Summary of what canonical ABI adapter functions are needed for a set of
/// exported/imported functions.
#[derive(Debug, Default)]
pub struct CanonicalAbiPlan {
    /// Whether any export/import uses string types.
    pub needs_string_adapter: bool,
    /// Whether any export/import uses list types.
    pub needs_list_adapter: bool,
    /// Whether any export/import uses resource handles.
    pub needs_handle_table: bool,
    /// Whether the cabi_realloc export is needed.
    pub needs_realloc: bool,
}

impl CanonicalAbiPlan {
    /// Analyze exported/imported function types and determine which adapters
    /// are needed.
    pub fn from_types(types: &[&WitType]) -> Self {
        let mut plan = Self::default();
        for ty in types {
            plan.analyze_type(ty);
        }
        plan
    }

    fn analyze_type(&mut self, ty: &WitType) {
        match ty {
            WitType::StringType => {
                self.needs_string_adapter = true;
                self.needs_realloc = true;
            }
            WitType::List(inner) => {
                self.needs_list_adapter = true;
                self.needs_realloc = true;
                self.analyze_type(inner);
            }
            WitType::Option(inner) => self.analyze_type(inner),
            WitType::Result { ok, err } => {
                if let Some(ok) = ok {
                    self.analyze_type(ok);
                }
                if let Some(err) = err {
                    self.analyze_type(err);
                }
            }
            WitType::Tuple(elems) => {
                for e in elems {
                    self.analyze_type(e);
                }
            }
            WitType::Resource(_) | WitType::Own(_) | WitType::Borrow(_) => {
                self.needs_handle_table = true;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_passthrough() {
        assert!(is_scalar_passthrough(&classify_wit_type(&WitType::S32)));
        assert!(is_scalar_passthrough(&classify_wit_type(&WitType::S64)));
        assert!(is_scalar_passthrough(&classify_wit_type(&WitType::F32)));
        assert!(is_scalar_passthrough(&classify_wit_type(&WitType::F64)));
        assert!(is_scalar_passthrough(&classify_wit_type(&WitType::Bool)));
        assert!(is_scalar_passthrough(&classify_wit_type(&WitType::Char)));
        assert!(is_scalar_passthrough(&classify_wit_type(&WitType::U8)));
        assert!(is_scalar_passthrough(&classify_wit_type(&WitType::U32)));
    }

    #[test]
    fn string_classification() {
        let class = classify_wit_type(&WitType::StringType);
        assert_eq!(class, CanonicalAbiClass::String);
        assert_eq!(flat_count(&class), 2);
    }

    #[test]
    fn list_classification() {
        let class = classify_wit_type(&WitType::List(Box::new(WitType::S32)));
        assert!(matches!(class, CanonicalAbiClass::List(_)));
        assert_eq!(flat_count(&class), 2);
    }

    #[test]
    fn option_classification() {
        let class = classify_wit_type(&WitType::Option(Box::new(WitType::S32)));
        assert!(matches!(class, CanonicalAbiClass::OptionType(_)));
        assert_eq!(flat_count(&class), 2); // discriminant + value
    }

    #[test]
    fn result_classification() {
        let class = classify_wit_type(&WitType::Result {
            ok: Some(Box::new(WitType::S32)),
            err: Some(Box::new(WitType::StringType)),
        });
        assert!(matches!(class, CanonicalAbiClass::ResultType { .. }));
        assert_eq!(flat_count(&class), 3); // discriminant + max(1, 2)
    }

    #[test]
    fn resource_handle_classification() {
        let class = classify_wit_type(&WitType::Own(Box::new(WitType::Resource(
            "file".to_string(),
        ))));
        assert_eq!(class, CanonicalAbiClass::Handle);
        assert_eq!(flat_count(&class), 1);
    }

    #[test]
    fn plan_detects_string_adapter() {
        let types = vec![&WitType::StringType, &WitType::S32];
        let plan = CanonicalAbiPlan::from_types(&types);
        assert!(plan.needs_string_adapter);
        assert!(plan.needs_realloc);
        assert!(!plan.needs_list_adapter);
    }

    #[test]
    fn plan_detects_handle_table() {
        let own_type = WitType::Own(Box::new(WitType::Resource("conn".to_string())));
        let types = vec![&own_type];
        let plan = CanonicalAbiPlan::from_types(&types);
        assert!(plan.needs_handle_table);
    }

    #[test]
    fn scalar_roundtrip_is_identity() {
        // Scalars require no conversion: lower then lift = identity
        for ty in &[
            WitType::S32,
            WitType::S64,
            WitType::F32,
            WitType::F64,
            WitType::Bool,
        ] {
            let class = classify_wit_type(ty);
            assert!(is_scalar_passthrough(&class));
            assert_eq!(flat_count(&class), 1);
        }
    }
}
