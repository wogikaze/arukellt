use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ark_hir::Program;
use ark_parser::ast;
use ark_resolve::{ResolvedModule, ResolvedProgram};
use ark_target::TargetId;
use ark_typecheck::CheckOutput;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase {
    Parse,
    Bind,
    Load,
    Analyze,
    Resolve,
    CheckCoreHir,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PhaseKey {
    pub source_hash: u64,
    pub target_key: String,
    pub feature_key: String,
    pub optimization_profile: String,
}

impl PhaseKey {
    pub fn for_path(path: &Path, source: &str, config: &PipelineConfig) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        path.hash(&mut hasher);
        source.hash(&mut hasher);
        config.target_key.hash(&mut hasher);
        config.feature_key.hash(&mut hasher);
        config.optimization_profile.hash(&mut hasher);
        let source_hash = hasher.finish();
        Self {
            source_hash,
            target_key: config.target_key.clone(),
            feature_key: config.feature_key.clone(),
            optimization_profile: config.optimization_profile.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub target_key: String,
    pub feature_key: String,
    pub optimization_profile: String,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            target_key: TargetId::Wasm32WasiP1.canonical_name().to_string(),
            feature_key: "default".to_string(),
            optimization_profile: "default".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BoundArtifact {
    pub module: ast::Module,
}

#[derive(Debug, Clone)]
pub struct LoadArtifact {
    pub program: Option<ResolvedProgram>,
}

#[derive(Debug, Clone)]
pub struct AnalyzeArtifact {
    pub program: Option<ResolvedProgram>,
}

#[derive(Debug, Clone)]
pub struct ResolveArtifact {
    pub resolved: ResolvedModule,
}

#[derive(Debug, Clone)]
pub struct CoreHirArtifact {
    pub output: CheckOutput,
}

#[derive(Debug, Default)]
pub struct ArtifactStore {
    pub parse: HashMap<PhaseKey, ast::Module>,
    pub bind: HashMap<PhaseKey, BoundArtifact>,
    pub load: HashMap<PhaseKey, LoadArtifact>,
    pub analyze: HashMap<PhaseKey, AnalyzeArtifact>,
    pub resolve: HashMap<PhaseKey, ResolveArtifact>,
    pub core_hir: HashMap<PhaseKey, CoreHirArtifact>,
    pub path_keys: HashMap<PathBuf, PhaseKey>,
}

impl ArtifactStore {
    pub fn remember_key(&mut self, path: PathBuf, key: PhaseKey) {
        self.path_keys.insert(path, key);
    }

    pub fn key_for_path(&self, path: &Path) -> Option<&PhaseKey> {
        self.path_keys.get(path)
    }
}

pub fn _program_ref(program: &Program) -> &Program {
    program
}
