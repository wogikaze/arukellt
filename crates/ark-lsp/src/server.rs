//! LSP server implementation.

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use ark_diagnostics::{DiagnosticSink, Severity};
use ark_lexer::{Lexer, TokenKind};
use ark_parser::ast;
use ark_parser::parse;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
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
    /// Project root discovered from ark.toml; None in single-file mode.
    project_root: Mutex<Option<PathBuf>>,
    /// All workspace folder roots provided by the editor (multi-root support).
    workspace_roots: Mutex<Vec<PathBuf>>,
}

impl ArukellBackend {
    fn new(client: Client) -> Self {
        Self {
            client,
            documents: Mutex::new(HashMap::new()),
            analysis_cache: Mutex::new(HashMap::new()),
            project_root: Mutex::new(None),
            workspace_roots: Mutex::new(Vec::new()),
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

    /// Collect inlay hints from a block (recursive).
    fn collect_hints_from_block(
        source: &str,
        block: &ast::Block,
        checker: &ark_typecheck::TypeChecker,
        fn_param_names: &HashMap<String, Vec<String>>,
        range_start: u32,
        range_end: u32,
        hints: &mut Vec<InlayHint>,
    ) {
        for stmt in &block.stmts {
            Self::collect_hints_from_stmt(
                source,
                stmt,
                checker,
                fn_param_names,
                range_start,
                range_end,
                hints,
            );
        }
    }

    /// Collect inlay hints from a single statement.
    fn collect_hints_from_stmt(
        source: &str,
        stmt: &ast::Stmt,
        checker: &ark_typecheck::TypeChecker,
        fn_param_names: &HashMap<String, Vec<String>>,
        range_start: u32,
        range_end: u32,
        hints: &mut Vec<InlayHint>,
    ) {
        match stmt {
            ast::Stmt::Let {
                name,
                ty,
                init,
                span,
                ..
            } => {
                // Type hint for let bindings without explicit annotation.
                if ty.is_none() && span.start >= range_start && span.start <= range_end {
                    if let Some(inferred) =
                        Self::infer_expr_type_simple(init, checker, fn_param_names)
                    {
                        // Position hint right after the binding name.
                        // Find name end: span.start + "let " + name.len()
                        // More robust: search for the name in the source around the span.
                        let name_end = Self::find_let_name_end(source, span.start, name);
                        let pos = Self::offset_to_position(source, name_end);
                        hints.push(InlayHint {
                            position: pos,
                            label: InlayHintLabel::String(format!(": {inferred}")),
                            kind: Some(InlayHintKind::TYPE),
                            text_edits: None,
                            tooltip: None,
                            padding_left: None,
                            padding_right: Some(true),
                            data: None,
                        });
                    }
                }
                // Also recurse into the init expression for call-site param hints.
                Self::collect_hints_from_expr(
                    source,
                    init,
                    checker,
                    fn_param_names,
                    range_start,
                    range_end,
                    hints,
                );
            }
            ast::Stmt::Expr(expr) => {
                Self::collect_hints_from_expr(
                    source,
                    expr,
                    checker,
                    fn_param_names,
                    range_start,
                    range_end,
                    hints,
                );
            }
            ast::Stmt::While { body, cond, .. } => {
                Self::collect_hints_from_expr(
                    source,
                    cond,
                    checker,
                    fn_param_names,
                    range_start,
                    range_end,
                    hints,
                );
                Self::collect_hints_from_block(
                    source,
                    body,
                    checker,
                    fn_param_names,
                    range_start,
                    range_end,
                    hints,
                );
            }
            ast::Stmt::Loop { body, .. } => {
                Self::collect_hints_from_block(
                    source,
                    body,
                    checker,
                    fn_param_names,
                    range_start,
                    range_end,
                    hints,
                );
            }
            ast::Stmt::For { body, .. } => {
                Self::collect_hints_from_block(
                    source,
                    body,
                    checker,
                    fn_param_names,
                    range_start,
                    range_end,
                    hints,
                );
            }
        }
    }

    /// Collect parameter-name inlay hints from expressions (recursive).
    fn collect_hints_from_expr(
        source: &str,
        expr: &ast::Expr,
        checker: &ark_typecheck::TypeChecker,
        fn_param_names: &HashMap<String, Vec<String>>,
        range_start: u32,
        range_end: u32,
        hints: &mut Vec<InlayHint>,
    ) {
        let span = expr.span();
        // Skip expressions entirely outside the visible range.
        if span.end < range_start || span.start > range_end {
            return;
        }

        match expr {
            ast::Expr::Call { callee, args, .. } => {
                // Resolve callee name.
                let callee_name = match callee.as_ref() {
                    ast::Expr::Ident { name, .. } => Some(name.clone()),
                    ast::Expr::FieldAccess { field, object, .. } => {
                        // Method call: try struct_name::method via method_resolution.
                        if let ast::Expr::Ident { name: obj_name, .. } = object.as_ref() {
                            // Check if this is a method on a known type.
                            checker
                                .method_resolution(object.span().start)
                                .map(|(struct_name, _)| format!("{struct_name}::{field}"))
                                .or_else(|| Some(format!("{obj_name}::{field}")))
                        } else {
                            None
                        }
                    }
                    ast::Expr::QualifiedIdent { name, .. } => Some(name.clone()),
                    _ => None,
                };

                if let Some(ref name) = callee_name {
                    if let Some(param_names) = fn_param_names.get(name) {
                        // Skip single-param functions and skip if arg is already named
                        // or is a simple identifier matching the param name.
                        for (i, arg) in args.iter().enumerate() {
                            if i >= param_names.len() {
                                break;
                            }
                            let pname = &param_names[i];
                            // Don't show hint if the argument is already the same name.
                            if Self::arg_matches_param_name(arg, pname) {
                                continue;
                            }
                            // Don't show hints for "self" parameters.
                            if pname == "self" {
                                continue;
                            }
                            let pos = Self::offset_to_position(source, arg.span().start);
                            hints.push(InlayHint {
                                position: pos,
                                label: InlayHintLabel::String(format!("{pname}:")),
                                kind: Some(InlayHintKind::PARAMETER),
                                text_edits: None,
                                tooltip: None,
                                padding_left: None,
                                padding_right: Some(true),
                                data: None,
                            });
                        }
                    }
                }

                // Recurse into callee and args.
                Self::collect_hints_from_expr(
                    source,
                    callee,
                    checker,
                    fn_param_names,
                    range_start,
                    range_end,
                    hints,
                );
                for arg in args {
                    Self::collect_hints_from_expr(
                        source,
                        arg,
                        checker,
                        fn_param_names,
                        range_start,
                        range_end,
                        hints,
                    );
                }
            }
            ast::Expr::If {
                cond,
                then_block,
                else_block,
                ..
            } => {
                Self::collect_hints_from_expr(
                    source,
                    cond,
                    checker,
                    fn_param_names,
                    range_start,
                    range_end,
                    hints,
                );
                Self::collect_hints_from_block(
                    source,
                    then_block,
                    checker,
                    fn_param_names,
                    range_start,
                    range_end,
                    hints,
                );
                if let Some(eb) = else_block {
                    Self::collect_hints_from_block(
                        source,
                        eb,
                        checker,
                        fn_param_names,
                        range_start,
                        range_end,
                        hints,
                    );
                }
            }
            ast::Expr::Match {
                scrutinee, arms, ..
            } => {
                Self::collect_hints_from_expr(
                    source,
                    scrutinee,
                    checker,
                    fn_param_names,
                    range_start,
                    range_end,
                    hints,
                );
                for arm in arms {
                    Self::collect_hints_from_expr(
                        source,
                        &arm.body,
                        checker,
                        fn_param_names,
                        range_start,
                        range_end,
                        hints,
                    );
                }
            }
            ast::Expr::Binary { left, right, .. } => {
                Self::collect_hints_from_expr(
                    source,
                    left,
                    checker,
                    fn_param_names,
                    range_start,
                    range_end,
                    hints,
                );
                Self::collect_hints_from_expr(
                    source,
                    right,
                    checker,
                    fn_param_names,
                    range_start,
                    range_end,
                    hints,
                );
            }
            ast::Expr::Unary { operand, .. } => {
                Self::collect_hints_from_expr(
                    source,
                    operand,
                    checker,
                    fn_param_names,
                    range_start,
                    range_end,
                    hints,
                );
            }
            ast::Expr::Block(block) => {
                Self::collect_hints_from_block(
                    source,
                    block,
                    checker,
                    fn_param_names,
                    range_start,
                    range_end,
                    hints,
                );
            }
            ast::Expr::Return { value: Some(v), .. } => {
                Self::collect_hints_from_expr(
                    source,
                    v,
                    checker,
                    fn_param_names,
                    range_start,
                    range_end,
                    hints,
                );
            }
            ast::Expr::Closure { body, .. } => {
                Self::collect_hints_from_expr(
                    source,
                    body,
                    checker,
                    fn_param_names,
                    range_start,
                    range_end,
                    hints,
                );
            }
            ast::Expr::Index { object, index, .. } => {
                Self::collect_hints_from_expr(
                    source,
                    object,
                    checker,
                    fn_param_names,
                    range_start,
                    range_end,
                    hints,
                );
                Self::collect_hints_from_expr(
                    source,
                    index,
                    checker,
                    fn_param_names,
                    range_start,
                    range_end,
                    hints,
                );
            }
            ast::Expr::FieldAccess { object, .. } => {
                Self::collect_hints_from_expr(
                    source,
                    object,
                    checker,
                    fn_param_names,
                    range_start,
                    range_end,
                    hints,
                );
            }
            ast::Expr::Assign { target, value, .. } => {
                Self::collect_hints_from_expr(
                    source,
                    target,
                    checker,
                    fn_param_names,
                    range_start,
                    range_end,
                    hints,
                );
                Self::collect_hints_from_expr(
                    source,
                    value,
                    checker,
                    fn_param_names,
                    range_start,
                    range_end,
                    hints,
                );
            }
            ast::Expr::Tuple { elements, .. } | ast::Expr::Array { elements, .. } => {
                for e in elements {
                    Self::collect_hints_from_expr(
                        source,
                        e,
                        checker,
                        fn_param_names,
                        range_start,
                        range_end,
                        hints,
                    );
                }
            }
            ast::Expr::StructInit { fields, base, .. } => {
                for (_, val) in fields {
                    Self::collect_hints_from_expr(
                        source,
                        val,
                        checker,
                        fn_param_names,
                        range_start,
                        range_end,
                        hints,
                    );
                }
                if let Some(b) = base {
                    Self::collect_hints_from_expr(
                        source,
                        b,
                        checker,
                        fn_param_names,
                        range_start,
                        range_end,
                        hints,
                    );
                }
            }
            ast::Expr::Try { expr: inner, .. }
            | ast::Expr::Break {
                value: Some(inner), ..
            } => {
                Self::collect_hints_from_expr(
                    source,
                    inner,
                    checker,
                    fn_param_names,
                    range_start,
                    range_end,
                    hints,
                );
            }
            ast::Expr::Loop { body, .. } => {
                Self::collect_hints_from_block(
                    source,
                    body,
                    checker,
                    fn_param_names,
                    range_start,
                    range_end,
                    hints,
                );
            }
            ast::Expr::ArrayRepeat { value, count, .. } => {
                Self::collect_hints_from_expr(
                    source,
                    value,
                    checker,
                    fn_param_names,
                    range_start,
                    range_end,
                    hints,
                );
                Self::collect_hints_from_expr(
                    source,
                    count,
                    checker,
                    fn_param_names,
                    range_start,
                    range_end,
                    hints,
                );
            }
            // Leaf expressions: no sub-expressions to recurse into.
            _ => {}
        }
    }

    /// Try to infer a simple type string from an expression without full
    /// type-checking env. Covers the common cases that produce useful hints.
    #[allow(clippy::only_used_in_recursion)]
    fn infer_expr_type_simple(
        expr: &ast::Expr,
        checker: &ark_typecheck::TypeChecker,
        fn_param_names: &HashMap<String, Vec<String>>,
    ) -> Option<String> {
        match expr {
            ast::Expr::IntLit { suffix, .. } => {
                let ty = suffix.as_deref().unwrap_or("i32");
                Some(ty.to_string())
            }
            ast::Expr::FloatLit { suffix, .. } => {
                let ty = suffix.as_deref().unwrap_or("f64");
                Some(ty.to_string())
            }
            ast::Expr::StringLit { .. } => Some("String".to_string()),
            ast::Expr::CharLit { .. } => Some("char".to_string()),
            ast::Expr::BoolLit { .. } => Some("bool".to_string()),
            ast::Expr::Call { callee, .. } => {
                let callee_name = match callee.as_ref() {
                    ast::Expr::Ident { name, .. } => Some(name.clone()),
                    ast::Expr::QualifiedIdent { name, .. } => Some(name.clone()),
                    _ => None,
                };
                callee_name.and_then(|name| checker.fn_sig(&name).map(|sig| format!("{}", sig.ret)))
            }
            ast::Expr::StructInit { name, .. } => Some(name.clone()),
            ast::Expr::Array { elements, .. } => {
                if let Some(first) = elements.first() {
                    let elem_ty = Self::infer_expr_type_simple(first, checker, fn_param_names)
                        .unwrap_or_else(|| "?".to_string());
                    Some(format!("[{}; {}]", elem_ty, elements.len()))
                } else {
                    Some("[?; 0]".to_string())
                }
            }
            ast::Expr::Tuple { elements, .. } => {
                let types: Vec<String> = elements
                    .iter()
                    .map(|e| {
                        Self::infer_expr_type_simple(e, checker, fn_param_names)
                            .unwrap_or_else(|| "?".to_string())
                    })
                    .collect();
                Some(format!("({})", types.join(", ")))
            }
            _ => None,
        }
    }

    /// Find the byte offset right after the binding name in a `let` statement.
    fn find_let_name_end(source: &str, span_start: u32, name: &str) -> u32 {
        let start = span_start as usize;
        // Search for the binding name after "let" / "let mut".
        if let Some(slice) = source.get(start..) {
            // Skip "let " or "let mut ".
            if let Some(name_pos) = slice.find(name) {
                return (start + name_pos + name.len()) as u32;
            }
        }
        // Fallback: just after span_start + reasonable offset.
        span_start + 4 + name.len() as u32
    }

    /// Check if an argument expression is a simple identifier matching the
    /// parameter name (in which case showing the hint would be redundant).
    fn arg_matches_param_name(arg: &ast::Expr, param_name: &str) -> bool {
        matches!(arg, ast::Expr::Ident { name, .. } if name == param_name)
    }

    /// Build nested selection ranges for a cursor offset, from innermost to
    /// outermost AST node.
    fn collect_selection_ranges(source: &str, module: &ast::Module, offset: u32) -> SelectionRange {
        // Collect all spans that contain the offset (from outermost to innermost).
        let mut containing_spans: Vec<ark_diagnostics::Span> = Vec::new();

        for item in &module.items {
            match item {
                ast::Item::FnDef(f) => {
                    if f.span.start <= offset && offset <= f.span.end {
                        containing_spans.push(f.span);
                        Self::collect_containing_spans_block(
                            &f.body,
                            offset,
                            &mut containing_spans,
                        );
                    }
                }
                ast::Item::StructDef(s) => {
                    if s.span.start <= offset && offset <= s.span.end {
                        containing_spans.push(s.span);
                    }
                }
                ast::Item::EnumDef(e) => {
                    if e.span.start <= offset && offset <= e.span.end {
                        containing_spans.push(e.span);
                    }
                }
                ast::Item::ImplBlock(ib) => {
                    if ib.span.start <= offset && offset <= ib.span.end {
                        containing_spans.push(ib.span);
                        for m in &ib.methods {
                            if m.span.start <= offset && offset <= m.span.end {
                                containing_spans.push(m.span);
                                Self::collect_containing_spans_block(
                                    &m.body,
                                    offset,
                                    &mut containing_spans,
                                );
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // Always add the whole-file range as outermost.
        let file_end = source.len() as u32;
        let file_range = Self::span_to_range(
            source,
            ark_diagnostics::Span {
                file_id: 0,
                start: 0,
                end: file_end,
            },
        );

        // Build the chain from outermost to innermost.
        let mut current = SelectionRange {
            range: file_range,
            parent: None,
        };

        for span in &containing_spans {
            let range = Self::span_to_range(source, *span);
            current = SelectionRange {
                range,
                parent: Some(Box::new(current)),
            };
        }

        current
    }

    /// Recursively collect spans of AST nodes containing the offset.
    fn collect_containing_spans_block(
        block: &ast::Block,
        offset: u32,
        spans: &mut Vec<ark_diagnostics::Span>,
    ) {
        if block.span.start <= offset && offset <= block.span.end {
            spans.push(block.span);
        }
        for stmt in &block.stmts {
            Self::collect_containing_spans_stmt(stmt, offset, spans);
        }
    }

    fn collect_containing_spans_stmt(
        stmt: &ast::Stmt,
        offset: u32,
        spans: &mut Vec<ark_diagnostics::Span>,
    ) {
        match stmt {
            ast::Stmt::Let { init, span, .. } => {
                if span.start <= offset && offset <= span.end {
                    spans.push(*span);
                    Self::collect_containing_spans_expr(init, offset, spans);
                }
            }
            ast::Stmt::Expr(expr) => {
                Self::collect_containing_spans_expr(expr, offset, spans);
            }
            ast::Stmt::While {
                cond, body, span, ..
            } => {
                if span.start <= offset && offset <= span.end {
                    spans.push(*span);
                    Self::collect_containing_spans_expr(cond, offset, spans);
                    Self::collect_containing_spans_block(body, offset, spans);
                }
            }
            ast::Stmt::Loop { body, span, .. } => {
                if span.start <= offset && offset <= span.end {
                    spans.push(*span);
                    Self::collect_containing_spans_block(body, offset, spans);
                }
            }
            ast::Stmt::For { body, span, .. } => {
                if span.start <= offset && offset <= span.end {
                    spans.push(*span);
                    Self::collect_containing_spans_block(body, offset, spans);
                }
            }
        }
    }

    fn collect_containing_spans_expr(
        expr: &ast::Expr,
        offset: u32,
        spans: &mut Vec<ark_diagnostics::Span>,
    ) {
        let span = expr.span();
        if span.start > offset || offset > span.end {
            return;
        }
        spans.push(span);

        match expr {
            ast::Expr::Call { callee, args, .. } => {
                Self::collect_containing_spans_expr(callee, offset, spans);
                for arg in args {
                    Self::collect_containing_spans_expr(arg, offset, spans);
                }
            }
            ast::Expr::If {
                cond,
                then_block,
                else_block,
                ..
            } => {
                Self::collect_containing_spans_expr(cond, offset, spans);
                Self::collect_containing_spans_block(then_block, offset, spans);
                if let Some(eb) = else_block {
                    Self::collect_containing_spans_block(eb, offset, spans);
                }
            }
            ast::Expr::Match {
                scrutinee, arms, ..
            } => {
                Self::collect_containing_spans_expr(scrutinee, offset, spans);
                for arm in arms {
                    Self::collect_containing_spans_expr(&arm.body, offset, spans);
                }
            }
            ast::Expr::Binary { left, right, .. } => {
                Self::collect_containing_spans_expr(left, offset, spans);
                Self::collect_containing_spans_expr(right, offset, spans);
            }
            ast::Expr::Unary { operand, .. } => {
                Self::collect_containing_spans_expr(operand, offset, spans);
            }
            ast::Expr::Block(block) => {
                Self::collect_containing_spans_block(block, offset, spans);
            }
            ast::Expr::Return { value: Some(v), .. } => {
                Self::collect_containing_spans_expr(v, offset, spans);
            }
            ast::Expr::FieldAccess { object, .. } => {
                Self::collect_containing_spans_expr(object, offset, spans);
            }
            ast::Expr::Index { object, index, .. } => {
                Self::collect_containing_spans_expr(object, offset, spans);
                Self::collect_containing_spans_expr(index, offset, spans);
            }
            ast::Expr::Closure { body, .. } => {
                Self::collect_containing_spans_expr(body, offset, spans);
            }
            ast::Expr::Tuple { elements, .. } | ast::Expr::Array { elements, .. } => {
                for e in elements {
                    Self::collect_containing_spans_expr(e, offset, spans);
                }
            }
            ast::Expr::StructInit { fields, base, .. } => {
                for (_, val) in fields {
                    Self::collect_containing_spans_expr(val, offset, spans);
                }
                if let Some(b) = base {
                    Self::collect_containing_spans_expr(b, offset, spans);
                }
            }
            ast::Expr::Assign { target, value, .. } => {
                Self::collect_containing_spans_expr(target, offset, spans);
                Self::collect_containing_spans_expr(value, offset, spans);
            }
            ast::Expr::Try { expr: inner, .. } => {
                Self::collect_containing_spans_expr(inner, offset, spans);
            }
            ast::Expr::Loop { body, .. } => {
                Self::collect_containing_spans_block(body, offset, spans);
            }
            _ => {}
        }
    }

    // --- go-to-type-definition helpers ---

    /// Resolve the type name for the identifier at `name` by examining let bindings and params.
    fn resolve_type_name_for_ident(module: &ast::Module, name: &str) -> Option<String> {
        for item in &module.items {
            if let ast::Item::FnDef(f) = item {
                // Check params
                for p in &f.params {
                    if p.name == name {
                        return Self::type_expr_root_name(&p.ty);
                    }
                }
                // Check let bindings in body
                if let Some(ty_name) = Self::find_let_type_name_in_block(&f.body, name) {
                    return Some(ty_name);
                }
            }
        }
        None
    }

    /// Extract the root type name from a TypeExpr (e.g. `Vec<i32>` → `Vec`, `MyStruct` → `MyStruct`).
    fn type_expr_root_name(ty: &ast::TypeExpr) -> Option<String> {
        match ty {
            ast::TypeExpr::Named { name, .. } => Some(name.clone()),
            ast::TypeExpr::Generic { name, .. } => Some(name.clone()),
            ast::TypeExpr::Qualified { name, .. } => Some(name.clone()),
            _ => None,
        }
    }

    /// Search a block for a let binding with `name` and return the root type name.
    fn find_let_type_name_in_block(block: &ast::Block, name: &str) -> Option<String> {
        for stmt in &block.stmts {
            match stmt {
                ast::Stmt::Let {
                    name: n, ty, init, ..
                } if n == name => {
                    if let Some(ty_expr) = ty {
                        return Self::type_expr_root_name(ty_expr);
                    }
                    return Self::infer_type_name_from_expr(init);
                }
                ast::Stmt::While { body, .. }
                | ast::Stmt::Loop { body, .. }
                | ast::Stmt::For { body, .. } => {
                    if let Some(t) = Self::find_let_type_name_in_block(body, name) {
                        return Some(t);
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Infer a type name from an expression (e.g. StructInit name, Call callee name).
    fn infer_type_name_from_expr(expr: &ast::Expr) -> Option<String> {
        match expr {
            ast::Expr::StructInit { name, .. } => Some(name.clone()),
            ast::Expr::Call { callee, .. } => {
                if let ast::Expr::Ident { name, .. } = callee.as_ref() {
                    // Constructor-style: `MyType(...)` or `MyType::new(...)`
                    Some(name.clone())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Find the definition span of a type by name (struct, enum, trait).
    fn find_type_definition_span(
        module: &ast::Module,
        type_name: &str,
    ) -> Option<ark_diagnostics::Span> {
        for item in &module.items {
            match item {
                ast::Item::StructDef(s) if s.name == type_name => return Some(s.span),
                ast::Item::EnumDef(e) if e.name == type_name => return Some(e.span),
                ast::Item::TraitDef(t) if t.name == type_name => return Some(t.span),
                _ => {}
            }
        }
        None
    }

    // --- call hierarchy helpers ---

    /// Find the function definition that contains the given offset.
    fn find_fn_at_offset(module: &ast::Module, offset: u32) -> Option<&ast::FnDef> {
        for item in &module.items {
            match item {
                ast::Item::FnDef(f) => {
                    if offset >= f.span.start && offset < f.span.end {
                        return Some(f);
                    }
                }
                ast::Item::ImplBlock(ib) => {
                    for m in &ib.methods {
                        if offset >= m.span.start && offset < m.span.end {
                            return Some(m);
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Collect all call expressions in a block, returning (callee_name, call_span).
    fn collect_calls_in_block(block: &ast::Block) -> Vec<(String, ark_diagnostics::Span)> {
        let mut calls = Vec::new();
        for stmt in &block.stmts {
            Self::collect_calls_in_stmt(stmt, &mut calls);
        }
        if let Some(tail) = &block.tail_expr {
            Self::collect_calls_in_expr(tail, &mut calls);
        }
        calls
    }

    fn collect_calls_in_stmt(stmt: &ast::Stmt, calls: &mut Vec<(String, ark_diagnostics::Span)>) {
        match stmt {
            ast::Stmt::Let { init, .. } => Self::collect_calls_in_expr(init, calls),
            ast::Stmt::Expr(e) => Self::collect_calls_in_expr(e, calls),
            ast::Stmt::While { cond, body, .. } => {
                Self::collect_calls_in_expr(cond, calls);
                calls.extend(Self::collect_calls_in_block(body));
            }
            ast::Stmt::Loop { body, .. } => {
                calls.extend(Self::collect_calls_in_block(body));
            }
            ast::Stmt::For { body, .. } => {
                calls.extend(Self::collect_calls_in_block(body));
            }
        }
    }

    fn collect_calls_in_expr(expr: &ast::Expr, calls: &mut Vec<(String, ark_diagnostics::Span)>) {
        match expr {
            ast::Expr::Call {
                callee, args, span, ..
            } => {
                if let ast::Expr::Ident { name, .. } = callee.as_ref() {
                    calls.push((name.clone(), *span));
                } else if let ast::Expr::FieldAccess { field, .. } = callee.as_ref() {
                    calls.push((field.clone(), *span));
                }
                Self::collect_calls_in_expr(callee, calls);
                for arg in args {
                    Self::collect_calls_in_expr(arg, calls);
                }
            }
            ast::Expr::Binary { left, right, .. } => {
                Self::collect_calls_in_expr(left, calls);
                Self::collect_calls_in_expr(right, calls);
            }
            ast::Expr::Unary { operand, .. } => {
                Self::collect_calls_in_expr(operand, calls);
            }
            ast::Expr::If {
                cond,
                then_block,
                else_block,
                ..
            } => {
                Self::collect_calls_in_expr(cond, calls);
                calls.extend(Self::collect_calls_in_block(then_block));
                if let Some(eb) = else_block {
                    calls.extend(Self::collect_calls_in_block(eb));
                }
            }
            ast::Expr::Match {
                scrutinee, arms, ..
            } => {
                Self::collect_calls_in_expr(scrutinee, calls);
                for arm in arms {
                    Self::collect_calls_in_expr(&arm.body, calls);
                }
            }
            ast::Expr::Block(block) => {
                calls.extend(Self::collect_calls_in_block(block));
            }
            ast::Expr::FieldAccess { object, .. } => {
                Self::collect_calls_in_expr(object, calls);
            }
            ast::Expr::Index { object, index, .. } => {
                Self::collect_calls_in_expr(object, calls);
                Self::collect_calls_in_expr(index, calls);
            }
            ast::Expr::StructInit { fields, base, .. } => {
                for (_, val) in fields {
                    Self::collect_calls_in_expr(val, calls);
                }
                if let Some(b) = base {
                    Self::collect_calls_in_expr(b, calls);
                }
            }
            ast::Expr::Array { elements, .. } | ast::Expr::Tuple { elements, .. } => {
                for e in elements {
                    Self::collect_calls_in_expr(e, calls);
                }
            }
            ast::Expr::Return { value, .. } | ast::Expr::Break { value, .. } => {
                if let Some(v) = value {
                    Self::collect_calls_in_expr(v, calls);
                }
            }
            ast::Expr::Assign { target, value, .. } => {
                Self::collect_calls_in_expr(target, calls);
                Self::collect_calls_in_expr(value, calls);
            }
            ast::Expr::Closure { body, .. } => {
                Self::collect_calls_in_expr(body, calls);
            }
            ast::Expr::Try { expr, .. } => {
                Self::collect_calls_in_expr(expr, calls);
            }
            ast::Expr::ArrayRepeat { value, count, .. } => {
                Self::collect_calls_in_expr(value, calls);
                Self::collect_calls_in_expr(count, calls);
            }
            ast::Expr::Loop { body, .. } => {
                calls.extend(Self::collect_calls_in_block(body));
            }
            _ => {}
        }
    }

    /// Build a CallHierarchyItem for a function definition.
    fn fn_to_call_hierarchy_item(source: &str, uri: &Url, f: &ast::FnDef) -> CallHierarchyItem {
        let range = Self::span_to_range(source, f.span);
        let name_end = f.span.start + f.name.len() as u32 + 3; // "fn " prefix
        let selection_range = Self::span_to_range(
            source,
            ark_diagnostics::Span {
                file_id: 0,
                start: f.span.start,
                end: name_end.min(f.span.end),
            },
        );
        CallHierarchyItem {
            name: f.name.clone(),
            kind: SymbolKind::FUNCTION,
            tags: None,
            detail: None,
            uri: uri.clone(),
            range,
            selection_range,
            data: None,
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for ArukellBackend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        // Resolve workspace roots from all workspace folders (multi-root support).
        // Store all workspace folders so per-file project resolution can find
        // the best matching root, mirroring what `arukellt build` does via
        // `Manifest::find_and_load`.
        let mut ws_roots = Vec::new();
        if let Some(folders) = params.workspace_folders.as_deref() {
            for wf in folders {
                if let Ok(path) = wf.uri.to_file_path() {
                    ws_roots.push(path);
                }
            }
        }
        if ws_roots.is_empty() {
            // Fallback to deprecated root_uri / root_path
            let start_dir: Option<PathBuf> = {
                #[allow(deprecated)]
                params
                    .root_uri
                    .as_ref()
                    .and_then(|u| u.to_file_path().ok())
                    .or_else(|| {
                        #[allow(deprecated)]
                        params.root_path.as_deref().map(PathBuf::from)
                    })
            };
            if let Some(dir) = start_dir {
                ws_roots.push(dir);
            }
        }

        // Set primary project root from first workspace folder
        if let Some(dir) = ws_roots.first() {
            match ark_manifest::Manifest::find_root(dir) {
                Some(root) => {
                    *self.project_root.lock().unwrap() = Some(root);
                }
                None => {
                    // Single-file mode: no ark.toml found, operate on individual files.
                }
            }
        }

        *self.workspace_roots.lock().unwrap() = ws_roots;

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
                signature_help_provider: Some(SignatureHelpOptions {
                    trigger_characters: Some(vec!["(".to_string(), ",".to_string()]),
                    ..Default::default()
                }),
                document_highlight_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: Default::default(),
                })),
                code_action_provider: Some(CodeActionProviderCapability::Options(
                    CodeActionOptions {
                        code_action_kinds: Some(vec![
                            CodeActionKind::QUICKFIX,
                            CodeActionKind::SOURCE,
                            CodeActionKind::new("source.organizeImports"),
                            CodeActionKind::new("source.fixAll"),
                            CodeActionKind::REFACTOR_EXTRACT,
                        ]),
                        ..Default::default()
                    },
                )),
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
                inlay_hint_provider: Some(OneOf::Left(true)),
                folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
                selection_range_provider: Some(SelectionRangeProviderCapability::Simple(true)),
                type_definition_provider: Some(TypeDefinitionProviderCapability::Simple(true)),
                call_hierarchy_provider: Some(CallHierarchyServerCapability::Simple(true)),
                implementation_provider: Some(ImplementationProviderCapability::Simple(true)),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "arukellt-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        let root_msg = match self.project_root.lock().unwrap().as_deref() {
            Some(root) => format!("project root: {}", root.display()),
            None => "single-file mode (no ark.toml found)".to_string(),
        };
        self.client
            .log_message(
                MessageType::INFO,
                format!("arukellt LSP server initialized — {root_msg}"),
            )
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    /// Re-resolve the project root when ark.toml changes on disk so that
    /// the LSP stays aligned with the CLI (which re-reads ark.toml on each
    /// invocation via `Manifest::find_and_load`).
    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        let manifest_changed = params
            .changes
            .iter()
            .any(|c| c.uri.path().ends_with("ark.toml"));
        if manifest_changed {
            let current_root = self.project_root.lock().unwrap().clone();
            if let Some(root) = current_root {
                // Re-resolve from the same root to pick up changes.
                match ark_manifest::Manifest::find_root(&root) {
                    Some(new_root) => {
                        *self.project_root.lock().unwrap() = Some(new_root.clone());
                        self.client
                            .log_message(
                                MessageType::INFO,
                                format!(
                                    "ark.toml changed — project root reloaded: {}",
                                    new_root.display()
                                ),
                            )
                            .await;
                    }
                    None => {
                        *self.project_root.lock().unwrap() = None;
                        self.client
                            .log_message(
                                MessageType::INFO,
                                "ark.toml removed — switched to single-file mode",
                            )
                            .await;
                    }
                }
            }
        }
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
                .next_back()
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

                    let active_parameter = before[open_paren + 1..]
                        .chars()
                        .filter(|&c| c == ',')
                        .count() as u32;

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

        let source = {
            let docs = self.documents.lock().unwrap();
            docs.get(&uri).cloned()
        };

        // --- Quick fix code actions (diagnostic-driven) ---
        for diag in &params.context.diagnostics {
            if let Some(NumberOrString::String(ref code)) = diag.code {
                if code == "E0100" {
                    // Auto-import for unresolved name
                    let name = &diag.message;
                    let import_candidates = [
                        ("stdio", "std::host::stdio"),
                        ("fs", "std::host::fs"),
                        ("env", "std::host::env"),
                        ("Path", "std::path"),
                        ("Time", "std::time"),
                        ("Test", "std::test"),
                        ("math", "std::math"),
                        ("string", "std::string"),
                        ("collections", "std::collections"),
                        ("process", "std::host::process"),
                        ("clock", "std::host::clock"),
                        ("random", "std::host::random"),
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
                                diagnostics: Some(vec![diag.clone()]),
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

        // --- Source actions ---

        // source.organizeImports — sort imports (stdlib first, then external)
        if let Some(ref src) = source {
            if params.context.only.as_ref().is_none_or(|kinds| {
                kinds.iter().any(|k| {
                    k == &CodeActionKind::SOURCE || k.as_str().starts_with("source.organizeImports")
                })
            }) {
                let formatted = ark_parser::fmt::format_source(src);
                if let Some(formatted) = formatted {
                    if formatted != *src {
                        let full_range = Range {
                            start: Position {
                                line: 0,
                                character: 0,
                            },
                            end: Self::offset_to_position(src, src.len() as u32),
                        };
                        let mut changes = HashMap::new();
                        changes.insert(
                            uri.clone(),
                            vec![TextEdit {
                                range: full_range,
                                new_text: formatted,
                            }],
                        );
                        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                            title: "Organize Imports".to_string(),
                            kind: Some(CodeActionKind::new("source.organizeImports")),
                            edit: Some(WorkspaceEdit {
                                changes: Some(changes),
                                ..Default::default()
                            }),
                            ..Default::default()
                        }));
                    }
                }
            }

            // source.fixAll — apply formatter as a fix-all
            if params.context.only.as_ref().is_none_or(|kinds| {
                kinds
                    .iter()
                    .any(|k| k == &CodeActionKind::SOURCE || k.as_str() == "source.fixAll")
            }) {
                let formatted = ark_parser::fmt::format_source(src);
                if let Some(formatted) = formatted {
                    if formatted != *src {
                        let full_range = Range {
                            start: Position {
                                line: 0,
                                character: 0,
                            },
                            end: Self::offset_to_position(src, src.len() as u32),
                        };
                        let mut changes = HashMap::new();
                        changes.insert(
                            uri.clone(),
                            vec![TextEdit {
                                range: full_range,
                                new_text: formatted,
                            }],
                        );
                        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                            title: "Fix All".to_string(),
                            kind: Some(CodeActionKind::new("source.fixAll")),
                            edit: Some(WorkspaceEdit {
                                changes: Some(changes),
                                ..Default::default()
                            }),
                            ..Default::default()
                        }));
                    }
                }
            }
        }

        // --- Refactor: extract variable ---
        if let Some(ref src) = source {
            let sel = params.range;
            // Only offer extract variable if there's a non-empty selection
            if sel.start != sel.end {
                let start_off = Self::position_to_offset(src, sel.start);
                let end_off = Self::position_to_offset(src, sel.end);
                if end_off > start_off {
                    let selected_text = &src[start_off..end_off.min(src.len())];
                    let trimmed = selected_text.trim();
                    // Only offer if the selection looks like an expression (not empty, no newlines)
                    if !trimmed.is_empty() && !trimmed.contains('\n') {
                        let var_name = "extracted";
                        let let_text = format!("let {} = {}\n", var_name, trimmed);
                        // Find the line start of the selection
                        let line_start = src[..start_off].rfind('\n').map(|p| p + 1).unwrap_or(0);
                        let indent_str: String = src[line_start..start_off]
                            .chars()
                            .take_while(|c| c.is_whitespace())
                            .collect();

                        let insert_pos = Self::offset_to_position(src, line_start as u32);

                        let mut changes = HashMap::new();
                        changes.insert(
                            uri.clone(),
                            vec![
                                // Insert let binding before the current line
                                TextEdit {
                                    range: Range {
                                        start: insert_pos,
                                        end: insert_pos,
                                    },
                                    new_text: format!("{}{}", indent_str, let_text),
                                },
                                // Replace selected expression with variable name
                                TextEdit {
                                    range: sel,
                                    new_text: var_name.to_string(),
                                },
                            ],
                        );
                        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                            title: format!("Extract to variable '{}'", var_name),
                            kind: Some(CodeActionKind::REFACTOR_EXTRACT),
                            edit: Some(WorkspaceEdit {
                                changes: Some(changes),
                                ..Default::default()
                            }),
                            ..Default::default()
                        }));
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

        // AST-based formatter — skip if source has parse errors
        let formatted = match ark_parser::fmt::format_source(&source) {
            Some(f) => f,
            None => return Ok(None),
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

    async fn inlay_hint(&self, params: InlayHintParams) -> Result<Option<Vec<InlayHint>>> {
        let uri = params.text_document.uri;
        let docs = self.documents.lock().unwrap();
        let source = match docs.get(&uri) {
            Some(s) => s.clone(),
            None => return Ok(None),
        };
        drop(docs);

        let mut cache = self.analysis_cache.lock().unwrap();
        let analysis = cache
            .entry(uri)
            .or_insert_with(|| Self::analyze_source(&source));

        let checker = match &analysis.checker {
            Some(c) => c,
            None => return Ok(None),
        };

        // Build fn-name → param-names map from AST (FnSig only has types).
        let mut fn_param_names: HashMap<String, Vec<String>> = HashMap::new();
        for item in &analysis.module.items {
            match item {
                ast::Item::FnDef(f) => {
                    fn_param_names.insert(
                        f.name.clone(),
                        f.params.iter().map(|p| p.name.clone()).collect(),
                    );
                }
                ast::Item::ImplBlock(ib) => {
                    for m in &ib.methods {
                        // Methods are stored with mangled names in TypeChecker.
                        let mangled = format!("{}::{}", ib.target_type, m.name);
                        fn_param_names
                            .insert(mangled, m.params.iter().map(|p| p.name.clone()).collect());
                    }
                }
                _ => {}
            }
        }

        let range_start = Self::position_to_offset(&source, params.range.start) as u32;
        let range_end = Self::position_to_offset(&source, params.range.end) as u32;

        let mut hints = Vec::new();

        // Walk all items and collect hints within the requested range.
        for item in &analysis.module.items {
            match item {
                ast::Item::FnDef(f) => {
                    Self::collect_hints_from_block(
                        &source,
                        &f.body,
                        checker,
                        &fn_param_names,
                        range_start,
                        range_end,
                        &mut hints,
                    );
                }
                ast::Item::ImplBlock(ib) => {
                    for m in &ib.methods {
                        Self::collect_hints_from_block(
                            &source,
                            &m.body,
                            checker,
                            &fn_param_names,
                            range_start,
                            range_end,
                            &mut hints,
                        );
                    }
                }
                _ => {}
            }
        }

        if hints.is_empty() {
            Ok(None)
        } else {
            Ok(Some(hints))
        }
    }

    async fn folding_range(&self, params: FoldingRangeParams) -> Result<Option<Vec<FoldingRange>>> {
        let uri = params.text_document.uri;
        let docs = self.documents.lock().unwrap();
        let source = match docs.get(&uri) {
            Some(s) => s.clone(),
            None => return Ok(None),
        };
        drop(docs);

        let cache = self.analysis_cache.lock().unwrap();
        match cache.get(&uri) {
            Some(a) => {
                let mut ranges = Vec::new();
                for item in &a.module.items {
                    match item {
                        ark_parser::ast::Item::FnDef(f) => {
                            let start = Self::offset_to_position(&source, f.span.start);
                            let end = Self::offset_to_position(&source, f.span.end);
                            if end.line > start.line {
                                ranges.push(FoldingRange {
                                    start_line: start.line,
                                    start_character: Some(start.character),
                                    end_line: end.line,
                                    end_character: Some(end.character),
                                    kind: Some(FoldingRangeKind::Region),
                                    collapsed_text: Some(format!("fn {}(...) {{ ... }}", f.name)),
                                });
                            }
                        }
                        ark_parser::ast::Item::StructDef(s) => {
                            let start = Self::offset_to_position(&source, s.span.start);
                            let end = Self::offset_to_position(&source, s.span.end);
                            if end.line > start.line {
                                ranges.push(FoldingRange {
                                    start_line: start.line,
                                    start_character: Some(start.character),
                                    end_line: end.line,
                                    end_character: Some(end.character),
                                    kind: Some(FoldingRangeKind::Region),
                                    collapsed_text: Some(format!("struct {} {{ ... }}", s.name)),
                                });
                            }
                        }
                        ark_parser::ast::Item::EnumDef(e) => {
                            let start = Self::offset_to_position(&source, e.span.start);
                            let end = Self::offset_to_position(&source, e.span.end);
                            if end.line > start.line {
                                ranges.push(FoldingRange {
                                    start_line: start.line,
                                    start_character: Some(start.character),
                                    end_line: end.line,
                                    end_character: Some(end.character),
                                    kind: Some(FoldingRangeKind::Region),
                                    collapsed_text: Some(format!("enum {} {{ ... }}", e.name)),
                                });
                            }
                        }
                        _ => {}
                    }
                }
                return Ok(if ranges.is_empty() {
                    None
                } else {
                    Some(ranges)
                });
            }
            None => return Ok(None),
        };
    }

    async fn selection_range(
        &self,
        params: SelectionRangeParams,
    ) -> Result<Option<Vec<SelectionRange>>> {
        let uri = params.text_document.uri;
        let docs = self.documents.lock().unwrap();
        let source = match docs.get(&uri) {
            Some(s) => s.clone(),
            None => return Ok(None),
        };
        drop(docs);

        let mut cache = self.analysis_cache.lock().unwrap();
        let analysis = cache
            .entry(uri)
            .or_insert_with(|| Self::analyze_source(&source));

        let mut results = Vec::new();
        for pos in &params.positions {
            let offset = Self::position_to_offset(&source, *pos) as u32;
            let ranges = Self::collect_selection_ranges(&source, &analysis.module, offset);
            results.push(ranges);
        }
        Ok(Some(results))
    }

    async fn goto_type_definition(
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

        // Resolve the type name for this identifier
        let type_name = match Self::resolve_type_name_for_ident(&analysis.module, &name) {
            Some(t) => t,
            None => return Ok(None),
        };

        // Find the type definition
        let span = match Self::find_type_definition_span(&analysis.module, &type_name) {
            Some(s) => s,
            None => return Ok(None),
        };

        let range = Self::span_to_range(&source, span);
        Ok(Some(GotoDefinitionResponse::Scalar(Location {
            uri,
            range,
        })))
    }

    async fn prepare_call_hierarchy(
        &self,
        params: CallHierarchyPrepareParams,
    ) -> Result<Option<Vec<CallHierarchyItem>>> {
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

        let offset = Self::position_to_offset(&source, pos) as u32;

        // Find the function at cursor position
        let f = match Self::find_fn_at_offset(&analysis.module, offset) {
            Some(f) => f.clone(),
            None => return Ok(None),
        };

        Ok(Some(vec![Self::fn_to_call_hierarchy_item(
            &source, &uri, &f,
        )]))
    }

    async fn incoming_calls(
        &self,
        params: CallHierarchyIncomingCallsParams,
    ) -> Result<Option<Vec<CallHierarchyIncomingCall>>> {
        let target_name = &params.item.name;
        let uri = &params.item.uri;

        let source = {
            let docs = self.documents.lock().unwrap();
            match docs.get(uri) {
                Some(s) => s.clone(),
                None => return Ok(None),
            }
        };

        let mut cache = self.analysis_cache.lock().unwrap();
        let analysis = cache
            .entry(uri.clone())
            .or_insert_with(|| Self::analyze_source(&source));

        let mut results = Vec::new();

        // Search all functions for calls to target_name
        let all_fns: Vec<ast::FnDef> = analysis
            .module
            .items
            .iter()
            .filter_map(|item| match item {
                ast::Item::FnDef(f) => Some(f.clone()),
                ast::Item::ImplBlock(_) => None,
                _ => None,
            })
            .collect();

        // Also collect methods from impl blocks
        let mut all_methods: Vec<ast::FnDef> = Vec::new();
        for item in &analysis.module.items {
            if let ast::Item::ImplBlock(ib) = item {
                all_methods.extend(ib.methods.iter().cloned());
            }
        }

        for f in all_fns.iter().chain(all_methods.iter()) {
            let calls = Self::collect_calls_in_block(&f.body);
            let matching: Vec<_> = calls
                .iter()
                .filter(|(name, _)| name == target_name)
                .collect();

            if !matching.is_empty() {
                let from_item = Self::fn_to_call_hierarchy_item(&source, uri, f);
                let from_ranges: Vec<Range> = matching
                    .iter()
                    .map(|(_, span)| Self::span_to_range(&source, *span))
                    .collect();

                results.push(CallHierarchyIncomingCall {
                    from: from_item,
                    from_ranges,
                });
            }
        }

        Ok(Some(results))
    }

    async fn outgoing_calls(
        &self,
        params: CallHierarchyOutgoingCallsParams,
    ) -> Result<Option<Vec<CallHierarchyOutgoingCall>>> {
        let uri = &params.item.uri;

        let source = {
            let docs = self.documents.lock().unwrap();
            match docs.get(uri) {
                Some(s) => s.clone(),
                None => return Ok(None),
            }
        };

        let mut cache = self.analysis_cache.lock().unwrap();
        let analysis = cache
            .entry(uri.clone())
            .or_insert_with(|| Self::analyze_source(&source));

        // Find the function definition matching the item name
        let target_name = &params.item.name;
        let f = {
            let mut found: Option<ast::FnDef> = None;
            for item in &analysis.module.items {
                match item {
                    ast::Item::FnDef(f) if f.name == *target_name => {
                        found = Some(f.clone());
                        break;
                    }
                    ast::Item::ImplBlock(ib) => {
                        for m in &ib.methods {
                            if m.name == *target_name {
                                found = Some(m.clone());
                                break;
                            }
                        }
                    }
                    _ => {}
                }
            }
            match found {
                Some(f) => f,
                None => return Ok(None),
            }
        };

        let calls = Self::collect_calls_in_block(&f.body);

        // Group by callee name, deduplicate
        let mut call_map: HashMap<String, Vec<ark_diagnostics::Span>> = HashMap::new();
        for (name, span) in &calls {
            call_map.entry(name.clone()).or_default().push(*span);
        }

        let mut results = Vec::new();
        for (callee_name, spans) in &call_map {
            // Try to find the callee's definition for a proper CallHierarchyItem
            let to_item = if let Some(callee_fn) =
                analysis.module.items.iter().find_map(|item| match item {
                    ast::Item::FnDef(f) if f.name == *callee_name => Some(f),
                    _ => None,
                }) {
                Self::fn_to_call_hierarchy_item(&source, uri, callee_fn)
            } else {
                // External/unknown function — create minimal item
                let first_span = spans[0];
                let range = Self::span_to_range(&source, first_span);
                CallHierarchyItem {
                    name: callee_name.clone(),
                    kind: SymbolKind::FUNCTION,
                    tags: None,
                    detail: Some("(external)".to_string()),
                    uri: uri.clone(),
                    range,
                    selection_range: range,
                    data: None,
                }
            };

            let from_ranges: Vec<Range> = spans
                .iter()
                .map(|s| Self::span_to_range(&source, *s))
                .collect();

            results.push(CallHierarchyOutgoingCall {
                to: to_item,
                from_ranges,
            });
        }

        Ok(Some(results))
    }

    async fn goto_implementation(
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

        // Find impl blocks and trait method implementations for this name
        let mut locations = Vec::new();

        for item in &analysis.module.items {
            match item {
                // If the name is a type, find its impl blocks
                ast::Item::ImplBlock(ib) if ib.target_type == name => {
                    let range = Self::span_to_range(&source, ib.span);
                    locations.push(Location {
                        uri: uri.clone(),
                        range,
                    });
                }
                // If the name is a method, find its implementation in impl blocks
                ast::Item::ImplBlock(ib) => {
                    for method in &ib.methods {
                        if method.name == name {
                            let range = Self::span_to_range(&source, method.span);
                            locations.push(Location {
                                uri: uri.clone(),
                                range,
                            });
                        }
                    }
                }
                // If the name is a trait, find all impl blocks for it
                ast::Item::TraitDef(t) if t.name == name => {
                    // Find all impl blocks that implement this trait
                    for other in &analysis.module.items {
                        if let ast::Item::ImplBlock(ib) = other {
                            if ib.trait_name.as_deref() == Some(&name) {
                                let range = Self::span_to_range(&source, ib.span);
                                locations.push(Location {
                                    uri: uri.clone(),
                                    range,
                                });
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        if locations.is_empty() {
            Ok(None)
        } else if locations.len() == 1 {
            // SAFETY: len() == 1 guarantees next() returns Some
            match locations.into_iter().next() {
                Some(loc) => Ok(Some(GotoDefinitionResponse::Scalar(loc))),
                None => Ok(None),
            }
        } else {
            Ok(Some(GotoDefinitionResponse::Array(locations)))
        }
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
