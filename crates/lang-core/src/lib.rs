mod ast;
mod diagnostics;
mod fmt;
mod lexer;
mod parser;
mod typecheck;
mod types;

pub use ast::{
    BinaryOp, Expr, Function, MatchArm, Module, Param, Pattern, TypeDecl, VariantDecl, VariantField,
};
pub use diagnostics::{
    CompileResult, Diagnostic, DiagnosticLevel, DiagnosticStage, RelatedInformation, Span,
};
pub use fmt::format_module;
pub use lexer::{LexOutput, Token, TokenKind, lex};
pub use parser::ParseOutput;
pub use typecheck::{
    TypedExpr, TypedExprKind, TypedFunction, TypedMatchArm, TypedModule, TypedParam,
};
pub use types::Type;

use parser::parse;
use typecheck::{typecheck, typecheck_partial};

/// Split a leading shebang line from source so executable `.ar` files can be
/// compiled without treating the launcher directive as language syntax.
pub fn split_shebang(source: &str) -> (Option<&str>, &str) {
    if !source.starts_with("#!") {
        return (None, source);
    }

    match source.find('\n') {
        Some(index) => (Some(&source[..index + 1]), &source[index + 1..]),
        None => (Some(source), ""),
    }
}

/// Lex and parse source into a raw `Module` AST without typechecking.
pub fn parse_source(source: &str) -> ParseOutput {
    let (_, source) = split_shebang(source);
    let lex_output = lex(source);
    parse(&lex_output.tokens)
}

pub fn compile_module(source: &str) -> CompileResult<TypedModule> {
    let (_, source) = split_shebang(source);
    let lex_output = lex(source);
    let parse_output = parse(&lex_output.tokens);
    let mut diagnostics = lex_output.diagnostics;
    diagnostics.extend(parse_output.diagnostics);
    typecheck(parse_output.module, diagnostics)
}

pub fn compile_module_partial(source: &str) -> CompileResult<TypedModule> {
    let (_, source) = split_shebang(source);
    let lex_output = lex(source);
    let parse_output = parse(&lex_output.tokens);
    let mut diagnostics = lex_output.diagnostics;
    diagnostics.extend(parse_output.diagnostics);
    typecheck_partial(parse_output.module, diagnostics)
}
