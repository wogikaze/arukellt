use ark_diagnostics::{Diagnostic, DiagnosticCode, Span};
use ark_lexer::TokenKind;

use crate::ast::*;

use super::Parser;

impl Parser<'_> {
    pub(crate) fn parse_expr(&mut self) -> Expr {
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
            TokenKind::Fn => {
                // Inline fn expression: fn(x: i32, y: i32) -> i32 { x + y }
                // Parses as Closure to reuse existing closure lowering
                self.advance(); // consume `fn`
                self.expect(&TokenKind::LParen);
                let mut params = Vec::new();
                while *self.peek() != TokenKind::RParen && *self.peek() != TokenKind::Eof {
                    let pstart = self.span();
                    let name = self.expect_ident();
                    let ty = if self.eat(&TokenKind::Colon) {
                        Some(self.parse_type_expr())
                    } else {
                        None
                    };
                    params.push(ClosureParam { name, ty, span: pstart.merge(self.span()) });
                    if !self.eat(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(&TokenKind::RParen);
                let return_type = if self.eat(&TokenKind::Arrow) {
                    Some(self.parse_type_expr())
                } else {
                    None
                };
                let body = Expr::Block(self.parse_block());
                let span = start.merge(body.span());
                Expr::Closure { params, return_type, body: Box::new(body), span }
            }
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
        if i + 1 < self.tokens.len() && matches!(&self.tokens[i].kind, TokenKind::Ident(_)) {
            // Skip newlines between ident and potential colon
            let mut j = i + 1;
            while j < self.tokens.len() && self.tokens[j].kind == TokenKind::Newline {
                j += 1;
            }
            if j < self.tokens.len() && self.tokens[j].kind == TokenKind::Colon {
                return true;
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
