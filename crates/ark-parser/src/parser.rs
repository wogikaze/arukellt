//! Pratt parser for Arukellt.

use ark_diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSink, Span};
use ark_lexer::{Token, TokenKind};

use crate::ast::*;

pub struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
    sink: &'a mut DiagnosticSink,
    pending_gt: bool,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [Token], sink: &'a mut DiagnosticSink) -> Self {
        let mut p = Self {
            tokens,
            pos: 0,
            sink,
            pending_gt: false,
        };
        p.skip_newlines();
        p
    }

    fn skip_newlines(&mut self) {
        while self.pos < self.tokens.len() && self.tokens[self.pos].kind == TokenKind::Newline {
            self.pos += 1;
        }
    }

    fn peek(&self) -> &TokenKind {
        self.tokens
            .get(self.pos)
            .map(|t| &t.kind)
            .unwrap_or(&TokenKind::Eof)
    }

    fn span(&self) -> Span {
        self.tokens
            .get(self.pos)
            .map(|t| t.span)
            .unwrap_or(Span::dummy())
    }

    fn advance(&mut self) -> &Token {
        let tok = &self.tokens[self.pos.min(self.tokens.len() - 1)];
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        self.skip_newlines();
        tok
    }

    fn expect(&mut self, expected: &TokenKind) -> Span {
        if self.peek() == expected {
            self.advance().span
        } else {
            let sp = self.span();
            self.sink.emit(
                Diagnostic::new(DiagnosticCode::E0002)
                    .with_message(format!(
                        "expected `{:?}`, found `{:?}`",
                        expected,
                        self.peek()
                    ))
                    .with_label(sp, "here"),
            );
            sp
        }
    }

    fn eat(&mut self, kind: &TokenKind) -> bool {
        if self.peek() == kind {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect_ident(&mut self) -> String {
        if let TokenKind::Ident(name) = self.peek().clone() {
            self.advance();
            name
        } else {
            let sp = self.span();
            self.sink.emit(
                Diagnostic::new(DiagnosticCode::E0002)
                    .with_message(format!("expected identifier, found `{:?}`", self.peek()))
                    .with_label(sp, "here"),
            );
            "<error>".to_string()
        }
    }

    fn collect_outer_doc_comments(&mut self) -> Vec<String> {
        let mut docs = Vec::new();
        while let TokenKind::OuterDocComment(text) = self.peek().clone() {
            self.advance();
            docs.push(text);
        }
        docs
    }

    fn collect_inner_doc_comments(&mut self) -> Vec<String> {
        let mut docs = Vec::new();
        while let TokenKind::InnerDocComment(text) = self.peek().clone() {
            self.advance();
            docs.push(text);
        }
        docs
    }

    fn emit_doc_comment_error(&mut self, message: &str) {
        let sp = self.span();
        self.sink.emit(
            Diagnostic::new(DiagnosticCode::E0001)
                .with_message(message)
                .with_label(sp, "here"),
        );
    }

    // === Module parsing ===

    pub fn parse_module(&mut self) -> Module {
        let docs = self.collect_inner_doc_comments();
        let mut imports = Vec::new();
        let mut items = Vec::new();

        while *self.peek() != TokenKind::Eof {
            let item_docs = self.collect_outer_doc_comments();
            if matches!(self.peek(), TokenKind::InnerDocComment(_)) {
                self.emit_doc_comment_error("inner doc comments are only allowed at module start");
                self.advance();
                continue;
            }

            // Check for reserved keyword violations
            if let TokenKind::Reserved(kw) = self.peek() {
                let kw = kw.clone();
                let sp = self.span();
                let code = DiagnosticCode::E0003;
                self.sink.emit(
                    Diagnostic::new(code)
                        .with_label(sp, format!("`{}` is not available in this version", kw)),
                );
                self.advance();
                self.synchronize();
                continue;
            }

            if *self.peek() == TokenKind::Import {
                if !item_docs.is_empty() {
                    self.emit_doc_comment_error("doc comments on imports are not supported");
                }
                imports.push(self.parse_import());
            } else if *self.peek() == TokenKind::Use {
                if !item_docs.is_empty() {
                    self.emit_doc_comment_error("doc comments on imports are not supported");
                }
                imports.push(self.parse_use_import());
            } else {
                match self.parse_item(item_docs) {
                    Some(item) => items.push(item),
                    None => {
                        self.advance();
                        self.synchronize();
                    }
                }
            }
        }

        Module {
            docs,
            imports,
            items,
        }
    }

    fn synchronize(&mut self) {
        while *self.peek() != TokenKind::Eof {
            match self.peek() {
                TokenKind::Fn
                | TokenKind::Struct
                | TokenKind::Enum
                | TokenKind::Trait
                | TokenKind::Impl
                | TokenKind::Import
                | TokenKind::OuterDocComment(_)
                | TokenKind::InnerDocComment(_)
                | TokenKind::Pub => return,
                TokenKind::RBrace => {
                    self.advance();
                    return;
                }
                _ => {
                    self.advance();
                }
            }
        }
    }

    fn parse_import(&mut self) -> Import {
        let start = self.span();
        self.expect(&TokenKind::Import);
        let name = self.expect_ident();
        let alias = if self.eat(&TokenKind::As) {
            Some(self.expect_ident())
        } else {
            None
        };
        Import {
            module_name: name,
            alias,
            span: start.merge(self.span()),
        }
    }

    /// Parse `use std::foo::bar` or `use std::foo::{bar, baz}`.
    /// Produces one Import per path segment (destructuring expands to multiple).
    fn parse_use_import(&mut self) -> Import {
        let start = self.span();
        self.expect(&TokenKind::Use);

        // Parse path segments separated by ::
        let mut segments = vec![self.expect_ident()];
        while self.eat(&TokenKind::ColonColon) {
            // Check for destructuring: use std::foo::{bar, baz}
            // For now we don't support destructuring — just parse a single path
            segments.push(self.expect_ident());
        }

        let module_name = segments.join("::");
        let alias = if self.eat(&TokenKind::As) {
            Some(self.expect_ident())
        } else {
            // Default alias is the last segment
            None
        };

        Import {
            module_name,
            alias,
            span: start.merge(self.span()),
        }
    }

    fn parse_item(&mut self, docs: Vec<String>) -> Option<Item> {
        let is_pub = self.eat(&TokenKind::Pub);
        match self.peek() {
            TokenKind::Fn => Some(Item::FnDef(self.parse_fn_def(docs, is_pub))),
            TokenKind::Struct => Some(Item::StructDef(self.parse_struct_def(docs, is_pub))),
            TokenKind::Enum => Some(Item::EnumDef(self.parse_enum_def(docs, is_pub))),
            TokenKind::Trait => Some(Item::TraitDef(self.parse_trait_def(docs, is_pub))),
            TokenKind::Impl => {
                if is_pub {
                    let sp = self.span();
                    self.sink.emit(
                        Diagnostic::new(DiagnosticCode::E0001)
                            .with_message("`pub` is not allowed on impl blocks")
                            .with_label(sp, "here"),
                    );
                }
                Some(Item::ImplBlock(self.parse_impl_block(docs)))
            }
            _ => {
                let sp = self.span();
                self.sink.emit(
                    Diagnostic::new(DiagnosticCode::E0001)
                        .with_message(format!(
                            "expected item (fn, struct, enum, trait, impl), found `{:?}`",
                            self.peek()
                        ))
                        .with_label(sp, "here"),
                );
                None
            }
        }
    }

    fn parse_fn_def(&mut self, docs: Vec<String>, is_pub: bool) -> FnDef {
        let start = self.span();
        self.expect(&TokenKind::Fn);
        let name = self.expect_ident();

        // Generic params with optional bounds
        let (type_params, type_param_bounds) = if *self.peek() == TokenKind::Lt {
            self.parse_type_params_with_bounds()
        } else {
            (Vec::new(), Vec::new())
        };

        // Params
        self.expect(&TokenKind::LParen);
        let params = self.parse_params();
        self.expect(&TokenKind::RParen);

        // Return type
        let return_type = if self.eat(&TokenKind::Arrow) {
            Some(self.parse_type_expr())
        } else {
            None
        };

        let body = self.parse_block();
        let span = start.merge(body.span);
        FnDef {
            docs,
            name,
            type_params,
            type_param_bounds,
            params,
            return_type,
            body,
            is_pub,
            span,
        }
    }

    fn parse_type_params(&mut self) -> Vec<String> {
        let (names, _) = self.parse_type_params_with_bounds();
        names
    }

    /// Parse type params with optional bounds: `<T: Display, U>` → (["T","U"], [("T",["Display"])])
    fn parse_type_params_with_bounds(&mut self) -> (Vec<String>, Vec<(String, Vec<String>)>) {
        self.expect(&TokenKind::Lt);
        let mut names = Vec::new();
        let mut bounds = Vec::new();
        loop {
            if *self.peek() == TokenKind::Gt {
                break;
            }
            let name = self.expect_ident();
            if self.eat(&TokenKind::Colon) {
                let mut trait_bounds = Vec::new();
                trait_bounds.push(self.expect_ident());
                while self.eat(&TokenKind::Plus) {
                    trait_bounds.push(self.expect_ident());
                }
                bounds.push((name.clone(), trait_bounds));
            }
            names.push(name);
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(&TokenKind::Gt);
        (names, bounds)
    }

    fn parse_params(&mut self) -> Vec<Param> {
        let mut params = Vec::new();
        while *self.peek() != TokenKind::RParen && *self.peek() != TokenKind::Eof {
            let start = self.span();
            let name = self.expect_ident();
            if *self.peek() != TokenKind::Colon {
                let sp = self.span();
                self.sink.emit(
                    Diagnostic::new(DiagnosticCode::E0201)
                        .with_message("missing type annotation")
                        .with_label(
                            sp,
                            format!("parameter `{}` requires a type annotation", name),
                        )
                        .with_suggestion(format!("add a type: `{}: Type`", name)),
                );
                // Skip to next comma or rparen
                while *self.peek() != TokenKind::Comma
                    && *self.peek() != TokenKind::RParen
                    && *self.peek() != TokenKind::Eof
                {
                    self.advance();
                }
                if self.eat(&TokenKind::Comma) {
                    continue;
                }
                break;
            }
            self.expect(&TokenKind::Colon);
            let ty = self.parse_type_expr();
            let span = start.merge(self.span());
            params.push(Param { name, ty, span });
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        params
    }

    fn parse_struct_def(&mut self, docs: Vec<String>, is_pub: bool) -> StructDef {
        let start = self.span();
        self.expect(&TokenKind::Struct);
        let name = self.expect_ident();
        let type_params = if *self.peek() == TokenKind::Lt {
            self.parse_type_params()
        } else {
            Vec::new()
        };
        self.expect(&TokenKind::LBrace);
        let fields = self.parse_fields();
        let end = self.expect(&TokenKind::RBrace);
        StructDef {
            docs,
            name,
            type_params,
            fields,
            is_pub,
            span: start.merge(end),
        }
    }

    fn parse_fields(&mut self) -> Vec<Field> {
        let mut fields = Vec::new();
        while *self.peek() != TokenKind::RBrace && *self.peek() != TokenKind::Eof {
            let start = self.span();
            let name = self.expect_ident();
            self.expect(&TokenKind::Colon);
            let ty = self.parse_type_expr();
            let span = start.merge(self.span());
            fields.push(Field { name, ty, span });
            // Optional comma or newline separation
            self.eat(&TokenKind::Comma);
        }
        fields
    }

    fn parse_enum_def(&mut self, docs: Vec<String>, is_pub: bool) -> EnumDef {
        let start = self.span();
        self.expect(&TokenKind::Enum);
        let name = self.expect_ident();

        let type_params = if *self.peek() == TokenKind::Lt {
            self.parse_type_params()
        } else {
            Vec::new()
        };

        self.expect(&TokenKind::LBrace);
        let mut variants = Vec::new();
        while *self.peek() != TokenKind::RBrace && *self.peek() != TokenKind::Eof {
            let vstart = self.span();
            let vname = self.expect_ident();
            if *self.peek() == TokenKind::LParen {
                // Tuple variant
                self.advance();
                let mut fields = Vec::new();
                while *self.peek() != TokenKind::RParen && *self.peek() != TokenKind::Eof {
                    fields.push(self.parse_type_expr());
                    if !self.eat(&TokenKind::Comma) {
                        break;
                    }
                }
                let end = self.expect(&TokenKind::RParen);
                variants.push(Variant::Tuple {
                    name: vname,
                    fields,
                    span: vstart.merge(end),
                });
            } else if *self.peek() == TokenKind::LBrace {
                // Struct variant
                self.advance();
                let fields = self.parse_fields();
                let end = self.expect(&TokenKind::RBrace);
                variants.push(Variant::Struct {
                    name: vname,
                    fields,
                    span: vstart.merge(end),
                });
            } else {
                variants.push(Variant::Unit {
                    name: vname,
                    span: vstart.merge(self.span()),
                });
            }
            self.eat(&TokenKind::Comma);
        }
        let end = self.expect(&TokenKind::RBrace);
        EnumDef {
            docs,
            name,
            type_params,
            variants,
            is_pub,
            span: start.merge(end),
        }
    }

    // === Trait / Impl parsing ===

    fn parse_trait_def(&mut self, docs: Vec<String>, is_pub: bool) -> TraitDef {
        let start = self.span();
        self.expect(&TokenKind::Trait);
        let name = self.expect_ident();

        let type_params = if *self.peek() == TokenKind::Lt {
            self.parse_type_params()
        } else {
            Vec::new()
        };

        self.expect(&TokenKind::LBrace);
        let mut methods = Vec::new();
        while *self.peek() != TokenKind::RBrace && *self.peek() != TokenKind::Eof {
            let method_docs = self.collect_outer_doc_comments();
            if *self.peek() == TokenKind::Fn {
                let m_start = self.span();
                self.expect(&TokenKind::Fn);
                let m_name = self.expect_ident();
                self.expect(&TokenKind::LParen);
                let params = self.parse_method_params(&name);
                self.expect(&TokenKind::RParen);
                let return_type = if self.eat(&TokenKind::Arrow) {
                    Some(self.parse_type_expr())
                } else {
                    None
                };
                let span = m_start.merge(self.span());
                methods.push(TraitMethodSig {
                    docs: method_docs,
                    name: m_name,
                    params,
                    return_type,
                    span,
                });
            } else {
                if !method_docs.is_empty() {
                    self.emit_doc_comment_error("doc comments inside traits must attach to methods");
                }
                // Skip unexpected tokens inside trait
                self.advance();
            }
        }
        let end = self.expect(&TokenKind::RBrace);
        TraitDef {
            docs,
            name,
            type_params,
            methods,
            is_pub,
            span: start.merge(end),
        }
    }

    fn parse_impl_block(&mut self, docs: Vec<String>) -> ImplBlock {
        let start = self.span();
        self.expect(&TokenKind::Impl);
        let first_name = self.expect_ident();

        // Distinguish `impl Trait for Type` vs `impl Type`
        let (trait_name, target_type) = if *self.peek() == TokenKind::For {
            self.advance(); // eat `for`
            let target = self.expect_ident();
            (Some(first_name), target)
        } else {
            (None, first_name)
        };

        self.expect(&TokenKind::LBrace);
        let mut methods = Vec::new();
        while *self.peek() != TokenKind::RBrace && *self.peek() != TokenKind::Eof {
            let method_docs = self.collect_outer_doc_comments();
            if *self.peek() == TokenKind::Fn {
                let m_start = self.span();
                self.expect(&TokenKind::Fn);
                let m_name = self.expect_ident();
                self.expect(&TokenKind::LParen);
                let params = self.parse_method_params(&target_type);
                self.expect(&TokenKind::RParen);
                let return_type = if self.eat(&TokenKind::Arrow) {
                    Some(self.parse_type_expr())
                } else {
                    None
                };
                let body = self.parse_block();
                let span = m_start.merge(body.span);
                methods.push(FnDef {
                    docs: method_docs,
                    name: m_name,
                    type_params: vec![],
                    type_param_bounds: vec![],
                    params,
                    return_type,
                    body,
                    is_pub: false,
                    span,
                });
            } else {
                if !method_docs.is_empty() {
                    self.emit_doc_comment_error("doc comments inside impl blocks must attach to methods");
                }
                self.advance();
            }
        }
        let end = self.expect(&TokenKind::RBrace);
        ImplBlock {
            docs,
            trait_name,
            target_type,
            methods,
            span: start.merge(end),
        }
    }

    /// Parse method parameters with support for bare `self`.
    fn parse_method_params(&mut self, self_type_name: &str) -> Vec<Param> {
        let mut params = Vec::new();
        while *self.peek() != TokenKind::RParen && *self.peek() != TokenKind::Eof {
            let start = self.span();
            let name = self.expect_ident();

            if name == "self" && *self.peek() != TokenKind::Colon {
                // Bare `self` — type is inferred from impl target
                let span = start.merge(self.span());
                params.push(Param {
                    name,
                    ty: TypeExpr::Named {
                        name: self_type_name.to_string(),
                        span,
                    },
                    span,
                });
                if !self.eat(&TokenKind::Comma) {
                    break;
                }
                continue;
            }

            // Regular parameter with type annotation
            if *self.peek() != TokenKind::Colon {
                let sp = self.span();
                self.sink.emit(
                    Diagnostic::new(DiagnosticCode::E0201)
                        .with_message("missing type annotation")
                        .with_label(
                            sp,
                            format!("parameter `{}` requires a type annotation", name),
                        )
                        .with_suggestion(format!("add a type: `{}: Type`", name)),
                );
                while *self.peek() != TokenKind::Comma
                    && *self.peek() != TokenKind::RParen
                    && *self.peek() != TokenKind::Eof
                {
                    self.advance();
                }
                if self.eat(&TokenKind::Comma) {
                    continue;
                }
                break;
            }
            self.expect(&TokenKind::Colon);
            let ty = self.parse_type_expr();
            let span = start.merge(self.span());
            params.push(Param { name, ty, span });
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        params
    }

    // === Type expressions ===

    fn parse_type_expr(&mut self) -> TypeExpr {
        let start = self.span();
        match self.peek().clone() {
            TokenKind::LParen => {
                self.advance();
                if *self.peek() == TokenKind::RParen {
                    let end = self.advance().span;
                    return TypeExpr::Unit(start.merge(end));
                }
                let mut types = vec![self.parse_type_expr()];
                while self.eat(&TokenKind::Comma) {
                    types.push(self.parse_type_expr());
                }
                self.expect(&TokenKind::RParen);
                if types.len() == 1 {
                    types.into_iter().next().unwrap()
                } else {
                    TypeExpr::Tuple(types, start.merge(self.span()))
                }
            }
            TokenKind::LBracket => {
                self.advance();
                let elem = self.parse_type_expr();
                if self.eat(&TokenKind::Semi) {
                    // Array [T; N]
                    if let TokenKind::IntLit(n) = self.peek().clone() {
                        self.advance();
                        let end = self.expect(&TokenKind::RBracket);
                        TypeExpr::Array {
                            elem: Box::new(elem),
                            size: n as u64,
                            span: start.merge(end),
                        }
                    } else {
                        self.expect(&TokenKind::RBracket);
                        TypeExpr::Array {
                            elem: Box::new(elem),
                            size: 0,
                            span: start.merge(self.span()),
                        }
                    }
                } else {
                    // Slice [T]
                    let end = self.expect(&TokenKind::RBracket);
                    TypeExpr::Slice {
                        elem: Box::new(elem),
                        span: start.merge(end),
                    }
                }
            }
            TokenKind::Fn => {
                self.advance();
                self.expect(&TokenKind::LParen);
                let mut params = Vec::new();
                while *self.peek() != TokenKind::RParen && *self.peek() != TokenKind::Eof {
                    params.push(self.parse_type_expr());
                    if !self.eat(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(&TokenKind::RParen);
                self.expect(&TokenKind::Arrow);
                let ret = self.parse_type_expr();
                TypeExpr::Function {
                    params,
                    ret: Box::new(ret),
                    span: start.merge(self.span()),
                }
            }
            TokenKind::Ident(name) => {
                self.advance();
                // Check for qualified type: mod.Type
                if *self.peek() == TokenKind::Dot {
                    self.advance();
                    let type_name = self.expect_ident();
                    return TypeExpr::Qualified {
                        module: name,
                        name: type_name,
                        span: start.merge(self.span()),
                    };
                }
                // Check for generic type: Type<A, B>
                if *self.peek() == TokenKind::Lt {
                    self.advance();
                    let mut args = Vec::new();
                    loop {
                        if *self.peek() == TokenKind::Gt {
                            break;
                        }
                        args.push(self.parse_type_expr());
                        if !self.eat(&TokenKind::Comma) {
                            break;
                        }
                    }
                    // Handle nested generics: Vec<Vec<i32>> produces Shr (`>>`) token.
                    // Split Shr into two `>` by consuming it and leaving a pending `>`.
                    if *self.peek() == TokenKind::Shr {
                        self.advance();
                        self.pending_gt = true;
                        return TypeExpr::Generic {
                            name,
                            args,
                            span: start.merge(self.span()),
                        };
                    }
                    if self.pending_gt {
                        self.pending_gt = false;
                    } else {
                        self.expect(&TokenKind::Gt);
                    }
                    return TypeExpr::Generic {
                        name,
                        args,
                        span: start.merge(self.span()),
                    };
                }
                TypeExpr::Named {
                    name,
                    span: start.merge(self.span()),
                }
            }
            _ => {
                let sp = self.span();
                self.sink.emit(
                    Diagnostic::new(DiagnosticCode::E0001)
                        .with_message(format!("expected type, found `{:?}`", self.peek()))
                        .with_label(sp, "here"),
                );
                self.advance();
                TypeExpr::Named {
                    name: "<error>".to_string(),
                    span: sp,
                }
            }
        }
    }

    // === Block ===

    fn parse_block(&mut self) -> Block {
        let start = self.span();
        self.expect(&TokenKind::LBrace);
        let mut stmts = Vec::new();
        let mut tail_expr = None;

        while *self.peek() != TokenKind::RBrace && *self.peek() != TokenKind::Eof {
            // Check for reserved keyword violations inside blocks
            if let TokenKind::Reserved(kw) = self.peek() {
                let kw = kw.clone();
                let sp = self.span();
                let code = DiagnosticCode::E0003;
                self.sink.emit(
                    Diagnostic::new(code)
                        .with_label(sp, format!("`{}` is not available in this version", kw)),
                );
                self.advance();
                self.synchronize();
                continue;
            }
            // Try to parse as statement
            match self.peek() {
                TokenKind::Let => {
                    stmts.push(self.parse_let_stmt());
                }
                TokenKind::While => {
                    stmts.push(self.parse_while_stmt());
                }
                TokenKind::Loop => {
                    stmts.push(self.parse_loop_stmt());
                }
                TokenKind::For => {
                    stmts.push(self.parse_for_stmt());
                }
                _ => {
                    let expr = self.parse_expr();
                    if self.eat(&TokenKind::Semi) {
                        stmts.push(Stmt::Expr(expr));
                    } else if *self.peek() == TokenKind::RBrace {
                        // tail expression
                        tail_expr = Some(Box::new(expr));
                    } else {
                        // expression statement (no semicolon but not at end)
                        stmts.push(Stmt::Expr(expr));
                    }
                }
            }
        }
        let end = self.expect(&TokenKind::RBrace);
        Block {
            stmts,
            tail_expr,
            span: start.merge(end),
        }
    }

    fn parse_let_stmt(&mut self) -> Stmt {
        let start = self.span();
        self.expect(&TokenKind::Let);
        let is_mut = self.eat(&TokenKind::Mut);

        // Check for tuple destructuring: let (a, b) = ...
        let (name, pattern) = if *self.peek() == TokenKind::LParen {
            let pat = self.parse_pattern();
            ("_tuple".to_string(), Some(pat))
        } else {
            (self.expect_ident(), None)
        };

        let ty = if self.eat(&TokenKind::Colon) {
            Some(self.parse_type_expr())
        } else {
            None
        };

        self.expect(&TokenKind::Eq);
        let init = self.parse_expr();
        // Optional semicolon
        self.eat(&TokenKind::Semi);

        Stmt::Let {
            name,
            ty,
            init,
            is_mut,
            pattern,
            span: start.merge(self.span()),
        }
    }

    fn parse_while_stmt(&mut self) -> Stmt {
        let start = self.span();
        self.expect(&TokenKind::While);
        let cond = self.parse_expr();
        let body = self.parse_block();
        Stmt::While {
            cond,
            body,
            span: start.merge(self.span()),
        }
    }

    fn parse_loop_stmt(&mut self) -> Stmt {
        let start = self.span();
        self.expect(&TokenKind::Loop);
        let body = self.parse_block();
        Stmt::Loop {
            body,
            span: start.merge(self.span()),
        }
    }

    fn parse_for_stmt(&mut self) -> Stmt {
        let start = self.span();
        self.expect(&TokenKind::For);
        let target = self.expect_ident();
        self.expect(&TokenKind::In);

        // Parse iterator: `values(expr)`, `start..end` (range), or generic `expr` (Iterator)
        let iter = if let TokenKind::Ident(name) = self.peek() {
            if name == "values" {
                // values(expr) form
                self.advance(); // consume 'values'
                self.expect(&TokenKind::LParen);
                let expr = self.parse_expr();
                self.expect(&TokenKind::RParen);
                ForIter::Values(expr)
            } else {
                // Parse expression, then check for `..` (range) or treat as iterator
                let expr = self.parse_expr();
                if *self.peek() == TokenKind::DotDot {
                    self.advance(); // consume '..'
                    let range_end = self.parse_expr();
                    ForIter::Range {
                        start: expr,
                        end: range_end,
                    }
                } else {
                    // Generic iterator expression
                    ForIter::Iter(expr)
                }
            }
        } else {
            // Numeric or expression: parse, then check for `..`
            let expr = self.parse_expr();
            if *self.peek() == TokenKind::DotDot {
                self.advance(); // consume '..'
                let range_end = self.parse_expr();
                ForIter::Range {
                    start: expr,
                    end: range_end,
                }
            } else {
                ForIter::Iter(expr)
            }
        };

        let body = self.parse_block();
        Stmt::For {
            target,
            iter,
            body,
            span: start.merge(self.span()),
        }
    }

    // === Expressions (Pratt parser) ===

    fn parse_expr(&mut self) -> Expr {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Expr {
        let expr = self.parse_or();
        if *self.peek() == TokenKind::Eq {
            let start = expr.span();
            self.advance();
            let value = self.parse_assignment();
            let span = start.merge(value.span());
            Expr::Assign {
                target: Box::new(expr),
                value: Box::new(value),
                span,
            }
        } else {
            expr
        }
    }

    fn parse_or(&mut self) -> Expr {
        let mut left = self.parse_and();
        while *self.peek() == TokenKind::PipePipe {
            let start = left.span();
            self.advance();
            let right = self.parse_and();
            let span = start.merge(right.span());
            left = Expr::Binary {
                left: Box::new(left),
                op: BinOp::Or,
                right: Box::new(right),
                span,
            };
        }
        left
    }

    fn parse_and(&mut self) -> Expr {
        let mut left = self.parse_equality();
        while *self.peek() == TokenKind::AmpAmp {
            let start = left.span();
            self.advance();
            let right = self.parse_equality();
            let span = start.merge(right.span());
            left = Expr::Binary {
                left: Box::new(left),
                op: BinOp::And,
                right: Box::new(right),
                span,
            };
        }
        left
    }

    fn parse_equality(&mut self) -> Expr {
        let mut left = self.parse_comparison();
        loop {
            let op = match self.peek() {
                TokenKind::EqEq => BinOp::Eq,
                TokenKind::BangEq => BinOp::Ne,
                _ => break,
            };
            let start = left.span();
            self.advance();
            let right = self.parse_comparison();
            let span = start.merge(right.span());
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        left
    }

    fn parse_comparison(&mut self) -> Expr {
        let mut left = self.parse_bitor();
        loop {
            let op = match self.peek() {
                TokenKind::Lt => BinOp::Lt,
                TokenKind::LtEq => BinOp::Le,
                TokenKind::Gt => BinOp::Gt,
                TokenKind::GtEq => BinOp::Ge,
                _ => break,
            };
            let start = left.span();
            self.advance();
            let right = self.parse_bitor();
            let span = start.merge(right.span());
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        left
    }

    fn parse_bitor(&mut self) -> Expr {
        let mut left = self.parse_bitxor();
        while *self.peek() == TokenKind::Pipe {
            let start = left.span();
            self.advance();
            let right = self.parse_bitxor();
            let span = start.merge(right.span());
            left = Expr::Binary {
                left: Box::new(left),
                op: BinOp::BitOr,
                right: Box::new(right),
                span,
            };
        }
        left
    }

    fn parse_bitxor(&mut self) -> Expr {
        let mut left = self.parse_bitand();
        while *self.peek() == TokenKind::Caret {
            let start = left.span();
            self.advance();
            let right = self.parse_bitand();
            let span = start.merge(right.span());
            left = Expr::Binary {
                left: Box::new(left),
                op: BinOp::BitXor,
                right: Box::new(right),
                span,
            };
        }
        left
    }

    fn parse_bitand(&mut self) -> Expr {
        let mut left = self.parse_shift();
        while *self.peek() == TokenKind::Amp {
            let start = left.span();
            self.advance();
            let right = self.parse_shift();
            let span = start.merge(right.span());
            left = Expr::Binary {
                left: Box::new(left),
                op: BinOp::BitAnd,
                right: Box::new(right),
                span,
            };
        }
        left
    }

    fn parse_shift(&mut self) -> Expr {
        let mut left = self.parse_additive();
        loop {
            let op = match self.peek() {
                TokenKind::Shl => BinOp::Shl,
                TokenKind::Shr => BinOp::Shr,
                _ => break,
            };
            let start = left.span();
            self.advance();
            let right = self.parse_additive();
            let span = start.merge(right.span());
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        left
    }

    fn parse_additive(&mut self) -> Expr {
        let mut left = self.parse_multiplicative();
        loop {
            let op = match self.peek() {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            let start = left.span();
            self.advance();
            let right = self.parse_multiplicative();
            let span = start.merge(right.span());
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        left
    }

    fn parse_multiplicative(&mut self) -> Expr {
        let mut left = self.parse_unary();
        loop {
            let op = match self.peek() {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                TokenKind::Percent => BinOp::Mod,
                _ => break,
            };
            let start = left.span();
            self.advance();
            let right = self.parse_unary();
            let span = start.merge(right.span());
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        left
    }

    fn parse_unary(&mut self) -> Expr {
        let start = self.span();
        match self.peek() {
            TokenKind::Minus => {
                self.advance();
                let operand = self.parse_unary();
                let span = start.merge(operand.span());
                Expr::Unary {
                    op: UnaryOp::Neg,
                    operand: Box::new(operand),
                    span,
                }
            }
            TokenKind::Bang => {
                self.advance();
                let operand = self.parse_unary();
                let span = start.merge(operand.span());
                Expr::Unary {
                    op: UnaryOp::Not,
                    operand: Box::new(operand),
                    span,
                }
            }
            TokenKind::Tilde => {
                self.advance();
                let operand = self.parse_unary();
                let span = start.merge(operand.span());
                Expr::Unary {
                    op: UnaryOp::BitNot,
                    operand: Box::new(operand),
                    span,
                }
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Expr {
        let mut expr = self.parse_primary();
        loop {
            match self.peek() {
                TokenKind::LParen => {
                    // Function call
                    self.advance();
                    let args = self.parse_call_args();
                    let end = self.expect(&TokenKind::RParen);
                    let span = expr.span().merge(end);
                    expr = Expr::Call {
                        callee: Box::new(expr),
                        type_args: Vec::new(),
                        args,
                        span,
                    };
                }
                TokenKind::LBracket => {
                    // Index
                    self.advance();
                    let index = self.parse_expr();
                    let end = self.expect(&TokenKind::RBracket);
                    let span = expr.span().merge(end);
                    expr = Expr::Index {
                        object: Box::new(expr),
                        index: Box::new(index),
                        span,
                    };
                }
                TokenKind::Dot => {
                    self.advance();
                    let field = self.expect_ident();
                    if *self.peek() == TokenKind::LParen {
                        // Method call: expr.field(args) → Call { callee: FieldAccess, args }
                        let field_span = expr.span().merge(self.span());
                        let callee = Expr::FieldAccess {
                            object: Box::new(expr),
                            field,
                            span: field_span,
                        };
                        self.advance(); // consume '('
                        let args = self.parse_call_args();
                        let end = self.span();
                        self.expect(&TokenKind::RParen);
                        let span = callee.span().merge(end);
                        expr = Expr::Call {
                            callee: Box::new(callee),
                            args,
                            type_args: Vec::new(),
                            span,
                        };
                    } else {
                        let span = expr.span().merge(self.span());
                        expr = Expr::FieldAccess {
                            object: Box::new(expr),
                            field,
                            span,
                        };
                    }
                }
                TokenKind::Question => {
                    let start = expr.span();
                    self.advance();
                    let span = start.merge(self.span());
                    expr = Expr::Try {
                        expr: Box::new(expr),
                        span,
                    };
                }
                _ => break,
            }
        }
        expr
    }

    fn parse_call_args(&mut self) -> Vec<Expr> {
        let mut args = Vec::new();
        while *self.peek() != TokenKind::RParen && *self.peek() != TokenKind::Eof {
            args.push(self.parse_expr());
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        args
    }

    fn parse_primary(&mut self) -> Expr {
        let start = self.span();
        match self.peek().clone() {
            TokenKind::IntLit(v) => {
                self.advance();
                Expr::IntLit {
                    value: v,
                    suffix: None,
                    span: start,
                }
            }
            TokenKind::FloatLit(v) => {
                self.advance();
                Expr::FloatLit {
                    value: v,
                    suffix: None,
                    span: start,
                }
            }
            TokenKind::TypedIntLit(v, s) => {
                self.advance();
                Expr::IntLit {
                    value: v,
                    suffix: Some(s),
                    span: start,
                }
            }
            TokenKind::TypedFloatLit(v, s) => {
                self.advance();
                Expr::FloatLit {
                    value: v,
                    suffix: Some(s),
                    span: start,
                }
            }
            TokenKind::StringLit(v) => {
                self.advance();
                Expr::StringLit {
                    value: v,
                    span: start,
                }
            }
            TokenKind::FStringLit(parts) => {
                self.advance();
                self.desugar_fstring(&parts, start)
            }
            TokenKind::CharLit(v) => {
                self.advance();
                Expr::CharLit {
                    value: v,
                    span: start,
                }
            }
            TokenKind::BoolLit(v) => {
                self.advance();
                Expr::BoolLit {
                    value: v,
                    span: start,
                }
            }
            TokenKind::Ident(name) => {
                self.advance();
                // Qualified: Name::Variant or module.name
                if *self.peek() == TokenKind::ColonColon {
                    self.advance();
                    let variant = self.expect_ident();
                    // Could be enum variant constructor: Enum::Variant(args)
                    if *self.peek() == TokenKind::LParen {
                        self.advance();
                        let args = self.parse_call_args();
                        let end = self.expect(&TokenKind::RParen);
                        let span = start.merge(end);
                        let callee = Expr::QualifiedIdent {
                            module: name,
                            name: variant,
                            span: start.merge(self.span()),
                        };
                        return Expr::Call {
                            callee: Box::new(callee),
                            type_args: Vec::new(),
                            args,
                            span,
                        };
                    }
                    // Enum struct variant constructor: Enum::Variant { field: value }
                    if *self.peek() == TokenKind::LBrace {
                        let qualified = format!("{}::{}", name, variant);
                        if self.could_be_struct_init(&qualified) {
                            return self.parse_struct_init(qualified, start);
                        }
                    }
                    return Expr::QualifiedIdent {
                        module: name,
                        name: variant,
                        span: start.merge(self.span()),
                    };
                }
                // Struct init: Name { field: value, ... }
                if *self.peek() == TokenKind::LBrace && self.could_be_struct_init(&name) {
                    return self.parse_struct_init(name, start);
                }
                Expr::Ident { name, span: start }
            }
            TokenKind::LParen => {
                self.advance();
                if *self.peek() == TokenKind::RParen {
                    // Unit literal ()
                    let end = self.advance().span;
                    return Expr::Tuple {
                        elements: Vec::new(),
                        span: start.merge(end),
                    };
                }
                let expr = self.parse_expr();
                if self.eat(&TokenKind::Comma) {
                    // Tuple
                    let mut elements = vec![expr];
                    while *self.peek() != TokenKind::RParen && *self.peek() != TokenKind::Eof {
                        elements.push(self.parse_expr());
                        if !self.eat(&TokenKind::Comma) {
                            break;
                        }
                    }
                    let end = self.expect(&TokenKind::RParen);
                    Expr::Tuple {
                        elements,
                        span: start.merge(end),
                    }
                } else {
                    self.expect(&TokenKind::RParen);
                    expr // parenthesized expression
                }
            }
            TokenKind::LBracket => {
                self.advance();
                if *self.peek() == TokenKind::RBracket {
                    let end = self.advance().span;
                    return Expr::Array {
                        elements: Vec::new(),
                        span: start.merge(end),
                    };
                }
                let first = self.parse_expr();
                if self.eat(&TokenKind::Semi) {
                    // Array repeat [value; count]
                    let count = self.parse_expr();
                    let end = self.expect(&TokenKind::RBracket);
                    Expr::ArrayRepeat {
                        value: Box::new(first),
                        count: Box::new(count),
                        span: start.merge(end),
                    }
                } else {
                    // Array literal [a, b, c]
                    let mut elements = vec![first];
                    while self.eat(&TokenKind::Comma) {
                        if *self.peek() == TokenKind::RBracket {
                            break;
                        }
                        elements.push(self.parse_expr());
                    }
                    let end = self.expect(&TokenKind::RBracket);
                    Expr::Array {
                        elements,
                        span: start.merge(end),
                    }
                }
            }
            TokenKind::LBrace => Expr::Block(self.parse_block()),
            TokenKind::If => self.parse_if_expr(),
            TokenKind::Match => self.parse_match_expr(),
            TokenKind::Return => {
                self.advance();
                let value = if *self.peek() != TokenKind::Semi && *self.peek() != TokenKind::RBrace
                {
                    Some(Box::new(self.parse_expr()))
                } else {
                    None
                };
                Expr::Return {
                    value,
                    span: start.merge(self.span()),
                }
            }
            TokenKind::Break => {
                self.advance();
                let value = if *self.peek() != TokenKind::Semi && *self.peek() != TokenKind::RBrace
                {
                    Some(Box::new(self.parse_expr()))
                } else {
                    None
                };
                Expr::Break {
                    value,
                    span: start.merge(self.span()),
                }
            }
            TokenKind::Continue => {
                self.advance();
                Expr::Continue { span: start }
            }
            TokenKind::Loop => {
                self.advance();
                let body = self.parse_block();
                Expr::Loop {
                    body,
                    span: start.merge(self.span()),
                }
            }
            TokenKind::Pipe => self.parse_closure(start),
            TokenKind::PipePipe => {
                // || closure (no params)
                self.advance();
                let body = self.parse_expr();
                let span = start.merge(body.span());
                Expr::Closure {
                    params: Vec::new(),
                    return_type: None,
                    body: Box::new(body),
                    span,
                }
            }
            _ => {
                let sp = self.span();
                self.sink.emit(
                    Diagnostic::new(DiagnosticCode::E0001)
                        .with_message(format!("expected expression, found `{:?}`", self.peek()))
                        .with_label(sp, "here"),
                );
                self.advance();
                Expr::IntLit {
                    value: 0,
                    suffix: None,
                    span: sp,
                }
            }
        }
    }

    fn could_be_struct_init(&self, _name: &str) -> bool {
        // Heuristic: Name { ident :  (skip newlines after {)
        let mut i = self.pos + 1; // skip past LBrace
        // Skip newlines after {
        while i < self.tokens.len() && self.tokens[i].kind == TokenKind::Newline {
            i += 1;
        }
        if i + 1 < self.tokens.len() {
            if let TokenKind::Ident(_) = &self.tokens[i].kind {
                // Skip newlines between ident and potential colon
                let mut j = i + 1;
                while j < self.tokens.len() && self.tokens[j].kind == TokenKind::Newline {
                    j += 1;
                }
                if j < self.tokens.len() && self.tokens[j].kind == TokenKind::Colon {
                    return true;
                }
            }
        }
        false
    }

    fn parse_struct_init(&mut self, name: String, start: Span) -> Expr {
        self.expect(&TokenKind::LBrace);
        let mut fields = Vec::new();
        let mut base = None;
        while *self.peek() != TokenKind::RBrace && *self.peek() != TokenKind::Eof {
            // Check for `..expr` (struct field update / base expression)
            if *self.peek() == TokenKind::DotDot {
                self.advance();
                base = Some(Box::new(self.parse_expr()));
                self.eat(&TokenKind::Comma);
                break;
            }
            let fname = self.expect_ident();
            self.expect(&TokenKind::Colon);
            let fval = self.parse_expr();
            fields.push((fname, fval));
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        let end = self.expect(&TokenKind::RBrace);
        Expr::StructInit {
            name,
            fields,
            base,
            span: start.merge(end),
        }
    }

    fn parse_if_expr(&mut self) -> Expr {
        let start = self.span();
        self.expect(&TokenKind::If);
        let cond = self.parse_expr();
        let then_block = self.parse_block();
        let else_block = if self.eat(&TokenKind::Else) {
            if *self.peek() == TokenKind::If {
                // else if -> wrap in block
                let elif = self.parse_if_expr();
                let sp = elif.span();
                Some(Block {
                    stmts: Vec::new(),
                    tail_expr: Some(Box::new(elif)),
                    span: sp,
                })
            } else {
                Some(self.parse_block())
            }
        } else {
            None
        };
        let span = start.merge(self.span());
        Expr::If {
            cond: Box::new(cond),
            then_block,
            else_block,
            span,
        }
    }

    fn parse_match_expr(&mut self) -> Expr {
        let start = self.span();
        self.expect(&TokenKind::Match);
        let scrutinee = self.parse_expr();
        self.expect(&TokenKind::LBrace);
        let mut arms = Vec::new();
        while *self.peek() != TokenKind::RBrace && *self.peek() != TokenKind::Eof {
            let arm_start = self.span();
            let pattern = self.parse_pattern_with_or();
            // Parse optional match guard: `if condition`
            let guard = if *self.peek() == TokenKind::If {
                self.advance();
                Some(Box::new(self.parse_expr()))
            } else {
                None
            };
            self.expect(&TokenKind::FatArrow);
            let body = self.parse_expr();
            let arm_span = arm_start.merge(body.span());
            arms.push(MatchArm {
                pattern,
                guard,
                body,
                span: arm_span,
            });
            self.eat(&TokenKind::Comma);
        }
        let end = self.expect(&TokenKind::RBrace);
        Expr::Match {
            scrutinee: Box::new(scrutinee),
            arms,
            span: start.merge(end),
        }
    }

    /// Parse a pattern with optional or-alternatives: `A | B | C`
    fn parse_pattern_with_or(&mut self) -> Pattern {
        let start = self.span();
        let first = self.parse_pattern();
        if *self.peek() != TokenKind::Pipe {
            return first;
        }
        let mut patterns = vec![first];
        while self.eat(&TokenKind::Pipe) {
            patterns.push(self.parse_pattern());
        }
        Pattern::Or {
            span: start.merge(self.span()),
            patterns,
        }
    }

    fn parse_pattern(&mut self) -> Pattern {
        let start = self.span();
        match self.peek().clone() {
            TokenKind::Ident(name) if name == "_" => {
                self.advance();
                Pattern::Wildcard(start)
            }
            TokenKind::Ident(name) => {
                self.advance();
                // Check for Enum::Variant(...) or Enum::Variant { ... }
                if *self.peek() == TokenKind::ColonColon {
                    self.advance();
                    let variant = self.expect_ident();
                    if *self.peek() == TokenKind::LParen {
                        self.advance();
                        let mut pats = Vec::new();
                        while *self.peek() != TokenKind::RParen && *self.peek() != TokenKind::Eof {
                            pats.push(self.parse_pattern());
                            if !self.eat(&TokenKind::Comma) {
                                break;
                            }
                        }
                        self.expect(&TokenKind::RParen);
                        return Pattern::Enum {
                            path: name,
                            variant,
                            fields: pats,
                            span: start.merge(self.span()),
                        };
                    }
                    // Enum struct variant pattern: Enum::Variant { field, ... }
                    if *self.peek() == TokenKind::LBrace {
                        self.advance();
                        let mut fields = Vec::new();
                        while *self.peek() != TokenKind::RBrace && *self.peek() != TokenKind::Eof {
                            let fname = self.expect_ident();
                            let fpat = if self.eat(&TokenKind::Colon) {
                                Some(self.parse_pattern())
                            } else {
                                None
                            };
                            fields.push((fname, fpat));
                            if !self.eat(&TokenKind::Comma) {
                                break;
                            }
                        }
                        self.expect(&TokenKind::RBrace);
                        let qualified = format!("{}::{}", name, variant);
                        return Pattern::Struct {
                            name: qualified,
                            fields,
                            span: start.merge(self.span()),
                        };
                    }
                    // Unit enum variant pattern
                    return Pattern::Enum {
                        path: name,
                        variant,
                        fields: Vec::new(),
                        span: start.merge(self.span()),
                    };
                }
                // Plain identifier or variant without path
                // Check if it matches Some/None/Ok/Err (common enum constructors)
                if *self.peek() == TokenKind::LParen
                    && matches!(name.as_str(), "Some" | "Ok" | "Err")
                {
                    self.advance();
                    let mut fields = Vec::new();
                    while *self.peek() != TokenKind::RParen && *self.peek() != TokenKind::Eof {
                        fields.push(self.parse_pattern());
                        if !self.eat(&TokenKind::Comma) {
                            break;
                        }
                    }
                    self.expect(&TokenKind::RParen);
                    let enum_name = match name.as_str() {
                        "Some" | "None" => "Option",
                        "Ok" | "Err" => "Result",
                        _ => "<unknown>",
                    };
                    return Pattern::Enum {
                        path: enum_name.to_string(),
                        variant: name,
                        fields,
                        span: start.merge(self.span()),
                    };
                }
                // Check for struct pattern: Point { x, y } or Point { x: pat, y: pat }
                if *self.peek() == TokenKind::LBrace
                    && name.starts_with(|c: char| c.is_uppercase())
                    && !matches!(name.as_str(), "Some" | "None" | "Ok" | "Err")
                {
                    self.advance(); // consume `{`
                    let mut fields = Vec::new();
                    while *self.peek() != TokenKind::RBrace && *self.peek() != TokenKind::Eof {
                        let fname = self.expect_ident();
                        let fpat = if self.eat(&TokenKind::Colon) {
                            Some(self.parse_pattern())
                        } else {
                            None // shorthand: `x` means bind x
                        };
                        fields.push((fname, fpat));
                        if !self.eat(&TokenKind::Comma) {
                            break;
                        }
                    }
                    self.expect(&TokenKind::RBrace);
                    return Pattern::Struct {
                        name,
                        fields,
                        span: start.merge(self.span()),
                    };
                }
                Pattern::Ident { name, span: start }
            }
            TokenKind::IntLit(v) => {
                self.advance();
                Pattern::IntLit {
                    value: v,
                    suffix: None,
                    span: start,
                }
            }
            TokenKind::FloatLit(v) => {
                self.advance();
                Pattern::FloatLit {
                    value: v,
                    suffix: None,
                    span: start,
                }
            }
            TokenKind::TypedIntLit(v, s) => {
                self.advance();
                Pattern::IntLit {
                    value: v,
                    suffix: Some(s),
                    span: start,
                }
            }
            TokenKind::TypedFloatLit(v, s) => {
                self.advance();
                Pattern::FloatLit {
                    value: v,
                    suffix: Some(s),
                    span: start,
                }
            }
            TokenKind::StringLit(v) => {
                self.advance();
                Pattern::StringLit {
                    value: v,
                    span: start,
                }
            }
            TokenKind::CharLit(v) => {
                self.advance();
                Pattern::CharLit {
                    value: v,
                    span: start,
                }
            }
            TokenKind::BoolLit(v) => {
                self.advance();
                Pattern::BoolLit {
                    value: v,
                    span: start,
                }
            }
            TokenKind::Minus => {
                // Negative literal pattern
                self.advance();
                if let TokenKind::IntLit(v) = self.peek().clone() {
                    self.advance();
                    Pattern::IntLit {
                        value: -v,
                        suffix: None,
                        span: start.merge(self.span()),
                    }
                } else {
                    Pattern::IntLit {
                        value: 0,
                        suffix: None,
                        span: start,
                    }
                }
            }
            TokenKind::LParen => {
                self.advance();
                let mut elements = Vec::new();
                while *self.peek() != TokenKind::RParen && *self.peek() != TokenKind::Eof {
                    elements.push(self.parse_pattern());
                    if !self.eat(&TokenKind::Comma) {
                        break;
                    }
                }
                let end = self.expect(&TokenKind::RParen);
                Pattern::Tuple {
                    elements,
                    span: start.merge(end),
                }
            }
            _ => {
                let sp = self.span();
                self.sink.emit(
                    Diagnostic::new(DiagnosticCode::E0001)
                        .with_message(format!("expected pattern, found `{:?}`", self.peek()))
                        .with_label(sp, "here"),
                );
                self.advance();
                Pattern::Wildcard(sp)
            }
        }
    }

    fn parse_closure(&mut self, start: Span) -> Expr {
        self.expect(&TokenKind::Pipe);
        let mut params = Vec::new();
        while *self.peek() != TokenKind::Pipe && *self.peek() != TokenKind::Eof {
            let pstart = self.span();
            let name = self.expect_ident();
            let ty = if self.eat(&TokenKind::Colon) {
                Some(self.parse_type_expr())
            } else {
                None
            };
            params.push(ClosureParam {
                name,
                ty,
                span: pstart.merge(self.span()),
            });
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(&TokenKind::Pipe);
        // Optional return type annotation: -> Type
        let return_type = if self.eat(&TokenKind::Arrow) {
            Some(self.parse_type_expr())
        } else {
            None
        };
        let body = if *self.peek() == TokenKind::LBrace {
            Expr::Block(self.parse_block())
        } else {
            self.parse_expr()
        };
        let span = start.merge(body.span());
        Expr::Closure {
            params,
            return_type,
            body: Box::new(body),
            span,
        }
    }

    /// Desugar `f"hello {x} world"` to `concat(concat("hello ", to_string(x)), " world")`.
    fn desugar_fstring(&mut self, parts: &[ark_lexer::FStringPart], span: Span) -> Expr {
        let mut exprs: Vec<Expr> = Vec::new();

        for part in parts {
            match part {
                ark_lexer::FStringPart::Lit(s) => {
                    exprs.push(Expr::StringLit {
                        value: s.clone(),
                        span,
                    });
                }
                ark_lexer::FStringPart::Expr(text) => {
                    // Parse the expression text
                    let (tokens, _diags) = ark_lexer::tokenize(0, text);
                    let mut sub_parser = Parser::new(&tokens, self.sink);
                    let expr = sub_parser.parse_expr();
                    // Wrap in to_string() call
                    exprs.push(Expr::Call {
                        callee: Box::new(Expr::Ident {
                            name: "to_string".to_string(),
                            span,
                        }),
                        type_args: vec![],
                        args: vec![expr],
                        span,
                    });
                }
            }
        }

        // If empty: return ""
        if exprs.is_empty() {
            return Expr::StringLit {
                value: String::new(),
                span,
            };
        }

        // If single: return it directly
        if exprs.len() == 1 {
            return exprs.pop().unwrap();
        }

        // Chain with concat: concat(concat(a, b), c)
        let mut result = exprs.remove(0);
        for expr in exprs {
            result = Expr::Call {
                callee: Box::new(Expr::Ident {
                    name: "concat".to_string(),
                    span,
                }),
                type_args: vec![],
                args: vec![result, expr],
                span,
            };
        }
        result
    }
}
