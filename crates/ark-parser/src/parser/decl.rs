use ark_diagnostics::{Diagnostic, DiagnosticCode};
use ark_lexer::TokenKind;

use crate::ast::*;

use super::Parser;

impl Parser<'_> {
    pub(crate) fn parse_import(&mut self) -> Import {
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
            kind: ImportKind::Simple,
            span: start.merge(self.span()),
        }
    }

    /// Parse `use std::foo::bar`, `pub use std::foo::bar`, or `use std::foo::{bar, baz}`.
    ///
    /// - `use a::b::c` → `ImportKind::ModulePath`
    /// - `pub use a::b::c` → `ImportKind::PublicModulePath`
    /// - `use a::b::c [as alias]` → `ImportKind::ModulePath` with alias
    /// - `use a::b::{c, d}` → `ImportKind::DestructureImport { names: ["c", "d"] }`
    pub(crate) fn parse_use_import(&mut self, is_pub: bool) -> Import {
        let start = self.span();
        if is_pub {
            self.expect(&TokenKind::Pub);
        }
        self.expect(&TokenKind::Use);

        // Parse path segments separated by ::
        let mut segments = vec![self.expect_ident()];
        loop {
            if !self.eat(&TokenKind::ColonColon) {
                break;
            }
            // Destructuring: `use a::b::{c, d}`
            if *self.peek() == TokenKind::LBrace {
                self.advance(); // consume `{`
                let mut names = Vec::new();
                while *self.peek() != TokenKind::RBrace && *self.peek() != TokenKind::Eof {
                    names.push(self.expect_ident());
                    if !self.eat(&TokenKind::Comma) {
                        break;
                    }
                }
                let end = self.expect(&TokenKind::RBrace);
                let module_name = segments.join("::");
                return Import {
                    module_name,
                    alias: None,
                    kind: ImportKind::DestructureImport { names },
                    span: start.merge(end),
                };
            }
            segments.push(self.expect_ident());
        }

        let module_name = segments.join("::");
        let alias = if self.eat(&TokenKind::As) {
            Some(self.expect_ident())
        } else {
            None
        };

        Import {
            module_name,
            alias,
            kind: if is_pub {
                ImportKind::PublicModulePath
            } else {
                ImportKind::ModulePath
            },
            span: start.merge(self.span()),
        }
    }

    pub(crate) fn parse_item(&mut self, docs: Vec<String>) -> Option<Item> {
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
                    self.emit_doc_comment_error(
                        "doc comments inside traits must attach to methods",
                    );
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
                    self.emit_doc_comment_error(
                        "doc comments inside impl blocks must attach to methods",
                    );
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
}
