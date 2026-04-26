//! Typed AST node ID system.
//!
//! Provides stable expression and statement identifiers that are independent
//! of source span positions, enabling robust type-info lookups even when
//! AST transforms or prelude synthesis shift spans.

use std::collections::HashMap;

use crate::types::Type;

/// Stable expression identifier, independent of span position.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ExprId(pub u32);

/// Stable statement identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StmtId(pub u32);

/// Allocator for typed AST node IDs.
#[derive(Debug)]
pub struct NodeIdAllocator {
    next_expr: u32,
    #[allow(dead_code)]
    next_stmt: u32,
}

impl NodeIdAllocator {
    pub fn new() -> Self {
        Self {
            next_expr: 0,
            next_stmt: 0,
        }
    }

    pub fn fresh_expr(&mut self) -> ExprId {
        let id = ExprId(self.next_expr);
        self.next_expr += 1;
        id
    }

    #[allow(dead_code)]
    pub fn fresh_stmt(&mut self) -> StmtId {
        let id = StmtId(self.next_stmt);
        self.next_stmt += 1;
        id
    }
}

impl Default for NodeIdAllocator {
    fn default() -> Self {
        Self::new()
    }
}

/// Per-expression type and resolution info, keyed by `ExprId`.
#[derive(Debug)]
pub struct TypedExprInfo {
    pub ty: Type,
    pub method_resolution: Option<(String, String)>,
}

/// Registry mapping `ExprId` → type/resolution info.
#[derive(Debug)]
pub struct TypedAstMap {
    exprs: HashMap<ExprId, TypedExprInfo>,
    /// Maps span.start → ExprId for transition period.
    span_to_expr: HashMap<u32, ExprId>,
}

impl TypedAstMap {
    pub fn new() -> Self {
        Self {
            exprs: HashMap::new(),
            span_to_expr: HashMap::new(),
        }
    }

    pub fn insert_expr(&mut self, id: ExprId, info: TypedExprInfo) {
        self.exprs.insert(id, info);
    }

    pub fn register_span(&mut self, span_start: u32, id: ExprId) {
        self.span_to_expr.insert(span_start, id);
    }

    pub fn expr_info(&self, id: ExprId) -> Option<&TypedExprInfo> {
        self.exprs.get(&id)
    }

    pub fn expr_by_span(&self, span_start: u32) -> Option<ExprId> {
        self.span_to_expr.get(&span_start).copied()
    }
}

impl Default for TypedAstMap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Type;

    #[test]
    fn allocator_produces_unique_ids() {
        let mut alloc = NodeIdAllocator::new();
        let e0 = alloc.fresh_expr();
        let e1 = alloc.fresh_expr();
        let s0 = alloc.fresh_stmt();
        assert_ne!(e0, e1);
        assert_eq!(e0, ExprId(0));
        assert_eq!(e1, ExprId(1));
        assert_eq!(s0, StmtId(0));
    }

    #[test]
    fn typed_ast_map_roundtrip() {
        let mut alloc = NodeIdAllocator::new();
        let mut map = TypedAstMap::new();

        let id = alloc.fresh_expr();
        let info = TypedExprInfo {
            ty: Type::I32,
            method_resolution: Some(("Vec__push".into(), "Vec".into())),
        };

        map.register_span(42, id);
        map.insert_expr(id, info);

        let resolved = map.expr_by_span(42).unwrap();
        assert_eq!(resolved, id);

        let info = map.expr_info(resolved).unwrap();
        assert_eq!(info.ty, Type::I32);
        assert_eq!(
            info.method_resolution,
            Some(("Vec__push".into(), "Vec".into()))
        );
    }

    #[test]
    fn missing_span_returns_none() {
        let map = TypedAstMap::new();
        assert!(map.expr_by_span(999).is_none());
    }
}
