//! Lexer for the Arukellt language (v0).
//!
//! Tokenizes UTF-8 source into a stream of [`Token`]s with accurate [`Span`] tracking.
//! Supports line comments (`//`), nested block comments (`/* ... */`), and shebang lines.

use ark_diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSink, Span};

// ── Token types ──────────────────────────────────────────────────────────────

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

    // Reserved keywords (future)
    Reserved(String),

    // Literals
    IntLit(i64),
    FloatLit(f64),
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

// ── Lexer ────────────────────────────────────────────────────────────────────

/// Lexer for Arukellt source code.
///
/// Implements [`Iterator`] yielding [`Token`]s. The final token is always
/// [`TokenKind::Eof`], after which the iterator returns `None`.
pub struct Lexer<'src> {
    source: &'src str,
    bytes: &'src [u8],
    file_id: u32,
    pos: usize,
    diagnostics: DiagnosticSink,
    eof_returned: bool,
}

impl<'src> Lexer<'src> {
    /// Create a new lexer for the given source.
    ///
    /// If the source starts with a `#!` shebang line, it is skipped.
    pub fn new(file_id: u32, source: &'src str) -> Self {
        let mut pos = 0;
        if source.starts_with("#!") {
            pos = source.find('\n').map_or(source.len(), |i| i + 1);
        }
        Lexer {
            source,
            bytes: source.as_bytes(),
            file_id,
            pos,
            diagnostics: DiagnosticSink::new(),
            eof_returned: false,
        }
    }

    /// Lex all tokens and return them along with any diagnostics.
    pub fn tokenize(mut self) -> (Vec<Token>, Vec<Diagnostic>) {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token();
            let done = token.kind == TokenKind::Eof;
            tokens.push(token);
            if done {
                break;
            }
        }
        (tokens, self.diagnostics.into_diagnostics())
    }

    /// Access accumulated diagnostics.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        self.diagnostics.diagnostics()
    }

    /// Returns `true` if any error diagnostics have been emitted.
    pub fn has_errors(&self) -> bool {
        self.diagnostics.has_errors()
    }

    // ── Core helpers ─────────────────────────────────────────────────────

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<u8> {
        self.bytes.get(self.pos + offset).copied()
    }

    fn advance(&mut self) -> u8 {
        let b = self.bytes[self.pos];
        self.pos += 1;
        b
    }

    fn span(&self, start: usize) -> Span {
        Span::new(self.file_id, start as u32, self.pos as u32)
    }

    fn emit(&mut self, span: Span, code: DiagnosticCode, msg: &str) {
        self.diagnostics.emit(
            Diagnostic::new(code)
                .with_message(msg)
                .with_label(span, msg),
        );
    }

    // ── Main dispatch ────────────────────────────────────────────────────

    fn next_token(&mut self) -> Token {
        self.skip_whitespace_and_comments();

        let start = self.pos;

        let Some(byte) = self.peek() else {
            return Token::new(TokenKind::Eof, self.span(start));
        };

        match byte {
            // Newlines
            b'\n' => {
                self.advance();
                Token::new(TokenKind::Newline, self.span(start))
            }
            b'\r' => {
                self.advance();
                if self.peek() == Some(b'\n') {
                    self.advance();
                }
                Token::new(TokenKind::Newline, self.span(start))
            }

            // Identifiers / keywords
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => self.lex_ident(start),

            // Numbers
            b'0'..=b'9' => self.lex_number(start),

            // Strings
            b'"' => self.lex_string(start),

            // Chars
            b'\'' => self.lex_char(start),

            // Two-char operators / punctuation
            b'=' => self.lex_two(
                start,
                b'=',
                TokenKind::EqEq,
                Some((b'>', TokenKind::FatArrow)),
                TokenKind::Eq,
            ),
            b'!' => self.lex_two(start, b'=', TokenKind::BangEq, None, TokenKind::Bang),
            b'<' => self.lex_two(
                start,
                b'=',
                TokenKind::LtEq,
                Some((b'<', TokenKind::Shl)),
                TokenKind::Lt,
            ),
            b'>' => self.lex_two(
                start,
                b'=',
                TokenKind::GtEq,
                Some((b'>', TokenKind::Shr)),
                TokenKind::Gt,
            ),
            b'&' => self.lex_two(start, b'&', TokenKind::AmpAmp, None, TokenKind::Amp),
            b'|' => self.lex_two(start, b'|', TokenKind::PipePipe, None, TokenKind::Pipe),
            b'-' => self.lex_two(start, b'>', TokenKind::Arrow, None, TokenKind::Minus),
            b':' => self.lex_two(start, b':', TokenKind::ColonColon, None, TokenKind::Colon),

            // Single-char operators
            b'+' => self.lex_single(start, TokenKind::Plus),
            b'*' => self.lex_single(start, TokenKind::Star),
            b'/' => self.lex_single(start, TokenKind::Slash),
            b'%' => self.lex_single(start, TokenKind::Percent),
            b'^' => self.lex_single(start, TokenKind::Caret),
            b'~' => self.lex_single(start, TokenKind::Tilde),

            // Delimiters
            b'(' => self.lex_single(start, TokenKind::LParen),
            b')' => self.lex_single(start, TokenKind::RParen),
            b'{' => self.lex_single(start, TokenKind::LBrace),
            b'}' => self.lex_single(start, TokenKind::RBrace),
            b'[' => self.lex_single(start, TokenKind::LBracket),
            b']' => self.lex_single(start, TokenKind::RBracket),
            b',' => self.lex_single(start, TokenKind::Comma),
            b';' => self.lex_single(start, TokenKind::Semi),
            b'.' => {
                if self.peek_at(1) == Some(b'.') {
                    self.pos += 2;
                    Token::new(TokenKind::DotDot, self.span(start))
                } else {
                    self.lex_single(start, TokenKind::Dot)
                }
            }
            b'?' => self.lex_single(start, TokenKind::Question),

            // Non-ASCII — possibly Unicode identifier
            _ if byte >= 128 => {
                let ch = self.source[self.pos..].chars().next().unwrap();
                if ch.is_alphabetic() {
                    self.lex_ident(start)
                } else {
                    self.pos += ch.len_utf8();
                    let span = self.span(start);
                    self.emit(
                        span,
                        DiagnosticCode::E0001,
                        &format!("unexpected character '{ch}'"),
                    );
                    Token::new(TokenKind::Error, span)
                }
            }

            // Unknown ASCII character
            _ => {
                self.advance();
                let span = self.span(start);
                self.emit(
                    span,
                    DiagnosticCode::E0001,
                    &format!("unexpected character '{}'", byte as char),
                );
                Token::new(TokenKind::Error, span)
            }
        }
    }

    // ── Whitespace & comments ────────────────────────────────────────────

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            // Skip spaces and tabs (not newlines — those become tokens)
            while matches!(self.peek(), Some(b' ' | b'\t')) {
                self.pos += 1;
            }
            if self.peek() == Some(b'/') {
                match self.peek_at(1) {
                    Some(b'/') => {
                        self.skip_line_comment();
                        continue;
                    }
                    Some(b'*') => {
                        self.skip_block_comment();
                        continue;
                    }
                    _ => break,
                }
            }
            break;
        }
    }

    fn skip_line_comment(&mut self) {
        while let Some(b) = self.peek() {
            if b == b'\n' || b == b'\r' {
                break;
            }
            self.pos += 1;
        }
    }

    fn skip_block_comment(&mut self) {
        let start = self.pos;
        self.pos += 2; // skip /*
        let mut depth: u32 = 1;
        while depth > 0 {
            match self.peek() {
                None => {
                    let span = Span::new(self.file_id, start as u32, self.pos as u32);
                    self.emit(span, DiagnosticCode::E0003, "unterminated block comment");
                    return;
                }
                Some(b'/') if self.peek_at(1) == Some(b'*') => {
                    self.pos += 2;
                    depth += 1;
                }
                Some(b'*') if self.peek_at(1) == Some(b'/') => {
                    self.pos += 2;
                    depth -= 1;
                }
                _ => {
                    self.pos += 1;
                }
            }
        }
    }

    // ── Simple token helpers ─────────────────────────────────────────────

    fn lex_single(&mut self, start: usize, kind: TokenKind) -> Token {
        self.advance();
        Token::new(kind, self.span(start))
    }

    /// Try to match a two-character operator, with an optional alternative second char.
    fn lex_two(
        &mut self,
        start: usize,
        next1: u8,
        kind1: TokenKind,
        alt: Option<(u8, TokenKind)>,
        fallback: TokenKind,
    ) -> Token {
        self.advance();
        if self.peek() == Some(next1) {
            self.advance();
            return Token::new(kind1, self.span(start));
        }
        if let Some((next2, kind2)) = alt {
            if self.peek() == Some(next2) {
                self.advance();
                return Token::new(kind2, self.span(start));
            }
        }
        Token::new(fallback, self.span(start))
    }

    // ── Identifiers & keywords ───────────────────────────────────────────

    fn lex_ident(&mut self, start: usize) -> Token {
        while self.pos < self.source.len() {
            let b = self.bytes[self.pos];
            if b.is_ascii_alphanumeric() || b == b'_' {
                self.pos += 1;
            } else if b >= 128 {
                let ch = self.source[self.pos..].chars().next().unwrap();
                if ch.is_alphanumeric() || ch == '_' {
                    self.pos += ch.len_utf8();
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        let text = &self.source[start..self.pos];
        let span = self.span(start);
        let kind = match text {
            "fn" => TokenKind::Fn,
            "struct" => TokenKind::Struct,
            "enum" => TokenKind::Enum,
            "let" => TokenKind::Let,
            "mut" => TokenKind::Mut,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "match" => TokenKind::Match,
            "while" => TokenKind::While,
            "loop" => TokenKind::Loop,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "return" => TokenKind::Return,
            "pub" => TokenKind::Pub,
            "import" => TokenKind::Import,
            "as" => TokenKind::As,
            "trait" => TokenKind::Trait,
            "impl" => TokenKind::Impl,
            "true" => TokenKind::BoolLit(true),
            "false" => TokenKind::BoolLit(false),
            "async" | "await" | "dyn" | "where" | "type" | "const" | "unsafe" | "extern"
            | "use" | "mod" | "super" | "Self" => TokenKind::Reserved(text.to_owned()),
            _ => {
                // f"..." string interpolation
                if text == "f" && self.peek() == Some(b'"') {
                    return self.lex_fstring(start);
                }
                TokenKind::Ident(text.to_owned())
            }
        };
        Token::new(kind, span)
    }

    // ── Numbers ──────────────────────────────────────────────────────────

    fn lex_number(&mut self, start: usize) -> Token {
        if self.bytes[self.pos] == b'0' {
            match self.peek_at(1) {
                Some(b'x' | b'X') => {
                    self.pos += 2;
                    return self.lex_radix_int(start, 16, is_hex_digit);
                }
                Some(b'b' | b'B') => {
                    self.pos += 2;
                    return self.lex_radix_int(start, 2, is_bin_digit);
                }
                _ => {}
            }
        }

        self.eat_digits(|b| b.is_ascii_digit());

        let mut is_float = false;

        // Fractional part: '.' followed by a digit
        if self.peek() == Some(b'.') && self.peek_at(1).is_some_and(|b| b.is_ascii_digit()) {
            is_float = true;
            self.advance();
            self.eat_digits(|b| b.is_ascii_digit());
        }

        // Exponent
        if matches!(self.peek(), Some(b'e' | b'E')) {
            is_float = true;
            self.advance();
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.advance();
            }
            self.eat_digits(|b| b.is_ascii_digit());
        }

        let text = &self.source[start..self.pos];
        let span = self.span(start);
        let clean: String = text.chars().filter(|&c| c != '_').collect();

        if is_float {
            match clean.parse::<f64>() {
                Ok(v) => Token::new(TokenKind::FloatLit(v), span),
                Err(_) => {
                    self.emit(span, DiagnosticCode::E0003, "invalid float literal");
                    Token::new(TokenKind::Error, span)
                }
            }
        } else {
            match clean.parse::<i64>() {
                Ok(v) => Token::new(TokenKind::IntLit(v), span),
                Err(_) => {
                    self.emit(span, DiagnosticCode::E0003, "invalid integer literal");
                    Token::new(TokenKind::Error, span)
                }
            }
        }
    }

    fn lex_radix_int(&mut self, start: usize, radix: u32, is_digit: fn(u8) -> bool) -> Token {
        let digit_start = self.pos;
        while let Some(b) = self.peek() {
            if is_digit(b) || b == b'_' {
                self.pos += 1;
            } else {
                break;
            }
        }
        let span = self.span(start);
        if self.pos == digit_start {
            self.emit(
                span,
                DiagnosticCode::E0003,
                "expected digits after integer prefix",
            );
            return Token::new(TokenKind::Error, span);
        }
        let digits: String = self.source[digit_start..self.pos]
            .chars()
            .filter(|&c| c != '_')
            .collect();
        match i64::from_str_radix(&digits, radix) {
            Ok(v) => Token::new(TokenKind::IntLit(v), span),
            Err(_) => {
                self.emit(span, DiagnosticCode::E0003, "integer literal out of range");
                Token::new(TokenKind::Error, span)
            }
        }
    }

    fn eat_digits(&mut self, is_digit: fn(u8) -> bool) {
        while let Some(b) = self.peek() {
            if is_digit(b) || b == b'_' {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    // ── Strings ──────────────────────────────────────────────────────────

    fn lex_string(&mut self, start: usize) -> Token {
        self.advance(); // opening '"'
        let mut value = String::new();
        let mut has_error = false;

        loop {
            match self.peek() {
                None | Some(b'\n') | Some(b'\r') => {
                    let span = self.span(start);
                    self.emit(span, DiagnosticCode::E0003, "unterminated string literal");
                    return Token::new(TokenKind::Error, span);
                }
                Some(b'"') => {
                    self.advance();
                    let span = self.span(start);
                    return if has_error {
                        Token::new(TokenKind::Error, span)
                    } else {
                        Token::new(TokenKind::StringLit(value), span)
                    };
                }
                Some(b'\\') => match self.parse_escape() {
                    Some(ch) => value.push(ch),
                    None => has_error = true,
                },
                Some(_) => {
                    let ch = self.source[self.pos..].chars().next().unwrap();
                    value.push(ch);
                    self.pos += ch.len_utf8();
                }
            }
        }
    }

    fn lex_fstring(&mut self, start: usize) -> Token {
        self.advance(); // opening '"'
        let mut parts: Vec<FStringPart> = Vec::new();
        let mut current_lit = String::new();

        loop {
            match self.peek() {
                None | Some(b'\n') | Some(b'\r') => {
                    let span = self.span(start);
                    self.emit(span, DiagnosticCode::E0003, "unterminated f-string literal");
                    return Token::new(TokenKind::Error, span);
                }
                Some(b'"') => {
                    self.advance();
                    if !current_lit.is_empty() {
                        parts.push(FStringPart::Lit(current_lit));
                    }
                    return Token::new(TokenKind::FStringLit(parts), self.span(start));
                }
                Some(b'{') => {
                    self.advance();
                    // {{ is literal {
                    if self.peek() == Some(b'{') {
                        self.advance();
                        current_lit.push('{');
                        continue;
                    }
                    if !current_lit.is_empty() {
                        parts.push(FStringPart::Lit(std::mem::take(&mut current_lit)));
                    }
                    // Collect expression text until matching }
                    let mut expr_text = String::new();
                    let mut depth = 1u32;
                    loop {
                        match self.peek() {
                            None | Some(b'\n') | Some(b'\r') => {
                                let span = self.span(start);
                                self.emit(
                                    span,
                                    DiagnosticCode::E0003,
                                    "unterminated interpolation in f-string",
                                );
                                return Token::new(TokenKind::Error, span);
                            }
                            Some(b'{') => {
                                depth += 1;
                                expr_text.push('{');
                                self.advance();
                            }
                            Some(b'}') => {
                                depth -= 1;
                                self.advance();
                                if depth == 0 {
                                    break;
                                }
                                expr_text.push('}');
                            }
                            Some(_) => {
                                let ch = self.source[self.pos..].chars().next().unwrap();
                                expr_text.push(ch);
                                self.pos += ch.len_utf8();
                            }
                        }
                    }
                    parts.push(FStringPart::Expr(expr_text));
                }
                Some(b'}') => {
                    self.advance();
                    // }} is literal }
                    if self.peek() == Some(b'}') {
                        self.advance();
                        current_lit.push('}');
                    } else {
                        current_lit.push('}');
                    }
                }
                Some(b'\\') => match self.parse_escape() {
                    Some(ch) => current_lit.push(ch),
                    None => {
                        let span = self.span(start);
                        return Token::new(TokenKind::Error, span);
                    }
                },
                Some(_) => {
                    let ch = self.source[self.pos..].chars().next().unwrap();
                    current_lit.push(ch);
                    self.pos += ch.len_utf8();
                }
            }
        }
    }

    // ── Chars ────────────────────────────────────────────────────────────

    fn lex_char(&mut self, start: usize) -> Token {
        self.advance(); // opening '\''

        // Empty char literal
        if self.peek() == Some(b'\'') {
            self.advance();
            let span = self.span(start);
            self.emit(span, DiagnosticCode::E0003, "empty character literal");
            return Token::new(TokenKind::Error, span);
        }

        // Unterminated at EOF or newline
        if matches!(self.peek(), None | Some(b'\n') | Some(b'\r')) {
            let span = self.span(start);
            self.emit(
                span,
                DiagnosticCode::E0003,
                "unterminated character literal",
            );
            return Token::new(TokenKind::Error, span);
        }

        // Parse the character value
        let ch = if self.peek() == Some(b'\\') {
            match self.parse_escape() {
                Some(c) => c,
                None => {
                    // Recover: skip to closing quote or end of line
                    while !matches!(self.peek(), None | Some(b'\'') | Some(b'\n') | Some(b'\r')) {
                        self.pos += 1;
                    }
                    if self.peek() == Some(b'\'') {
                        self.advance();
                    }
                    return Token::new(TokenKind::Error, self.span(start));
                }
            }
        } else {
            let c = self.source[self.pos..].chars().next().unwrap();
            self.pos += c.len_utf8();
            c
        };

        // Expect closing quote
        if self.peek() != Some(b'\'') {
            let span = self.span(start);
            self.emit(
                span,
                DiagnosticCode::E0003,
                "unterminated character literal",
            );
            while !matches!(self.peek(), None | Some(b'\'') | Some(b'\n') | Some(b'\r')) {
                self.pos += 1;
            }
            if self.peek() == Some(b'\'') {
                self.advance();
            }
            return Token::new(TokenKind::Error, self.span(start));
        }
        self.advance(); // closing '\''
        Token::new(TokenKind::CharLit(ch), self.span(start))
    }

    // ── Escape sequences ─────────────────────────────────────────────────

    fn parse_escape(&mut self) -> Option<char> {
        let esc_start = self.pos;
        self.advance(); // skip '\\'

        let Some(b) = self.peek() else {
            let span = self.span(esc_start);
            self.emit(
                span,
                DiagnosticCode::E0003,
                "unexpected end of input in escape sequence",
            );
            return None;
        };
        self.advance();

        match b {
            b'\\' => Some('\\'),
            b'"' => Some('"'),
            b'\'' => Some('\''),
            b'n' => Some('\n'),
            b't' => Some('\t'),
            b'r' => Some('\r'),
            b'0' => Some('\0'),
            b'x' => self.parse_hex_byte(esc_start),
            b'u' => self.parse_unicode(esc_start),
            _ => {
                let span = self.span(esc_start);
                self.emit(
                    span,
                    DiagnosticCode::E0003,
                    &format!("invalid escape sequence: \\{}", b as char),
                );
                None
            }
        }
    }

    fn parse_hex_byte(&mut self, esc_start: usize) -> Option<char> {
        let mut value: u8 = 0;
        for _ in 0..2 {
            match self.peek().and_then(hex_val) {
                Some(d) => {
                    value = value * 16 + d;
                    self.advance();
                }
                None => {
                    let span = self.span(esc_start);
                    self.emit(span, DiagnosticCode::E0003, "invalid hex escape \\xNN");
                    return None;
                }
            }
        }
        Some(value as char)
    }

    fn parse_unicode(&mut self, esc_start: usize) -> Option<char> {
        if self.peek() != Some(b'{') {
            let span = self.span(esc_start);
            self.emit(
                span,
                DiagnosticCode::E0003,
                "expected '{' in unicode escape \\u{...}",
            );
            return None;
        }
        self.advance(); // '{'

        let digits_start = self.pos;
        while self.peek().and_then(hex_val).is_some() {
            self.advance();
        }
        let hex_str = &self.source[digits_start..self.pos];

        if self.peek() != Some(b'}') {
            let span = self.span(esc_start);
            self.emit(
                span,
                DiagnosticCode::E0003,
                "expected '}' in unicode escape \\u{...}",
            );
            return None;
        }
        self.advance(); // '}'

        if hex_str.is_empty() {
            let span = self.span(esc_start);
            self.emit(span, DiagnosticCode::E0003, "empty unicode escape \\u{}");
            return None;
        }

        let Ok(code) = u32::from_str_radix(hex_str, 16) else {
            let span = self.span(esc_start);
            self.emit(
                span,
                DiagnosticCode::E0003,
                "unicode escape value out of range",
            );
            return None;
        };

        match char::from_u32(code) {
            Some(ch) => Some(ch),
            None => {
                let span = self.span(esc_start);
                self.emit(span, DiagnosticCode::E0003, "invalid unicode codepoint");
                None
            }
        }
    }
}

// ── Free helpers ─────────────────────────────────────────────────────────────

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn is_hex_digit(b: u8) -> bool {
    b.is_ascii_hexdigit()
}

fn is_bin_digit(b: u8) -> bool {
    matches!(b, b'0' | b'1')
}

/// Tokenize source fully, including the EOF token.
pub fn tokenize(file_id: u32, source: &str) -> (Vec<Token>, Vec<Diagnostic>) {
    Lexer::new(file_id, source).tokenize()
}

// ── Iterator ─────────────────────────────────────────────────────────────────

impl Iterator for Lexer<'_> {
    type Item = Token;

    fn next(&mut self) -> Option<Token> {
        if self.eof_returned {
            return None;
        }
        let token = self.next_token();
        if token.kind == TokenKind::Eof {
            self.eof_returned = true;
        }
        Some(token)
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Collect all token kinds (including Eof).
    fn kinds(source: &str) -> Vec<TokenKind> {
        Lexer::new(0, source).map(|t| t.kind).collect()
    }

    /// Collect all tokens (including Eof).
    fn tokens(source: &str) -> Vec<Token> {
        Lexer::new(0, source).collect()
    }

    // ── Keywords ─────────────────────────────────────────────────────────

    #[test]
    fn v0_keywords() {
        let src = "fn struct enum let mut if else match while loop for in break continue return pub import as";
        assert_eq!(
            kinds(src),
            vec![
                TokenKind::Fn,
                TokenKind::Struct,
                TokenKind::Enum,
                TokenKind::Let,
                TokenKind::Mut,
                TokenKind::If,
                TokenKind::Else,
                TokenKind::Match,
                TokenKind::While,
                TokenKind::Loop,
                TokenKind::For,
                TokenKind::In,
                TokenKind::Break,
                TokenKind::Continue,
                TokenKind::Return,
                TokenKind::Pub,
                TokenKind::Import,
                TokenKind::As,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn bool_literals() {
        assert_eq!(
            kinds("true false"),
            vec![
                TokenKind::BoolLit(true),
                TokenKind::BoolLit(false),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn reserved_keywords() {
        let reserved = [
            "async", "await", "dyn", "where", "type", "const", "unsafe", "extern", "use", "mod",
            "super", "Self",
        ];
        for kw in reserved {
            assert_eq!(
                kinds(kw),
                vec![TokenKind::Reserved(kw.to_string()), TokenKind::Eof],
                "failed for reserved keyword: {kw}"
            );
        }
        // trait and impl are first-class keywords, not reserved
        assert_eq!(kinds("trait"), vec![TokenKind::Trait, TokenKind::Eof]);
        assert_eq!(kinds("impl"), vec![TokenKind::Impl, TokenKind::Eof]);
        // self is a regular identifier
        assert_eq!(
            kinds("self"),
            vec![TokenKind::Ident("self".to_string()), TokenKind::Eof]
        );
    }

    // ── Identifiers ──────────────────────────────────────────────────────

    #[test]
    fn identifiers() {
        assert_eq!(
            kinds("foo _bar baz42"),
            vec![
                TokenKind::Ident("foo".into()),
                TokenKind::Ident("_bar".into()),
                TokenKind::Ident("baz42".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn unicode_identifier() {
        assert_eq!(
            kinds("café"),
            vec![TokenKind::Ident("café".into()), TokenKind::Eof]
        );
    }

    // ── Integer literals ─────────────────────────────────────────────────

    #[test]
    fn integer_decimal() {
        assert_eq!(kinds("42"), vec![TokenKind::IntLit(42), TokenKind::Eof]);
        assert_eq!(kinds("0"), vec![TokenKind::IntLit(0), TokenKind::Eof]);
    }

    #[test]
    fn integer_hex() {
        assert_eq!(kinds("0xFF"), vec![TokenKind::IntLit(255), TokenKind::Eof]);
        assert_eq!(kinds("0X1A"), vec![TokenKind::IntLit(26), TokenKind::Eof]);
    }

    #[test]
    fn integer_binary() {
        assert_eq!(kinds("0b1010"), vec![TokenKind::IntLit(10), TokenKind::Eof]);
        assert_eq!(kinds("0B110"), vec![TokenKind::IntLit(6), TokenKind::Eof]);
    }

    #[test]
    fn integer_with_separators() {
        assert_eq!(
            kinds("1_000_000"),
            vec![TokenKind::IntLit(1_000_000), TokenKind::Eof]
        );
        assert_eq!(
            kinds("0xFF_FF"),
            vec![TokenKind::IntLit(0xFFFF), TokenKind::Eof]
        );
        assert_eq!(
            kinds("0b1010_0101"),
            vec![TokenKind::IntLit(0b1010_0101), TokenKind::Eof]
        );
    }

    // ── Float literals ───────────────────────────────────────────────────

    #[test]
    fn float_basic() {
        assert_eq!(
            kinds("3.14"),
            vec![TokenKind::FloatLit(3.14), TokenKind::Eof]
        );
        assert_eq!(kinds("0.5"), vec![TokenKind::FloatLit(0.5), TokenKind::Eof]);
    }

    #[test]
    fn float_scientific() {
        assert_eq!(
            kinds("1.0e10"),
            vec![TokenKind::FloatLit(1.0e10), TokenKind::Eof]
        );
        assert_eq!(
            kinds("2.5E-3"),
            vec![TokenKind::FloatLit(2.5e-3), TokenKind::Eof]
        );
        assert_eq!(kinds("1e5"), vec![TokenKind::FloatLit(1e5), TokenKind::Eof]);
    }

    #[test]
    fn integer_not_float_before_dot_ident() {
        // `1.method` should lex as IntLit(1), Dot, Ident("method")
        assert_eq!(
            kinds("1.method"),
            vec![
                TokenKind::IntLit(1),
                TokenKind::Dot,
                TokenKind::Ident("method".into()),
                TokenKind::Eof,
            ]
        );
    }

    // ── String literals ──────────────────────────────────────────────────

    #[test]
    fn string_basic() {
        assert_eq!(
            kinds(r#""hello""#),
            vec![TokenKind::StringLit("hello".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn string_escapes() {
        assert_eq!(
            kinds(r#""\\\"\n\t\r\0""#),
            vec![TokenKind::StringLit("\\\"\n\t\r\0".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn string_hex_escape() {
        assert_eq!(
            kinds(r#""\x41""#),
            vec![TokenKind::StringLit("A".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn string_unicode_escape() {
        assert_eq!(
            kinds(r#""\u{1F600}""#),
            vec![TokenKind::StringLit("😀".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn string_empty() {
        assert_eq!(
            kinds(r#""""#),
            vec![TokenKind::StringLit(String::new()), TokenKind::Eof]
        );
    }

    // ── Char literals ────────────────────────────────────────────────────

    #[test]
    fn char_basic() {
        assert_eq!(kinds("'a'"), vec![TokenKind::CharLit('a'), TokenKind::Eof]);
    }

    #[test]
    fn char_escapes() {
        assert_eq!(
            kinds(r"'\n'"),
            vec![TokenKind::CharLit('\n'), TokenKind::Eof]
        );
        assert_eq!(
            kinds(r"'\\'"),
            vec![TokenKind::CharLit('\\'), TokenKind::Eof]
        );
        assert_eq!(
            kinds(r"'\''"),
            vec![TokenKind::CharLit('\''), TokenKind::Eof]
        );
        assert_eq!(
            kinds(r"'\x41'"),
            vec![TokenKind::CharLit('A'), TokenKind::Eof]
        );
        assert_eq!(
            kinds(r"'\u{1F600}'"),
            vec![TokenKind::CharLit('😀'), TokenKind::Eof]
        );
    }

    // ── Operators ────────────────────────────────────────────────────────

    #[test]
    fn all_operators() {
        assert_eq!(
            kinds("+ - * / % == != < <= > >= && || ! & | ^ ~ << >>"),
            vec![
                TokenKind::Plus,
                TokenKind::Minus,
                TokenKind::Star,
                TokenKind::Slash,
                TokenKind::Percent,
                TokenKind::EqEq,
                TokenKind::BangEq,
                TokenKind::Lt,
                TokenKind::LtEq,
                TokenKind::Gt,
                TokenKind::GtEq,
                TokenKind::AmpAmp,
                TokenKind::PipePipe,
                TokenKind::Bang,
                TokenKind::Amp,
                TokenKind::Pipe,
                TokenKind::Caret,
                TokenKind::Tilde,
                TokenKind::Shl,
                TokenKind::Shr,
                TokenKind::Eof,
            ]
        );
    }

    // ── Delimiters ───────────────────────────────────────────────────────

    #[test]
    fn all_delimiters() {
        assert_eq!(
            kinds("( ) { } [ ] , ; . -> => ? : ::"),
            vec![
                TokenKind::LParen,
                TokenKind::RParen,
                TokenKind::LBrace,
                TokenKind::RBrace,
                TokenKind::LBracket,
                TokenKind::RBracket,
                TokenKind::Comma,
                TokenKind::Semi,
                TokenKind::Dot,
                TokenKind::Arrow,
                TokenKind::FatArrow,
                TokenKind::Question,
                TokenKind::Colon,
                TokenKind::ColonColon,
                TokenKind::Eof,
            ]
        );
    }

    // ── Assignment ───────────────────────────────────────────────────────

    #[test]
    fn assignment() {
        assert_eq!(kinds("="), vec![TokenKind::Eq, TokenKind::Eof]);
    }

    // ── Comments ─────────────────────────────────────────────────────────

    #[test]
    fn line_comment() {
        assert_eq!(
            kinds("foo // this is a comment\nbar"),
            vec![
                TokenKind::Ident("foo".into()),
                TokenKind::Newline,
                TokenKind::Ident("bar".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn block_comment() {
        assert_eq!(
            kinds("foo /* comment */ bar"),
            vec![
                TokenKind::Ident("foo".into()),
                TokenKind::Ident("bar".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn nested_block_comment() {
        assert_eq!(
            kinds("a /* outer /* inner */ still comment */ b"),
            vec![
                TokenKind::Ident("a".into()),
                TokenKind::Ident("b".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn unterminated_block_comment() {
        let mut lexer = Lexer::new(0, "/* never closed");
        let tok = lexer.next().unwrap();
        assert_eq!(tok.kind, TokenKind::Eof);
        assert!(lexer.has_errors());
    }

    // ── Shebang ──────────────────────────────────────────────────────────

    #[test]
    fn shebang() {
        let src = "#!/usr/bin/env arukellt\nfn main";
        assert_eq!(
            kinds(src),
            vec![
                TokenKind::Fn,
                TokenKind::Ident("main".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn shebang_only() {
        assert_eq!(kinds("#!/usr/bin/env arukellt"), vec![TokenKind::Eof]);
    }

    // ── Newlines ─────────────────────────────────────────────────────────

    #[test]
    fn newlines() {
        assert_eq!(
            kinds("a\nb"),
            vec![
                TokenKind::Ident("a".into()),
                TokenKind::Newline,
                TokenKind::Ident("b".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn crlf_newline() {
        assert_eq!(
            kinds("a\r\nb"),
            vec![
                TokenKind::Ident("a".into()),
                TokenKind::Newline,
                TokenKind::Ident("b".into()),
                TokenKind::Eof,
            ]
        );
    }

    // ── Error cases ──────────────────────────────────────────────────────

    #[test]
    fn unterminated_string() {
        let mut lexer = Lexer::new(0, r#""hello"#);
        let tok = lexer.next().unwrap();
        assert_eq!(tok.kind, TokenKind::Error);
        assert!(lexer.has_errors());
    }

    #[test]
    fn unterminated_string_at_newline() {
        let k = kinds("\"hello\nworld");
        assert_eq!(k[0], TokenKind::Error);
    }

    #[test]
    fn invalid_escape_in_string() {
        let mut lexer = Lexer::new(0, r#""\q""#);
        let tok = lexer.next().unwrap();
        assert_eq!(tok.kind, TokenKind::Error);
        assert!(lexer.has_errors());
    }

    #[test]
    fn empty_char_literal() {
        let mut lexer = Lexer::new(0, "''");
        let tok = lexer.next().unwrap();
        assert_eq!(tok.kind, TokenKind::Error);
        assert!(lexer.has_errors());
    }

    #[test]
    fn unterminated_char_literal() {
        let mut lexer = Lexer::new(0, "'a");
        let tok = lexer.next().unwrap();
        assert_eq!(tok.kind, TokenKind::Error);
        assert!(lexer.has_errors());
    }

    #[test]
    fn unknown_character() {
        let mut lexer = Lexer::new(0, "$");
        let tok = lexer.next().unwrap();
        assert_eq!(tok.kind, TokenKind::Error);
        assert!(lexer.has_errors());
    }

    #[test]
    fn hex_prefix_no_digits() {
        let k = kinds("0x");
        assert_eq!(k[0], TokenKind::Error);
    }

    // ── Spans ────────────────────────────────────────────────────────────

    #[test]
    fn span_keyword_and_ident() {
        let toks = tokens("fn main");
        assert_eq!(toks[0].span, Span::new(0, 0, 2)); // "fn"
        assert_eq!(toks[1].span, Span::new(0, 3, 7)); // "main"
    }

    #[test]
    fn span_string() {
        let toks = tokens(r#""hi""#);
        assert_eq!(toks[0].span, Span::new(0, 0, 4)); // including quotes
    }

    #[test]
    fn span_two_char_operator() {
        let toks = tokens("==");
        assert_eq!(toks[0].span, Span::new(0, 0, 2));
    }

    #[test]
    fn span_with_file_id() {
        let toks: Vec<Token> = Lexer::new(5, "x").collect();
        assert_eq!(toks[0].span, Span::new(5, 0, 1));
    }

    // ── tokenize convenience ─────────────────────────────────────────────

    #[test]
    fn tokenize_returns_diagnostics() {
        let (tokens, diags) = Lexer::new(0, r#""unterminated"#).tokenize();
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Error));
        assert!(!diags.is_empty());
    }

    #[test]
    fn tokenize_no_errors() {
        let (tokens, diags) = Lexer::new(0, "fn main() {}").tokenize();
        assert!(diags.is_empty());
        assert!(tokens.last().unwrap().kind == TokenKind::Eof);
    }

    // ── Helper methods ───────────────────────────────────────────────────

    #[test]
    fn is_keyword_check() {
        assert!(TokenKind::Fn.is_keyword());
        assert!(TokenKind::BoolLit(true).is_keyword());
        assert!(TokenKind::Trait.is_keyword());
        assert!(!TokenKind::Ident("foo".into()).is_keyword());
        assert!(!TokenKind::Plus.is_keyword());
    }

    #[test]
    fn precedence_ordering() {
        assert!(TokenKind::Star.precedence() > TokenKind::Plus.precedence());
        assert!(TokenKind::AmpAmp.precedence() > TokenKind::PipePipe.precedence());
        assert_eq!(TokenKind::Ident("x".into()).precedence(), None);
    }

    // ── Iterator fused ───────────────────────────────────────────────────

    #[test]
    fn iterator_ends_after_eof() {
        let mut lexer = Lexer::new(0, "");
        assert_eq!(lexer.next().unwrap().kind, TokenKind::Eof);
        assert!(lexer.next().is_none());
        assert!(lexer.next().is_none());
    }

    // ── Combined / integration ───────────────────────────────────────────

    #[test]
    fn lex_function_definition() {
        let src = "fn add(a: i32, b: i32) -> i32 { return a + b; }";
        let k: Vec<TokenKind> = kinds(src)
            .into_iter()
            .filter(|t| !matches!(t, TokenKind::Eof | TokenKind::Newline))
            .collect();
        assert_eq!(
            k,
            vec![
                TokenKind::Fn,
                TokenKind::Ident("add".into()),
                TokenKind::LParen,
                TokenKind::Ident("a".into()),
                TokenKind::Colon,
                TokenKind::Ident("i32".into()),
                TokenKind::Comma,
                TokenKind::Ident("b".into()),
                TokenKind::Colon,
                TokenKind::Ident("i32".into()),
                TokenKind::RParen,
                TokenKind::Arrow,
                TokenKind::Ident("i32".into()),
                TokenKind::LBrace,
                TokenKind::Return,
                TokenKind::Ident("a".into()),
                TokenKind::Plus,
                TokenKind::Ident("b".into()),
                TokenKind::Semi,
                TokenKind::RBrace,
            ]
        );
    }

    #[test]
    fn lex_mixed_literals() {
        let src = r#"42 3.14 "hello" 'x' true 0xFF"#;
        let k: Vec<TokenKind> = kinds(src)
            .into_iter()
            .filter(|t| !matches!(t, TokenKind::Eof))
            .collect();
        assert_eq!(
            k,
            vec![
                TokenKind::IntLit(42),
                TokenKind::FloatLit(3.14),
                TokenKind::StringLit("hello".into()),
                TokenKind::CharLit('x'),
                TokenKind::BoolLit(true),
                TokenKind::IntLit(255),
            ]
        );
    }
}
