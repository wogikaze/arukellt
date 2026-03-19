use crate::Type;
use crate::ast::{
    BinaryOp, CapabilityImport, Expr, Function, MatchArm, Module, Param, Pattern, TypeDecl,
    VariantDecl, VariantField,
};
use crate::diagnostics::{Diagnostic, DiagnosticLevel, DiagnosticStage, Span};
use crate::lexer::{Token, TokenKind};

#[derive(Clone, Debug)]
pub struct ParseOutput {
    pub module: Module,
    pub diagnostics: Vec<Diagnostic>,
}

impl ParseOutput {
    #[must_use]
    pub fn has_errors(&self) -> bool {
        use crate::diagnostics::DiagnosticLevel;
        self.diagnostics
            .iter()
            .any(|d| d.level == DiagnosticLevel::Error)
    }
}

pub fn parse(tokens: &[Token]) -> ParseOutput {
    Parser::new(tokens).parse_module()
}

struct Parser<'a> {
    tokens: &'a [Token],
    position: usize,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        Self {
            tokens,
            position: 0,
            diagnostics: Vec::new(),
        }
    }

    fn parse_module(mut self) -> ParseOutput {
        let mut imports = Vec::new();
        let mut types = Vec::new();
        let mut functions = Vec::new();
        let mut stage = TopLevelStage::Imports;

        while !self.at_eof() {
            self.skip_newlines();
            if self.at_eof() {
                break;
            }

            if self.match_kind(&TokenKind::Import) {
                if stage > TopLevelStage::Imports {
                    self.push_error(
                        "E_TOPLEVEL_ORDER",
                        "Imports must appear before type and function declarations",
                        self.previous_span(),
                        "import capability declarations first",
                        "import after later item".to_owned(),
                        "toplevel_order_violation",
                        "Move imports to the top of the file.",
                    );
                }
                imports.push(self.parse_import());
                continue;
            }

            if self.match_kind(&TokenKind::Type) {
                if stage > TopLevelStage::Types {
                    self.push_error(
                        "E_TOPLEVEL_ORDER",
                        "Type declarations must appear before functions",
                        self.previous_span(),
                        "type declarations before functions",
                        "type after function".to_owned(),
                        "toplevel_order_violation",
                        "Move type declarations before functions.",
                    );
                }
                stage = TopLevelStage::Types;
                types.push(self.parse_type_decl());
                continue;
            }

            stage = TopLevelStage::Functions;
            functions.push(self.parse_function());
        }

        ParseOutput {
            module: Module {
                imports,
                types,
                functions,
            },
            diagnostics: self.diagnostics,
        }
    }

    fn parse_import(&mut self) -> CapabilityImport {
        // Accept both `import capability NAME` and bare `import NAME`
        self.match_kind(&TokenKind::Capability);
        let name = self.expect_ident("capability name");
        self.consume_newline();
        CapabilityImport { name }
    }

    fn parse_type_decl(&mut self) -> TypeDecl {
        let name = self.expect_ident("type name");
        self.expect_kind(TokenKind::Equal, "type definition marker", "=");
        self.consume_newline();
        self.expect_kind(TokenKind::Indent, "indented type body", "indent");
        let mut variants = Vec::new();
        while !self.check(&TokenKind::Dedent) && !self.at_eof() {
            self.skip_newlines();
            if self.check(&TokenKind::Dedent) {
                break;
            }
            variants.push(self.parse_variant_decl());
        }
        self.expect_kind(TokenKind::Dedent, "dedent", "dedent");
        TypeDecl { name, variants }
    }

    fn parse_variant_decl(&mut self) -> VariantDecl {
        let name = self.expect_ident("variant name");
        let mut fields = Vec::new();
        if self.match_kind(&TokenKind::LParen) {
            while !self.check(&TokenKind::RParen) && !self.at_eof() {
                let field_name = self.expect_ident("variant field name");
                self.expect_kind(TokenKind::Colon, "field type separator", ":");
                let field_ty = self.parse_type();
                fields.push(VariantField {
                    name: field_name,
                    ty: field_ty,
                });
                if !self.match_kind(&TokenKind::Comma) {
                    break;
                }
            }
            self.expect_kind(TokenKind::RParen, "closing parenthesis", ")");
        }
        self.consume_newline();
        VariantDecl { name, fields }
    }

    fn parse_function(&mut self) -> Function {
        let public = self.match_kind(&TokenKind::Pub);
        self.expect_kind(TokenKind::Fn, "fn keyword", "fn");
        let name = self.expect_ident("function name");
        self.expect_kind(TokenKind::LParen, "opening parenthesis", "(");
        let mut params = Vec::new();
        while !self.check(&TokenKind::RParen) && !self.at_eof() {
            let param_name = self.expect_ident("parameter name");
            self.expect_kind(TokenKind::Colon, "type annotation", ":");
            let param_ty = self.parse_type();
            params.push(Param {
                name: param_name,
                ty: param_ty,
            });
            if !self.match_kind(&TokenKind::Comma) {
                break;
            }
        }
        self.expect_kind(TokenKind::RParen, "closing parenthesis", ")");
        // Return type is optional; defaults to Unit
        let return_type = if self.match_kind(&TokenKind::Arrow) {
            self.parse_type()
        } else {
            Type::Unit
        };
        self.expect_kind(TokenKind::Colon, "function body marker", ":");
        let body = self.parse_block_expr();

        Function {
            public,
            name,
            params,
            return_type,
            body,
        }
    }

    fn parse_type(&mut self) -> Type {
        match self.advance().kind {
            TokenKind::Ident(name) => {
                if self.match_kind(&TokenKind::Less) {
                    // Generic type: Result<T, E>, Option<T>, Fn<A, B>, Iter<T>, etc.
                    let mut args = Vec::new();
                    while !self.check(&TokenKind::Greater) && !self.at_eof() {
                        args.push(self.parse_type());
                        if !self.match_kind(&TokenKind::Comma) {
                            break;
                        }
                    }
                    self.expect_kind(TokenKind::Greater, "closing >", ">");
                    match name.as_str() {
                        "Result" if args.len() >= 2 => {
                            Type::Result(Box::new(args[0].clone()), Box::new(args[1].clone()))
                        }
                        "Option" if !args.is_empty() => Type::Option(Box::new(args[0].clone())),
                        "List" if !args.is_empty() => Type::List(Box::new(args[0].clone())),
                        "Seq" | "Iter" if !args.is_empty() => Type::Seq(Box::new(args[0].clone())),
                        "Fn" if args.len() >= 2 => {
                            Type::Fn(Box::new(args[0].clone()), Box::new(args[1].clone()))
                        }
                        _ => Type::Unknown,
                    }
                } else {
                    Type::from_name(&name)
                }
            }
            other => {
                self.push_error(
                    "E_EXPECTED_TYPE",
                    "Expected a type name",
                    self.previous_span(),
                    "type name",
                    format!("{other:?}"),
                    "missing_type_name",
                    "Insert a type name such as Int or Bool.",
                );
                Type::Unknown
            }
        }
    }

    fn parse_block_expr(&mut self) -> Expr {
        self.consume_newline();
        self.expect_kind(TokenKind::Indent, "indented block", "indent");
        let expr = self.parse_block_contents();
        self.expect_kind(TokenKind::Dedent, "dedent", "dedent");
        expr
    }

    /// Parse one or more lines inside an indented block.
    /// Lines starting with `let` introduce a binding; the last line is the value.
    fn parse_block_contents(&mut self) -> Expr {
        self.skip_newlines();
        if self.match_kind(&TokenKind::Let) {
            let name = self.expect_ident("binding name");
            self.expect_kind(TokenKind::Equal, "assignment operator", "=");
            let value = self.parse_expr();
            self.consume_newline();
            let body = self.parse_block_contents();
            Expr::Let {
                name,
                value: Box::new(value),
                body: Box::new(body),
            }
        } else {
            let expr = self.parse_expr();
            self.skip_newlines();
            expr
        }
    }

    fn parse_expr(&mut self) -> Expr {
        // Lambda: Ident -> body
        if let TokenKind::Ident(name) = self.current().kind.clone() {
            if self.peek_kind(1) == Some(&TokenKind::Arrow) {
                self.position += 1; // consume ident
                self.position += 1; // consume ->
                let body = self.parse_lambda_body();
                return Expr::Lambda {
                    param: name,
                    body: Box::new(body),
                };
            }
        }

        if self.match_kind(&TokenKind::If) {
            let expr = self.parse_if_expr();
            return self.parse_chain(expr);
        }
        if self.match_kind(&TokenKind::Match) {
            let expr = self.parse_match_expr();
            return self.parse_chain(expr);
        }

        let expr = self.parse_logical();
        self.parse_chain(expr)
    }

    fn parse_logical(&mut self) -> Expr {
        let mut expr = self.parse_comparison();
        loop {
            let op = if self.match_kind(&TokenKind::And) {
                Some(BinaryOp::And)
            } else if self.match_kind(&TokenKind::Or) {
                Some(BinaryOp::Or)
            } else {
                None
            };
            let Some(op) = op else { break };
            let right = self.parse_comparison();
            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        expr
    }

    fn parse_lambda_body(&mut self) -> Expr {
        if self.check(&TokenKind::Newline) {
            self.consume_newline();
            self.expect_kind(TokenKind::Indent, "lambda body indent", "indent");
            let expr = self.parse_block_contents();
            self.expect_kind(TokenKind::Dedent, "lambda body dedent", "dedent");
            expr
        } else {
            self.parse_expr()
        }
    }

    /// Handle postfix chain: .method(), |> pipe, [index], (Apply), ..=range
    /// Skips newlines to handle multi-line continuation chains.
    fn parse_chain(&mut self, mut expr: Expr) -> Expr {
        loop {
            let saved = self.position;
            // Skip newlines to handle continuation lines
            while self.match_kind(&TokenKind::Newline) {}

            if self.match_kind(&TokenKind::Pipe) {
                // |> callee
                let callee = self.parse_pipe_callee();
                expr = Expr::Call {
                    callee,
                    args: vec![expr],
                };
            } else if self.match_kind(&TokenKind::Dot) {
                // .method(args) or .method
                let method = self.expect_ident("method name");
                let mut args = Vec::new();
                if self.match_kind(&TokenKind::LParen) {
                    while !self.check(&TokenKind::RParen) && !self.at_eof() {
                        args.push(self.parse_expr());
                        if !self.match_kind(&TokenKind::Comma) {
                            break;
                        }
                    }
                    self.expect_kind(TokenKind::RParen, "closing parenthesis", ")");
                }
                expr = Expr::MethodCall {
                    receiver: Box::new(expr),
                    method,
                    args,
                };
            } else if self.check(&TokenKind::LBracket) {
                self.position += 1; // consume [
                let index = self.parse_expr();
                self.expect_kind(TokenKind::RBracket, "closing bracket", "]");
                expr = Expr::Index {
                    receiver: Box::new(expr),
                    index: Box::new(index),
                };
            } else {
                // No continuation found; restore and stop
                self.position = saved;
                break;
            }
        }
        expr
    }

    fn parse_pipe_callee(&mut self) -> String {
        let name = self.expect_ident("pipe callee");
        if self.match_kind(&TokenKind::Dot) {
            let method = self.expect_ident("method name");
            format!("{name}.{method}")
        } else {
            name
        }
    }

    fn parse_if_expr(&mut self) -> Expr {
        let condition = self.parse_comparison();
        self.expect_kind(TokenKind::Colon, "if block marker", ":");
        let then_branch = self.parse_block_expr();
        self.skip_newlines();
        let else_branch = if self.match_kind(&TokenKind::Else) {
            self.expect_kind(TokenKind::Colon, "else block marker", ":");
            self.parse_block_expr()
        } else {
            self.push_error(
                "E_IF_ELSE_REQUIRED",
                "If expressions must include an else branch",
                self.current_span(),
                "else branch",
                "end of block".to_owned(),
                "missing_else_branch",
                "Add an `else:` block with the fallback expression.",
            );
            Expr::Error
        };
        Expr::If {
            condition: Box::new(condition),
            then_branch: Box::new(then_branch),
            else_branch: Box::new(else_branch),
        }
    }

    fn parse_match_expr(&mut self) -> Expr {
        let subject = self.parse_comparison();
        self.expect_kind(TokenKind::Colon, "match block marker", ":");
        self.consume_newline();
        self.expect_kind(TokenKind::Indent, "indented match body", "indent");
        let mut arms = Vec::new();
        while !self.check(&TokenKind::Dedent) && !self.at_eof() {
            self.skip_newlines();
            if self.check(&TokenKind::Dedent) {
                break;
            }
            let pattern = self.parse_pattern();
            let pattern_span = self.previous_span();
            self.expect_kind(TokenKind::Arrow, "match arm arrow", "->");
            // Arm body can be inline or on the next line (indented block)
            let expr = self.parse_arm_body();
            self.consume_newline();
            arms.push((pattern, pattern_span, expr));
        }
        self.expect_kind(TokenKind::Dedent, "dedent", "dedent");

        let arm_count = arms.len();
        let mut final_arms = Vec::with_capacity(arm_count);
        for (index, (pattern, span, expr)) in arms.into_iter().enumerate() {
            if matches!(pattern, Pattern::Wildcard) && index + 1 < arm_count {
                self.diagnostics.push(Diagnostic {
                    code: "W_WILDCARD_NOT_LAST".to_owned(),
                    message: "Wildcard match arms should appear last".to_owned(),
                    level: DiagnosticLevel::Warning,
                    stage: DiagnosticStage::Parser,
                    range: span,
                    expected: "wildcard arm as the last arm".to_owned(),
                    actual: "wildcard before a later arm".to_owned(),
                    cause: "wildcard_not_last".to_owned(),
                    related: Vec::new(),
                    suggested_fix: "Move `_ -> ...` to the end of the match expression.".to_owned(),
                    alternatives: vec!["List the specific variants first.".to_owned()],
                    confidence: 0.88,
                });
            }
            final_arms.push(MatchArm { pattern, expr });
        }

        Expr::Match {
            subject: Box::new(subject),
            arms: final_arms,
        }
    }

    fn parse_arm_body(&mut self) -> Expr {
        if self.check(&TokenKind::Newline) {
            // Block arm body: -> NEWLINE INDENT [let ...] expr DEDENT
            self.consume_newline();
            self.expect_kind(TokenKind::Indent, "arm body indent", "indent");
            let expr = self.parse_block_contents();
            self.expect_kind(TokenKind::Dedent, "arm body dedent", "dedent");
            expr
        } else {
            // Inline arm body
            self.parse_expr()
        }
    }

    fn parse_pattern(&mut self) -> Pattern {
        match self.advance().kind {
            TokenKind::Ident(name) if name == "_" => Pattern::Wildcard,
            TokenKind::Ident(name) => {
                let mut bindings = Vec::new();
                if self.match_kind(&TokenKind::LParen) {
                    while !self.check(&TokenKind::RParen) && !self.at_eof() {
                        bindings.push(self.expect_ident("pattern binding"));
                        if !self.match_kind(&TokenKind::Comma) {
                            break;
                        }
                    }
                    self.expect_kind(TokenKind::RParen, "closing parenthesis", ")");
                }
                Pattern::Variant { name, bindings }
            }
            other => {
                self.push_error(
                    "E_EXPECTED_PATTERN",
                    "Expected a match pattern",
                    self.previous_span(),
                    "pattern",
                    format!("{other:?}"),
                    "missing_pattern",
                    "Insert a variant pattern or `_`.",
                );
                Pattern::Wildcard
            }
        }
    }

    fn parse_comparison(&mut self) -> Expr {
        let mut expr = self.parse_term();
        loop {
            let op = if self.match_kind(&TokenKind::Greater) {
                Some(BinaryOp::Greater)
            } else if self.match_kind(&TokenKind::EqualEqual) {
                Some(BinaryOp::Equal)
            } else if self.match_kind(&TokenKind::Less) {
                Some(BinaryOp::Less)
            } else {
                None
            };

            let Some(op) = op else {
                break;
            };

            let right = self.parse_term();
            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        expr
    }

    fn parse_term(&mut self) -> Expr {
        let mut expr = self.parse_factor();
        loop {
            let op = if self.match_kind(&TokenKind::Plus) {
                Some(BinaryOp::Add)
            } else if self.match_kind(&TokenKind::Minus) {
                Some(BinaryOp::Subtract)
            } else {
                None
            };

            let Some(op) = op else {
                break;
            };

            let right = self.parse_factor();
            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        expr
    }

    fn parse_factor(&mut self) -> Expr {
        let mut expr = self.parse_primary();
        loop {
            let op = if self.match_kind(&TokenKind::Star) {
                Some(BinaryOp::Multiply)
            } else if self.match_kind(&TokenKind::Slash) {
                Some(BinaryOp::Divide)
            } else if self.match_kind(&TokenKind::Percent) {
                Some(BinaryOp::Modulo)
            } else {
                None
            };

            let Some(op) = op else {
                break;
            };

            let right = self.parse_primary();
            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        expr
    }

    /// Parse a primary expression with same-line postfix: [index], (Apply), ..= range.
    fn parse_primary(&mut self) -> Expr {
        let mut expr = self.parse_atom();
        loop {
            if self.match_kind(&TokenKind::DotDotEq) {
                // Range: expr ..= end
                let end = self.parse_atom();
                expr = Expr::Range {
                    start: Box::new(expr),
                    end: Box::new(end),
                };
            } else if self.check(&TokenKind::LBracket) {
                self.position += 1; // consume [
                let index = self.parse_expr();
                self.expect_kind(TokenKind::RBracket, "closing bracket", "]");
                expr = Expr::Index {
                    receiver: Box::new(expr),
                    index: Box::new(index),
                };
            } else if self.check(&TokenKind::LParen) {
                // Closure / function application: expr(args)
                self.position += 1; // consume (
                let mut args = Vec::new();
                while !self.check(&TokenKind::RParen) && !self.at_eof() {
                    args.push(self.parse_expr());
                    if !self.match_kind(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect_kind(TokenKind::RParen, "closing parenthesis", ")");
                expr = Expr::Apply {
                    func: Box::new(expr),
                    args,
                };
            } else {
                break;
            }
        }
        expr
    }

    /// Parse a base (atom) expression: literals, identifiers, calls, parenthesized, lists.
    fn parse_atom(&mut self) -> Expr {
        let token = self.advance();
        match token.kind {
            TokenKind::Int(value) => Expr::Int(value),
            TokenKind::True => Expr::Bool(true),
            TokenKind::False => Expr::Bool(false),
            TokenKind::String(value) => Expr::String(value),
            TokenKind::Ident(name) => {
                if name == "null" {
                    self.push_error(
                        "E_NULL_FORBIDDEN",
                        "The language does not allow `null`",
                        token.span,
                        "Option<T> or Result<T, E>",
                        "null".to_owned(),
                        "null_forbidden",
                        "Replace `null` with an explicit Option or Result value.",
                    );
                    return Expr::Error;
                }
                if self.match_kind(&TokenKind::Dot) {
                    // Dotted call: capability.method(...) or module.fn(...)
                    let method = self.expect_ident("method name");
                    if self.match_kind(&TokenKind::LParen) {
                        let callee = format!("{name}.{method}");
                        let args = self.parse_call_args();
                        Expr::Call { callee, args }
                    } else {
                        // Dotted access without call (e.g., in match subject)
                        Expr::Ident(format!("{name}.{method}"))
                    }
                } else if self.match_kind(&TokenKind::LParen) {
                    let args = self.parse_call_args();
                    Expr::Call { callee: name, args }
                } else {
                    Expr::Ident(name)
                }
            }
            TokenKind::LParen => {
                // Could be a grouped expression or a tuple
                let first = self.parse_expr();
                if self.match_kind(&TokenKind::Comma) {
                    // Tuple
                    let mut items = vec![first];
                    while !self.check(&TokenKind::RParen) && !self.at_eof() {
                        items.push(self.parse_expr());
                        if !self.match_kind(&TokenKind::Comma) {
                            break;
                        }
                    }
                    self.expect_kind(TokenKind::RParen, "closing parenthesis", ")");
                    Expr::Tuple(items)
                } else {
                    self.expect_kind(TokenKind::RParen, "closing parenthesis", ")");
                    first
                }
            }
            TokenKind::LBracket => {
                // List literal: [e1, e2, ...]
                let mut items = Vec::new();
                while !self.check(&TokenKind::RBracket) && !self.at_eof() {
                    items.push(self.parse_expr());
                    if !self.match_kind(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect_kind(TokenKind::RBracket, "closing bracket", "]");
                Expr::List(items)
            }
            other => {
                self.push_error(
                    "E_EXPECTED_EXPR",
                    "Expected an expression",
                    token.span,
                    "expression",
                    format!("{other:?}"),
                    "missing_expression",
                    "Insert a literal, identifier, or expression.",
                );
                Expr::Error
            }
        }
    }

    fn parse_call_args(&mut self) -> Vec<Expr> {
        let mut args = Vec::new();
        while !self.check(&TokenKind::RParen) && !self.at_eof() {
            args.push(self.parse_expr());
            if !self.match_kind(&TokenKind::Comma) {
                break;
            }
        }
        self.expect_kind(TokenKind::RParen, "closing parenthesis", ")");
        args
    }

    fn expect_ident(&mut self, expected: &str) -> String {
        match self.advance().kind {
            TokenKind::Ident(name) => name,
            other => {
                self.push_error(
                    "E_EXPECTED_IDENT",
                    "Expected an identifier",
                    self.previous_span(),
                    expected,
                    format!("{other:?}"),
                    "missing_identifier",
                    "Insert an identifier name.",
                );
                "__error".to_owned()
            }
        }
    }

    fn expect_kind(&mut self, kind: TokenKind, expected: &str, expected_short: &str) {
        if self.match_kind(&kind) {
            return;
        }
        self.push_error(
            "E_EXPECTED_TOKEN",
            &format!("Expected {expected}"),
            self.current_span(),
            expected_short,
            format!("{:?}", self.current().kind),
            "missing_token",
            &format!("Insert `{expected_short}`."),
        );
    }

    fn consume_newline(&mut self) {
        self.match_kind(&TokenKind::Newline);
    }

    fn skip_newlines(&mut self) {
        while self.match_kind(&TokenKind::Newline) {}
    }

    fn check(&self, kind: &TokenKind) -> bool {
        self.same_variant(&self.current().kind, kind)
    }

    fn match_kind(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.position += 1;
            true
        } else {
            false
        }
    }

    fn advance(&mut self) -> Token {
        let token = self.current().clone();
        if !self.at_eof() {
            self.position += 1;
        }
        token
    }

    fn current(&self) -> &Token {
        &self.tokens[self.position.min(self.tokens.len().saturating_sub(1))]
    }

    fn current_span(&self) -> Span {
        self.current().span
    }

    fn previous_span(&self) -> Span {
        if self.position == 0 {
            self.current_span()
        } else {
            self.tokens[self.position - 1].span
        }
    }

    fn peek_kind(&self, offset: usize) -> Option<&TokenKind> {
        self.tokens.get(self.position + offset).map(|t| &t.kind)
    }

    fn at_eof(&self) -> bool {
        matches!(self.current().kind, TokenKind::Eof)
    }

    fn same_variant(&self, left: &TokenKind, right: &TokenKind) -> bool {
        std::mem::discriminant(left) == std::mem::discriminant(right)
    }

    fn push_error(
        &mut self,
        code: &str,
        message: &str,
        span: Span,
        expected: impl Into<String>,
        actual: impl Into<String>,
        cause: &str,
        suggested_fix: &str,
    ) {
        self.diagnostics.push(Diagnostic {
            code: code.to_owned(),
            message: message.to_owned(),
            level: DiagnosticLevel::Error,
            stage: DiagnosticStage::Parser,
            range: span,
            expected: expected.into(),
            actual: actual.into(),
            cause: cause.to_owned(),
            related: Vec::new(),
            suggested_fix: suggested_fix.to_owned(),
            alternatives: vec![
                "Keep one expression per indented block.".to_owned(),
                "Check the surrounding punctuation and indentation.".to_owned(),
            ],
            confidence: 0.9,
        });
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum TopLevelStage {
    Imports,
    Types,
    Functions,
}
