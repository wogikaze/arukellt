use std::collections::HashMap;

use ark_diagnostics::Span;

use crate::ids::{BodyId, ExprId, ItemId, ModuleId, PatternId, ProgramId};

#[derive(Debug, Clone, Default)]
pub struct SourceMap {
    pub program_spans: HashMap<ProgramId, Span>,
    pub module_spans: HashMap<ModuleId, Span>,
    pub item_spans: HashMap<ItemId, Span>,
    pub body_spans: HashMap<BodyId, Span>,
    pub expr_spans: HashMap<ExprId, Span>,
    pub pattern_spans: HashMap<PatternId, Span>,
}

impl SourceMap {
    pub fn insert_program(&mut self, id: ProgramId, span: Span) {
        self.program_spans.insert(id, span);
    }

    pub fn insert_module(&mut self, id: ModuleId, span: Span) {
        self.module_spans.insert(id, span);
    }

    pub fn insert_item(&mut self, id: ItemId, span: Span) {
        self.item_spans.insert(id, span);
    }

    pub fn insert_body(&mut self, id: BodyId, span: Span) {
        self.body_spans.insert(id, span);
    }

    pub fn insert_expr(&mut self, id: ExprId, span: Span) {
        self.expr_spans.insert(id, span);
    }

    pub fn insert_pattern(&mut self, id: PatternId, span: Span) {
        self.pattern_spans.insert(id, span);
    }

    pub fn expr_span(&self, id: ExprId) -> Option<Span> {
        self.expr_spans.get(&id).copied()
    }

    pub fn pattern_span(&self, id: PatternId) -> Option<Span> {
        self.pattern_spans.get(&id).copied()
    }
}
