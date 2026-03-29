//! Lexer / scanner implementation for Arukellt source code.

use ark_diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSink, Span};

use crate::keywords::lookup_keyword;
use crate::token::{FStringPart, Token, TokenKind};

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
            b'/' if matches!(self.peek_at(1), Some(b'/'))
                && matches!(self.peek_at(2), Some(b'/' | b'!')) =>
            {
                self.lex_doc_comment(start)
            }
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
                        if matches!(self.peek_at(2), Some(b'/' | b'!')) {
                            break;
                        }
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

    fn lex_doc_comment(&mut self, start: usize) -> Token {
        self.pos += 2; // `//`
        let kind = match self.advance() {
            b'/' => "outer",
            b'!' => "inner",
            _ => unreachable!("lex_doc_comment called on a non-doc comment"),
        };
        let comment_start = self.pos;
        while let Some(b) = self.peek() {
            if b == b'\n' || b == b'\r' {
                break;
            }
            self.pos += 1;
        }
        let text = self.source[comment_start..self.pos]
            .strip_prefix(' ')
            .unwrap_or(&self.source[comment_start..self.pos])
            .to_string();
        let span = self.span(start);
        match kind {
            "outer" => Token::new(TokenKind::OuterDocComment(text), span),
            "inner" => Token::new(TokenKind::InnerDocComment(text), span),
            _ => unreachable!(),
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
        if let Some((next2, kind2)) = alt
            && self.peek() == Some(next2)
        {
            self.advance();
            return Token::new(kind2, self.span(start));
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

        if let Some(kind) = lookup_keyword(text) {
            return Token::new(kind, span);
        }

        // f"..." string interpolation
        if text == "f" && self.peek() == Some(b'"') {
            return self.lex_fstring(start);
        }

        Token::new(TokenKind::Ident(text.to_owned()), span)
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
        let span_before_suffix = self.span(start);
        let clean: String = text.chars().filter(|&c| c != '_').collect();

        // Check for type suffix: u8, u16, u32, u64, i8, i16, i32, i64, f32, f64
        let suffix = self.try_eat_type_suffix();

        let span = self.span(start);

        if is_float {
            match clean.parse::<f64>() {
                Ok(v) => {
                    if let Some(s) = suffix {
                        Token::new(TokenKind::TypedFloatLit(v, s), span)
                    } else {
                        Token::new(TokenKind::FloatLit(v), span)
                    }
                }
                Err(_) => {
                    self.emit(span, DiagnosticCode::E0003, "invalid float literal");
                    Token::new(TokenKind::Error, span)
                }
            }
        } else {
            match clean.parse::<i64>() {
                Ok(v) => {
                    if let Some(ref s) = suffix {
                        if s == "f32" || s == "f64" {
                            // Integer with float suffix: treat as float
                            Token::new(TokenKind::TypedFloatLit(v as f64, s.clone()), span)
                        } else {
                            Token::new(TokenKind::TypedIntLit(v, s.clone()), span)
                        }
                    } else {
                        Token::new(TokenKind::IntLit(v), span)
                    }
                }
                Err(_) => {
                    self.emit(
                        span_before_suffix,
                        DiagnosticCode::E0003,
                        "invalid integer literal",
                    );
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
            Ok(v) => {
                let suffix = self.try_eat_type_suffix();
                let span = self.span(start);
                if let Some(s) = suffix {
                    Token::new(TokenKind::TypedIntLit(v, s), span)
                } else {
                    Token::new(TokenKind::IntLit(v), span)
                }
            }
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

    /// Try to consume a type suffix after a numeric literal.
    /// Recognized suffixes: u8, u16, u32, u64, i8, i16, i32, i64, f32, f64
    fn try_eat_type_suffix(&mut self) -> Option<String> {
        let remaining = &self.source[self.pos..];
        let suffixes = [
            "u64", "u32", "u16", "u8", "i64", "i32", "i16", "i8", "f64", "f32",
        ];
        for suffix in &suffixes {
            if remaining.starts_with(suffix) {
                // Make sure the suffix is not followed by an alphanumeric char
                // (to avoid matching `u8x` as suffix `u8` + ident `x`)
                let after = self.pos + suffix.len();
                if after < self.source.len()
                    && (self.bytes[after].is_ascii_alphanumeric() || self.bytes[after] == b'_')
                {
                    continue;
                }
                self.pos += suffix.len();
                return Some(suffix.to_string());
            }
        }
        None
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
