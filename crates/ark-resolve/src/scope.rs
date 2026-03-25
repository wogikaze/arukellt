//! Symbol table and scope management.

use ark_diagnostics::Span;
use std::collections::HashMap;

/// Unique scope identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScopeId(pub u32);

/// Unique symbol identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(pub u32);

/// What kind of symbol this is.
#[derive(Debug, Clone)]
pub enum SymbolKind {
    Variable { is_mut: bool },
    Function { is_pub: bool },
    Struct { is_pub: bool },
    Enum { is_pub: bool },
    EnumVariant { enum_name: String },
    TypeParam,
    Module,
    BuiltinFn,
    BuiltinType,
}

/// A resolved symbol.
#[derive(Debug, Clone)]
pub struct Symbol {
    pub id: SymbolId,
    pub name: String,
    pub kind: SymbolKind,
    pub span: Span,
}

/// A lexical scope containing symbol bindings.
#[derive(Debug)]
pub struct Scope {
    pub id: ScopeId,
    pub parent: Option<ScopeId>,
    pub symbols: HashMap<String, SymbolId>,
}

/// The symbol table managing all scopes and symbols.
#[derive(Debug)]
pub struct SymbolTable {
    scopes: Vec<Scope>,
    symbols: Vec<Symbol>,
    next_scope: u32,
    next_symbol: u32,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            scopes: Vec::new(),
            symbols: Vec::new(),
            next_scope: 0,
            next_symbol: 0,
        }
    }

    pub fn create_scope(&mut self, parent: Option<ScopeId>) -> ScopeId {
        let id = ScopeId(self.next_scope);
        self.next_scope += 1;
        self.scopes.push(Scope {
            id,
            parent,
            symbols: HashMap::new(),
        });
        id
    }

    pub fn define(&mut self, scope: ScopeId, name: String, kind: SymbolKind, span: Span) -> SymbolId {
        let id = SymbolId(self.next_symbol);
        self.next_symbol += 1;
        self.symbols.push(Symbol {
            id,
            name: name.clone(),
            kind,
            span,
        });
        self.scopes[scope.0 as usize].symbols.insert(name, id);
        id
    }

    pub fn lookup(&self, scope: ScopeId, name: &str) -> Option<SymbolId> {
        let s = &self.scopes[scope.0 as usize];
        if let Some(&id) = s.symbols.get(name) {
            return Some(id);
        }
        if let Some(parent) = s.parent {
            return self.lookup(parent, name);
        }
        None
    }

    pub fn lookup_local(&self, scope: ScopeId, name: &str) -> Option<SymbolId> {
        self.scopes[scope.0 as usize].symbols.get(name).copied()
    }

    pub fn get_symbol(&self, id: SymbolId) -> &Symbol {
        &self.symbols[id.0 as usize]
    }

    pub fn get_scope(&self, id: ScopeId) -> &Scope {
        &self.scopes[id.0 as usize]
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_lookup() {
        let mut table = SymbolTable::new();
        let global = table.create_scope(None);
        let inner = table.create_scope(Some(global));

        table.define(global, "x".into(), SymbolKind::Variable { is_mut: false }, Span::dummy());
        table.define(inner, "y".into(), SymbolKind::Variable { is_mut: true }, Span::dummy());

        // inner can see both x and y
        assert!(table.lookup(inner, "x").is_some());
        assert!(table.lookup(inner, "y").is_some());

        // global can only see x
        assert!(table.lookup(global, "x").is_some());
        assert!(table.lookup(global, "y").is_none());
    }

    #[test]
    fn test_shadowing() {
        let mut table = SymbolTable::new();
        let global = table.create_scope(None);
        let inner = table.create_scope(Some(global));

        let id1 = table.define(global, "x".into(), SymbolKind::Variable { is_mut: false }, Span::dummy());
        let id2 = table.define(inner, "x".into(), SymbolKind::Variable { is_mut: true }, Span::dummy());

        assert_ne!(id1, id2);
        assert_eq!(table.lookup(inner, "x"), Some(id2)); // shadows
        assert_eq!(table.lookup(global, "x"), Some(id1));
    }
}
