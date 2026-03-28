//! Function definition checking.

use ark_diagnostics::DiagnosticSink;
use ark_parser::ast;

use crate::types::Type;

use super::{TypeChecker, TypeEnv};

impl TypeChecker {
    pub(crate) fn check_function(&mut self, f: &ast::FnDef, sink: &mut DiagnosticSink) {
        let mut env = TypeEnv::new();

        // Bind parameters
        for param in &f.params {
            let ty = self.resolve_type_expr(&param.ty);
            env.bind(param.name.clone(), ty);
        }

        // Bind type parameters
        for tp in &f.type_params {
            env.bind(tp.clone(), self.fresh_type_var());
        }

        let expected_ret = f
            .return_type
            .as_ref()
            .map(|t| self.resolve_type_expr(t))
            .unwrap_or(Type::Unit);

        self.current_fn_return_type = Some(expected_ret.clone());

        // Check body block
        let _body_type = self.check_block(&f.body, &mut env, &expected_ret, sink);

        self.current_fn_return_type = None;
    }
}
