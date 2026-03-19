use crate::diagnostics::{Diagnostic, DiagnosticLevel, DiagnosticStage, Span};

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    Pub,
    Fn,
    If,
    Else,
    Let,
    And,
    Or,
    Import,
    Capability,
    Type,
    Match,
    True,
    False,
    Ident(String),
    Int(i64),
    String(String),
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Colon,
    Dot,
    DotDotEq,
    Equal,
    Arrow,
    Pipe,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Greater,
    Less,
    EqualEqual,
    Indent,
    Dedent,
    Newline,
    Eof,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LexOutput {
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<Diagnostic>,
}

#[must_use]
pub fn lex(source: &str) -> LexOutput {
    let mut diagnostics = Vec::new();
    let mut tokens = Vec::new();
    let mut indent_stack = vec![0usize];
    let mut offset = 0usize;

    for raw_line in source.split_inclusive('\n') {
        let line = raw_line.trim_end_matches('\n');
        let line_start = offset;
        offset += raw_line.len();

        if line.trim().is_empty() {
            continue;
        }

        // Skip comment lines
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") || trimmed.starts_with('#') {
            continue;
        }

        let indent_spaces = line.chars().take_while(|ch| *ch == ' ').count();
        let trimmed = &line[indent_spaces..];
        let is_continuation = trimmed.starts_with('.') || trimmed.starts_with("|>");
        if indent_spaces % 2 != 0 {
            diagnostics.push(Diagnostic {
                code: "E_ODD_INDENT".to_owned(),
                message: "Indentation must use multiples of two spaces".to_owned(),
                level: DiagnosticLevel::Error,
                stage: DiagnosticStage::Lexer,
                range: Span {
                    start: line_start,
                    end: line_start + indent_spaces,
                },
                expected: "indent of 0, 2, 4, ... spaces".to_owned(),
                actual: indent_spaces.to_string(),
                cause: "invalid_indent_width".to_owned(),
                related: Vec::new(),
                suggested_fix: "Adjust indentation to a multiple of two spaces.".to_owned(),
                alternatives: vec!["Use two spaces per indent level.".to_owned()],
                confidence: 0.99,
            });
        }

        let indent = indent_spaces / 2;
        let current_indent = *indent_stack.last().expect("indent stack");
        if !is_continuation && indent > current_indent {
            for level in current_indent + 1..=indent {
                indent_stack.push(level);
                tokens.push(Token {
                    kind: TokenKind::Indent,
                    span: Span {
                        start: line_start,
                        end: line_start + indent_spaces,
                    },
                });
            }
        } else if !is_continuation && indent < current_indent {
            while indent < *indent_stack.last().expect("indent stack") {
                indent_stack.pop();
                tokens.push(Token {
                    kind: TokenKind::Dedent,
                    span: Span {
                        start: line_start,
                        end: line_start + indent_spaces,
                    },
                });
            }
        }

        let mut column = indent_spaces;
        while column < line.len() {
            let ch = line.as_bytes()[column] as char;
            if ch == ' ' {
                column += 1;
                continue;
            }

            // Skip inline comments
            if column + 1 < line.len()
                && ch == '/'
                && line.as_bytes()[column + 1] as char == '/'
            {
                break;
            }

            let token_start = line_start + column;
            let remaining = &line[column..];
            let push = |kind: TokenKind, width: usize, tokens: &mut Vec<Token>| {
                tokens.push(Token {
                    kind,
                    span: Span {
                        start: token_start,
                        end: token_start + width,
                    },
                });
            };

            // Multi-character tokens first
            if remaining.starts_with("..=") {
                push(TokenKind::DotDotEq, 3, &mut tokens);
                column += 3;
                continue;
            }
            if remaining.starts_with("->") {
                push(TokenKind::Arrow, 2, &mut tokens);
                column += 2;
                continue;
            }
            if remaining.starts_with("==") {
                push(TokenKind::EqualEqual, 2, &mut tokens);
                column += 2;
                continue;
            }
            if remaining.starts_with("|>") {
                push(TokenKind::Pipe, 2, &mut tokens);
                column += 2;
                continue;
            }

            match ch {
                '(' => {
                    push(TokenKind::LParen, 1, &mut tokens);
                    column += 1;
                }
                ')' => {
                    push(TokenKind::RParen, 1, &mut tokens);
                    column += 1;
                }
                '[' => {
                    push(TokenKind::LBracket, 1, &mut tokens);
                    column += 1;
                }
                ']' => {
                    push(TokenKind::RBracket, 1, &mut tokens);
                    column += 1;
                }
                ',' => {
                    push(TokenKind::Comma, 1, &mut tokens);
                    column += 1;
                }
                ':' => {
                    push(TokenKind::Colon, 1, &mut tokens);
                    column += 1;
                }
                '.' => {
                    push(TokenKind::Dot, 1, &mut tokens);
                    column += 1;
                }
                '=' => {
                    push(TokenKind::Equal, 1, &mut tokens);
                    column += 1;
                }
                '+' => {
                    push(TokenKind::Plus, 1, &mut tokens);
                    column += 1;
                }
                '-' => {
                    push(TokenKind::Minus, 1, &mut tokens);
                    column += 1;
                }
                '*' => {
                    push(TokenKind::Star, 1, &mut tokens);
                    column += 1;
                }
                '/' => {
                    push(TokenKind::Slash, 1, &mut tokens);
                    column += 1;
                }
                '%' => {
                    push(TokenKind::Percent, 1, &mut tokens);
                    column += 1;
                }
                '>' => {
                    push(TokenKind::Greater, 1, &mut tokens);
                    column += 1;
                }
                '<' => {
                    push(TokenKind::Less, 1, &mut tokens);
                    column += 1;
                }
                '"' => {
                    let mut end = column + 1;
                    while end < line.len() && line.as_bytes()[end] as char != '"' {
                        end += 1;
                    }
                    let content = if end < line.len() {
                        unescape_string_literal(&line[column + 1..end])
                    } else {
                        unescape_string_literal(&line[column + 1..])
                    };
                    push(
                        TokenKind::String(content),
                        end.saturating_sub(column) + 1,
                        &mut tokens,
                    );
                    column = if end < line.len() {
                        end + 1
                    } else {
                        line.len()
                    };
                }
                _ if ch.is_ascii_digit() => {
                    let mut end = column + 1;
                    while end < line.len() && (line.as_bytes()[end] as char).is_ascii_digit() {
                        end += 1;
                    }
                    let value = line[column..end].parse().unwrap_or_default();
                    push(TokenKind::Int(value), end - column, &mut tokens);
                    column = end;
                }
                _ if ch.is_ascii_alphabetic() || ch == '_' => {
                    let mut end = column + 1;
                    while end < line.len() {
                        let next = line.as_bytes()[end] as char;
                        if next.is_ascii_alphanumeric() || next == '_' {
                            end += 1;
                        } else {
                            break;
                        }
                    }
                    let ident = &line[column..end];
                    let kind = match ident {
                        "pub" => TokenKind::Pub,
                        "fn" => TokenKind::Fn,
                        "if" => TokenKind::If,
                        "else" => TokenKind::Else,
                        "let" => TokenKind::Let,
                        "and" => TokenKind::And,
                        "or" => TokenKind::Or,
                        "import" => TokenKind::Import,
                        "capability" => TokenKind::Capability,
                        "type" => TokenKind::Type,
                        "match" => TokenKind::Match,
                        "true" => TokenKind::True,
                        "false" => TokenKind::False,
                        _ => TokenKind::Ident(ident.to_owned()),
                    };
                    push(kind, end - column, &mut tokens);
                    column = end;
                }
                _ => {
                    diagnostics.push(Diagnostic {
                        code: "E_UNKNOWN_TOKEN".to_owned(),
                        message: format!("Unknown token `{ch}`"),
                        level: DiagnosticLevel::Error,
                        stage: DiagnosticStage::Lexer,
                        range: Span {
                            start: token_start,
                            end: token_start + 1,
                        },
                        expected: "language token".to_owned(),
                        actual: ch.to_string(),
                        cause: "unknown_token".to_owned(),
                        related: Vec::new(),
                        suggested_fix: "Remove or replace the unknown token.".to_owned(),
                        alternatives: vec![
                            "Check whether a symbol is misspelled.".to_owned(),
                            "Use one of the supported operators.".to_owned(),
                        ],
                        confidence: 0.95,
                    });
                    column += 1;
                }
            }
        }

        tokens.push(Token {
            kind: TokenKind::Newline,
            span: Span {
                start: line_start + line.len(),
                end: line_start + line.len(),
            },
        });
    }

    while indent_stack.len() > 1 {
        indent_stack.pop();
        tokens.push(Token {
            kind: TokenKind::Dedent,
            span: Span {
                start: offset,
                end: offset,
            },
        });
    }

    tokens.push(Token {
        kind: TokenKind::Eof,
        span: Span {
            start: offset,
            end: offset,
        },
    });

    LexOutput {
        tokens,
        diagnostics,
    }
}

fn unescape_string_literal(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(ch);
        }
    }
    out
}
