use ark_diagnostics::{Diagnostic, DiagnosticCode};
use ark_lexer::TokenKind;

use crate::ast::*;

use super::Parser;

impl<'a> Parser<'a> {
    pub(crate) fn parse_block(&mut self) -> Block {
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
}
