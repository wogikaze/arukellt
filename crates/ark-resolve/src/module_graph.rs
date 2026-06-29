use std::collections::HashMap;
use std::path::PathBuf;

use ark_parser::ast;

use crate::resolve::LoadedModule;

#[derive(Debug, Clone)]
pub struct ModuleGraph {
    pub entry_module: ast::Module,
    pub loaded: HashMap<PathBuf, LoadedModule>,
    pub _std_root: PathBuf,
}
