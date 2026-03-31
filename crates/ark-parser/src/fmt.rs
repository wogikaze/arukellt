//! AST-based formatter for Arukellt source code.
//!
//! Parses source text into AST, then pretty-prints it back with canonical
//! formatting (consistent indentation, brace placement, import ordering).
//! Comments (line and block) are preserved by collecting them from the
//! source text and re-inserting them at the appropriate positions.

use crate::ast;

const INDENT: &str = "    ";

/// A comment extracted from source text for preservation during formatting.
#[derive(Debug, Clone)]
struct Comment {
    /// 0-based line number in the original source
    line: usize,
    /// The raw comment text including `//` or `/* */` delimiters
    text: String,
    /// Whether this is a trailing comment (appears after code on the same line)
    trailing: bool,
}

/// Extract all non-doc comments from source text.
///
/// Doc comments (`///`, `//!`) are handled by the AST, so we only collect
/// regular line comments (`//`) and block comments (`/* */`).
fn collect_comments(source: &str) -> Vec<Comment> {
    let mut comments = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    for (line_idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Skip doc comments — the AST already preserves these
        if trimmed.starts_with("///") || trimmed.starts_with("//!") {
            continue;
        }

        // Standalone line comment (entire line is a comment)
        if trimmed.starts_with("//") {
            comments.push(Comment {
                line: line_idx,
                text: trimmed.to_string(),
                trailing: false,
            });
            continue;
        }

        // Trailing line comment (code followed by //)
        if let Some(pos) = find_line_comment_start(line) {
            let comment_text = line[pos..].trim().to_string();
            // Skip doc comments in trailing position
            if !comment_text.starts_with("///") && !comment_text.starts_with("//!") {
                comments.push(Comment {
                    line: line_idx,
                    text: comment_text,
                    trailing: true,
                });
            }
        }

        // Standalone block comment on a single line
        if trimmed.starts_with("/*") && trimmed.ends_with("*/") && !trimmed.starts_with("/**") {
            comments.push(Comment {
                line: line_idx,
                text: trimmed.to_string(),
                trailing: false,
            });
        }
    }

    comments
}

/// Find the start of a `//` comment in a line, skipping string literals.
fn find_line_comment_start(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut i = 0;
    let mut in_string = false;
    let mut in_char = false;

    while i < bytes.len() {
        match bytes[i] {
            b'"' if !in_char => {
                if in_string {
                    in_string = false;
                } else {
                    in_string = true;
                }
            }
            b'\'' if !in_string => {
                in_char = !in_char;
            }
            b'\\' if in_string || in_char => {
                i += 1; // skip escaped char
            }
            b'/' if !in_string && !in_char && i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
                // Check it's not a doc comment
                let rest = &line[i..];
                if rest.starts_with("///") || rest.starts_with("//!") {
                    return None;
                }
                return Some(i);
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Compute the 0-based line number for a byte offset in source text.
fn byte_offset_to_line(source: &str, offset: u32) -> usize {
    source[..offset as usize]
        .bytes()
        .filter(|&b| b == b'\n')
        .count()
}

/// Sort imports in Arukellt source code without re-formatting the rest.
///
/// Returns `None` if the source has lex/parse errors.
/// Stdlib (`std::`) imports come first (sorted), then others (sorted),
/// with a blank line separating the groups.
pub fn sort_imports(source: &str) -> Option<String> {
    let (tokens, lex_errors) = ark_lexer::Lexer::new(0, source).tokenize();
    if !lex_errors.is_empty() {
        return None;
    }
    let mut sink = ark_diagnostics::DiagnosticSink::new();
    let module = crate::parse(&tokens, &mut sink);
    if sink.has_errors() {
        return None;
    }
    sort_imports_in_module(source, &module)
}

/// Given the original source and parsed module, rewrite just the import
/// block with canonical ordering while preserving everything else verbatim.
fn sort_imports_in_module(source: &str, module: &ast::Module) -> Option<String> {
    if module.imports.is_empty() {
        return Some(source.to_string());
    }

    // Find the line range covering all imports
    // We search for the first "use " from the top and the last "use " line
    let lines: Vec<&str> = source.lines().collect();
    let mut first_import_line = None;
    let mut last_import_line = None;
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("use ") {
            if first_import_line.is_none() {
                first_import_line = Some(i);
            }
            last_import_line = Some(i);
        }
    }

    let first_import_line = first_import_line?;
    let last_import_line = last_import_line?;

    // Build sorted import text
    let mut std_imports: Vec<&ast::Import> = Vec::new();
    let mut other_imports: Vec<&ast::Import> = Vec::new();
    for imp in &module.imports {
        if imp.module_name.starts_with("std::") || imp.module_name == "std" {
            std_imports.push(imp);
        } else {
            other_imports.push(imp);
        }
    }
    std_imports.sort_by(|a, b| a.module_name.cmp(&b.module_name));
    other_imports.sort_by(|a, b| a.module_name.cmp(&b.module_name));

    let mut sorted_block = String::new();
    for imp in &std_imports {
        write_import(&mut sorted_block, imp);
    }
    if !std_imports.is_empty() && !other_imports.is_empty() {
        sorted_block.push('\n');
    }
    for imp in &other_imports {
        write_import(&mut sorted_block, imp);
    }

    // Reconstruct: lines before imports + sorted imports + lines after imports
    let mut result = String::with_capacity(source.len());
    for line in &lines[..first_import_line] {
        result.push_str(line);
        result.push('\n');
    }
    result.push_str(&sorted_block);
    // Skip blank lines immediately after the import block (we'll reconstruct them)
    let mut rest_start = last_import_line + 1;
    // Keep blank lines that were after imports as-is
    for line in &lines[rest_start..] {
        result.push_str(line);
        result.push('\n');
        rest_start += 1;
    }
    // The original source might not end with a newline
    if !source.ends_with('\n') && result.ends_with('\n') {
        result.pop();
    }
    Some(result)
}

fn write_import(out: &mut String, imp: &ast::Import) {
    out.push_str("use ");
    out.push_str(&imp.module_name);
    if let Some(ref alias) = imp.alias {
        out.push_str(" as ");
        out.push_str(alias);
    }
    out.push('\n');
}

/// Format Arukellt source code to canonical form.
///
/// Returns `None` if the source contains lex or parse errors, preventing
/// the formatter from corrupting invalid code. Comments are preserved.
pub fn format_source(source: &str) -> Option<String> {
    let (tokens, lex_errors) = ark_lexer::Lexer::new(0, source).tokenize();
    if !lex_errors.is_empty() {
        return None;
    }
    let mut sink = ark_diagnostics::DiagnosticSink::new();
    let module = crate::parse(&tokens, &mut sink);
    if sink.has_errors() {
        return None;
    }
    let comments = collect_comments(source);
    Some(format_module_with_comments(&module, source, &comments))
}

/// Format a parsed AST module back to source text.
pub fn format_module(module: &ast::Module) -> String {
    let mut out = String::new();
    let mut printer = Printer {
        out: &mut out,
        indent: 0,
        comments: &[],
        source: "",
        emitted_comment_lines: std::collections::HashSet::new(),
    };
    printer.print_module(module);
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// Format a parsed AST module back to source text, preserving comments.
fn format_module_with_comments(
    module: &ast::Module,
    source: &str,
    comments: &[Comment],
) -> String {
    let mut out = String::new();
    let mut printer = Printer {
        out: &mut out,
        indent: 0,
        comments,
        source,
        emitted_comment_lines: std::collections::HashSet::new(),
    };
    printer.print_module(module);

    // Emit any trailing comments that appear after all items
    printer.emit_trailing_comments();

    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

struct Printer<'a> {
    out: &'a mut String,
    indent: usize,
    comments: &'a [Comment],
    source: &'a str,
    emitted_comment_lines: std::collections::HashSet<usize>,
}

impl<'a> Printer<'a> {
    fn push_indent(&mut self) {
        for _ in 0..self.indent {
            self.out.push_str(INDENT);
        }
    }

    /// Emit non-trailing comments that appear before a given source line.
    fn emit_leading_comments_before(&mut self, before_line: usize) {
        for comment in self.comments {
            if comment.trailing || comment.line >= before_line {
                continue;
            }
            if self.emitted_comment_lines.contains(&comment.line) {
                continue;
            }
            self.emitted_comment_lines.insert(comment.line);
            self.push_indent();
            self.out.push_str(&comment.text);
            self.out.push('\n');
        }
    }

    /// Emit non-trailing comments between two source lines (exclusive start, exclusive end).
    fn emit_comments_between(&mut self, after_line: usize, before_line: usize) {
        for comment in self.comments {
            if comment.trailing {
                continue;
            }
            if comment.line <= after_line || comment.line >= before_line {
                continue;
            }
            if self.emitted_comment_lines.contains(&comment.line) {
                continue;
            }
            self.emitted_comment_lines.insert(comment.line);
            self.push_indent();
            self.out.push_str(&comment.text);
            self.out.push('\n');
        }
    }

    /// Check if there are unemitted non-trailing comments between two source lines.
    fn has_comments_between(&self, after_line: usize, before_line: usize) -> bool {
        self.comments.iter().any(|c| {
            !c.trailing
                && c.line > after_line
                && c.line < before_line
                && !self.emitted_comment_lines.contains(&c.line)
        })
    }

    /// Emit any comments not yet emitted (typically at end of file).
    fn emit_trailing_comments(&mut self) {
        for comment in self.comments {
            if self.emitted_comment_lines.contains(&comment.line) {
                continue;
            }
            self.emitted_comment_lines.insert(comment.line);
            if comment.trailing {
                // Trailing comments at end of file — emit as standalone
                self.push_indent();
                self.out.push_str(&comment.text);
                self.out.push('\n');
            } else {
                self.push_indent();
                self.out.push_str(&comment.text);
                self.out.push('\n');
            }
        }
    }

    /// Get the source line for an AST item's span.
    fn item_start_line(&self, item: &ast::Item) -> usize {
        let span = match item {
            ast::Item::FnDef(f) => f.span,
            ast::Item::StructDef(s) => s.span,
            ast::Item::EnumDef(e) => e.span,
            ast::Item::TraitDef(t) => t.span,
            ast::Item::ImplBlock(ib) => ib.span,
        };
        if self.source.is_empty() {
            return 0;
        }
        byte_offset_to_line(self.source, span.start)
    }

    /// Get the end line for an AST item's span.
    fn item_end_line(&self, item: &ast::Item) -> usize {
        let span = match item {
            ast::Item::FnDef(f) => f.span,
            ast::Item::StructDef(s) => s.span,
            ast::Item::EnumDef(e) => e.span,
            ast::Item::TraitDef(t) => t.span,
            ast::Item::ImplBlock(ib) => ib.span,
        };
        if self.source.is_empty() {
            return 0;
        }
        byte_offset_to_line(self.source, span.end)
    }

    fn print_module(&mut self, module: &ast::Module) {
        // Module-level doc comments
        for doc in &module.docs {
            self.out.push_str("/// ");
            self.out.push_str(doc.trim());
            self.out.push('\n');
        }

        // Emit leading comments before imports
        let first_import_line = module
            .imports
            .first()
            .map(|imp| byte_offset_to_line(self.source, imp.span.start));
        let first_item_line = module
            .items
            .first()
            .map(|item| self.item_start_line(item));
        let first_code_line = first_import_line
            .or(first_item_line)
            .unwrap_or(usize::MAX);

        if !self.source.is_empty() {
            self.emit_leading_comments_before(first_code_line);
        }

        // Imports — sorted: stdlib (std::) first, then others
        let mut std_imports: Vec<&ast::Import> = Vec::new();
        let mut other_imports: Vec<&ast::Import> = Vec::new();
        for imp in &module.imports {
            if imp.module_name.starts_with("std::") || imp.module_name == "std" {
                std_imports.push(imp);
            } else {
                other_imports.push(imp);
            }
        }
        std_imports.sort_by(|a, b| a.module_name.cmp(&b.module_name));
        other_imports.sort_by(|a, b| a.module_name.cmp(&b.module_name));

        for imp in &std_imports {
            self.print_import(imp);
        }
        if !std_imports.is_empty() && !other_imports.is_empty() {
            self.out.push('\n');
        }
        for imp in &other_imports {
            self.print_import(imp);
        }

        if !module.imports.is_empty() && !module.items.is_empty() {
            self.out.push('\n');
        }

        // Determine the end line of the import region
        let last_import_end_line = module
            .imports
            .last()
            .map(|imp| byte_offset_to_line(self.source, imp.span.end))
            .unwrap_or(0);

        // Items — separated by blank lines, with comments preserved
        let mut prev_item_end_line = last_import_end_line;
        for (i, item) in module.items.iter().enumerate() {
            let item_start = if !self.source.is_empty() {
                self.item_start_line(item)
            } else {
                0
            };

            // Emit comments between previous item and this item
            if !self.source.is_empty() && (i > 0 || !module.imports.is_empty()) {
                let has_interleaved = self.has_comments_between(prev_item_end_line, item_start);
                if i > 0 || has_interleaved {
                    self.out.push('\n');
                }
                self.emit_comments_between(prev_item_end_line, item_start);
            } else if i > 0 {
                self.out.push('\n');
            }

            self.print_item(item);

            if !self.source.is_empty() {
                prev_item_end_line = self.item_end_line(item);
            }
        }
    }

    fn print_import(&mut self, imp: &ast::Import) {
        self.push_indent();
        self.out.push_str("use ");
        self.out.push_str(&imp.module_name);
        if let Some(alias) = &imp.alias {
            self.out.push_str(" as ");
            self.out.push_str(alias);
        }
        self.out.push('\n');
    }

    fn print_item(&mut self, item: &ast::Item) {
        match item {
            ast::Item::FnDef(f) => self.print_fn_def(f),
            ast::Item::StructDef(s) => self.print_struct_def(s),
            ast::Item::EnumDef(e) => self.print_enum_def(e),
            ast::Item::TraitDef(t) => self.print_trait_def(t),
            ast::Item::ImplBlock(ib) => self.print_impl_block(ib),
        }
    }

    fn print_docs(&mut self, docs: &[String]) {
        for doc in docs {
            self.push_indent();
            self.out.push_str("/// ");
            self.out.push_str(doc.trim());
            self.out.push('\n');
        }
    }

    fn print_fn_def(&mut self, f: &ast::FnDef) {
        self.print_docs(&f.docs);
        self.push_indent();
        if f.is_pub {
            self.out.push_str("pub ");
        }
        self.out.push_str("fn ");
        self.out.push_str(&f.name);
        self.print_type_params(&f.type_params, &f.type_param_bounds);
        self.out.push('(');
        for (i, p) in f.params.iter().enumerate() {
            if i > 0 {
                self.out.push_str(", ");
            }
            self.out.push_str(&p.name);
            self.out.push_str(": ");
            self.print_type_expr(&p.ty);
        }
        self.out.push(')');
        if let Some(ret) = &f.return_type {
            self.out.push_str(" -> ");
            self.print_type_expr(ret);
        }
        self.out.push_str(" {\n");
        self.indent += 1;
        self.print_block_body(&f.body);
        self.indent -= 1;
        self.push_indent();
        self.out.push_str("}\n");
    }

    fn print_type_params(&mut self, params: &[String], bounds: &[(String, Vec<String>)]) {
        if params.is_empty() {
            return;
        }
        self.out.push('<');
        for (i, p) in params.iter().enumerate() {
            if i > 0 {
                self.out.push_str(", ");
            }
            self.out.push_str(p);
            // Check for bounds
            for (name, trait_bounds) in bounds {
                if name == p && !trait_bounds.is_empty() {
                    self.out.push_str(": ");
                    for (j, b) in trait_bounds.iter().enumerate() {
                        if j > 0 {
                            self.out.push_str(" + ");
                        }
                        self.out.push_str(b);
                    }
                }
            }
        }
        self.out.push('>');
    }

    fn print_struct_def(&mut self, s: &ast::StructDef) {
        self.print_docs(&s.docs);
        self.push_indent();
        if s.is_pub {
            self.out.push_str("pub ");
        }
        self.out.push_str("struct ");
        self.out.push_str(&s.name);
        self.print_type_params(&s.type_params, &[]);
        self.out.push_str(" {\n");
        self.indent += 1;
        for field in &s.fields {
            self.push_indent();
            self.out.push_str(&field.name);
            self.out.push_str(": ");
            self.print_type_expr(&field.ty);
            self.out.push_str(",\n");
        }
        self.indent -= 1;
        self.push_indent();
        self.out.push_str("}\n");
    }

    fn print_enum_def(&mut self, e: &ast::EnumDef) {
        self.print_docs(&e.docs);
        self.push_indent();
        if e.is_pub {
            self.out.push_str("pub ");
        }
        self.out.push_str("enum ");
        self.out.push_str(&e.name);
        self.print_type_params(&e.type_params, &[]);
        self.out.push_str(" {\n");
        self.indent += 1;
        for variant in &e.variants {
            self.print_variant(variant);
        }
        self.indent -= 1;
        self.push_indent();
        self.out.push_str("}\n");
    }

    fn print_variant(&mut self, variant: &ast::Variant) {
        self.push_indent();
        match variant {
            ast::Variant::Unit { name, .. } => {
                self.out.push_str(name);
                self.out.push_str(",\n");
            }
            ast::Variant::Tuple { name, fields, .. } => {
                self.out.push_str(name);
                self.out.push('(');
                for (i, ty) in fields.iter().enumerate() {
                    if i > 0 {
                        self.out.push_str(", ");
                    }
                    self.print_type_expr(ty);
                }
                self.out.push_str("),\n");
            }
            ast::Variant::Struct { name, fields, .. } => {
                self.out.push_str(name);
                self.out.push_str(" {\n");
                self.indent += 1;
                for field in fields {
                    self.push_indent();
                    self.out.push_str(&field.name);
                    self.out.push_str(": ");
                    self.print_type_expr(&field.ty);
                    self.out.push_str(",\n");
                }
                self.indent -= 1;
                self.push_indent();
                self.out.push_str("},\n");
            }
        }
    }

    fn print_trait_def(&mut self, t: &ast::TraitDef) {
        self.print_docs(&t.docs);
        self.push_indent();
        if t.is_pub {
            self.out.push_str("pub ");
        }
        self.out.push_str("trait ");
        self.out.push_str(&t.name);
        self.print_type_params(&t.type_params, &[]);
        self.out.push_str(" {\n");
        self.indent += 1;
        for method in &t.methods {
            self.print_trait_method_sig(method);
        }
        self.indent -= 1;
        self.push_indent();
        self.out.push_str("}\n");
    }

    fn print_trait_method_sig(&mut self, m: &ast::TraitMethodSig) {
        self.print_docs(&m.docs);
        self.push_indent();
        self.out.push_str("fn ");
        self.out.push_str(&m.name);
        self.out.push('(');
        for (i, p) in m.params.iter().enumerate() {
            if i > 0 {
                self.out.push_str(", ");
            }
            self.out.push_str(&p.name);
            self.out.push_str(": ");
            self.print_type_expr(&p.ty);
        }
        self.out.push(')');
        if let Some(ret) = &m.return_type {
            self.out.push_str(" -> ");
            self.print_type_expr(ret);
        }
        self.out.push('\n');
    }

    fn print_impl_block(&mut self, ib: &ast::ImplBlock) {
        self.print_docs(&ib.docs);
        self.push_indent();
        self.out.push_str("impl ");
        if let Some(trait_name) = &ib.trait_name {
            self.out.push_str(trait_name);
            self.out.push_str(" for ");
        }
        self.out.push_str(&ib.target_type);
        self.out.push_str(" {\n");
        self.indent += 1;
        for (i, method) in ib.methods.iter().enumerate() {
            if i > 0 {
                self.out.push('\n');
            }
            self.print_fn_def(method);
        }
        self.indent -= 1;
        self.push_indent();
        self.out.push_str("}\n");
    }

    fn print_type_expr(&mut self, ty: &ast::TypeExpr) {
        match ty {
            ast::TypeExpr::Named { name, .. } => self.out.push_str(name),
            ast::TypeExpr::Generic { name, args, .. } => {
                self.out.push_str(name);
                self.out.push('<');
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        self.out.push_str(", ");
                    }
                    self.print_type_expr(a);
                }
                self.out.push('>');
            }
            ast::TypeExpr::Tuple(elems, _) => {
                self.out.push('(');
                for (i, e) in elems.iter().enumerate() {
                    if i > 0 {
                        self.out.push_str(", ");
                    }
                    self.print_type_expr(e);
                }
                self.out.push(')');
            }
            ast::TypeExpr::Array { elem, size, .. } => {
                self.out.push('[');
                self.print_type_expr(elem);
                self.out.push_str(&format!("; {}]", size));
            }
            ast::TypeExpr::Slice { elem, .. } => {
                self.out.push('[');
                self.print_type_expr(elem);
                self.out.push(']');
            }
            ast::TypeExpr::Function { params, ret, .. } => {
                self.out.push_str("fn(");
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        self.out.push_str(", ");
                    }
                    self.print_type_expr(p);
                }
                self.out.push_str(") -> ");
                self.print_type_expr(ret);
            }
            ast::TypeExpr::Unit(_) => self.out.push_str("()"),
            ast::TypeExpr::Qualified { module, name, .. } => {
                self.out.push_str(module);
                self.out.push_str("::");
                self.out.push_str(name);
            }
        }
    }

    fn print_block_body(&mut self, block: &ast::Block) {
        for stmt in &block.stmts {
            self.print_stmt(stmt);
        }
        if let Some(tail) = &block.tail_expr {
            self.push_indent();
            self.print_expr(tail);
            self.out.push('\n');
        }
    }

    fn print_stmt(&mut self, stmt: &ast::Stmt) {
        match stmt {
            ast::Stmt::Let {
                name,
                ty,
                init,
                is_mut,
                pattern,
                ..
            } => {
                self.push_indent();
                self.out.push_str("let ");
                if *is_mut {
                    self.out.push_str("mut ");
                }
                if let Some(pat) = pattern {
                    self.print_pattern(pat);
                } else {
                    self.out.push_str(name);
                }
                if let Some(ty_expr) = ty {
                    self.out.push_str(": ");
                    self.print_type_expr(ty_expr);
                }
                self.out.push_str(" = ");
                self.print_expr(init);
                self.out.push('\n');
            }
            ast::Stmt::Expr(expr) => {
                self.push_indent();
                self.print_expr(expr);
                self.out.push('\n');
            }
            ast::Stmt::While { cond, body, .. } => {
                self.push_indent();
                self.out.push_str("while ");
                self.print_expr(cond);
                self.out.push_str(" {\n");
                self.indent += 1;
                self.print_block_body(body);
                self.indent -= 1;
                self.push_indent();
                self.out.push_str("}\n");
            }
            ast::Stmt::Loop { body, .. } => {
                self.push_indent();
                self.out.push_str("loop {\n");
                self.indent += 1;
                self.print_block_body(body);
                self.indent -= 1;
                self.push_indent();
                self.out.push_str("}\n");
            }
            ast::Stmt::For {
                target, iter, body, ..
            } => {
                self.push_indent();
                self.out.push_str("for ");
                self.out.push_str(target);
                self.out.push_str(" in ");
                self.print_for_iter(iter);
                self.out.push_str(" {\n");
                self.indent += 1;
                self.print_block_body(body);
                self.indent -= 1;
                self.push_indent();
                self.out.push_str("}\n");
            }
        }
    }

    fn print_for_iter(&mut self, iter: &ast::ForIter) {
        match iter {
            ast::ForIter::Range { start, end } => {
                self.print_expr(start);
                self.out.push_str("..");
                self.print_expr(end);
            }
            ast::ForIter::Values(expr) | ast::ForIter::Iter(expr) => {
                self.print_expr(expr);
            }
        }
    }

    fn print_expr(&mut self, expr: &ast::Expr) {
        match expr {
            ast::Expr::IntLit { value, suffix, .. } => {
                self.out.push_str(&value.to_string());
                if let Some(s) = suffix {
                    self.out.push_str(s);
                }
            }
            ast::Expr::FloatLit { value, suffix, .. } => {
                let s = format!("{}", value);
                self.out.push_str(&s);
                // Ensure there's a decimal point for readability
                if !s.contains('.') && !s.contains('e') && !s.contains('E') {
                    self.out.push_str(".0");
                }
                if let Some(suf) = suffix {
                    self.out.push_str(suf);
                }
            }
            ast::Expr::StringLit { value, .. } => {
                self.out.push('"');
                self.out.push_str(&escape_string(value));
                self.out.push('"');
            }
            ast::Expr::CharLit { value, .. } => {
                self.out.push('\'');
                self.out.push_str(&escape_char(*value));
                self.out.push('\'');
            }
            ast::Expr::BoolLit { value, .. } => {
                self.out.push_str(if *value { "true" } else { "false" });
            }
            ast::Expr::Ident { name, .. } => {
                self.out.push_str(name);
            }
            ast::Expr::QualifiedIdent { module, name, .. } => {
                self.out.push_str(module);
                self.out.push_str("::");
                self.out.push_str(name);
            }
            ast::Expr::Binary {
                left, op, right, ..
            } => {
                self.print_expr(left);
                self.out.push(' ');
                self.out.push_str(binop_str(*op));
                self.out.push(' ');
                self.print_expr(right);
            }
            ast::Expr::Unary { op, operand, .. } => {
                self.out.push_str(unaryop_str(*op));
                self.print_expr(operand);
            }
            ast::Expr::Call {
                callee,
                type_args,
                args,
                ..
            } => {
                self.print_expr(callee);
                if !type_args.is_empty() {
                    self.out.push_str("::<");
                    for (i, ta) in type_args.iter().enumerate() {
                        if i > 0 {
                            self.out.push_str(", ");
                        }
                        self.print_type_expr(ta);
                    }
                    self.out.push('>');
                }
                self.out.push('(');
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.out.push_str(", ");
                    }
                    self.print_expr(arg);
                }
                self.out.push(')');
            }
            ast::Expr::FieldAccess { object, field, .. } => {
                self.print_expr(object);
                self.out.push('.');
                self.out.push_str(field);
            }
            ast::Expr::Index { object, index, .. } => {
                self.print_expr(object);
                self.out.push('[');
                self.print_expr(index);
                self.out.push(']');
            }
            ast::Expr::If {
                cond,
                then_block,
                else_block,
                ..
            } => {
                self.out.push_str("if ");
                self.print_expr(cond);
                self.out.push_str(" {\n");
                self.indent += 1;
                self.print_block_body(then_block);
                self.indent -= 1;
                self.push_indent();
                if let Some(eb) = else_block {
                    self.out.push_str("} else {\n");
                    self.indent += 1;
                    self.print_block_body(eb);
                    self.indent -= 1;
                    self.push_indent();
                    self.out.push('}');
                } else {
                    self.out.push('}');
                }
            }
            ast::Expr::Match {
                scrutinee, arms, ..
            } => {
                self.out.push_str("match ");
                self.print_expr(scrutinee);
                self.out.push_str(" {\n");
                self.indent += 1;
                for arm in arms {
                    self.print_match_arm(arm);
                }
                self.indent -= 1;
                self.push_indent();
                self.out.push('}');
            }
            ast::Expr::Block(block) => {
                self.out.push_str("{\n");
                self.indent += 1;
                self.print_block_body(block);
                self.indent -= 1;
                self.push_indent();
                self.out.push('}');
            }
            ast::Expr::Tuple { elements, .. } => {
                self.out.push('(');
                for (i, e) in elements.iter().enumerate() {
                    if i > 0 {
                        self.out.push_str(", ");
                    }
                    self.print_expr(e);
                }
                if elements.len() == 1 {
                    self.out.push(',');
                }
                self.out.push(')');
            }
            ast::Expr::Array { elements, .. } => {
                self.out.push('[');
                for (i, e) in elements.iter().enumerate() {
                    if i > 0 {
                        self.out.push_str(", ");
                    }
                    self.print_expr(e);
                }
                self.out.push(']');
            }
            ast::Expr::ArrayRepeat { value, count, .. } => {
                self.out.push('[');
                self.print_expr(value);
                self.out.push_str("; ");
                self.print_expr(count);
                self.out.push(']');
            }
            ast::Expr::StructInit {
                name, fields, base, ..
            } => {
                self.out.push_str(name);
                self.out.push_str(" { ");
                for (i, (fname, fval)) in fields.iter().enumerate() {
                    if i > 0 {
                        self.out.push_str(", ");
                    }
                    self.out.push_str(fname);
                    self.out.push_str(": ");
                    self.print_expr(fval);
                }
                if let Some(b) = base {
                    if !fields.is_empty() {
                        self.out.push_str(", ");
                    }
                    self.out.push_str("..");
                    self.print_expr(b);
                }
                self.out.push_str(" }");
            }
            ast::Expr::Closure {
                params,
                return_type,
                body,
                ..
            } => {
                self.out.push('|');
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        self.out.push_str(", ");
                    }
                    self.out.push_str(&p.name);
                    if let Some(ty) = &p.ty {
                        self.out.push_str(": ");
                        self.print_type_expr(ty);
                    }
                }
                self.out.push('|');
                if let Some(ret) = return_type {
                    self.out.push_str(" -> ");
                    self.print_type_expr(ret);
                }
                self.out.push(' ');
                self.print_expr(body);
            }
            ast::Expr::Return { value, .. } => {
                self.out.push_str("return");
                if let Some(v) = value {
                    self.out.push(' ');
                    self.print_expr(v);
                }
            }
            ast::Expr::Break { value, .. } => {
                self.out.push_str("break");
                if let Some(v) = value {
                    self.out.push(' ');
                    self.print_expr(v);
                }
            }
            ast::Expr::Continue { .. } => {
                self.out.push_str("continue");
            }
            ast::Expr::Try { expr, .. } => {
                self.print_expr(expr);
                self.out.push('?');
            }
            ast::Expr::Assign { target, value, .. } => {
                self.print_expr(target);
                self.out.push_str(" = ");
                self.print_expr(value);
            }
            ast::Expr::Loop { body, .. } => {
                self.out.push_str("loop {\n");
                self.indent += 1;
                self.print_block_body(body);
                self.indent -= 1;
                self.push_indent();
                self.out.push('}');
            }
        }
    }

    fn print_match_arm(&mut self, arm: &ast::MatchArm) {
        self.push_indent();
        self.print_pattern(&arm.pattern);
        if let Some(guard) = &arm.guard {
            self.out.push_str(" if ");
            self.print_expr(guard);
        }
        self.out.push_str(" => ");
        self.print_expr(&arm.body);
        self.out.push_str(",\n");
    }

    fn print_pattern(&mut self, pat: &ast::Pattern) {
        match pat {
            ast::Pattern::Wildcard(_) => self.out.push('_'),
            ast::Pattern::Ident { name, .. } => self.out.push_str(name),
            ast::Pattern::IntLit { value, suffix, .. } => {
                self.out.push_str(&value.to_string());
                if let Some(s) = suffix {
                    self.out.push_str(s);
                }
            }
            ast::Pattern::FloatLit { value, suffix, .. } => {
                self.out.push_str(&format!("{}", value));
                if let Some(s) = suffix {
                    self.out.push_str(s);
                }
            }
            ast::Pattern::StringLit { value, .. } => {
                self.out.push('"');
                self.out.push_str(&escape_string(value));
                self.out.push('"');
            }
            ast::Pattern::CharLit { value, .. } => {
                self.out.push('\'');
                self.out.push_str(&escape_char(*value));
                self.out.push('\'');
            }
            ast::Pattern::BoolLit { value, .. } => {
                self.out.push_str(if *value { "true" } else { "false" });
            }
            ast::Pattern::Tuple { elements, .. } => {
                self.out.push('(');
                for (i, e) in elements.iter().enumerate() {
                    if i > 0 {
                        self.out.push_str(", ");
                    }
                    self.print_pattern(e);
                }
                self.out.push(')');
            }
            ast::Pattern::Enum {
                path,
                variant,
                fields,
                ..
            } => {
                if !path.is_empty() {
                    self.out.push_str(path);
                    self.out.push_str("::");
                }
                self.out.push_str(variant);
                if !fields.is_empty() {
                    self.out.push('(');
                    for (i, f) in fields.iter().enumerate() {
                        if i > 0 {
                            self.out.push_str(", ");
                        }
                        self.print_pattern(f);
                    }
                    self.out.push(')');
                }
            }
            ast::Pattern::Or { patterns, .. } => {
                for (i, p) in patterns.iter().enumerate() {
                    if i > 0 {
                        self.out.push_str(" | ");
                    }
                    self.print_pattern(p);
                }
            }
            ast::Pattern::Struct { name, fields, .. } => {
                self.out.push_str(name);
                self.out.push_str(" { ");
                for (i, (fname, fpat)) in fields.iter().enumerate() {
                    if i > 0 {
                        self.out.push_str(", ");
                    }
                    self.out.push_str(fname);
                    if let Some(p) = fpat {
                        self.out.push_str(": ");
                        self.print_pattern(p);
                    }
                }
                self.out.push_str(" }");
            }
        }
    }
}

fn binop_str(op: ast::BinOp) -> &'static str {
    match op {
        ast::BinOp::Add => "+",
        ast::BinOp::Sub => "-",
        ast::BinOp::Mul => "*",
        ast::BinOp::Div => "/",
        ast::BinOp::Mod => "%",
        ast::BinOp::Eq => "==",
        ast::BinOp::Ne => "!=",
        ast::BinOp::Lt => "<",
        ast::BinOp::Le => "<=",
        ast::BinOp::Gt => ">",
        ast::BinOp::Ge => ">=",
        ast::BinOp::And => "&&",
        ast::BinOp::Or => "||",
        ast::BinOp::BitAnd => "&",
        ast::BinOp::BitOr => "|",
        ast::BinOp::BitXor => "^",
        ast::BinOp::Shl => "<<",
        ast::BinOp::Shr => ">>",
    }
}

fn unaryop_str(op: ast::UnaryOp) -> &'static str {
    match op {
        ast::UnaryOp::Neg => "-",
        ast::UnaryOp::Not => "!",
        ast::UnaryOp::BitNot => "~",
    }
}

fn escape_string(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '\0' => out.push_str("\\0"),
            c => out.push(c),
        }
    }
    out
}

fn escape_char(c: char) -> String {
    match c {
        '\\' => "\\\\".to_string(),
        '\'' => "\\'".to_string(),
        '\n' => "\\n".to_string(),
        '\t' => "\\t".to_string(),
        '\r' => "\\r".to_string(),
        '\0' => "\\0".to_string(),
        c => c.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_simple_function() {
        let source = "fn main() {\n  let x = 42\n  print(x)\n}\n";
        let formatted = format_source(source).unwrap();
        assert!(formatted.contains("fn main()"));
        assert!(formatted.contains("let x = 42"));
        assert!(formatted.contains("print(x)"));
    }

    #[test]
    fn format_imports_sorted() {
        let source = "use mylib\nuse std::io\n\nfn main() {\n}\n";
        let formatted = format_source(source).unwrap();
        // stdlib should come first
        let std_pos = formatted.find("use std::io").unwrap();
        let lib_pos = formatted.find("use mylib").unwrap();
        assert!(
            std_pos < lib_pos,
            "stdlib imports should come before others"
        );
    }

    #[test]
    fn format_idempotent() {
        let source = "fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n";
        let first = format_source(source).unwrap();
        let second = format_source(&first).unwrap();
        assert_eq!(first, second, "formatting should be idempotent");
    }

    #[test]
    fn format_struct_def() {
        let source = "struct Point {\n  x: f64,\n  y: f64,\n}\n";
        let formatted = format_source(source).unwrap();
        assert!(formatted.contains("struct Point {"));
        assert!(formatted.contains("    x: f64,"));
        assert!(formatted.contains("    y: f64,"));
    }

    #[test]
    fn format_enum_def() {
        let source = "enum Color {\n  Red,\n  Green(i32),\n  Blue(String, i32),\n}\n";
        let formatted = format_source(source).unwrap();
        assert!(formatted.contains("enum Color {"));
        assert!(formatted.contains("Red"));
        assert!(formatted.contains("Green(i32)"));
        assert!(formatted.contains("Blue(String, i32)"));
        // idempotent
        let second = format_source(&formatted).unwrap();
        assert_eq!(formatted, second);
    }

    #[test]
    fn format_trait_def() {
        let source = "trait Displayable {\n  fn show() -> String\n}\n";
        let formatted = format_source(source).unwrap();
        assert!(formatted.contains("trait Displayable {"));
        assert!(formatted.contains("fn show() -> String"));
        let second = format_source(&formatted).unwrap();
        assert_eq!(formatted, second);
    }

    #[test]
    fn format_impl_block() {
        let source = "impl Display for Point {\n  fn show() -> String {\n    \"point\"\n  }\n}\n";
        let formatted = format_source(source).unwrap();
        assert!(formatted.contains("impl Display for Point {"));
        assert!(formatted.contains("fn show() -> String {"));
        let second = format_source(&formatted).unwrap();
        assert_eq!(formatted, second);
    }

    #[test]
    fn format_match_expr() {
        let source = "fn f() {\n  match x {\n    1 => a,\n    2 => b,\n    _ => c,\n  }\n}\n";
        let formatted = format_source(source).unwrap();
        assert!(formatted.contains("match x {"));
        assert!(formatted.contains("1 => a"));
        assert!(formatted.contains("_ => c"));
        let second = format_source(&formatted).unwrap();
        assert_eq!(formatted, second);
    }

    #[test]
    fn format_generic_function() {
        let source = "fn identity<T>(x: T) -> T {\n  x\n}\n";
        let formatted = format_source(source).unwrap();
        assert!(formatted.contains("fn identity<T>(x: T) -> T {"));
        let second = format_source(&formatted).unwrap();
        assert_eq!(formatted, second);
    }

    #[test]
    fn format_while_loop() {
        let source = "fn f() {\n  while x < 10 {\n    x = x + 1\n  }\n}\n";
        let formatted = format_source(source).unwrap();
        assert!(formatted.contains("while x < 10 {"));
        let second = format_source(&formatted).unwrap();
        assert_eq!(formatted, second);
    }

    #[test]
    fn format_for_loop() {
        let source = "fn f() {\n  for item in items {\n    print(item)\n  }\n}\n";
        let formatted = format_source(source).unwrap();
        assert!(formatted.contains("for item in items {"));
        let second = format_source(&formatted).unwrap();
        assert_eq!(formatted, second);
    }

    #[test]
    fn format_if_else() {
        let source = "fn f() {\n  if x > 0 {\n    a\n  } else {\n    b\n  }\n}\n";
        let formatted = format_source(source).unwrap();
        assert!(formatted.contains("if x > 0 {"));
        assert!(formatted.contains("} else {"));
        let second = format_source(&formatted).unwrap();
        assert_eq!(formatted, second);
    }

    #[test]
    fn format_let_with_type() {
        let source = "fn f() {\n  let x: i32 = 42\n  let y: String = \"hello\"\n}\n";
        let formatted = format_source(source).unwrap();
        assert!(formatted.contains("let x: i32 = 42"));
        assert!(formatted.contains("let y: String = \"hello\""));
        let second = format_source(&formatted).unwrap();
        assert_eq!(formatted, second);
    }

    #[test]
    fn format_multiple_imports_grouped() {
        let source =
            "use mylib\nuse std::host::stdio\nuse std::host::fs\nuse another\n\nfn main() {\n}\n";
        let formatted = format_source(source).unwrap();
        // stdlib should be sorted and come first
        let fs_pos = formatted.find("use std::host::fs").unwrap();
        let stdio_pos = formatted.find("use std::host::stdio").unwrap();
        let mylib_pos = formatted.find("use mylib").unwrap();
        let another_pos = formatted.find("use another").unwrap();
        assert!(fs_pos < stdio_pos, "std imports should be sorted");
        assert!(stdio_pos < mylib_pos, "std imports before others");
        assert!(another_pos < mylib_pos, "non-std imports should be sorted");
    }

    #[test]
    fn format_method_call_chain() {
        let source = "fn f() {\n  obj.method1().method2()\n}\n";
        let formatted = format_source(source).unwrap();
        assert!(formatted.contains("obj.method1().method2()"));
        let second = format_source(&formatted).unwrap();
        assert_eq!(formatted, second);
    }

    #[test]
    fn format_array_literal() {
        let source = "fn f() {\n  let a = [1, 2, 3]\n}\n";
        let formatted = format_source(source).unwrap();
        assert!(formatted.contains("[1, 2, 3]"));
        let second = format_source(&formatted).unwrap();
        assert_eq!(formatted, second);
    }

    #[test]
    fn format_tuple_type() {
        let source = "fn f(x: (i32, String)) -> (bool, i32) {\n  (true, 0)\n}\n";
        let formatted = format_source(source).unwrap();
        assert!(formatted.contains("(i32, String)"));
        assert!(formatted.contains("(bool, i32)"));
        let second = format_source(&formatted).unwrap();
        assert_eq!(formatted, second);
    }

    #[test]
    fn format_result_type() {
        let source = "fn f() -> Result<String, String> {\n  Ok(\"hi\")\n}\n";
        let formatted = format_source(source).unwrap();
        assert!(formatted.contains("Result<String, String>"));
        let second = format_source(&formatted).unwrap();
        assert_eq!(formatted, second);
    }

    #[test]
    fn format_doc_comment() {
        let source = "/// A documented function.\nfn documented() {\n}\n";
        let formatted = format_source(source).unwrap();
        assert!(formatted.contains("/// A documented function."));
        assert!(formatted.contains("fn documented()"));
        let second = format_source(&formatted).unwrap();
        assert_eq!(formatted, second);
    }

    #[test]
    fn format_pub_function() {
        let source = "pub fn public_fn(x: i32) -> i32 {\n  x\n}\n";
        let formatted = format_source(source).unwrap();
        assert!(formatted.contains("pub fn public_fn(x: i32) -> i32 {"));
        let second = format_source(&formatted).unwrap();
        assert_eq!(formatted, second);
    }

    #[test]
    fn format_closure_expr() {
        let source = "fn f() {\n  let add = |a: i32, b: i32| -> i32 { a + b }\n}\n";
        let formatted = format_source(source).unwrap();
        assert!(formatted.contains("|a: i32, b: i32|"));
        let second = format_source(&formatted).unwrap();
        assert_eq!(formatted, second);
    }

    #[test]
    fn format_return_stmt() {
        let source = "fn f() -> i32 {\n  return 42\n}\n";
        let formatted = format_source(source).unwrap();
        assert!(formatted.contains("return 42"));
        let second = format_source(&formatted).unwrap();
        assert_eq!(formatted, second);
    }

    #[test]
    fn format_returns_none_on_parse_error() {
        let source = "fn broken( { let x = }\n";
        assert!(
            format_source(source).is_none(),
            "formatter should return None for invalid input"
        );
    }

    #[test]
    fn format_returns_none_on_unterminated_string() {
        let source = "fn f() {\n  let s = \"unterminated\n}\n";
        assert!(
            format_source(source).is_none(),
            "formatter should return None for lex errors"
        );
    }

    #[test]
    fn sort_imports_standalone_reorders() {
        let source =
            "use mylib\nuse std::math\nuse std::host::stdio\n\nfn main() {\n    let x = 1\n}\n";
        let sorted = sort_imports(source).unwrap();
        let math_pos = sorted.find("use std::math").unwrap();
        let stdio_pos = sorted.find("use std::host::stdio").unwrap();
        let mylib_pos = sorted.find("use mylib").unwrap();
        assert!(
            stdio_pos < math_pos,
            "std imports should be sorted alphabetically"
        );
        assert!(math_pos < mylib_pos, "std imports before non-std");
        // Body must be preserved verbatim
        assert!(sorted.contains("fn main()"));
        assert!(sorted.contains("let x = 1"));
    }

    #[test]
    fn sort_imports_no_imports_unchanged() {
        let source = "fn main() {\n    let x = 1\n}\n";
        let sorted = sort_imports(source).unwrap();
        assert_eq!(sorted, source);
    }

    #[test]
    fn sort_imports_returns_none_on_parse_error() {
        let source = "use std::math\nfn broken( {\n";
        assert!(sort_imports(source).is_none());
    }

    #[test]
    fn sort_imports_preserves_body_formatting() {
        // Body has weird indentation that should NOT be changed by sort_imports
        let source = "use std::string\nuse std::math\n\nfn   main()  {\n  let   x=1\n}\n";
        let sorted = sort_imports(source).unwrap();
        // Imports should be sorted
        assert!(sorted.find("use std::math").unwrap() < sorted.find("use std::string").unwrap());
        // Body should be preserved exactly as-is
        assert!(sorted.contains("fn   main()  {\n  let   x=1\n}"));
    }

    #[test]
    fn format_preserves_line_comments_between_items() {
        let source = "fn foo() -> i32 {\n    1\n}\n\n// helper function\nfn bar() -> i32 {\n    2\n}\n";
        let formatted = format_source(source).unwrap();
        assert!(
            formatted.contains("// helper function"),
            "line comment between items should be preserved: {}",
            formatted
        );
    }

    #[test]
    fn format_preserves_leading_comments() {
        let source = "// This is a top-level comment\nfn main() {\n    let x = 1\n}\n";
        let formatted = format_source(source).unwrap();
        assert!(
            formatted.contains("// This is a top-level comment"),
            "leading comment should be preserved: {}",
            formatted
        );
    }

    #[test]
    fn format_preserves_block_comments() {
        let source = "/* block comment */\nfn main() {\n    let x = 1\n}\n";
        let formatted = format_source(source).unwrap();
        assert!(
            formatted.contains("/* block comment */"),
            "block comment should be preserved: {}",
            formatted
        );
    }

    #[test]
    fn format_comment_idempotent() {
        let source = "// helper\nfn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n\n// another helper\nfn sub(a: i32, b: i32) -> i32 {\n    a - b\n}\n";
        let first = format_source(source).unwrap();
        let second = format_source(&first).unwrap();
        assert_eq!(first, second, "formatting with comments should be idempotent");
    }
}
