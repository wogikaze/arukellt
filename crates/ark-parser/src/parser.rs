//! Pratt parser for Arukellt.

use ark_diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSink, Span};
use ark_lexer::{Token, TokenKind};

use crate::ast::*;

pub struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
    sink: &'a mut DiagnosticSink,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [Token], sink: &'a mut DiagnosticSink) -> Self {
        let mut p = Self { tokens, pos: 0, sink };
        p.skip_newlines();
        p
    }

    fn skip_newlines(&mut self) {
        while self.pos < self.tokens.len() && self.tokens[self.pos].kind == TokenKind::Newline {
            self.pos += 1;
        }
    }

    fn peek(&self) -> &TokenKind {
        self.tokens.get(self.pos).map(|t| &t.kind).unwrap_or(&TokenKind::Eof)
    }

    fn span(&self) -> Span {
        self.tokens.get(self.pos).map(|t| t.span).unwrap_or(Span::dummy())
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
                    .with_message(format!("expected `{:?}`, found `{:?}`", expected, self.peek()))
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

    // === Module parsing ===

    pub fn parse_module(&mut self) -> Module {
        let mut imports = Vec::new();
        let mut items = Vec::new();

        while *self.peek() != TokenKind::Eof {
            // Check for v0 constraint violations
            if let TokenKind::Reserved(kw) = self.peek() {
                let kw = kw.clone();
                let sp = self.span();
                let code = match kw.as_str() {
                    "trait" => DiagnosticCode::E0300,
                    "impl" => DiagnosticCode::E0300,
                    "for" => DiagnosticCode::E0303,
                    _ => DiagnosticCode::E0003,
                };
                self.sink.emit(
                    Diagnostic::new(code)
                        .with_label(sp, format!("`{}` is not available in v0", kw)),
                );
                self.advance();
                self.synchronize();
                continue;
            }

            if *self.peek() == TokenKind::Import {
                imports.push(self.parse_import());
            } else {
                match self.parse_item() {
                    Some(item) => items.push(item),
                    None => {
                        self.advance();
                        self.synchronize();
                    }
                }
            }
        }

        Module { imports, items }
    }

    fn synchronize(&mut self) {
        while *self.peek() != TokenKind::Eof {
            match self.peek() {
                TokenKind::Fn | TokenKind::Struct | TokenKind::Enum
                | TokenKind::Import | TokenKind::Pub => return,
                TokenKind::RBrace => { self.advance(); return; }
                _ => { self.advance(); }
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
        Import { module_name: name, alias, span: start.merge(self.span()) }
    }

    fn parse_item(&mut self) -> Option<Item> {
        let is_pub = self.eat(&TokenKind::Pub);
        match self.peek() {
            TokenKind::Fn => Some(Item::FnDef(self.parse_fn_def(is_pub))),
            TokenKind::Struct => Some(Item::StructDef(self.parse_struct_def(is_pub))),
            TokenKind::Enum => Some(Item::EnumDef(self.parse_enum_def(is_pub))),
            _ => {
                let sp = self.span();
                self.sink.emit(
                    Diagnostic::new(DiagnosticCode::E0001)
                        .with_message(format!("expected item (fn, struct, enum), found `{:?}`", self.peek()))
                        .with_label(sp, "here"),
                );
                None
            }
        }
    }

    fn parse_fn_def(&mut self, is_pub: bool) -> FnDef {
        let start = self.span();
        self.expect(&TokenKind::Fn);
        let name = self.expect_ident();

        // Generic params
        let type_params = if *self.peek() == TokenKind::Lt {
            self.parse_type_params()
        } else {
            Vec::new()
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
        FnDef { name, type_params, params, return_type, body, is_pub, span }
    }

    fn parse_type_params(&mut self) -> Vec<String> {
        self.expect(&TokenKind::Lt);
        let mut params = Vec::new();
        loop {
            if *self.peek() == TokenKind::Gt { break; }
            params.push(self.expect_ident());
            if !self.eat(&TokenKind::Comma) { break; }
        }
        self.expect(&TokenKind::Gt);
        params
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
                        .with_label(sp, format!("parameter `{}` requires a type annotation", name)),
                );
                // Skip to next comma or rparen
                while *self.peek() != TokenKind::Comma && *self.peek() != TokenKind::RParen && *self.peek() != TokenKind::Eof {
                    self.advance();
                }
                if self.eat(&TokenKind::Comma) { continue; }
                break;
            }
            self.expect(&TokenKind::Colon);
            let ty = self.parse_type_expr();
            let span = start.merge(self.span());
            params.push(Param { name, ty, span });
            if !self.eat(&TokenKind::Comma) { break; }
        }
        params
    }

    fn parse_struct_def(&mut self, is_pub: bool) -> StructDef {
        let start = self.span();
        self.expect(&TokenKind::Struct);
        let name = self.expect_ident();
        self.expect(&TokenKind::LBrace);
        let fields = self.parse_fields();
        let end = self.expect(&TokenKind::RBrace);
        StructDef { name, fields, is_pub, span: start.merge(end) }
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

    fn parse_enum_def(&mut self, is_pub: bool) -> EnumDef {
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
                    if !self.eat(&TokenKind::Comma) { break; }
                }
                let end = self.expect(&TokenKind::RParen);
                variants.push(Variant::Tuple { name: vname, fields, span: vstart.merge(end) });
            } else if *self.peek() == TokenKind::LBrace {
                // Struct variant
                self.advance();
                let fields = self.parse_fields();
                let end = self.expect(&TokenKind::RBrace);
                variants.push(Variant::Struct { name: vname, fields, span: vstart.merge(end) });
            } else {
                variants.push(Variant::Unit { name: vname, span: vstart.merge(self.span()) });
            }
            self.eat(&TokenKind::Comma);
        }
        let end = self.expect(&TokenKind::RBrace);
        EnumDef { name, type_params, variants, is_pub, span: start.merge(end) }
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
                        TypeExpr::Array { elem: Box::new(elem), size: n as u64, span: start.merge(end) }
                    } else {
                        self.expect(&TokenKind::RBracket);
                        TypeExpr::Array { elem: Box::new(elem), size: 0, span: start.merge(self.span()) }
                    }
                } else {
                    // Slice [T]
                    let end = self.expect(&TokenKind::RBracket);
                    TypeExpr::Slice { elem: Box::new(elem), span: start.merge(end) }
                }
            }
            TokenKind::Fn => {
                self.advance();
                self.expect(&TokenKind::LParen);
                let mut params = Vec::new();
                while *self.peek() != TokenKind::RParen && *self.peek() != TokenKind::Eof {
                    params.push(self.parse_type_expr());
                    if !self.eat(&TokenKind::Comma) { break; }
                }
                self.expect(&TokenKind::RParen);
                self.expect(&TokenKind::Arrow);
                let ret = self.parse_type_expr();
                TypeExpr::Function { params, ret: Box::new(ret), span: start.merge(self.span()) }
            }
            TokenKind::Ident(name) => {
                self.advance();
                // Check for qualified type: mod.Type
                if *self.peek() == TokenKind::Dot {
                    self.advance();
                    let type_name = self.expect_ident();
                    return TypeExpr::Qualified { module: name, name: type_name, span: start.merge(self.span()) };
                }
                // Check for generic type: Type<A, B>
                if *self.peek() == TokenKind::Lt {
                    self.advance();
                    let mut args = Vec::new();
                    loop {
                        if *self.peek() == TokenKind::Gt { break; }
                        args.push(self.parse_type_expr());
                        if !self.eat(&TokenKind::Comma) { break; }
                    }
                    // Detect nested generics: Vec<Vec<i32>> produces Shr (`>>`) token
                    if *self.peek() == TokenKind::Shr {
                        let sp = self.span();
                        self.sink.emit(
                            Diagnostic::new(DiagnosticCode::E0203)
                                .with_message("nested generic types are not available in v0")
                                .with_label(sp, "nested generic type"),
                        );
                        self.advance();
                        return TypeExpr::Generic { name, args, span: start.merge(self.span()) };
                    }
                    self.expect(&TokenKind::Gt);
                    return TypeExpr::Generic { name, args, span: start.merge(self.span()) };
                }
                TypeExpr::Named { name, span: start.merge(self.span()) }
            }
            _ => {
                let sp = self.span();
                self.sink.emit(
                    Diagnostic::new(DiagnosticCode::E0001)
                        .with_message(format!("expected type, found `{:?}`", self.peek()))
                        .with_label(sp, "here"),
                );
                self.advance();
                TypeExpr::Named { name: "<error>".to_string(), span: sp }
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
            // Check for v0 constraint violations inside blocks
            if let TokenKind::Reserved(kw) = self.peek() {
                let kw = kw.clone();
                let sp = self.span();
                let code = match kw.as_str() {
                    "for" => DiagnosticCode::E0303,
                    "trait" | "impl" => DiagnosticCode::E0300,
                    _ => DiagnosticCode::E0003,
                };
                self.sink.emit(
                    Diagnostic::new(code)
                        .with_label(sp, format!("`{}` is not available in v0", kw)),
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
        Block { stmts, tail_expr, span: start.merge(end) }
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

        Stmt::Let { name, ty, init, is_mut, pattern, span: start.merge(self.span()) }
    }

    fn parse_while_stmt(&mut self) -> Stmt {
        let start = self.span();
        self.expect(&TokenKind::While);
        let cond = self.parse_expr();
        let body = self.parse_block();
        Stmt::While { cond, body, span: start.merge(self.span()) }
    }

    fn parse_loop_stmt(&mut self) -> Stmt {
        let start = self.span();
        self.expect(&TokenKind::Loop);
        let body = self.parse_block();
        Stmt::Loop { body, span: start.merge(self.span()) }
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
            Expr::Assign { target: Box::new(expr), value: Box::new(value), span }
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
            left = Expr::Binary { left: Box::new(left), op: BinOp::Or, right: Box::new(right), span };
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
            left = Expr::Binary { left: Box::new(left), op: BinOp::And, right: Box::new(right), span };
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
            left = Expr::Binary { left: Box::new(left), op, right: Box::new(right), span };
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
            left = Expr::Binary { left: Box::new(left), op, right: Box::new(right), span };
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
            left = Expr::Binary { left: Box::new(left), op: BinOp::BitOr, right: Box::new(right), span };
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
            left = Expr::Binary { left: Box::new(left), op: BinOp::BitXor, right: Box::new(right), span };
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
            left = Expr::Binary { left: Box::new(left), op: BinOp::BitAnd, right: Box::new(right), span };
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
            left = Expr::Binary { left: Box::new(left), op, right: Box::new(right), span };
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
            left = Expr::Binary { left: Box::new(left), op, right: Box::new(right), span };
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
            left = Expr::Binary { left: Box::new(left), op, right: Box::new(right), span };
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
                Expr::Unary { op: UnaryOp::Neg, operand: Box::new(operand), span }
            }
            TokenKind::Bang => {
                self.advance();
                let operand = self.parse_unary();
                let span = start.merge(operand.span());
                Expr::Unary { op: UnaryOp::Not, operand: Box::new(operand), span }
            }
            TokenKind::Tilde => {
                self.advance();
                let operand = self.parse_unary();
                let span = start.merge(operand.span());
                Expr::Unary { op: UnaryOp::BitNot, operand: Box::new(operand), span }
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
                    expr = Expr::Call { callee: Box::new(expr), type_args: Vec::new(), args, span };
                }
                TokenKind::LBracket => {
                    // Index
                    self.advance();
                    let index = self.parse_expr();
                    let end = self.expect(&TokenKind::RBracket);
                    let span = expr.span().merge(end);
                    expr = Expr::Index { object: Box::new(expr), index: Box::new(index), span };
                }
                TokenKind::Dot => {
                    self.advance();
                    // Check for method call (v0 forbidden)
                    let field = self.expect_ident();
                    if *self.peek() == TokenKind::LParen {
                        let sp = expr.span().merge(self.span());
                        self.sink.emit(
                            Diagnostic::new(DiagnosticCode::E0301)
                                .with_message("method call syntax is not available in v0; use function call syntax")
                                .with_label(sp, "method call here"),
                        );
                        // Parse the call anyway for error recovery
                        self.advance();
                        let _args = self.parse_call_args();
                        self.expect(&TokenKind::RParen);
                    }
                    let span = expr.span().merge(self.span());
                    expr = Expr::FieldAccess { object: Box::new(expr), field, span };
                }
                TokenKind::Question => {
                    let start = expr.span();
                    self.advance();
                    let span = start.merge(self.span());
                    expr = Expr::Try { expr: Box::new(expr), span };
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
            if !self.eat(&TokenKind::Comma) { break; }
        }
        args
    }

    fn parse_primary(&mut self) -> Expr {
        let start = self.span();
        match self.peek().clone() {
            TokenKind::IntLit(v) => { self.advance(); Expr::IntLit { value: v, span: start } }
            TokenKind::FloatLit(v) => { self.advance(); Expr::FloatLit { value: v, span: start } }
            TokenKind::StringLit(v) => { self.advance(); Expr::StringLit { value: v, span: start } }
            TokenKind::CharLit(v) => { self.advance(); Expr::CharLit { value: v, span: start } }
            TokenKind::BoolLit(v) => { self.advance(); Expr::BoolLit { value: v, span: start } }
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
                        let callee = Expr::QualifiedIdent { module: name, name: variant, span: start.merge(self.span()) };
                        return Expr::Call { callee: Box::new(callee), type_args: Vec::new(), args, span };
                    }
                    return Expr::QualifiedIdent { module: name, name: variant, span: start.merge(self.span()) };
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
                    return Expr::Tuple { elements: Vec::new(), span: start.merge(end) };
                }
                let expr = self.parse_expr();
                if self.eat(&TokenKind::Comma) {
                    // Tuple
                    let mut elements = vec![expr];
                    while *self.peek() != TokenKind::RParen && *self.peek() != TokenKind::Eof {
                        elements.push(self.parse_expr());
                        if !self.eat(&TokenKind::Comma) { break; }
                    }
                    let end = self.expect(&TokenKind::RParen);
                    Expr::Tuple { elements, span: start.merge(end) }
                } else {
                    self.expect(&TokenKind::RParen);
                    expr // parenthesized expression
                }
            }
            TokenKind::LBracket => {
                self.advance();
                if *self.peek() == TokenKind::RBracket {
                    let end = self.advance().span;
                    return Expr::Array { elements: Vec::new(), span: start.merge(end) };
                }
                let first = self.parse_expr();
                if self.eat(&TokenKind::Semi) {
                    // Array repeat [value; count]
                    let count = self.parse_expr();
                    let end = self.expect(&TokenKind::RBracket);
                    Expr::ArrayRepeat { value: Box::new(first), count: Box::new(count), span: start.merge(end) }
                } else {
                    // Array literal [a, b, c]
                    let mut elements = vec![first];
                    while self.eat(&TokenKind::Comma) {
                        if *self.peek() == TokenKind::RBracket { break; }
                        elements.push(self.parse_expr());
                    }
                    let end = self.expect(&TokenKind::RBracket);
                    Expr::Array { elements, span: start.merge(end) }
                }
            }
            TokenKind::LBrace => {
                Expr::Block(self.parse_block())
            }
            TokenKind::If => self.parse_if_expr(),
            TokenKind::Match => self.parse_match_expr(),
            TokenKind::Return => {
                self.advance();
                let value = if *self.peek() != TokenKind::Semi && *self.peek() != TokenKind::RBrace {
                    Some(Box::new(self.parse_expr()))
                } else {
                    None
                };
                Expr::Return { value, span: start.merge(self.span()) }
            }
            TokenKind::Break => {
                self.advance();
                let value = if *self.peek() != TokenKind::Semi && *self.peek() != TokenKind::RBrace {
                    Some(Box::new(self.parse_expr()))
                } else {
                    None
                };
                Expr::Break { value, span: start.merge(self.span()) }
            }
            TokenKind::Continue => {
                self.advance();
                Expr::Continue { span: start }
            }
            TokenKind::Loop => {
                self.advance();
                let body = self.parse_block();
                Expr::Loop { body, span: start.merge(self.span()) }
            }
            TokenKind::Pipe => {
                self.parse_closure(start)
            }
            TokenKind::PipePipe => {
                // || closure (no params)
                self.advance();
                let body = self.parse_expr();
                let span = start.merge(body.span());
                Expr::Closure { params: Vec::new(), return_type: None, body: Box::new(body), span }
            }
            _ => {
                let sp = self.span();
                self.sink.emit(
                    Diagnostic::new(DiagnosticCode::E0001)
                        .with_message(format!("expected expression, found `{:?}`", self.peek()))
                        .with_label(sp, "here"),
                );
                self.advance();
                Expr::IntLit { value: 0, span: sp }
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
        while *self.peek() != TokenKind::RBrace && *self.peek() != TokenKind::Eof {
            let fname = self.expect_ident();
            self.expect(&TokenKind::Colon);
            let fval = self.parse_expr();
            fields.push((fname, fval));
            if !self.eat(&TokenKind::Comma) { break; }
        }
        let end = self.expect(&TokenKind::RBrace);
        Expr::StructInit { name, fields, span: start.merge(end) }
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
                Some(Block { stmts: Vec::new(), tail_expr: Some(Box::new(elif)), span: sp })
            } else {
                Some(self.parse_block())
            }
        } else {
            None
        };
        let span = start.merge(self.span());
        Expr::If { cond: Box::new(cond), then_block, else_block, span }
    }

    fn parse_match_expr(&mut self) -> Expr {
        let start = self.span();
        self.expect(&TokenKind::Match);
        let scrutinee = self.parse_expr();
        self.expect(&TokenKind::LBrace);
        let mut arms = Vec::new();
        while *self.peek() != TokenKind::RBrace && *self.peek() != TokenKind::Eof {
            let arm_start = self.span();
            let pattern = self.parse_pattern();
            self.expect(&TokenKind::FatArrow);
            let body = self.parse_expr();
            let arm_span = arm_start.merge(body.span());
            arms.push(MatchArm { pattern, body, span: arm_span });
            self.eat(&TokenKind::Comma);
        }
        let end = self.expect(&TokenKind::RBrace);
        Expr::Match { scrutinee: Box::new(scrutinee), arms, span: start.merge(end) }
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
                // Check for Enum::Variant(...)
                if *self.peek() == TokenKind::ColonColon {
                    self.advance();
                    let variant = self.expect_ident();
                    let fields = if *self.peek() == TokenKind::LParen {
                        self.advance();
                        let mut pats = Vec::new();
                        while *self.peek() != TokenKind::RParen && *self.peek() != TokenKind::Eof {
                            pats.push(self.parse_pattern());
                            if !self.eat(&TokenKind::Comma) { break; }
                        }
                        self.expect(&TokenKind::RParen);
                        pats
                    } else {
                        Vec::new()
                    };
                    return Pattern::Enum { path: name, variant, fields, span: start.merge(self.span()) };
                }
                // Plain identifier or variant without path
                // Check if it matches Some/None/Ok/Err (common enum constructors)
                if *self.peek() == TokenKind::LParen && matches!(name.as_str(), "Some" | "Ok" | "Err") {
                    self.advance();
                    let mut fields = Vec::new();
                    while *self.peek() != TokenKind::RParen && *self.peek() != TokenKind::Eof {
                        fields.push(self.parse_pattern());
                        if !self.eat(&TokenKind::Comma) { break; }
                    }
                    self.expect(&TokenKind::RParen);
                    let enum_name = match name.as_str() {
                        "Some" | "None" => "Option",
                        "Ok" | "Err" => "Result",
                        _ => "<unknown>",
                    };
                    return Pattern::Enum { path: enum_name.to_string(), variant: name, fields, span: start.merge(self.span()) };
                }
                Pattern::Ident { name, span: start }
            }
            TokenKind::IntLit(v) => { self.advance(); Pattern::IntLit { value: v, span: start } }
            TokenKind::FloatLit(v) => { self.advance(); Pattern::FloatLit { value: v, span: start } }
            TokenKind::StringLit(v) => { self.advance(); Pattern::StringLit { value: v, span: start } }
            TokenKind::CharLit(v) => { self.advance(); Pattern::CharLit { value: v, span: start } }
            TokenKind::BoolLit(v) => { self.advance(); Pattern::BoolLit { value: v, span: start } }
            TokenKind::Minus => {
                // Negative literal pattern
                self.advance();
                if let TokenKind::IntLit(v) = self.peek().clone() {
                    self.advance();
                    Pattern::IntLit { value: -v, span: start.merge(self.span()) }
                } else {
                    Pattern::IntLit { value: 0, span: start }
                }
            }
            TokenKind::LParen => {
                self.advance();
                let mut elements = Vec::new();
                while *self.peek() != TokenKind::RParen && *self.peek() != TokenKind::Eof {
                    elements.push(self.parse_pattern());
                    if !self.eat(&TokenKind::Comma) { break; }
                }
                let end = self.expect(&TokenKind::RParen);
                Pattern::Tuple { elements, span: start.merge(end) }
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
            params.push(ClosureParam { name, ty, span: pstart.merge(self.span()) });
            if !self.eat(&TokenKind::Comma) { break; }
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
        Expr::Closure { params, return_type, body: Box::new(body), span }
    }
}
