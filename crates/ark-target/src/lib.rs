//! Target registry for the Arukellt compiler.
//!
//! Defines the 5 canonical targets from ADR-007, their profiles,
//! and alias resolution for backward compatibility.

mod plan;

use std::fmt;
use std::str::FromStr;

pub use plan::{
    AbiClass, BackendPlan, EmitCapability, ExportPlan, ImportPlan, LayoutClass, RuntimeModel,
    build_backend_plan, plan_matches_target_profile,
};

/// Canonical target identifiers (ADR-007).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TargetId {
    /// T1: Linear memory + WASI Preview 1 (AtCoder target)
    #[default]
    Wasm32WasiP1,
    /// T2: Wasm GC, no WASI (browser/embedded)
    Wasm32Freestanding,
    /// T3: Wasm GC + WASI Preview 2 + Component Model (canonical target)
    Wasm32WasiP2,
    /// T4: Native via LLVM (subordinate to Wasm semantics)
    Native,
    /// T5: Wasm GC + WASI Preview 3 (future, async-first)
    Wasm32WasiP3,
}

impl TargetId {
    /// All canonical target IDs.
    pub const ALL: &[TargetId] = &[
        TargetId::Wasm32WasiP1,
        TargetId::Wasm32Freestanding,
        TargetId::Wasm32WasiP2,
        TargetId::Native,
        TargetId::Wasm32WasiP3,
    ];

    /// Canonical string name.
    pub fn canonical_name(self) -> &'static str {
        match self {
            TargetId::Wasm32WasiP1 => "wasm32-wasi-p1",
            TargetId::Wasm32Freestanding => "wasm32-freestanding",
            TargetId::Wasm32WasiP2 => "wasm32-wasi-p2",
            TargetId::Native => "native",
            TargetId::Wasm32WasiP3 => "wasm32-wasi-p3",
        }
    }

    /// Short tier label (T1–T5).
    pub fn tier(self) -> &'static str {
        match self {
            TargetId::Wasm32WasiP1 => "T1",
            TargetId::Wasm32Freestanding => "T2",
            TargetId::Wasm32WasiP2 => "T3",
            TargetId::Native => "T4",
            TargetId::Wasm32WasiP3 => "T5",
        }
    }

    /// Get the profile for this target.
    pub fn profile(self) -> TargetProfile {
        match self {
            TargetId::Wasm32WasiP1 => TargetProfile {
                id: self,
                memory_model: MemoryModel::LinearArena,
                wasi_profile: WasiProfile::P1,
                component_model: false,
                abi_surface: AbiSurface::RawWasm,
                implemented: true,
                run_supported: true,
                default_emit_kind: EmitKind::CoreWasm,
                experimental: false,
            },
            TargetId::Wasm32Freestanding => TargetProfile {
                id: self,
                memory_model: MemoryModel::WasmGc,
                wasi_profile: WasiProfile::None,
                component_model: false,
                abi_surface: AbiSurface::RawWasm,
                implemented: false,
                run_supported: false,
                default_emit_kind: EmitKind::CoreWasm,
                experimental: false,
            },
            TargetId::Wasm32WasiP2 => TargetProfile {
                id: self,
                memory_model: MemoryModel::WasmGc,
                wasi_profile: WasiProfile::P2,
                component_model: true,
                abi_surface: AbiSurface::ComponentWit,
                implemented: true,
                run_supported: true,
                default_emit_kind: EmitKind::CoreWasm,
                experimental: false,
            },
            TargetId::Native => TargetProfile {
                id: self,
                memory_model: MemoryModel::Native,
                wasi_profile: WasiProfile::None,
                component_model: false,
                abi_surface: AbiSurface::NativeC,
                implemented: false,
                run_supported: false,
                default_emit_kind: EmitKind::NativeBinary,
                experimental: false,
            },
            TargetId::Wasm32WasiP3 => TargetProfile {
                id: self,
                memory_model: MemoryModel::WasmGc,
                wasi_profile: WasiProfile::P3,
                component_model: true,
                abi_surface: AbiSurface::ComponentWit,
                implemented: false,
                run_supported: false,
                default_emit_kind: EmitKind::Component,
                experimental: false,
            },
        }
    }
}

impl fmt::Display for TargetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.canonical_name())
    }
}

/// Known aliases that map to canonical target IDs.
const ALIASES: &[(&str, TargetId)] = &[
    ("wasm32-wasi", TargetId::Wasm32WasiP1),
    ("wasm-gc", TargetId::Wasm32WasiP2),
    ("wasm-gc-wasi-p2", TargetId::Wasm32WasiP2),
    ("wasm32", TargetId::Wasm32Freestanding),
];

/// Result of parsing a target string.
#[derive(Debug)]
pub enum TargetParseResult {
    /// Matched a canonical name directly.
    Canonical(TargetId),
    /// Matched via a deprecated alias; caller should emit a warning.
    Alias {
        target: TargetId,
        used_alias: String,
    },
}

impl TargetParseResult {
    pub fn target(&self) -> TargetId {
        match self {
            TargetParseResult::Canonical(id) => *id,
            TargetParseResult::Alias { target, .. } => *target,
        }
    }

    pub fn is_alias(&self) -> bool {
        matches!(self, TargetParseResult::Alias { .. })
    }

    /// If this was an alias, return the alias string and canonical name.
    pub fn alias_warning(&self) -> Option<String> {
        self.alias_parts().map(|(used_alias, canonical_name)| {
            format!(
                "warning: target alias `{}` is deprecated, use `{}` instead",
                used_alias, canonical_name
            )
        })
    }

    pub fn alias_parts(&self) -> Option<(&str, &str)> {
        match self {
            TargetParseResult::Alias { target, used_alias } => {
                Some((used_alias.as_str(), target.canonical_name()))
            }
            TargetParseResult::Canonical(_) => None,
        }
    }
}

/// Parse a target string, resolving aliases.
pub fn parse_target(s: &str) -> Result<TargetParseResult, String> {
    // Try canonical names first
    for id in TargetId::ALL {
        if s == id.canonical_name() {
            return Ok(TargetParseResult::Canonical(*id));
        }
    }
    // Try aliases
    for (alias, id) in ALIASES {
        if s == *alias {
            return Ok(TargetParseResult::Alias {
                target: *id,
                used_alias: s.to_string(),
            });
        }
    }
    let available: Vec<&str> = TargetId::ALL.iter().map(|t| t.canonical_name()).collect();
    Err(format!(
        "unknown target `{}`. Available targets: {}",
        s,
        available.join(", ")
    ))
}

impl FromStr for TargetId {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_target(s).map(|r| r.target())
    }
}

/// Target profile describing capabilities and constraints.
#[derive(Clone, Debug)]
pub struct TargetProfile {
    pub id: TargetId,
    pub memory_model: MemoryModel,
    pub wasi_profile: WasiProfile,
    pub component_model: bool,
    pub abi_surface: AbiSurface,
    pub implemented: bool,
    pub run_supported: bool,
    pub default_emit_kind: EmitKind,
    pub experimental: bool,
}

impl TargetProfile {
    /// Human-readable status string for CLI help.
    pub fn status_label(&self) -> &'static str {
        if self.experimental && self.run_supported {
            "experimental, run supported"
        } else if self.experimental && self.implemented {
            "experimental, compile only"
        } else if self.run_supported {
            "implemented, run supported"
        } else if self.implemented {
            "implemented, compile only"
        } else {
            "planned"
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryModel {
    LinearArena,
    WasmGc,
    Native,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WasiProfile {
    None,
    P1,
    P2,
    P3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbiSurface {
    RawWasm,
    ComponentWit,
    NativeC,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmitKind {
    CoreWasm,
    Component,
    Wit,
    NativeBinary,
    All,
}

impl fmt::Display for EmitKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EmitKind::CoreWasm => write!(f, "core-wasm"),
            EmitKind::Component => write!(f, "component"),
            EmitKind::Wit => write!(f, "wit"),
            EmitKind::NativeBinary => write!(f, "native-binary"),
            EmitKind::All => write!(f, "all"),
        }
    }
}

impl FromStr for EmitKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "core-wasm" => Ok(EmitKind::CoreWasm),
            "component" => Ok(EmitKind::Component),
            "wit" => Ok(EmitKind::Wit),
            "native-binary" => Ok(EmitKind::NativeBinary),
            "all" => Ok(EmitKind::All),
            _ => Err(format!(
                "unknown emit kind `{}`. Available: core-wasm, component, wit, all",
                s
            )),
        }
    }
}

/// Format a help table of all targets and their status.
pub fn targets_help() -> String {
    let mut out = String::from("Available targets:\n");
    for id in TargetId::ALL {
        let profile = id.profile();
        let experimental_tag = if profile.experimental {
            " [experimental]"
        } else {
            ""
        };
        out.push_str(&format!(
            "  {} ({}) - {}{} [{}]\n",
            id.canonical_name(),
            id.tier(),
            match id {
                TargetId::Wasm32WasiP1 => "Linear memory + WASI Preview 1",
                TargetId::Wasm32Freestanding => "Wasm GC, no WASI (browser/embedded)",
                TargetId::Wasm32WasiP2 => "Wasm GC + WASI Preview 2 + Component Model",
                TargetId::Native => "Native via LLVM",
                TargetId::Wasm32WasiP3 => "Wasm GC + WASI Preview 3 (future)",
            },
            experimental_tag,
            profile.status_label()
        ));
    }
    out.push_str("\nAliases (deprecated):\n");
    for (alias, id) in ALIASES {
        out.push_str(&format!("  {} -> {}\n", alias, id.canonical_name()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_names_parse() {
        for id in TargetId::ALL {
            let parsed = parse_target(id.canonical_name()).unwrap();
            assert!(matches!(parsed, TargetParseResult::Canonical(_)));
            assert_eq!(parsed.target(), *id);
        }
    }

    #[test]
    fn aliases_resolve() {
        let result = parse_target("wasm-gc").unwrap();
        assert!(result.is_alias());
        assert_eq!(result.target(), TargetId::Wasm32WasiP2);

        let result = parse_target("wasm-gc-wasi-p2").unwrap();
        assert!(result.is_alias());
        assert_eq!(result.target(), TargetId::Wasm32WasiP2);

        let result = parse_target("wasm32-wasi").unwrap();
        assert!(result.is_alias());
        assert_eq!(result.target(), TargetId::Wasm32WasiP1);

        let result = parse_target("wasm32").unwrap();
        assert!(result.is_alias());
        assert_eq!(result.target(), TargetId::Wasm32Freestanding);
    }

    #[test]
    fn unknown_target_error() {
        let result = parse_target("arm64");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown target"));
    }

    #[test]
    fn alias_warning_message() {
        let result = parse_target("wasm-gc").unwrap();
        let warning = result.alias_warning().unwrap();
        assert!(warning.contains("deprecated"));
        assert!(warning.contains("wasm32-wasi-p2"));
    }

    #[test]
    fn profile_fields() {
        let t1 = TargetId::Wasm32WasiP1.profile();
        assert!(t1.implemented);
        assert!(t1.run_supported);
        assert_eq!(t1.memory_model, MemoryModel::LinearArena);
        assert_eq!(t1.default_emit_kind, EmitKind::CoreWasm);

        let t3 = TargetId::Wasm32WasiP2.profile();
        assert!(t3.implemented);
        assert!(!t3.experimental);
        assert!(t3.component_model);
        assert_eq!(t3.memory_model, MemoryModel::WasmGc);
    }

    #[test]
    fn from_str_impl() {
        let id: TargetId = "wasm32-wasi-p1".parse().unwrap();
        assert_eq!(id, TargetId::Wasm32WasiP1);

        // Aliases also work via FromStr
        let id: TargetId = "wasm-gc".parse().unwrap();
        assert_eq!(id, TargetId::Wasm32WasiP2);
    }

    #[test]
    fn default_target() {
        assert_eq!(TargetId::default(), TargetId::Wasm32WasiP1);
    }

    #[test]
    fn display_canonical() {
        assert_eq!(TargetId::Wasm32WasiP1.to_string(), "wasm32-wasi-p1");
        assert_eq!(TargetId::Wasm32WasiP2.to_string(), "wasm32-wasi-p2");
        assert_eq!(TargetId::Native.to_string(), "native");
    }
}
