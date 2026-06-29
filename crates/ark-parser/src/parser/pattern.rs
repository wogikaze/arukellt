use ark_diagnostics::{Diagnostic, DiagnosticCode};
use ark_lexer::TokenKind;

use crate::ast::*;

use super::Parser;

impl Parser<'_> {
    /// Parse a pattern with optional or-alternatives: `A | B | C`
    pub(crate) fn parse_pattern_with_or(&mut self) -> Pattern {
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

    pub(crate) fn parse_pattern(&mut self) -> Pattern {
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
}
