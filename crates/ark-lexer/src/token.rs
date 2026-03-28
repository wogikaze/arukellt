//! Token types for the Arukellt lexer.

use ark_diagnostics::Span;

/// A part of an f-string literal.
#[derive(Debug, Clone, PartialEq)]
pub enum FStringPart {
    /// A literal string fragment.
    Lit(String),
    /// An expression to be interpolated (raw source text).
    Expr(String),
}

/// The kind of a lexical token.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords (v0 active)
    Fn,
    Struct,
    Enum,
    Let,
    Mut,
    If,
    Else,
    Match,
    While,
    Loop,
    For,
    In,
    Break,
    Continue,
    Return,
    Pub,
    Import,
    As,

    // Keywords (v1)
    Trait,
    Impl,

    // Keywords (v3)
    Use,

    // Reserved keywords (future)
    Reserved(String),

    // Literals
    IntLit(i64),
    FloatLit(f64),
    /// Integer literal with explicit type suffix (e.g. `42u8`, `1000u32`)
    TypedIntLit(i64, String),
    /// Float literal with explicit type suffix (e.g. `3.14f32`)
    TypedFloatLit(f64, String),
    StringLit(String),
    FStringLit(Vec<FStringPart>),
    CharLit(char),
    BoolLit(bool),

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    EqEq,
    BangEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    AmpAmp,
    PipePipe,
    Bang,
    Amp,
    Pipe,
    Caret,
    Tilde,
    Shl,
    Shr,

    // Delimiters
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Semi,
    Dot,
    DotDot,
    Arrow,
    FatArrow,
    Question,
    Colon,
    ColonColon,

    // Assignment
    Eq,

    // Special
    Ident(String),
    OuterDocComment(String),
    InnerDocComment(String),
    Newline,
    Eof,
    Error,
}

impl TokenKind {
    /// Returns `true` if this token originated from a keyword
    /// (v0 active, bool literal, or v1 reserved).
    pub fn is_keyword(&self) -> bool {
        matches!(
            self,
            TokenKind::Fn
                | TokenKind::Struct
                | TokenKind::Enum
                | TokenKind::Let
                | TokenKind::Mut
                | TokenKind::If
                | TokenKind::Else
                | TokenKind::Match
                | TokenKind::While
                | TokenKind::Loop
                | TokenKind::For
                | TokenKind::In
                | TokenKind::Break
                | TokenKind::Continue
                | TokenKind::Return
                | TokenKind::Pub
                | TokenKind::Import
                | TokenKind::As
                | TokenKind::Trait
                | TokenKind::Impl
                | TokenKind::Use
                | TokenKind::BoolLit(_)
                | TokenKind::Reserved(_)
        )
    }

    /// Returns the binary-operator precedence (higher binds tighter), or `None`.
    pub fn precedence(&self) -> Option<u8> {
        match self {
            TokenKind::PipePipe => Some(1),
            TokenKind::AmpAmp => Some(2),
            TokenKind::Pipe => Some(3),
            TokenKind::Caret => Some(4),
            TokenKind::Amp => Some(5),
            TokenKind::EqEq | TokenKind::BangEq => Some(6),
            TokenKind::Lt | TokenKind::LtEq | TokenKind::Gt | TokenKind::GtEq => Some(7),
            TokenKind::Shl | TokenKind::Shr => Some(8),
            TokenKind::Plus | TokenKind::Minus => Some(9),
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent => Some(10),
            _ => None,
        }
    }
}

/// A lexical token with its kind and source span.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}
