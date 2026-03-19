mod ast;
mod diagnostics;
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
pub use lexer::{LexOutput, Token, TokenKind, lex};
pub use typecheck::{
    TypedExpr, TypedExprKind, TypedFunction, TypedMatchArm, TypedModule, TypedParam,
};
pub use types::Type;

use parser::parse;
use typecheck::typecheck;

pub fn compile_module(source: &str) -> CompileResult<TypedModule> {
    let lex_output = lex(source);
    let parse_output = parse(&lex_output.tokens);
    let mut diagnostics = lex_output.diagnostics;
    diagnostics.extend(parse_output.diagnostics);
    typecheck(parse_output.module, diagnostics)
}
