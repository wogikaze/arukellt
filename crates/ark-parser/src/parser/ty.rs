use ark_diagnostics::{Diagnostic, DiagnosticCode};
use ark_lexer::TokenKind;

use crate::ast::*;

use super::Parser;

impl<'a> Parser<'a> {
    pub(crate) fn parse_type_expr(&mut self) -> TypeExpr {
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
}
