//! Handle table for WIT resource types at the canonical ABI boundary.
//!
//! Resources cross the component boundary as `i32` indices into a handle table.
//! The component maintains the table, mapping indices to internal GC references.
//!
//! ## Design
//!
//! The handle table is implemented as a Wasm `(table anyref)` with a free-list
//! for index recycling. Each resource type gets its own logical partition,
//! but v2 uses a single shared table for simplicity.
//!
//! ### Wasm globals and functions generated:
//! - `$__handle_next: (global (mut i32))` — next free index
//! - `$__handle_table: (table $capacity anyref)` — the handle storage
//! - `$__handle_insert: [anyref] -> [i32]` — insert a GC ref, return handle
//! - `$__handle_get: [i32] -> [anyref]` — look up a handle (borrow semantics)
//! - `$__handle_remove: [i32] -> [anyref]` — remove + return (own semantics)
//! - `$[resource-name].drop: [i32] -> []` — canonical resource.drop

/// Initial handle table capacity (number of slots).
pub const HANDLE_TABLE_INITIAL_CAPACITY: u32 = 64;

/// Descriptor for a resource type that needs handle table support.
#[derive(Debug, Clone)]
pub struct ResourceDescriptor {
    /// Resource name in WIT (kebab-case).
    pub wit_name: String,
    /// Whether this resource is exported (component owns) or imported (host owns).
    pub is_export: bool,
}

/// Plan for handle table generation in the emitter.
#[derive(Debug, Default)]
pub struct HandleTablePlan {
    /// Resource descriptors that need handle table entries.
    pub resources: Vec<ResourceDescriptor>,
}

impl HandleTablePlan {
    /// Returns true if any resource needs handle table support.
    pub fn is_needed(&self) -> bool {
        !self.resources.is_empty()
    }

    /// Names of Wasm functions that must be generated for the handle table.
    pub fn required_functions(&self) -> Vec<String> {
        if !self.is_needed() {
            return Vec::new();
        }
        let mut fns = vec![
            "$__handle_insert".to_string(),
            "$__handle_get".to_string(),
            "$__handle_remove".to_string(),
        ];
        for res in &self.resources {
            fns.push(format!("$__resource_drop_{}", res.wit_name));
        }
        fns
    }

    /// Names of Wasm globals that must be generated.
    pub fn required_globals(&self) -> Vec<String> {
        if !self.is_needed() {
            return Vec::new();
        }
        vec!["$__handle_next".to_string()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_plan_not_needed() {
        let plan = HandleTablePlan::default();
        assert!(!plan.is_needed());
        assert!(plan.required_functions().is_empty());
        assert!(plan.required_globals().is_empty());
    }

    #[test]
    fn single_export_resource() {
        let plan = HandleTablePlan {
            resources: vec![ResourceDescriptor {
                wit_name: "file".to_string(),
                is_export: true,
            }],
        };
        assert!(plan.is_needed());
        let fns = plan.required_functions();
        assert!(fns.contains(&"$__handle_insert".to_string()));
        assert!(fns.contains(&"$__handle_get".to_string()));
        assert!(fns.contains(&"$__handle_remove".to_string()));
        assert!(fns.contains(&"$__resource_drop_file".to_string()));
        assert_eq!(plan.required_globals(), vec!["$__handle_next"]);
    }

    #[test]
    fn multiple_resources() {
        let plan = HandleTablePlan {
            resources: vec![
                ResourceDescriptor {
                    wit_name: "file".to_string(),
                    is_export: true,
                },
                ResourceDescriptor {
                    wit_name: "conn".to_string(),
                    is_export: false,
                },
            ],
        };
        let fns = plan.required_functions();
        assert!(fns.contains(&"$__resource_drop_file".to_string()));
        assert!(fns.contains(&"$__resource_drop_conn".to_string()));
    }
}
