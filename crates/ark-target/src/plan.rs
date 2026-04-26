use crate::{AbiSurface, EmitKind, MemoryModel, TargetId, TargetProfile, WasiProfile, WasiVersion};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeModel {
    T1LinearP1,
    /// Minimal T2 scaffold: core Wasm only, no WASI, no runner support.
    T2Freestanding,
    /// Completed T3: WasmGC types + linear-memory bridge I/O via P1 fd_write.
    T3WasmGcP2,
    T4LlvmScaffold,
}

impl RuntimeModel {
    /// Whether this model represents completed T3.
    pub fn is_t3_complete(&self) -> bool {
        matches!(self, RuntimeModel::T3WasmGcP2)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AbiClass {
    Scalar,
    LinearMemoryPointer,
    WasmGcRef,
    NativePointer,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayoutClass {
    Scalar,
    Aggregate,
    Dynamic,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EmitCapability {
    CoreWasm,
    Wit,
    /// Component Model: core wasm + WIT → component binary via wasm-tools
    Component,
    NativeBinary,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportPlan {
    pub module: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportPlan {
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct BackendPlan {
    pub target: TargetId,
    pub emit_kind: EmitKind,
    pub runtime_model: RuntimeModel,
    pub abi_class: AbiClass,
    pub layout_class: LayoutClass,
    pub capability: EmitCapability,
    pub profile: TargetProfile,
    pub wasi_version: WasiVersion,
    pub imports: Vec<ImportPlan>,
    pub exports: Vec<ExportPlan>,
    pub requires_backend_validation: bool,
}

pub fn build_backend_plan(
    target: TargetId,
    emit_kind: EmitKind,
    wasi_version: WasiVersion,
) -> Result<BackendPlan, String> {
    let profile = target.profile();

    // Component emit requires a target with component_model support
    if (emit_kind == EmitKind::Component || emit_kind == EmitKind::All) && !profile.component_model
    {
        return Err(format!(
            "error: component model requires --target wasm32-wasi-p2 (target `{}` does not support components)",
            target
        ));
    }

    let (runtime_model, capability, abi_class, layout_class) = match target {
        TargetId::Wasm32WasiP1 => (
            RuntimeModel::T1LinearP1,
            match emit_kind {
                EmitKind::Wit => EmitCapability::Wit,
                _ => EmitCapability::CoreWasm,
            },
            AbiClass::LinearMemoryPointer,
            LayoutClass::Dynamic,
        ),
        TargetId::Wasm32Freestanding => {
            if emit_kind != EmitKind::CoreWasm {
                return Err(format!(
                    "target `{}` ({}) currently supports only core-wasm via the T2 scaffold",
                    target,
                    target.tier()
                ));
            }
            (
                RuntimeModel::T2Freestanding,
                EmitCapability::CoreWasm,
                AbiClass::WasmGcRef,
                LayoutClass::Dynamic,
            )
        }
        TargetId::Wasm32WasiP2 => (
            RuntimeModel::T3WasmGcP2,
            match emit_kind {
                EmitKind::Wit => EmitCapability::Wit,
                EmitKind::Component | EmitKind::All => EmitCapability::Component,
                _ => EmitCapability::CoreWasm,
            },
            AbiClass::WasmGcRef,
            LayoutClass::Dynamic,
        ),
        TargetId::Native => (
            RuntimeModel::T4LlvmScaffold,
            EmitCapability::NativeBinary,
            AbiClass::NativePointer,
            LayoutClass::Aggregate,
        ),
        other => {
            return Err(format!(
                "target `{}` ({}) is not yet implemented [{}]",
                other,
                other.tier(),
                other.profile().status_label()
            ));
        }
    };

    if matches!(target, TargetId::Wasm32WasiP1) && emit_kind == EmitKind::Wit {
        return Err(format!(
            "target `{}` ({}) does not support WIT generation. Use `--target wasm32-wasi-p2` for WIT support.",
            target,
            target.tier()
        ));
    }

    if !profile.implemented && target != TargetId::Native {
        return Err(format!(
            "target `{}` ({}) is not yet implemented [{}]",
            target,
            target.tier(),
            profile.status_label()
        ));
    }

    let mut imports = Vec::new();
    let mut exports = vec![ExportPlan {
        name: "_start".to_string(),
    }];

    match runtime_model {
        RuntimeModel::T1LinearP1 => {
            // T1: always WASI Preview 1
            imports.push(ImportPlan {
                module: "wasi_snapshot_preview1".to_string(),
                name: "fd_write".to_string(),
            });
        }
        RuntimeModel::T3WasmGcP2 => {
            // T3: branch on wasi_version (#510)
            if wasi_version == WasiVersion::P2 {
                // P2-native: WASI Preview 2 interface import names
                imports.push(ImportPlan {
                    module: "wasi:cli/stdout@0.2.0".to_string(),
                    name: "write".to_string(),
                });
                imports.push(ImportPlan {
                    module: "wasi:cli/environment@0.2.0".to_string(),
                    name: "args-sizes".to_string(),
                });
                imports.push(ImportPlan {
                    module: "wasi:cli/environment@0.2.0".to_string(),
                    name: "arguments".to_string(),
                });
                imports.push(ImportPlan {
                    module: "wasi:filesystem/types@0.2.0".to_string(),
                    name: "open-at".to_string(),
                });
                imports.push(ImportPlan {
                    module: "wasi:cli/stdin@0.2.0".to_string(),
                    name: "read".to_string(),
                });
                imports.push(ImportPlan {
                    module: "wasi:filesystem/types@0.2.0".to_string(),
                    name: "close".to_string(),
                });
                imports.push(ImportPlan {
                    module: "wasi:cli/exit@0.2.0".to_string(),
                    name: "exit".to_string(),
                });
            } else {
                // P1 fallback for T3 (wasi_snapshot_preview1, unchanged)
                imports.push(ImportPlan {
                    module: "wasi_snapshot_preview1".to_string(),
                    name: "fd_write".to_string(),
                });
            }
        }
        RuntimeModel::T2Freestanding => {
            exports.push(ExportPlan {
                name: "memory".to_string(),
            });
        }
        RuntimeModel::T4LlvmScaffold => {
            exports.clear();
            exports.push(ExportPlan {
                name: "main".to_string(),
            });
        }
    }

    Ok(BackendPlan {
        target,
        emit_kind,
        runtime_model,
        abi_class,
        layout_class,
        capability,
        profile,
        wasi_version,
        imports,
        exports,
        requires_backend_validation: !matches!(emit_kind, EmitKind::Wit),
    })
}

pub fn plan_matches_target_profile(plan: &BackendPlan) -> bool {
    match plan.runtime_model {
        RuntimeModel::T1LinearP1 => {
            plan.profile.memory_model == MemoryModel::LinearArena
                && plan.profile.wasi_profile == WasiProfile::P1
                && plan.profile.abi_surface == AbiSurface::RawWasm
        }
        RuntimeModel::T2Freestanding => {
            plan.profile.memory_model == MemoryModel::WasmGc
                && plan.profile.wasi_profile == WasiProfile::None
                && !plan.profile.run_supported
        }
        RuntimeModel::T3WasmGcP2 => {
            plan.profile.memory_model == MemoryModel::WasmGc
                && plan.profile.wasi_profile == WasiProfile::P2
        }
        RuntimeModel::T4LlvmScaffold => {
            plan.profile.memory_model == MemoryModel::Native
                && plan.profile.wasi_profile == WasiProfile::None
                && plan.profile.abi_surface == AbiSurface::NativeC
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t1_core_wasm_plan_uses_linear_runtime() {
        let plan = build_backend_plan(TargetId::Wasm32WasiP1, EmitKind::CoreWasm, WasiVersion::P1)
            .unwrap();
        assert_eq!(plan.runtime_model, RuntimeModel::T1LinearP1);
        assert_eq!(plan.capability, EmitCapability::CoreWasm);
        assert!(plan_matches_target_profile(&plan));
    }

    #[test]
    fn t3_core_wasm_plan_uses_completed_runtime() {
        let plan = build_backend_plan(TargetId::Wasm32WasiP2, EmitKind::CoreWasm, WasiVersion::P1)
            .unwrap();
        assert_eq!(plan.runtime_model, RuntimeModel::T3WasmGcP2);
        assert!(plan_matches_target_profile(&plan));
    }

    #[test]
    fn t2_core_wasm_plan_uses_scaffold_runtime() {
        let plan = build_backend_plan(
            TargetId::Wasm32Freestanding,
            EmitKind::CoreWasm,
            WasiVersion::P1,
        )
        .unwrap();
        assert_eq!(plan.runtime_model, RuntimeModel::T2Freestanding);
        assert!(plan.exports.iter().any(|export| export.name == "memory"));
        assert!(plan_matches_target_profile(&plan));
    }

    #[test]
    fn component_emit_plan_uses_component_capability() {
        let plan = build_backend_plan(TargetId::Wasm32WasiP2, EmitKind::Component, WasiVersion::P1)
            .unwrap();
        assert_eq!(plan.capability, EmitCapability::Component);
        assert_eq!(plan.runtime_model, RuntimeModel::T3WasmGcP2);
        assert!(plan_matches_target_profile(&plan));
    }

    #[test]
    fn component_emit_rejected_for_t1() {
        let err = build_backend_plan(TargetId::Wasm32WasiP1, EmitKind::Component, WasiVersion::P1)
            .unwrap_err();
        assert!(err.contains("component model requires"));
    }
}
