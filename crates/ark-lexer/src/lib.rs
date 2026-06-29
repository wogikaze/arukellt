//! Lexer for the Arukellt language (v0).
//!
//! Tokenizes UTF-8 source into a stream of [`Token`]s with accurate [`Span`] tracking.
//! Supports doc comments (`///`, `//!`), line comments (`//`), nested block comments
//! (`/* ... */`), and shebang lines.

mod keywords;
mod scan;
mod token;

pub use scan::Lexer;
pub use token::{FStringPart, Token, TokenKind};

use ark_diagnostics::Diagnostic;

/// Tokenize source fully, including the EOF token.
pub fn tokenize(file_id: u32, source: &str) -> (Vec<Token>, Vec<Diagnostic>) {
    Lexer::new(file_id, source).tokenize()
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use ark_diagnostics::Span;

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
            "async", "await", "dyn", "where", "type", "const", "unsafe", "extern", "mod", "super",
            "Self",
        ];
        for kw in reserved {
            assert_eq!(
                kinds(kw),
                vec![TokenKind::Reserved(kw), TokenKind::Eof],
                "failed for reserved keyword: {kw}"
            );
        }
        // trait, impl, use are first-class keywords, not reserved
        assert_eq!(kinds("trait"), vec![TokenKind::Trait, TokenKind::Eof]);
        assert_eq!(kinds("impl"), vec![TokenKind::Impl, TokenKind::Eof]);
        assert_eq!(kinds("use"), vec![TokenKind::Use, TokenKind::Eof]);
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
    #[allow(clippy::approx_constant)]
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
    fn outer_doc_comment() {
        assert_eq!(
            kinds("/// Adds two values.\nfn add"),
            vec![
                TokenKind::OuterDocComment("Adds two values.".into()),
                TokenKind::Newline,
                TokenKind::Fn,
                TokenKind::Ident("add".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn inner_doc_comment() {
        assert_eq!(
            kinds("//! std::math helpers\nfn add"),
            vec![
                TokenKind::InnerDocComment("std::math helpers".into()),
                TokenKind::Newline,
                TokenKind::Fn,
                TokenKind::Ident("add".into()),
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
    #[allow(clippy::approx_constant)]
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
