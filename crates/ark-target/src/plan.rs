use crate::{AbiSurface, EmitKind, MemoryModel, TargetId, TargetProfile, WasiProfile};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeModel {
    T1LinearP1,
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
    pub imports: Vec<ImportPlan>,
    pub exports: Vec<ExportPlan>,
    pub requires_backend_validation: bool,
}

pub fn build_backend_plan(target: TargetId, emit_kind: EmitKind) -> Result<BackendPlan, String> {
    let profile = target.profile();

    if emit_kind == EmitKind::Component {
        return Err(
            "--emit component is not yet implemented. Only core Wasm modules are currently supported. Use --emit core-wasm instead.".to_string(),
        );
    }
    if emit_kind == EmitKind::All {
        return Err(
            "--emit all is not yet supported because component model output is not implemented. Use --emit core-wasm instead.".to_string(),
        );
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
        TargetId::Wasm32WasiP2 => (
            RuntimeModel::T3WasmGcP2,
            match emit_kind {
                EmitKind::Wit => EmitCapability::Wit,
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
        RuntimeModel::T1LinearP1 | RuntimeModel::T3WasmGcP2 => {
            imports.push(ImportPlan {
                module: "wasi_snapshot_preview1".to_string(),
                name: "fd_write".to_string(),
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
        let plan = build_backend_plan(TargetId::Wasm32WasiP1, EmitKind::CoreWasm).unwrap();
        assert_eq!(plan.runtime_model, RuntimeModel::T1LinearP1);
        assert_eq!(plan.capability, EmitCapability::CoreWasm);
        assert!(plan_matches_target_profile(&plan));
    }

    #[test]
    fn t3_core_wasm_plan_uses_completed_runtime() {
        let plan = build_backend_plan(TargetId::Wasm32WasiP2, EmitKind::CoreWasm).unwrap();
        assert_eq!(plan.runtime_model, RuntimeModel::T3WasmGcP2);
        assert!(plan_matches_target_profile(&plan));
    }

    #[test]
    fn component_emit_is_rejected_at_plan_creation() {
        let err = build_backend_plan(TargetId::Wasm32WasiP2, EmitKind::Component).unwrap_err();
        assert!(err.contains("--emit component"));
    }
}
