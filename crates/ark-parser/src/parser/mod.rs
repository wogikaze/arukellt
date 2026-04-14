//! Pratt parser for Arukellt.

mod decl;
mod expr;
mod pattern;
mod stmt;
mod ty;

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

    fn peek_n(&self, n: usize) -> &TokenKind {
        self.tokens
            .get(self.pos + n)
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
                let kw = *kw;
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
                imports.push(self.parse_use_import(false));
            } else if *self.peek() == TokenKind::Pub && *self.peek_n(1) == TokenKind::Use {
                if !item_docs.is_empty() {
                    self.emit_doc_comment_error("doc comments on imports are not supported");
                }
                imports.push(self.parse_use_import(true));
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
}
