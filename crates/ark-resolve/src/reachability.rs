//! Conservative reachability for lazy multi-module resolve.
//!
//! Entry seeds are `main` when present, otherwise every `pub fn` in the entry
//! module (falling back to all entry `fn` items if there are no public fns).
//!
//! Limitations (intentional for this slice):
//! - Only callees `f(...)` in the same module and `mod::f(...)` for loaded
//!   modules are modeled. Unqualified calls through `use` imports, method
//!   calls, closures, and indirect calls are not traced yet.

use std::collections::{HashMap, HashSet, VecDeque};

use ark_parser::ast::{
    self, Block, Expr, ForIter, Item, MatchArm, Pattern, Stmt, TypeExpr,
};

use crate::module_graph::ModuleGraph;
use crate::resolve::LoadedModule;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum FnSlot {
    Entry(String),
    Loaded { module: String, name: String },
}

#[derive(Debug, Default, Clone)]
pub(crate) struct Reachability {
    pub entry_fns: HashSet<String>,
    pub entry_types: HashSet<String>,
    pub loaded_fns: HashSet<(String, String)>,
    pub loaded_types: HashSet<(String, String)>,
    pub needed_modules: HashSet<String>,
}

impl Reachability {
    pub(crate) fn include_entry_item(&self, item: &Item) -> bool {
        match item {
            Item::FnDef(f) => self.entry_fns.contains(&f.name),
            Item::StructDef(s) => self.entry_types.contains(&s.name),
            Item::EnumDef(e) => self.entry_types.contains(&e.name),
            Item::TraitDef(t) => self.entry_types.contains(&t.name),
            Item::ImplBlock(ib) => self.entry_types.contains(&ib.target_type),
        }
    }

    pub(crate) fn include_loaded_item(&self, mod_name: &str, item: &Item) -> bool {
        let m = mod_name.to_string();
        match item {
            Item::FnDef(f) => self.loaded_fns.contains(&(m, f.name.clone())),
            Item::StructDef(s) => self.loaded_types.contains(&(m, s.name.clone())),
            Item::EnumDef(e) => self.loaded_types.contains(&(m, e.name.clone())),
            Item::TraitDef(t) => self.loaded_types.contains(&(m, t.name.clone())),
            Item::ImplBlock(ib) => self.loaded_types.contains(&(m, ib.target_type.clone())),
        }
    }

    pub(crate) fn include_qualified_pub_item(&self, mod_name: &str, item: &Item) -> bool {
        let m = mod_name.to_string();
        match item {
            Item::FnDef(f) if f.is_pub => self.loaded_fns.contains(&(m, f.name.clone())),
            Item::StructDef(s) if s.is_pub => self.loaded_types.contains(&(m, s.name.clone())),
            Item::EnumDef(e) if e.is_pub => self.loaded_types.contains(&(m, e.name.clone())),
            _ => false,
        }
    }
}

pub(crate) fn compute_reachability(graph: &ModuleGraph, loaded: &[LoadedModule]) -> Reachability {
    let loaded_by_name: HashMap<&str, &LoadedModule> =
        loaded.iter().map(|m| (m.name.as_str(), m)).collect();

    let mut entry_fns: HashSet<String> = HashSet::new();
    let mut loaded_fns: HashSet<(String, String)> = HashSet::new();
    let mut entry_types: HashSet<String> = HashSet::new();
    let mut loaded_types: HashSet<(String, String)> = HashSet::new();

    let mut queue: VecDeque<FnSlot> = VecDeque::new();

    if fn_named(&graph.entry_module, "main").is_some() {
        queue.push_back(FnSlot::Entry("main".into()));
    } else {
        let mut any_pub = false;
        for item in &graph.entry_module.items {
            if let Item::FnDef(f) = item {
                if f.is_pub {
                    queue.push_back(FnSlot::Entry(f.name.clone()));
                    any_pub = true;
                }
            }
        }
        if !any_pub {
            for item in &graph.entry_module.items {
                if let Item::FnDef(f) = item {
                    queue.push_back(FnSlot::Entry(f.name.clone()));
                }
            }
        }
    }

    while let Some(slot) = queue.pop_front() {
        let (ast_mod, fname) = match &slot {
            FnSlot::Entry(name) => (&graph.entry_module, name.as_str()),
            FnSlot::Loaded { module, name } => {
                let Some(m) = loaded_by_name.get(module.as_str()) else {
                    continue;
                };
                (&m.ast, name.as_str())
            }
        };

        let inserted = match &slot {
            FnSlot::Entry(n) => entry_fns.insert(n.clone()),
            FnSlot::Loaded { module, name } => loaded_fns.insert((module.clone(), name.clone())),
        };
        if !inserted {
            continue;
        }

        let Some(Item::FnDef(fndef)) = ast_mod
            .items
            .iter()
            .find(|i| matches!(i, Item::FnDef(f) if f.name == fname))
        else {
            continue;
        };

        for p in &fndef.params {
            record_type(
                &p.ty,
                &slot,
                &graph.entry_module,
                &loaded_by_name,
                &mut entry_types,
                &mut loaded_types,
            );
        }
        if let Some(ret) = &fndef.return_type {
            record_type(
                ret,
                &slot,
                &graph.entry_module,
                &loaded_by_name,
                &mut entry_types,
                &mut loaded_types,
            );
        }
        scan_block(
            &fndef.body,
            &slot,
            &graph.entry_module,
            &loaded_by_name,
            &mut queue,
            &mut entry_types,
            &mut loaded_types,
        );
    }

    let mut needed_modules = HashSet::new();
    for (m, _) in &loaded_fns {
        needed_modules.insert(m.clone());
    }
    for (m, _) in &loaded_types {
        needed_modules.insert(m.clone());
    }

    Reachability {
        entry_fns,
        entry_types,
        loaded_fns,
        loaded_types,
        needed_modules,
    }
}

fn fn_named<'a>(module: &'a ast::Module, name: &str) -> Option<&'a ast::FnDef> {
    module.items.iter().find_map(|i| {
        if let Item::FnDef(f) = i {
            (f.name == name).then_some(f)
        } else {
            None
        }
    })
}

fn top_level_fn_named(module: &ast::Module, name: &str) -> bool {
    module.items.iter().any(|i| matches!(i, Item::FnDef(f) if f.name == name))
}

fn type_named(module: &ast::Module, name: &str) -> bool {
    module.items.iter().any(|i| {
        matches!(i, Item::StructDef(s) if s.name == name)
            || matches!(i, Item::EnumDef(e) if e.name == name)
            || matches!(i, Item::TraitDef(t) if t.name == name)
    })
}

fn record_named_type(
    name: &str,
    slot: &FnSlot,
    entry: &ast::Module,
    loaded_by_name: &HashMap<&str, &LoadedModule>,
    entry_types: &mut HashSet<String>,
    loaded_types: &mut HashSet<(String, String)>,
) {
    match slot {
        FnSlot::Entry(_) => {
            if type_named(entry, name) {
                entry_types.insert(name.to_string());
            }
        }
        FnSlot::Loaded { module, .. } => {
            if let Some(m) = loaded_by_name.get(module.as_str()) {
                if type_named(&m.ast, name) {
                    loaded_types.insert((module.clone(), name.to_string()));
                }
            }
        }
    }
}

fn record_type(
    ty: &TypeExpr,
    slot: &FnSlot,
    entry: &ast::Module,
    loaded_by_name: &HashMap<&str, &LoadedModule>,
    entry_types: &mut HashSet<String>,
    loaded_types: &mut HashSet<(String, String)>,
) {
    match ty {
        TypeExpr::Named { name, .. } => {
            record_named_type(name, slot, entry, loaded_by_name, entry_types, loaded_types);
        }
        TypeExpr::Qualified { module, name, .. } => {
            if loaded_by_name.contains_key(module.as_str()) {
                loaded_types.insert((module.clone(), name.clone()));
            }
        }
        TypeExpr::Generic { name, args, .. } => {
            record_named_type(name, slot, entry, loaded_by_name, entry_types, loaded_types);
            for a in args {
                record_type(a, slot, entry, loaded_by_name, entry_types, loaded_types);
            }
        }
        TypeExpr::Tuple(elems, _) => {
            for e in elems {
                record_type(e, slot, entry, loaded_by_name, entry_types, loaded_types);
            }
        }
        TypeExpr::Array { elem, .. } | TypeExpr::Slice { elem, .. } => {
            record_type(elem, slot, entry, loaded_by_name, entry_types, loaded_types);
        }
        TypeExpr::Function { params, ret, .. } => {
            for p in params {
                record_type(p, slot, entry, loaded_by_name, entry_types, loaded_types);
            }
            record_type(ret, slot, entry, loaded_by_name, entry_types, loaded_types);
        }
        TypeExpr::Unit(_) => {}
    }
}

fn push_same_module_fn(slot: &FnSlot, name: &str, queue: &mut VecDeque<FnSlot>) {
    match slot {
        FnSlot::Entry(_) => queue.push_back(FnSlot::Entry(name.to_string())),
        FnSlot::Loaded { module, .. } => queue.push_back(FnSlot::Loaded {
            module: module.clone(),
            name: name.to_string(),
        }),
    }
}

fn scan_block(
    block: &Block,
    slot: &FnSlot,
    entry: &ast::Module,
    loaded_by_name: &HashMap<&str, &LoadedModule>,
    queue: &mut VecDeque<FnSlot>,
    entry_types: &mut HashSet<String>,
    loaded_types: &mut HashSet<(String, String)>,
) {
    for st in &block.stmts {
        match st {
            Stmt::Let {
                ty,
                init,
                pattern,
                ..
            } => {
                if let Some(ty) = ty {
                    record_type(ty, slot, entry, loaded_by_name, entry_types, loaded_types);
                }
                scan_pattern_bindings(pattern, entry, loaded_by_name, entry_types, loaded_types);
                scan_expr(
                    init,
                    slot,
                    entry,
                    loaded_by_name,
                    queue,
                    entry_types,
                    loaded_types,
                );
            }
            Stmt::Expr(e) => scan_expr(
                e,
                slot,
                entry,
                loaded_by_name,
                queue,
                entry_types,
                loaded_types,
            ),
            Stmt::While { cond, body, .. } => {
                scan_expr(
                    cond,
                    slot,
                    entry,
                    loaded_by_name,
                    queue,
                    entry_types,
                    loaded_types,
                );
                scan_block(
                    body,
                    slot,
                    entry,
                    loaded_by_name,
                    queue,
                    entry_types,
                    loaded_types,
                );
            }
            Stmt::Loop { body, .. } => scan_block(
                body,
                slot,
                entry,
                loaded_by_name,
                queue,
                entry_types,
                loaded_types,
            ),
            Stmt::For { iter, body, .. } => {
                match iter {
                    ForIter::Range { start, end } => {
                        scan_expr(
                            start,
                            slot,
                            entry,
                            loaded_by_name,
                            queue,
                            entry_types,
                            loaded_types,
                        );
                        scan_expr(
                            end,
                            slot,
                            entry,
                            loaded_by_name,
                            queue,
                            entry_types,
                            loaded_types,
                        );
                    }
                    ForIter::Values(e) | ForIter::Iter(e) => scan_expr(
                        e,
                        slot,
                        entry,
                        loaded_by_name,
                        queue,
                        entry_types,
                        loaded_types,
                    ),
                }
                scan_block(
                    body,
                    slot,
                    entry,
                    loaded_by_name,
                    queue,
                    entry_types,
                    loaded_types,
                );
            }
        }
    }
    if let Some(tail) = &block.tail_expr {
        scan_expr(
            tail,
            slot,
            entry,
            loaded_by_name,
            queue,
            entry_types,
            loaded_types,
        );
    }
}

fn scan_pattern_bindings(
    pattern: &Option<Pattern>,
    entry: &ast::Module,
    loaded_by_name: &HashMap<&str, &LoadedModule>,
    entry_types: &mut HashSet<String>,
    loaded_types: &mut HashSet<(String, String)>,
) {
    let Some(p) = pattern else { return };
    match p {
        Pattern::Tuple { elements, .. } => {
            for sub in elements {
                scan_pattern_bindings(&Some(sub.clone()), entry, loaded_by_name, entry_types, loaded_types);
            }
        }
        Pattern::Struct { name, .. } => {
            // Struct pattern names a type; treat like Named in entry-only context.
            if type_named(entry, name) {
                entry_types.insert(name.clone());
            }
        }
        Pattern::Enum { .. } | Pattern::Or { .. } => {
            // Conservative: skip nested enum/or type extraction for this slice.
        }
        _ => {}
    }
}

fn scan_expr(
    expr: &Expr,
    slot: &FnSlot,
    entry: &ast::Module,
    loaded_by_name: &HashMap<&str, &LoadedModule>,
    queue: &mut VecDeque<FnSlot>,
    entry_types: &mut HashSet<String>,
    loaded_types: &mut HashSet<(String, String)>,
) {
    match expr {
        Expr::Call { callee, args, .. } => {
            match &**callee {
                Expr::Ident { name, .. } => {
                    let ast_mod = match slot {
                        FnSlot::Entry(_) => entry,
                        FnSlot::Loaded { module, .. } => {
                            if let Some(m) = loaded_by_name.get(module.as_str()) {
                                &m.ast
                            } else {
                                entry
                            }
                        }
                    };
                    if top_level_fn_named(ast_mod, name) {
                        push_same_module_fn(slot, name, queue);
                    }
                }
                Expr::QualifiedIdent { module, name, .. } => {
                    if loaded_by_name.contains_key(module.as_str()) {
                        let m = loaded_by_name[module.as_str()];
                        if top_level_fn_named(&m.ast, name) {
                            queue.push_back(FnSlot::Loaded {
                                module: module.clone(),
                                name: name.clone(),
                            });
                        }
                    }
                }
                other => scan_expr(
                    other,
                    slot,
                    entry,
                    loaded_by_name,
                    queue,
                    entry_types,
                    loaded_types,
                ),
            }
            for a in args {
                scan_expr(
                    a,
                    slot,
                    entry,
                    loaded_by_name,
                    queue,
                    entry_types,
                    loaded_types,
                );
            }
        }
        Expr::StructInit {
            name,
            fields,
            base,
            ..
        } => {
            record_named_type(name, slot, entry, loaded_by_name, entry_types, loaded_types);
            if let Some(b) = base {
                scan_expr(
                    b,
                    slot,
                    entry,
                    loaded_by_name,
                    queue,
                    entry_types,
                    loaded_types,
                );
            }
            for (_, e) in fields {
                scan_expr(
                    e,
                    slot,
                    entry,
                    loaded_by_name,
                    queue,
                    entry_types,
                    loaded_types,
                );
            }
        }
        Expr::Binary { left, right, .. } => {
            scan_expr(
                left,
                slot,
                entry,
                loaded_by_name,
                queue,
                entry_types,
                loaded_types,
            );
            scan_expr(
                right,
                slot,
                entry,
                loaded_by_name,
                queue,
                entry_types,
                loaded_types,
            );
        }
        Expr::Unary { operand, .. } => scan_expr(
            operand,
            slot,
            entry,
            loaded_by_name,
            queue,
            entry_types,
            loaded_types,
        ),
        Expr::FieldAccess { object, .. } => scan_expr(
            object,
            slot,
            entry,
            loaded_by_name,
            queue,
            entry_types,
            loaded_types,
        ),
        Expr::Index { object, index, .. } => {
            scan_expr(
                object,
                slot,
                entry,
                loaded_by_name,
                queue,
                entry_types,
                loaded_types,
            );
            scan_expr(
                index,
                slot,
                entry,
                loaded_by_name,
                queue,
                entry_types,
                loaded_types,
            );
        }
        Expr::If {
            cond,
            then_block,
            else_block,
            ..
        } => {
            scan_expr(
                cond,
                slot,
                entry,
                loaded_by_name,
                queue,
                entry_types,
                loaded_types,
            );
            scan_block(
                then_block,
                slot,
                entry,
                loaded_by_name,
                queue,
                entry_types,
                loaded_types,
            );
            if let Some(else_b) = else_block {
                scan_block(
                    else_b,
                    slot,
                    entry,
                    loaded_by_name,
                    queue,
                    entry_types,
                    loaded_types,
                );
            }
        }
        Expr::Match { scrutinee, arms, .. } => {
            scan_expr(
                scrutinee,
                slot,
                entry,
                loaded_by_name,
                queue,
                entry_types,
                loaded_types,
            );
            for MatchArm {
                pattern,
                guard,
                body,
                ..
            } in arms
            {
                scan_pattern_bindings(&Some(pattern.clone()), entry, loaded_by_name, entry_types, loaded_types);
                if let Some(g) = guard {
                    scan_expr(
                        g,
                        slot,
                        entry,
                        loaded_by_name,
                        queue,
                        entry_types,
                        loaded_types,
                    );
                }
                scan_expr(
                    body,
                    slot,
                    entry,
                    loaded_by_name,
                    queue,
                    entry_types,
                    loaded_types,
                );
            }
        }
        Expr::Block(block) => scan_block(
            block,
            slot,
            entry,
            loaded_by_name,
            queue,
            entry_types,
            loaded_types,
        ),
        Expr::Tuple { elements, .. } | Expr::Array { elements, .. } => {
            for e in elements {
                scan_expr(
                    e,
                    slot,
                    entry,
                    loaded_by_name,
                    queue,
                    entry_types,
                    loaded_types,
                );
            }
        }
        Expr::ArrayRepeat { value, count, .. } => {
            scan_expr(
                value,
                slot,
                entry,
                loaded_by_name,
                queue,
                entry_types,
                loaded_types,
            );
            scan_expr(
                count,
                slot,
                entry,
                loaded_by_name,
                queue,
                entry_types,
                loaded_types,
            );
        }
        Expr::Closure { body, .. } => scan_expr(
            body,
            slot,
            entry,
            loaded_by_name,
            queue,
            entry_types,
            loaded_types,
        ),
        Expr::Return { value, .. } => {
            if let Some(v) = value {
                scan_expr(
                    v,
                    slot,
                    entry,
                    loaded_by_name,
                    queue,
                    entry_types,
                    loaded_types,
                );
            }
        }
        Expr::Break { value, .. } => {
            if let Some(v) = value {
                scan_expr(
                    v,
                    slot,
                    entry,
                    loaded_by_name,
                    queue,
                    entry_types,
                    loaded_types,
                );
            }
        }
        Expr::Try { expr, .. } => scan_expr(
            expr,
            slot,
            entry,
            loaded_by_name,
            queue,
            entry_types,
            loaded_types,
        ),
        Expr::Assign { target, value, .. } => {
            scan_expr(
                target,
                slot,
                entry,
                loaded_by_name,
                queue,
                entry_types,
                loaded_types,
            );
            scan_expr(
                value,
                slot,
                entry,
                loaded_by_name,
                queue,
                entry_types,
                loaded_types,
            );
        }
        Expr::Loop { body, .. } => scan_block(
            body,
            slot,
            entry,
            loaded_by_name,
            queue,
            entry_types,
            loaded_types,
        ),
        Expr::IntLit { .. }
        | Expr::FloatLit { .. }
        | Expr::StringLit { .. }
        | Expr::CharLit { .. }
        | Expr::BoolLit { .. }
        | Expr::Ident { .. }
        | Expr::QualifiedIdent { .. }
        | Expr::Continue { .. } => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_diagnostics::Span;
    use std::path::PathBuf;

    fn empty_fn(name: &str, body: Block) -> Item {
        Item::FnDef(ast::FnDef {
            docs: vec![],
            name: name.into(),
            type_params: vec![],
            type_param_bounds: vec![],
            params: vec![],
            return_type: None,
            body,
            is_pub: name == "main",
            span: Span::dummy(),
        })
    }

    fn empty_block() -> Block {
        Block {
            stmts: vec![],
            tail_expr: None,
            span: Span::dummy(),
        }
    }

    fn call_stmt(callee: Expr) -> Stmt {
        Stmt::Expr(Expr::Call {
            callee: Box::new(callee),
            type_args: vec![],
            args: vec![],
            span: Span::dummy(),
        })
    }

    #[test]
    fn lazy_resolve_skips_unreachable_entry_fn() {
        let main_body = Block {
            stmts: vec![call_stmt(Expr::Ident {
                name: "used".into(),
                span: Span::dummy(),
            })],
            tail_expr: None,
            span: Span::dummy(),
        };
        let entry = ast::Module {
            docs: vec![],
            imports: vec![],
            items: vec![
                empty_fn("main", main_body),
                empty_fn("used", empty_block()),
                empty_fn("unused", empty_block()),
            ],
        };
        let graph = ModuleGraph {
            entry_module: entry,
            loaded: HashMap::new(),
            _std_root: PathBuf::new(),
        };
        let r = compute_reachability(&graph, &[]);
        assert!(r.entry_fns.contains("main"));
        assert!(r.entry_fns.contains("used"));
        assert!(!r.entry_fns.contains("unused"));
    }

    #[test]
    fn lazy_resolve_follows_qualified_call_into_loaded_module() {
        let main_body = Block {
            stmts: vec![call_stmt(Expr::QualifiedIdent {
                module: "lib".into(),
                name: "f".into(),
                span: Span::dummy(),
            })],
            tail_expr: None,
            span: Span::dummy(),
        };
        let entry = ast::Module {
            docs: vec![],
            imports: vec![],
            items: vec![empty_fn("main", main_body)],
        };
        let lib = ast::Module {
            docs: vec![],
            imports: vec![],
            items: vec![
                empty_fn("f", empty_block()),
                empty_fn("g", empty_block()),
            ],
        };
        let mut loaded_map = HashMap::new();
        let p = PathBuf::from("/tmp/lib.ark");
        loaded_map.insert(
            p.clone(),
            LoadedModule {
                name: "lib".into(),
                path: p,
                ast: lib,
            },
        );
        let graph = ModuleGraph {
            entry_module: entry,
            loaded: loaded_map,
            _std_root: PathBuf::new(),
        };
        let loaded: Vec<LoadedModule> = graph.loaded.values().cloned().collect();
        let r = compute_reachability(&graph, &loaded);
        assert!(r.loaded_fns.contains(&(String::from("lib"), String::from("f"))));
        assert!(!r.loaded_fns.contains(&(String::from("lib"), String::from("g"))));
        assert!(r.needed_modules.contains("lib"));
    }

    #[test]
    fn lazy_resolve_skips_unused_loaded_module() {
        let main_body = Block {
            stmts: vec![call_stmt(Expr::Ident {
                name: "used_here".into(),
                span: Span::dummy(),
            })],
            tail_expr: None,
            span: Span::dummy(),
        };
        let entry = ast::Module {
            docs: vec![],
            imports: vec![],
            items: vec![
                empty_fn("main", main_body),
                empty_fn("used_here", empty_block()),
            ],
        };
        let heavy = ast::Module {
            docs: vec![],
            imports: vec![],
            items: vec![empty_fn("heavy_work", empty_block())],
        };
        let mut loaded_map = HashMap::new();
        let p = PathBuf::from("/tmp/heavy.ark");
        loaded_map.insert(
            p.clone(),
            LoadedModule {
                name: "heavy".into(),
                path: p,
                ast: heavy,
            },
        );
        let graph = ModuleGraph {
            entry_module: entry,
            loaded: loaded_map,
            _std_root: PathBuf::new(),
        };
        let loaded: Vec<LoadedModule> = graph.loaded.values().cloned().collect();
        let r = compute_reachability(&graph, &loaded);
        assert!(!r.needed_modules.contains("heavy"));
    }
}
