//! LSP server implementation.

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use ark_diagnostics::{DiagnosticSink, Severity};
use ark_lexer::Lexer;
use ark_parser::parse;
use std::collections::HashMap;
use std::sync::Mutex;

struct ArukellBackend {
    client: Client,
    documents: Mutex<HashMap<Url, String>>,
}

impl ArukellBackend {
    fn new(client: Client) -> Self {
        Self {
            client,
            documents: Mutex::new(HashMap::new()),
        }
    }

    async fn refresh_diagnostics(&self, uri: Url, text: &str) {
        let diagnostics = self.check_source(text);
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    fn offset_to_position(source: &str, offset: u32) -> Position {
        let offset = offset as usize;
        let mut line = 0u32;
        let mut col = 0u32;
        for (i, ch) in source.char_indices() {
            if i >= offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
        }
        Position {
            line,
            character: col,
        }
    }

    fn check_source(&self, source: &str) -> Vec<Diagnostic> {
        let mut lsp_diags = Vec::new();
        let mut sink = DiagnosticSink::new();

        // Lex
        let lexer = Lexer::new(0, source);
        let tokens: Vec<_> = lexer.collect();

        // Parse
        let module = parse(&tokens, &mut sink);

        // Collect diagnostics from parsing
        if sink.has_errors() {
            for diag in sink.diagnostics() {
                let range = if let Some(label) = diag.labels.first() {
                    Range {
                        start: Self::offset_to_position(source, label.span.start),
                        end: Self::offset_to_position(source, label.span.end),
                    }
                } else {
                    Range::default()
                };
                let severity = match diag.severity() {
                    Severity::Error => DiagnosticSeverity::ERROR,
                    Severity::Warning => DiagnosticSeverity::WARNING,
                    Severity::Help => DiagnosticSeverity::INFORMATION,
                };
                lsp_diags.push(Diagnostic {
                    range,
                    severity: Some(severity),
                    code: Some(NumberOrString::String(diag.code.as_str().to_string())),
                    source: Some("arukellt".to_string()),
                    message: diag.message.clone(),
                    ..Default::default()
                });
            }
            return lsp_diags;
        }

        // Name resolution
        let resolved = ark_resolve::resolve_module(module, &mut sink);

        // Type check
        let mut checker = ark_typecheck::TypeChecker::new();
        checker.register_builtins();
        checker.check_module(&resolved, &mut sink);

        // Collect all diagnostics
        for diag in sink.diagnostics() {
            let range = if let Some(label) = diag.labels.first() {
                Range {
                    start: Self::offset_to_position(source, label.span.start),
                    end: Self::offset_to_position(source, label.span.end),
                }
            } else {
                Range::default()
            };
            let severity = match diag.severity() {
                Severity::Error => DiagnosticSeverity::ERROR,
                Severity::Warning => DiagnosticSeverity::WARNING,
                Severity::Help => DiagnosticSeverity::INFORMATION,
            };
            lsp_diags.push(Diagnostic {
                range,
                severity: Some(severity),
                code: Some(NumberOrString::String(diag.code.as_str().to_string())),
                source: Some("arukellt".to_string()),
                message: diag.message.clone(),
                ..Default::default()
            });
        }

        lsp_diags
    }

    fn get_completions(&self, source: &str) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        // Built-in functions
        let builtins = [
            ("println", "Print a value followed by newline"),
            ("print", "Print a value"),
            ("len", "Get length of a Vec or String"),
            ("push", "Push element to Vec"),
            ("get", "Get element from Vec by index"),
            ("set", "Set element in Vec at index"),
            ("pop", "Remove last element from Vec"),
            ("i32_to_string", "Convert i32 to String"),
            ("i64_to_string", "Convert i64 to String"),
            ("f64_to_string", "Convert f64 to String"),
            ("bool_to_string", "Convert bool to String"),
            ("concat", "Concatenate two strings"),
            ("Vec_new_i32", "Create new Vec<i32>"),
            ("Vec_new_String", "Create new Vec<String>"),
            ("sort_i32", "Sort Vec<i32> in place"),
            ("assert", "Assert a boolean condition"),
            ("assert_eq", "Assert two values are equal"),
            ("parse_i32", "Parse string to i32"),
        ];

        for (name, detail) in &builtins {
            items.push(CompletionItem {
                label: name.to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some(detail.to_string()),
                ..Default::default()
            });
        }

        // Keywords
        let keywords = [
            "fn", "let", "mut", "if", "else", "while", "for", "in", "match", "return", "break",
            "continue", "true", "false", "struct", "enum", "trait", "impl", "type", "use", "mod",
        ];
        for kw in &keywords {
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                ..Default::default()
            });
        }

        // Types
        let types = [
            "i32", "i64", "f32", "f64", "bool", "char", "String", "Vec", "Option", "Result",
        ];
        for ty in &types {
            items.push(CompletionItem {
                label: ty.to_string(),
                kind: Some(CompletionItemKind::CLASS),
                ..Default::default()
            });
        }

        // Extract identifiers from source
        let lexer = Lexer::new(0, source);
        let tokens: Vec<_> = lexer.collect();
        let mut seen = std::collections::HashSet::new();
        for tok in &tokens {
            if let ark_lexer::TokenKind::Ident(_) = &tok.kind {
                let start = tok.span.start as usize;
                let end = tok.span.end as usize;
                if end <= source.len() {
                    let name = &source[start..end];
                    if !seen.contains(name) && !keywords.contains(&name) {
                        seen.insert(name.to_string());
                        items.push(CompletionItem {
                            label: name.to_string(),
                            kind: Some(CompletionItemKind::VARIABLE),
                            ..Default::default()
                        });
                    }
                }
            }
        }

        items
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for ArukellBackend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".to_string(), ":".to_string()]),
                    ..Default::default()
                }),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "arukellt-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "arukellt LSP server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let text = params.text_document.text.clone();
        {
            let mut docs = self.documents.lock().unwrap();
            docs.insert(uri.clone(), text.clone());
        }
        self.refresh_diagnostics(uri, &text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        if let Some(change) = params.content_changes.into_iter().last() {
            let text = change.text;
            {
                let mut docs = self.documents.lock().unwrap();
                docs.insert(uri.clone(), text.clone());
            }
            self.refresh_diagnostics(uri, &text).await;
        }
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;

        let docs = self.documents.lock().unwrap();
        let source = match docs.get(&uri) {
            Some(s) => s.clone(),
            None => return Ok(None),
        };
        drop(docs);

        // Find the token at the cursor position
        let lexer = Lexer::new(0, &source);
        let tokens: Vec<_> = lexer.collect();
        let lines: Vec<&str> = source.lines().collect();
        let target_offset = lines
            .iter()
            .take(pos.line as usize)
            .map(|l| l.len() + 1)
            .sum::<usize>()
            + pos.character as usize;

        for tok in &tokens {
            let start = tok.span.start as usize;
            let end = tok.span.end as usize;
            if start <= target_offset && target_offset <= end && end <= source.len() {
                let text = &source[start..end];
                let info = match &tok.kind {
                    ark_lexer::TokenKind::Ident(_) => {
                        format!("identifier `{}`", text)
                    }
                    ark_lexer::TokenKind::IntLit(_) => {
                        format!("integer literal `{}`", text)
                    }
                    ark_lexer::TokenKind::FloatLit(_) => {
                        format!("float literal `{}`", text)
                    }
                    ark_lexer::TokenKind::StringLit(_) => "string literal".to_string(),
                    _ => format!("`{}`", text),
                };
                return Ok(Some(Hover {
                    contents: HoverContents::Scalar(MarkedString::String(info)),
                    range: None,
                }));
            }
        }

        Ok(None)
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let docs = self.documents.lock().unwrap();
        let source = match docs.get(&uri) {
            Some(s) => s.clone(),
            None => return Ok(None),
        };
        drop(docs);

        let items = self.get_completions(&source);
        Ok(Some(CompletionResponse::Array(items)))
    }
}

/// Run the LSP server on stdio.
pub async fn run_lsp() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(ArukellBackend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
