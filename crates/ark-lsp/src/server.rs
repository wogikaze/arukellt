//! LSP server implementation.

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use ark_diagnostics::{DiagnosticSink, Severity};
use ark_lexer::{Lexer, TokenKind};
use ark_parser::ast;
use ark_parser::parse;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

// Semantic token types registered with the client.
const SEMANTIC_TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::KEYWORD,
    SemanticTokenType::FUNCTION,
    SemanticTokenType::VARIABLE,
    SemanticTokenType::TYPE,
    SemanticTokenType::STRING,
    SemanticTokenType::NUMBER,
    SemanticTokenType::OPERATOR,
    SemanticTokenType::COMMENT,
];

/// Cached results of a full analysis pass (lex → parse → resolve → typecheck)
/// for a single document.
struct CachedAnalysis {
    tokens: Vec<ark_lexer::Token>,
    module: ast::Module,
    resolved: Option<ark_resolve::ResolvedModule>,
    checker: Option<ark_typecheck::TypeChecker>,
    diagnostics: Vec<Diagnostic>,
}

struct ArukellBackend {
    client: Client,
    documents: Mutex<HashMap<Url, String>>,
    analysis_cache: Mutex<HashMap<Url, CachedAnalysis>>,
}

impl ArukellBackend {
    fn new(client: Client) -> Self {
        Self {
            client,
            documents: Mutex::new(HashMap::new()),
            analysis_cache: Mutex::new(HashMap::new()),
        }
    }

    async fn refresh_diagnostics(&self, uri: Url, text: &str) {
        let analysis = Self::analyze_source(text);
        let diagnostics = analysis.diagnostics.clone();
        {
            let mut cache = self.analysis_cache.lock().unwrap();
            cache.insert(uri.clone(), analysis);
        }
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    fn offset_to_position(source: &str, offset: u32) -> Position {
        let offset = offset as usize;
        let mut line = 0u32;
        let mut col = 0u32;
        for (i, ch) in source.char_indices() {
            if i >= offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
        }
        Position {
            line,
            character: col,
        }
    }

    fn position_to_offset(source: &str, pos: Position) -> usize {
        let mut line = 0u32;
        let mut col = 0u32;
        for (i, ch) in source.char_indices() {
            if line == pos.line && col == pos.character {
                return i;
            }
            if ch == '\n' {
                if line == pos.line {
                    return i;
                }
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
        }
        source.len()
    }

    fn span_to_range(source: &str, span: ark_diagnostics::Span) -> Range {
        Range {
            start: Self::offset_to_position(source, span.start),
            end: Self::offset_to_position(source, span.end),
        }
    }

    /// Run the full analysis pipeline (lex → parse → resolve → typecheck) on
    /// in-memory source text and return cached results for reuse by all LSP
    /// features.
    fn analyze_source(source: &str) -> CachedAnalysis {
        let mut sink = DiagnosticSink::new();

        let lexer = Lexer::new(0, source);
        let tokens: Vec<_> = lexer.collect();
        let module = parse(&tokens, &mut sink);

        if sink.has_errors() {
            let diagnostics = Self::collect_lsp_diagnostics(source, &sink);
            return CachedAnalysis {
                tokens,
                module,
                resolved: None,
                checker: None,
                diagnostics,
            };
        }

        let cached_module = module.clone();
        let resolved = ark_resolve::resolve_module(module, &mut sink);
        let mut checker = ark_typecheck::TypeChecker::new();
        checker.register_builtins();
        checker.check_module(&resolved, &mut sink);

        let diagnostics = Self::collect_lsp_diagnostics(source, &sink);

        CachedAnalysis {
            tokens,
            module: cached_module,
            resolved: Some(resolved),
            checker: Some(checker),
            diagnostics,
        }
    }

    fn collect_lsp_diagnostics(source: &str, sink: &DiagnosticSink) -> Vec<Diagnostic> {
        sink.diagnostics()
            .iter()
            .map(|diag| {
                let range = if let Some(label) = diag.labels.first() {
                    Range {
                        start: Self::offset_to_position(source, label.span.start),
                        end: Self::offset_to_position(source, label.span.end),
                    }
                } else {
                    Range::default()
                };
                let severity = match diag.severity() {
                    Severity::Error => DiagnosticSeverity::ERROR,
                    Severity::Warning => DiagnosticSeverity::WARNING,
                    Severity::Help => DiagnosticSeverity::INFORMATION,
                };
                Diagnostic {
                    range,
                    severity: Some(severity),
                    code: Some(NumberOrString::String(diag.code.as_str().to_string())),
                    source: Some("arukellt".to_string()),
                    message: diag.message.clone(),
                    ..Default::default()
                }
            })
            .collect()
    }

    fn completion_prefix(source: &str, offset: usize) -> String {
        let capped = offset.min(source.len());
        let bytes = source.as_bytes();
        let mut start = capped;
        while start > 0 {
            let b = bytes[start - 1];
            let is_ident = b.is_ascii_alphanumeric() || b == b'_';
            if !is_ident {
                break;
            }
            start -= 1;
        }
        source[start..capped].to_string()
    }

    fn already_imported_modules(module: &ast::Module) -> HashSet<String> {
        module
            .imports
            .iter()
            .map(|import| import.module_name.clone())
            .collect()
    }

    fn imported_aliases(module: &ast::Module) -> HashSet<String> {
        module
            .imports
            .iter()
            .flat_map(|import| {
                let mut names = vec![import.module_name.clone()];
                if let Some(alias) = &import.alias {
                    names.push(alias.clone());
                }
                names
            })
            .collect()
    }

    fn expected_type_from_context(source: &str, offset: usize) -> Option<&'static str> {
        let capped = offset.min(source.len());
        let before = &source[..capped];
        if before.ends_with("println(") || before.ends_with("print(") {
            return Some("String");
        }
        if before.ends_with("len(") {
            return Some("collection");
        }
        None
    }

    fn push_completion(
        items: &mut Vec<CompletionItem>,
        seen: &mut HashSet<String>,
        item: CompletionItem,
    ) {
        if seen.insert(item.label.clone()) {
            items.push(item);
        }
    }

    fn get_completions(
        source: &str,
        tokens: &[ark_lexer::Token],
        module: &ast::Module,
        offset: usize,
    ) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        let mut seen = HashSet::new();
        let prefix = Self::completion_prefix(source, offset);
        let imported_modules = Self::already_imported_modules(module);
        let imported_aliases = Self::imported_aliases(module);
        let expected_type = Self::expected_type_from_context(source, offset);

        // Built-in functions with simple relevance ordering.
        let builtins = [
            (
                "println",
                "Print a value followed by newline",
                Some("String"),
                "010",
            ),
            ("print", "Print a value", Some("String"), "011"),
            (
                "len",
                "Get length of a Vec or String",
                Some("collection"),
                "020",
            ),
            ("push", "Push element to Vec", Some("collection"), "030"),
            (
                "get",
                "Get element from Vec by index",
                Some("collection"),
                "031",
            ),
            (
                "set",
                "Set element in Vec at index",
                Some("collection"),
                "032",
            ),
            (
                "pop",
                "Remove last element from Vec",
                Some("collection"),
                "033",
            ),
            (
                "to_string",
                "Convert a value to String",
                Some("String"),
                "012",
            ),
            (
                "i32_to_string",
                "Convert i32 to String",
                Some("String"),
                "013",
            ),
            (
                "i64_to_string",
                "Convert i64 to String",
                Some("String"),
                "014",
            ),
            (
                "f64_to_string",
                "Convert f64 to String",
                Some("String"),
                "015",
            ),
            (
                "bool_to_string",
                "Convert bool to String",
                Some("String"),
                "016",
            ),
            ("concat", "Concatenate two strings", Some("String"), "017"),
            (
                "Vec_new_i32",
                "Create new Vec<i32>",
                Some("collection"),
                "040",
            ),
            (
                "Vec_new_String",
                "Create new Vec<String>",
                Some("collection"),
                "041",
            ),
            (
                "sort_i32",
                "Sort Vec<i32> in place",
                Some("collection"),
                "042",
            ),
            ("assert", "Assert a boolean condition", None, "050"),
            ("assert_eq", "Assert two values are equal", None, "051"),
            ("parse_i32", "Parse string to i32", None, "052"),
        ];

        for (name, detail, expected, base_rank) in &builtins {
            if !prefix.is_empty() && !name.starts_with(&prefix) {
                continue;
            }
            let rank = if expected_type.is_some() && expected_type == *expected {
                format!("0-{base_rank}")
            } else {
                format!("1-{base_rank}")
            };
            Self::push_completion(
                &mut items,
                &mut seen,
                CompletionItem {
                    label: (*name).to_string(),
                    kind: Some(CompletionItemKind::FUNCTION),
                    detail: Some((*detail).to_string()),
                    sort_text: Some(rank),
                    filter_text: Some((*name).to_string()),
                    ..Default::default()
                },
            );
        }

        // Importable std/host modules with auto-import hints.
        let importable_modules = [
            "std::host::stdio",
            "std::host::fs",
            "std::host::env",
            "std::path",
            "std::time",
            "std::test",
        ];
        for module_name in &importable_modules {
            let alias = module_name.rsplit("::").next().unwrap_or(module_name);
            if !prefix.is_empty()
                && !alias.starts_with(&prefix)
                && !module_name.starts_with(&prefix)
            {
                continue;
            }
            let detail =
                if imported_modules.contains(*module_name) || imported_aliases.contains(alias) {
                    format!("module {module_name}")
                } else {
                    format!("module {module_name} (auto import candidate)")
                };
            let documentation =
                if imported_modules.contains(*module_name) || imported_aliases.contains(alias) {
                    None
                } else {
                    Some(Documentation::String(format!(
                        "Add `use {module_name}` to import this module alias."
                    )))
                };
            Self::push_completion(
                &mut items,
                &mut seen,
                CompletionItem {
                    label: alias.to_string(),
                    kind: Some(CompletionItemKind::MODULE),
                    detail: Some(detail),
                    documentation,
                    sort_text: Some(format!("2-{alias}")),
                    filter_text: Some(module_name.to_string()),
                    insert_text: Some(alias.to_string()),
                    ..Default::default()
                },
            );
        }

        // Keywords
        let keywords = [
            "fn", "let", "mut", "if", "else", "while", "for", "in", "match", "return", "break",
            "continue", "true", "false", "struct", "enum", "trait", "impl", "type", "use", "mod",
        ];
        for kw in &keywords {
            if !prefix.is_empty() && !kw.starts_with(&prefix) {
                continue;
            }
            Self::push_completion(
                &mut items,
                &mut seen,
                CompletionItem {
                    label: kw.to_string(),
                    kind: Some(CompletionItemKind::KEYWORD),
                    sort_text: Some(format!("3-{kw}")),
                    ..Default::default()
                },
            );
        }

        // Types
        let types = [
            "i32", "i64", "f32", "f64", "bool", "char", "String", "Vec", "Option", "Result",
        ];
        for ty in &types {
            if !prefix.is_empty() && !ty.starts_with(&prefix) {
                continue;
            }
            Self::push_completion(
                &mut items,
                &mut seen,
                CompletionItem {
                    label: ty.to_string(),
                    kind: Some(CompletionItemKind::CLASS),
                    sort_text: Some(format!("4-{ty}")),
                    ..Default::default()
                },
            );
        }

        // Extract identifiers from cached tokens.
        for tok in tokens {
            if let ark_lexer::TokenKind::Ident(_) = &tok.kind {
                let start = tok.span.start as usize;
                let end = tok.span.end as usize;
                if end <= source.len() {
                    let name = &source[start..end];
                    if !prefix.is_empty() && !name.starts_with(&prefix) {
                        continue;
                    }
                    if !keywords.contains(&name) {
                        Self::push_completion(
                            &mut items,
                            &mut seen,
                            CompletionItem {
                                label: name.to_string(),
                                kind: Some(CompletionItemKind::VARIABLE),
                                sort_text: Some(format!("5-{name}")),
                                ..Default::default()
                            },
                        );
                    }
                }
            }
        }

        items.sort_by(|a, b| a.sort_text.cmp(&b.sort_text).then(a.label.cmp(&b.label)));
        items
    }

    /// Find the identifier token at the given byte offset.
    fn find_ident_at_offset<'a>(
        source: &'a str,
        tokens: &[ark_lexer::Token],
        offset: usize,
    ) -> Option<&'a str> {
        for tok in tokens {
            let start = tok.span.start as usize;
            let end = tok.span.end as usize;
            if start <= offset && offset <= end && end <= source.len() {
                if let TokenKind::Ident(_) = &tok.kind {
                    return Some(&source[start..end]);
                }
            }
        }
        None
    }

    /// Walk AST items to find the definition site of a name. Returns the span
    /// of the defining identifier (function name, struct name, etc.).
    fn find_definition_span(module: &ast::Module, name: &str) -> Option<ark_diagnostics::Span> {
        // Search top-level items
        for item in &module.items {
            match item {
                ast::Item::FnDef(f) if f.name == name => return Some(f.span),
                ast::Item::StructDef(s) if s.name == name => return Some(s.span),
                ast::Item::EnumDef(e) if e.name == name => return Some(e.span),
                ast::Item::TraitDef(t) if t.name == name => return Some(t.span),
                ast::Item::ImplBlock(ib) => {
                    for m in &ib.methods {
                        if m.name == name {
                            return Some(m.span);
                        }
                    }
                }
                _ => {}
            }
        }

        // Search let bindings inside function bodies
        for item in &module.items {
            if let ast::Item::FnDef(f) = item {
                if let Some(span) = Self::find_let_in_block(&f.body, name) {
                    return Some(span);
                }
            }
        }

        // Search function parameters
        for item in &module.items {
            if let ast::Item::FnDef(f) = item {
                for p in &f.params {
                    if p.name == name {
                        return Some(p.span);
                    }
                }
            }
        }

        None
    }

    /// Recursively search for a `let` binding with `name` inside a block.
    fn find_let_in_block(block: &ast::Block, name: &str) -> Option<ark_diagnostics::Span> {
        for stmt in &block.stmts {
            match stmt {
                ast::Stmt::Let { name: n, span, .. } if n == name => return Some(*span),
                ast::Stmt::While { body, .. }
                | ast::Stmt::Loop { body, .. }
                | ast::Stmt::For { body, .. } => {
                    if let Some(s) = Self::find_let_in_block(body, name) {
                        return Some(s);
                    }
                }
                ast::Stmt::Expr(expr) => {
                    if let Some(s) = Self::find_let_in_expr(expr, name) {
                        return Some(s);
                    }
                }
                _ => {}
            }
        }
        if let Some(expr) = &block.tail_expr {
            if let Some(s) = Self::find_let_in_expr(expr, name) {
                return Some(s);
            }
        }
        None
    }

    fn find_let_in_expr(expr: &ast::Expr, name: &str) -> Option<ark_diagnostics::Span> {
        match expr {
            ast::Expr::Block(block) => Self::find_let_in_block(block, name),
            ast::Expr::If {
                then_block,
                else_block,
                ..
            } => {
                if let Some(s) = Self::find_let_in_block(then_block, name) {
                    return Some(s);
                }
                if let Some(eb) = else_block {
                    if let Some(s) = Self::find_let_in_block(eb, name) {
                        return Some(s);
                    }
                }
                None
            }
            ast::Expr::Loop { body, .. } => Self::find_let_in_block(body, name),
            ast::Expr::Match { arms, .. } => {
                for arm in arms {
                    if let Some(s) = Self::find_let_in_expr(&arm.body, name) {
                        return Some(s);
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Build type-aware hover information for an identifier using cached
    /// analysis results.
    fn type_hover_info(
        name: &str,
        module: &ast::Module,
        resolved: Option<&ark_resolve::ResolvedModule>,
        checker: Option<&ark_typecheck::TypeChecker>,
    ) -> Option<String> {
        let resolved = resolved?;
        let checker = checker?;

        // Collect AST param names so we can pair them with inferred types.
        let ast_param_names: Option<Vec<String>> = module.items.iter().find_map(|item| {
            if let ast::Item::FnDef(f) = item {
                if f.name == name {
                    return Some(f.params.iter().map(|p| p.name.clone()).collect());
                }
            }
            None
        });

        // Check function signatures
        if let Some(sig) = checker.fn_sig(name) {
            let params: Vec<String> = if let Some(ref names) = ast_param_names {
                names
                    .iter()
                    .zip(sig.params.iter())
                    .map(|(n, t)| format!("{n}: {t}"))
                    .collect()
            } else {
                sig.params.iter().map(|t| format!("{t}")).collect()
            };
            return Some(format!(
                "fn {}({}) -> {}",
                sig.name,
                params.join(", "),
                sig.ret
            ));
        }

        // Check struct definitions
        if let Some(info) = checker.struct_info(name) {
            let fields: Vec<String> = info
                .fields
                .iter()
                .map(|(n, t)| format!("{n}: {t}"))
                .collect();
            return Some(format!("struct {} {{ {} }}", info.name, fields.join(", ")));
        }

        // Check enum definitions
        if let Some(info) = checker.enum_info(name) {
            let variants: Vec<String> = info.variants.iter().map(|v| v.name.clone()).collect();
            return Some(format!("enum {} {{ {} }}", info.name, variants.join(", ")));
        }

        // Check let-bound variables via resolved symbol table
        let scope = resolved.global_scope;
        if let Some(sym_id) = resolved.symbols.lookup(scope, name) {
            let sym = resolved.symbols.get_symbol(sym_id);
            let kind_str = match &sym.kind {
                ark_resolve::SymbolKind::Variable { is_mut: true } => "let mut",
                ark_resolve::SymbolKind::Variable { is_mut: false } => "let",
                ark_resolve::SymbolKind::Function { .. } => "fn",
                ark_resolve::SymbolKind::Struct { .. } => "struct",
                ark_resolve::SymbolKind::Enum { .. } => "enum",
                ark_resolve::SymbolKind::EnumVariant { enum_name } => {
                    return Some(format!("variant {enum_name}::{name}"));
                }
                ark_resolve::SymbolKind::TypeParam => "type param",
                ark_resolve::SymbolKind::Module => "module",
                ark_resolve::SymbolKind::BuiltinFn => "builtin fn",
                ark_resolve::SymbolKind::BuiltinType => "builtin type",
            };

            // Try to find the type annotation from the AST let binding.
            if matches!(sym.kind, ark_resolve::SymbolKind::Variable { .. }) {
                let ty_ann = Self::find_let_type_annotation(module, name);
                if let Some(ty_str) = ty_ann {
                    return Some(format!("{kind_str} {name}: {ty_str}"));
                }
            }

            return Some(format!("{kind_str} {name}"));
        }

        None
    }

    /// Search the AST module for a `let` binding with `name` and return its
    /// type annotation (if any) as a displayable string.
    fn find_let_type_annotation(module: &ast::Module, name: &str) -> Option<String> {
        for item in &module.items {
            if let ast::Item::FnDef(f) = item {
                if let Some(ty) = Self::find_let_type_in_block(&f.body, name) {
                    return Some(ty);
                }
            }
        }
        None
    }

    fn find_let_type_in_block(block: &ast::Block, name: &str) -> Option<String> {
        for stmt in &block.stmts {
            if let ast::Stmt::Let {
                name: n,
                ty: Some(ty),
                ..
            } = stmt
            {
                if n == name {
                    return Some(Self::type_expr_to_string(ty));
                }
            }
        }
        None
    }

    /// Format a `TypeExpr` as a human-readable string.
    fn type_expr_to_string(ty: &ast::TypeExpr) -> String {
        match ty {
            ast::TypeExpr::Named { name, .. } => name.clone(),
            ast::TypeExpr::Generic { name, args, .. } => {
                let args_str: Vec<String> = args.iter().map(Self::type_expr_to_string).collect();
                format!("{}<{}>", name, args_str.join(", "))
            }
            ast::TypeExpr::Tuple(elems, _) => {
                let parts: Vec<String> = elems.iter().map(Self::type_expr_to_string).collect();
                format!("({})", parts.join(", "))
            }
            ast::TypeExpr::Array { elem, size, .. } => {
                format!("[{}; {}]", Self::type_expr_to_string(elem), size)
            }
            ast::TypeExpr::Slice { elem, .. } => {
                format!("[{}]", Self::type_expr_to_string(elem))
            }
            ast::TypeExpr::Function { params, ret, .. } => {
                let params_str: Vec<String> =
                    params.iter().map(Self::type_expr_to_string).collect();
                format!(
                    "fn({}) -> {}",
                    params_str.join(", "),
                    Self::type_expr_to_string(ret)
                )
            }
            ast::TypeExpr::Unit(_) => "()".to_string(),
            ast::TypeExpr::Qualified { module, name, .. } => {
                format!("{module}::{name}")
            }
        }
    }

    /// Classify a token kind into a semantic token type index.
    fn semantic_token_type_index(kind: &TokenKind) -> Option<u32> {
        if kind.is_keyword() {
            return Some(0); // KEYWORD
        }
        match kind {
            TokenKind::Ident(_) => Some(2), // VARIABLE
            TokenKind::StringLit(_) | TokenKind::CharLit(_) | TokenKind::FStringLit(_) => Some(4), // STRING
            TokenKind::IntLit(_) | TokenKind::FloatLit(_) => Some(5), // NUMBER
            TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Star
            | TokenKind::Slash
            | TokenKind::Percent
            | TokenKind::EqEq
            | TokenKind::BangEq
            | TokenKind::Lt
            | TokenKind::LtEq
            | TokenKind::Gt
            | TokenKind::GtEq
            | TokenKind::AmpAmp
            | TokenKind::PipePipe
            | TokenKind::Bang
            | TokenKind::Amp
            | TokenKind::Pipe
            | TokenKind::Caret
            | TokenKind::Tilde
            | TokenKind::Shl
            | TokenKind::Shr => Some(6), // OPERATOR
            _ => None,
        }
    }

    /// Produce semantic tokens for the whole document using cached analysis.
    fn compute_semantic_tokens(
        source: &str,
        tokens: &[ark_lexer::Token],
        module: &ast::Module,
    ) -> Vec<SemanticToken> {
        let mut fn_names = std::collections::HashSet::new();
        let mut type_names = std::collections::HashSet::new();
        for item in &module.items {
            match item {
                ast::Item::FnDef(f) => {
                    fn_names.insert(f.name.clone());
                }
                ast::Item::StructDef(s) => {
                    type_names.insert(s.name.clone());
                }
                ast::Item::EnumDef(e) => {
                    type_names.insert(e.name.clone());
                }
                ast::Item::TraitDef(t) => {
                    type_names.insert(t.name.clone());
                }
                ast::Item::ImplBlock(ib) => {
                    for m in &ib.methods {
                        fn_names.insert(m.name.clone());
                    }
                }
            }
        }

        let mut result = Vec::new();
        let mut prev_line = 0u32;
        let mut prev_start = 0u32;

        for tok in tokens {
            let start = tok.span.start;
            let end = tok.span.end;
            let length = end.saturating_sub(start);
            if length == 0 {
                continue;
            }
            if matches!(tok.kind, TokenKind::Newline | TokenKind::Eof) {
                continue;
            }

            let pos = Self::offset_to_position(source, start);

            // Determine token type with AST-aware overrides for identifiers.
            let token_type = if let TokenKind::Ident(_) = &tok.kind {
                let text_start = start as usize;
                let text_end = end as usize;
                let text = if text_end <= source.len() {
                    &source[text_start..text_end]
                } else {
                    ""
                };
                if fn_names.contains(text) {
                    1 // FUNCTION
                } else if type_names.contains(text) {
                    3 // TYPE
                } else {
                    2 // VARIABLE
                }
            } else if let Some(idx) = Self::semantic_token_type_index(&tok.kind) {
                idx
            } else {
                continue;
            };

            let delta_line = pos.line - prev_line;
            let delta_start = if delta_line == 0 {
                pos.character - prev_start
            } else {
                pos.character
            };

            result.push(SemanticToken {
                delta_line,
                delta_start,
                length,
                token_type,
                token_modifiers_bitset: 0,
            });

            prev_line = pos.line;
            prev_start = pos.character;
        }

        result
    }

    /// Extract document symbols (top-level items) from a cached AST.
    #[allow(deprecated)] // SymbolInformation::location etc.
    fn document_symbols(uri: &Url, source: &str, module: &ast::Module) -> Vec<SymbolInformation> {
        let mut symbols = Vec::new();

        for item in &module.items {
            let (name, kind, span) = match item {
                ast::Item::FnDef(f) => (f.name.clone(), SymbolKind::FUNCTION, f.span),
                ast::Item::StructDef(s) => (s.name.clone(), SymbolKind::STRUCT, s.span),
                ast::Item::EnumDef(e) => (e.name.clone(), SymbolKind::ENUM, e.span),
                ast::Item::TraitDef(t) => (t.name.clone(), SymbolKind::INTERFACE, t.span),
                ast::Item::ImplBlock(ib) => {
                    let impl_name = if let Some(tr) = &ib.trait_name {
                        format!("impl {} for {}", tr, ib.target_type)
                    } else {
                        format!("impl {}", ib.target_type)
                    };
                    // Also add methods inside the impl block.
                    for m in &ib.methods {
                        #[allow(deprecated)]
                        symbols.push(SymbolInformation {
                            name: m.name.clone(),
                            kind: SymbolKind::METHOD,
                            location: Location {
                                uri: uri.clone(),
                                range: Self::span_to_range(source, m.span),
                            },
                            tags: None,
                            deprecated: None,
                            container_name: Some(impl_name.clone()),
                        });
                    }
                    (impl_name, SymbolKind::OBJECT, ib.span)
                }
            };

            #[allow(deprecated)]
            symbols.push(SymbolInformation {
                name,
                kind,
                location: Location {
                    uri: uri.clone(),
                    range: Self::span_to_range(source, span),
                },
                tags: None,
                deprecated: None,
                container_name: None,
            });
        }

        symbols
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for ArukellBackend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".to_string(), ":".to_string()]),
                    ..Default::default()
                }),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: Default::default(),
                })),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                document_formatting_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                code_lens_provider: Some(CodeLensOptions {
                    resolve_provider: Some(false),
                }),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: SemanticTokensLegend {
                                token_types: SEMANTIC_TOKEN_TYPES.to_vec(),
                                token_modifiers: vec![],
                            },
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            range: None,
                            work_done_progress_options: Default::default(),
                        },
                    ),
                ),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "arukellt-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "arukellt LSP server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let text = params.text_document.text.clone();
        {
            let mut docs = self.documents.lock().unwrap();
            docs.insert(uri.clone(), text.clone());
        }
        self.refresh_diagnostics(uri, &text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        if let Some(change) = params.content_changes.into_iter().last() {
            let text = change.text;
            {
                let mut docs = self.documents.lock().unwrap();
                docs.insert(uri.clone(), text.clone());
            }
            self.refresh_diagnostics(uri, &text).await;
        }
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;

        let source = {
            let docs = self.documents.lock().unwrap();
            match docs.get(&uri) {
                Some(s) => s.clone(),
                None => return Ok(None),
            }
        };

        let mut cache = self.analysis_cache.lock().unwrap();
        let analysis = cache
            .entry(uri)
            .or_insert_with(|| Self::analyze_source(&source));

        let target_offset = Self::position_to_offset(&source, pos);

        for tok in &analysis.tokens {
            let start = tok.span.start as usize;
            let end = tok.span.end as usize;
            if start <= target_offset && target_offset <= end && end <= source.len() {
                let text = &source[start..end];
                let info = match &tok.kind {
                    TokenKind::Ident(_) => {
                        if let Some(type_info) = Self::type_hover_info(
                            text,
                            &analysis.module,
                            analysis.resolved.as_ref(),
                            analysis.checker.as_ref(),
                        ) {
                            type_info
                        } else {
                            format!("identifier `{}`", text)
                        }
                    }
                    TokenKind::IntLit(_) => {
                        format!("integer literal `{}`", text)
                    }
                    TokenKind::FloatLit(_) => {
                        format!("float literal `{}`", text)
                    }
                    TokenKind::StringLit(_) => "string literal".to_string(),
                    _ => format!("`{}`", text),
                };
                return Ok(Some(Hover {
                    contents: HoverContents::Scalar(MarkedString::String(info)),
                    range: Some(Self::span_to_range(&source, tok.span)),
                }));
            }
        }

        Ok(None)
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;

        let source = {
            let docs = self.documents.lock().unwrap();
            match docs.get(&uri) {
                Some(s) => s.clone(),
                None => return Ok(None),
            }
        };

        let mut cache = self.analysis_cache.lock().unwrap();
        let analysis = cache
            .entry(uri)
            .or_insert_with(|| Self::analyze_source(&source));

        let offset = Self::position_to_offset(&source, params.text_document_position.position);
        let items = Self::get_completions(&source, &analysis.tokens, &analysis.module, offset);
        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;

        let source = {
            let docs = self.documents.lock().unwrap();
            match docs.get(&uri) {
                Some(s) => s.clone(),
                None => return Ok(None),
            }
        };

        let mut cache = self.analysis_cache.lock().unwrap();
        let analysis = cache
            .entry(uri.clone())
            .or_insert_with(|| Self::analyze_source(&source));

        let target_offset = Self::position_to_offset(&source, pos);

        let name = match Self::find_ident_at_offset(&source, &analysis.tokens, target_offset) {
            Some(n) => n.to_string(),
            None => return Ok(None),
        };

        let span = match Self::find_definition_span(&analysis.module, &name) {
            Some(s) => s,
            None => return Ok(None),
        };

        let range = Self::span_to_range(&source, span);
        Ok(Some(GotoDefinitionResponse::Scalar(Location {
            uri,
            range,
        })))
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;

        let source = {
            let docs = self.documents.lock().unwrap();
            match docs.get(&uri) {
                Some(s) => s.clone(),
                None => return Ok(None),
            }
        };

        let mut cache = self.analysis_cache.lock().unwrap();
        let analysis = cache
            .entry(uri.clone())
            .or_insert_with(|| Self::analyze_source(&source));

        let target_offset = Self::position_to_offset(&source, pos);

        let name = match Self::find_ident_at_offset(&source, &analysis.tokens, target_offset) {
            Some(n) => n.to_string(),
            None => return Ok(None),
        };

        let mut locations = Vec::new();
        for tok in &analysis.tokens {
            if let TokenKind::Ident(_) = &tok.kind {
                let start = tok.span.start as usize;
                let end = tok.span.end as usize;
                if end <= source.len() && source[start..end] == *name {
                    locations.push(Location {
                        uri: uri.clone(),
                        range: Self::span_to_range(&source, tok.span),
                    });
                }
            }
        }

        if locations.is_empty() {
            Ok(None)
        } else {
            Ok(Some(locations))
        }
    }

    async fn document_highlight(
        &self,
        params: DocumentHighlightParams,
    ) -> Result<Option<Vec<DocumentHighlight>>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;

        let source = {
            let docs = self.documents.lock().unwrap();
            match docs.get(&uri) {
                Some(s) => s.clone(),
                None => return Ok(None),
            }
        };

        let mut cache = self.analysis_cache.lock().unwrap();
        let analysis = cache
            .entry(uri.clone())
            .or_insert_with(|| Self::analyze_source(&source));

        let target_offset = Self::position_to_offset(&source, pos);

        let name = match Self::find_ident_at_offset(&source, &analysis.tokens, target_offset) {
            Some(n) => n,
            None => return Ok(None),
        };

        let mut highlights = Vec::new();
        for tok in &analysis.tokens {
            if let TokenKind::Ident(_) = &tok.kind {
                let start = tok.span.start as usize;
                let end = tok.span.end as usize;
                if end <= source.len() && &source[start..end] == name {
                    highlights.push(DocumentHighlight {
                        range: Self::span_to_range(&source, tok.span),
                        kind: Some(DocumentHighlightKind::TEXT),
                    });
                }
            }
        }

        Ok(Some(highlights))
    }

    async fn signature_help(&self, params: SignatureHelpParams) -> Result<Option<SignatureHelp>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;

        let source = {
            let docs = self.documents.lock().unwrap();
            match docs.get(&uri) {
                Some(s) => s.clone(),
                None => return Ok(None),
            }
        };

        let mut cache = self.analysis_cache.lock().unwrap();
        let analysis = cache
            .entry(uri.clone())
            .or_insert_with(|| Self::analyze_source(&source));

        let offset = Self::position_to_offset(&source, pos);
        let before = &source[..offset];
        if let Some(open_paren) = before.rfind('(') {
            let func_name_part = &before[..open_paren].trim_end();
            let name = func_name_part
                .split(|c: char| !c.is_alphanumeric() && c != '_')
                .last()
                .unwrap_or("");

            if let Some(checker) = &analysis.checker {
                if let Some(sig) = checker.fn_sig(name) {
                    let param_infos: Vec<ParameterInformation> = sig
                        .params
                        .iter()
                        .map(|p| ParameterInformation {
                            label: ParameterLabel::Simple(p.to_string()),
                            documentation: None,
                        })
                        .collect();

                    let active_parameter =
                        before[open_paren + 1..].chars().filter(|&c| c == ',').count() as u32;

                    return Ok(Some(SignatureHelp {
                        signatures: vec![SignatureInformation {
                            label: format!(
                                "fn {}({}) -> {}",
                                sig.name,
                                sig.params
                                    .iter()
                                    .map(|p| p.to_string())
                                    .collect::<Vec<_>>()
                                    .join(", "),
                                sig.ret
                            ),
                            documentation: None,
                            parameters: Some(param_infos),
                            active_parameter: Some(active_parameter),
                        }],
                        active_signature: Some(0),
                        active_parameter: Some(active_parameter),
                    }));
                }
            }
        }

        Ok(None)
    }

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> Result<Option<Vec<SymbolInformation>>> {
        let query = params.query.to_lowercase();
        let mut all_symbols = Vec::new();

        let cache = self.analysis_cache.lock().unwrap();
        for (uri, analysis) in cache.iter() {
            let doc_symbols = Self::document_symbols(uri, "", &analysis.module);
            for sym in doc_symbols {
                if sym.name.to_lowercase().contains(&query) {
                    all_symbols.push(sym);
                }
            }
        }

        Ok(Some(all_symbols))
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        let new_name = params.new_name;

        let source = {
            let docs = self.documents.lock().unwrap();
            match docs.get(&uri) {
                Some(s) => s.clone(),
                None => return Ok(None),
            }
        };

        let mut cache = self.analysis_cache.lock().unwrap();
        let analysis = cache
            .entry(uri.clone())
            .or_insert_with(|| Self::analyze_source(&source));

        let target_offset = Self::position_to_offset(&source, pos);

        let old_name = match Self::find_ident_at_offset(&source, &analysis.tokens, target_offset) {
            Some(n) => n,
            None => return Ok(None),
        };

        let mut edits = Vec::new();
        for tok in &analysis.tokens {
            if let TokenKind::Ident(_) = &tok.kind {
                let start = tok.span.start as usize;
                let end = tok.span.end as usize;
                if end <= source.len() && &source[start..end] == old_name {
                    edits.push(TextEdit {
                        range: Self::span_to_range(&source, tok.span),
                        new_text: new_name.clone(),
                    });
                }
            }
        }

        let mut changes = HashMap::new();
        changes.insert(uri, edits);

        Ok(Some(WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        }))
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let uri = params.text_document.uri;
        let pos = params.position;

        let source = {
            let docs = self.documents.lock().unwrap();
            match docs.get(&uri) {
                Some(s) => s.clone(),
                None => return Ok(None),
            }
        };

        let mut cache = self.analysis_cache.lock().unwrap();
        let analysis = cache
            .entry(uri)
            .or_insert_with(|| Self::analyze_source(&source));

        let target_offset = Self::position_to_offset(&source, pos);

        for tok in &analysis.tokens {
            let start = tok.span.start as usize;
            let end = tok.span.end as usize;
            if start <= target_offset && target_offset <= end && end <= source.len() {
                if let TokenKind::Ident(_) = &tok.kind {
                    return Ok(Some(PrepareRenameResponse::Range(Self::span_to_range(
                        &source, tok.span,
                    ))));
                }
            }
        }

        Ok(None)
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = params.text_document.uri;
        let mut actions = Vec::new();

        // Auto-import code action for E0100 unresolved name
        for diag in params.context.diagnostics {
            if let Some(NumberOrString::String(ref code)) = diag.code {
                if code == "E0100" {
                    // Try to suggest an import
                    let name = &diag.message; // Heuristic: diag message might contain the name
                    let import_candidates = [
                        ("stdio", "std::host::stdio"),
                        ("fs", "std::host::fs"),
                        ("env", "std::host::env"),
                        ("Path", "std::path"),
                        ("Time", "std::time"),
                        ("Test", "std::test"),
                    ];
                    for (alias, module) in import_candidates {
                        if name.contains(alias) {
                            let mut changes = HashMap::new();
                            changes.insert(
                                uri.clone(),
                                vec![TextEdit {
                                    range: Range {
                                        start: Position {
                                            line: 0,
                                            character: 0,
                                        },
                                        end: Position {
                                            line: 0,
                                            character: 0,
                                        },
                                    },
                                    new_text: format!("use {}\n", module),
                                }],
                            );
                            actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                                title: format!("Import {}", module),
                                kind: Some(CodeActionKind::QUICKFIX),
                                edit: Some(WorkspaceEdit {
                                    changes: Some(changes),
                                    ..Default::default()
                                }),
                                is_preferred: Some(true),
                                ..Default::default()
                            }));
                        }
                    }
                }
            }
        }

        Ok(Some(actions))
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;

        let source = {
            let docs = self.documents.lock().unwrap();
            match docs.get(&uri) {
                Some(s) => s.clone(),
                None => return Ok(None),
            }
        };

        // Very basic formatter: trim trailing whitespace and ensure newline at EOF
        let lines: Vec<String> = source.lines().map(|l| l.trim_end().to_string()).collect();
        let formatted = if lines.is_empty() {
            "".to_string()
        } else {
            lines.join("\n") + "\n"
        };

        if formatted == source {
            return Ok(None);
        }

        let full_range = Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Self::offset_to_position(&source, source.len() as u32),
        };

        Ok(Some(vec![TextEdit {
            range: full_range,
            new_text: formatted,
        }]))
    }

    async fn code_lens(&self, params: CodeLensParams) -> Result<Option<Vec<CodeLens>>> {
        let uri = params.text_document.uri;

        let source = {
            let docs = self.documents.lock().unwrap();
            match docs.get(&uri) {
                Some(s) => s.clone(),
                None => return Ok(None),
            }
        };

        let mut cache = self.analysis_cache.lock().unwrap();
        let analysis = cache
            .entry(uri)
            .or_insert_with(|| Self::analyze_source(&source));

        let mut lenses = Vec::new();

        for item in &analysis.module.items {
            if let ast::Item::FnDef(f) = item {
                let range = Self::span_to_range(&source, f.span);
                lenses.push(CodeLens {
                    range,
                    command: Some(Command {
                        title: "Open Docs".to_string(),
                        command: "arukellt.openDocs".to_string(),
                        arguments: Some(vec![serde_json::json!(f.name)]),
                    }),
                    data: None,
                });
                lenses.push(CodeLens {
                    range,
                    command: Some(Command {
                        title: "Explain Function".to_string(),
                        command: "arukellt.explainCode".to_string(),
                        arguments: Some(vec![serde_json::json!(f.name)]),
                    }),
                    data: None,
                });
            }
        }

        Ok(Some(lenses))
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;

        let source = {
            let docs = self.documents.lock().unwrap();
            match docs.get(&uri) {
                Some(s) => s.clone(),
                None => return Ok(None),
            }
        };

        let mut cache = self.analysis_cache.lock().unwrap();
        let analysis = cache
            .entry(uri.clone())
            .or_insert_with(|| Self::analyze_source(&source));

        let symbols = Self::document_symbols(&uri, &source, &analysis.module);
        Ok(Some(DocumentSymbolResponse::Flat(symbols)))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;

        let source = {
            let docs = self.documents.lock().unwrap();
            match docs.get(&uri) {
                Some(s) => s.clone(),
                None => return Ok(None),
            }
        };

        let mut cache = self.analysis_cache.lock().unwrap();
        let analysis = cache
            .entry(uri)
            .or_insert_with(|| Self::analyze_source(&source));

        let data = Self::compute_semantic_tokens(&source, &analysis.tokens, &analysis.module);
        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data,
        })))
    }
}

/// Run the LSP server on stdio.
pub async fn run_lsp() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(ArukellBackend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_diagnostics::Span;

    fn empty_module() -> ast::Module {
        ast::Module {
            docs: vec![],
            imports: vec![],
            items: vec![],
        }
    }

    #[test]
    fn completion_includes_auto_import_candidate_for_stdio() {
        let source = "std";
        let tokens = vec![];
        let items = ArukellBackend::get_completions(source, &tokens, &empty_module(), source.len());
        let stdio = items
            .iter()
            .find(|item| item.label == "stdio")
            .expect("stdio completion");
        assert_eq!(stdio.kind, Some(CompletionItemKind::MODULE));
        assert!(
            stdio
                .detail
                .as_deref()
                .unwrap_or_default()
                .contains("auto import candidate")
        );
    }

    #[test]
    fn completion_prefers_string_helpers_in_print_context() {
        let source = "fn main() { println(to_";
        let tokens = vec![];
        let items = ArukellBackend::get_completions(source, &tokens, &empty_module(), source.len());
        assert_eq!(
            items.first().map(|item| item.label.as_str()),
            Some("to_string")
        );
    }

    #[test]
    fn completion_marks_imported_modules_as_already_imported() {
        let source = "use std::host::stdio\nfn main() { std";
        let module = ast::Module {
            docs: vec![],
            imports: vec![ast::Import {
                module_name: "std::host::stdio".to_string(),
                alias: None,
                span: Span::new(0, 0, 16),
            }],
            items: vec![],
        };
        let items = ArukellBackend::get_completions(source, &[], &module, source.len());
        let stdio = items
            .iter()
            .find(|item| item.label == "stdio")
            .expect("stdio completion");
        assert_eq!(stdio.detail.as_deref(), Some("module std::host::stdio"));
    }
}
