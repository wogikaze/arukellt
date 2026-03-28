//! Keyword lookup table for the Arukellt lexer.

use crate::token::TokenKind;

/// Map an identifier string to its keyword `TokenKind`, if any.
///
/// Returns `None` for ordinary identifiers (including `"self"`).
pub(crate) fn lookup_keyword(text: &str) -> Option<TokenKind> {
    match text {
        "fn" => Some(TokenKind::Fn),
        "struct" => Some(TokenKind::Struct),
        "enum" => Some(TokenKind::Enum),
        "let" => Some(TokenKind::Let),
        "mut" => Some(TokenKind::Mut),
        "if" => Some(TokenKind::If),
        "else" => Some(TokenKind::Else),
        "match" => Some(TokenKind::Match),
        "while" => Some(TokenKind::While),
        "loop" => Some(TokenKind::Loop),
        "for" => Some(TokenKind::For),
        "in" => Some(TokenKind::In),
        "break" => Some(TokenKind::Break),
        "continue" => Some(TokenKind::Continue),
        "return" => Some(TokenKind::Return),
        "pub" => Some(TokenKind::Pub),
        "import" => Some(TokenKind::Import),
        "as" => Some(TokenKind::As),
        "trait" => Some(TokenKind::Trait),
        "impl" => Some(TokenKind::Impl),
        "use" => Some(TokenKind::Use),
        "true" => Some(TokenKind::BoolLit(true)),
        "false" => Some(TokenKind::BoolLit(false)),
        "async" | "await" | "dyn" | "where" | "type" | "const" | "unsafe" | "extern" | "mod"
        | "super" | "Self" => Some(TokenKind::Reserved(text.to_owned())),
        _ => None,
    }
}
