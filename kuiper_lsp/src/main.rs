use std::collections::HashMap;

use kuiper_lang::{compile_expression, ExpressionType};
use tokio::sync::Mutex;
use tower_lsp::{
    lsp_types::{
        Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams, DidChangeWatchedFilesParams,
        DidCloseTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams,
        Documentation, Hover, HoverContents, HoverParams, HoverProviderCapability,
        InitializeParams, InitializeResult, InitializedParams, MarkupContent, MarkupKind,
        MessageType, Position, Range, ServerCapabilities, SignatureHelp, SignatureHelpOptions,
        SignatureHelpParams, SignatureInformation, TextDocumentSyncCapability,
        TextDocumentSyncKind, Url,
    },
    Client, LanguageServer, LspService, Server,
};

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend {
        client,
        state: Default::default(),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}

#[derive(Default)]
struct DocumentState {
    text: String,
    #[allow(unused)]
    expression: Option<ExpressionType>,
}

#[derive(Default)]
struct LanguageState {
    documents: Mutex<HashMap<Url, DocumentState>>,
}

fn compute_positions(start: usize, end: usize, text: &str) -> (Position, Position) {
    let mut start_pos: Option<Position> = None;
    let mut end_pos: Option<Position> = None;
    let mut line = 0;
    let mut pos = 0;
    let mut idx = 0;
    for b in text.chars() {
        if idx == start {
            start_pos = Some(Position {
                line,
                character: pos,
            });
        } else if idx == end {
            end_pos = Some(Position {
                line,
                character: pos,
            });
            break;
        }
        idx += b.len_utf8();
        if b == '\n' {
            line += 1;
            pos = 0;
        } else {
            pos += 1;
        }
    }
    (
        start_pos.unwrap_or_else(|| Position {
            line,
            character: pos,
        }),
        end_pos.unwrap_or_else(|| Position {
            line,
            character: pos,
        }),
    )
}

fn get_hover_symbol(position: Position, text: &str) -> Option<(Position, Position, &str)> {
    let line = text.lines().skip(position.line as usize).next()?;
    let chars: Vec<_> = line.char_indices().collect();
    let mut start = position.character as usize;
    let mut end = position.character as usize;
    if chars
        .get(start)
        .is_some_and(|(_, c)| !(c.is_alphanumeric() || matches!(c, '_')))
    {
        return None;
    }
    let mut start_pos = chars.get(start).map(|s| s.0)?;
    let mut end_pos = start_pos;

    loop {
        let mut iter = false;
        if start > 0 {
            if let Some((i, c)) = chars.get(start - 1) {
                if c.is_alphanumeric() || matches!(c, '_') {
                    start_pos = *i;
                    start -= 1;
                    iter = true;
                }
            }
        }

        if let Some((i, c)) = chars.get(end + 1) {
            if c.is_alphanumeric() || matches!(c, '_') {
                end_pos = *i;
                end += 1;
                iter = true;
            }
        }
        if !iter {
            break;
        }
    }

    if start_pos != end_pos {
        Some((
            Position {
                line: position.line,
                character: start as u32,
            },
            Position {
                line: position.line,
                character: end as u32,
            },
            &line[start_pos..=end_pos],
        ))
    } else {
        None
    }
}

impl LanguageState {
    pub async fn apply_edit(&self, edit: DidChangeTextDocumentParams, client: &Client) {
        let mut docs = self.documents.lock().await;
        let doc = docs.entry(edit.text_document.uri.clone()).or_default();
        for edit in edit.content_changes {
            if let (Some(_range), Some(_range_length)) = (edit.range, edit.range_length) {
                client
                    .log_message(
                        MessageType::WARNING,
                        "Got partial diff, not sure what to do with this",
                    )
                    .await;
            } else {
                client
                    .log_message(
                        MessageType::INFO,
                        format!("Got diff! New text: {}", edit.text),
                    )
                    .await;
                doc.text = edit.text;
            }
        }
        self.run_diagnostic(&edit.text_document.uri, &doc.text, client)
            .await;
    }

    pub async fn open_doc(&self, edit: DidOpenTextDocumentParams, client: &Client) {
        let mut docs = self.documents.lock().await;
        let doc = docs.entry(edit.text_document.uri.clone()).or_default();
        doc.text = edit.text_document.text;
        self.run_diagnostic(&edit.text_document.uri, &doc.text, client)
            .await;
    }

    async fn run_diagnostic(&self, url: &Url, text: &str, client: &Client) {
        if let Err(e) = compile_expression(text, &["input", "context"]) {
            let range = match e.span() {
                Some(s) => {
                    let (start, end) = compute_positions(s.start, s.end, text);
                    Range { start, end }
                }
                None => {
                    let (start, end) = compute_positions(0, text.len(), text);
                    Range { start, end }
                }
            };

            client
                .publish_diagnostics(
                    url.clone(),
                    vec![Diagnostic {
                        range,
                        severity: Some(DiagnosticSeverity::ERROR),
                        code: None,
                        code_description: None,
                        source: Some("kuiper_lang".to_string()),
                        message: e.to_string(),
                        related_information: None,
                        tags: None,
                        data: None,
                    }],
                    None,
                )
                .await;
        } else {
            client
                .publish_diagnostics(url.clone(), Vec::new(), None)
                .await;
        }
    }

    async fn get_hover_tooltip(&self, params: HoverParams, _client: &Client) -> Option<Hover> {
        let docs = self.documents.lock().await;
        let doc = docs.get(&params.text_document_position_params.text_document.uri)?;

        let (start, end, symbol) =
            get_hover_symbol(params.text_document_position_params.position, &doc.text)?;

        let doc = kuiper_lang::get_method_docs(&symbol)?;
        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!(
                    r#"`{}`: {}

Examples:

{}"#,
                    doc.signature(),
                    doc.documentation(),
                    doc.examples()
                        .iter()
                        .map(|e| format!("`{e}`"))
                        .collect::<Vec<_>>()
                        .join("\n\n")
                ),
            }),
            range: Some(Range { start, end }),
        })
    }

    async fn get_signature_help(
        &self,
        params: SignatureHelpParams,
        client: &Client,
    ) -> Option<SignatureHelp> {
        let docs = self.documents.lock().await;
        let doc = docs.get(&params.text_document_position_params.text_document.uri)?;

        let position = Position {
            line: params.text_document_position_params.position.line,
            character: params
                .text_document_position_params
                .position
                .character
                .saturating_sub(2),
        };

        client
            .log_message(
                MessageType::INFO,
                format!("Get signature help {:?}", position),
            )
            .await;

        let (_, _, symbol) = get_hover_symbol(position, &doc.text)?;

        client
            .log_message(MessageType::INFO, format!("Get symbol {symbol}"))
            .await;

        let doc = kuiper_lang::get_method_docs(&symbol)?;

        Some(SignatureHelp {
            signatures: vec![SignatureInformation {
                label: doc.signature().to_string(),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: format!(
                        r#"{}
    
    Examples:
    
    {}"#,
                        doc.documentation(),
                        doc.examples()
                            .iter()
                            .map(|e| format!("`{e}`"))
                            .collect::<Vec<_>>()
                            .join("\n\n")
                    ),
                })),
                parameters: None,
                active_parameter: None,
            }],
            active_parameter: None,
            active_signature: None,
        })
    }
}

struct Backend {
    client: Client,
    state: LanguageState,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(
        &self,
        _: InitializeParams,
    ) -> tower_lsp::jsonrpc::Result<InitializeResult> {
        self.client
            .log_message(MessageType::INFO, "server initializing!")
            .await;
        Ok(InitializeResult {
            server_info: None,
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                signature_help_provider: Some(SignatureHelpOptions {
                    trigger_characters: Some(vec!["(".to_owned()]),
                    ..Default::default()
                }),
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "server initialized!")
            .await;
    }

    async fn did_change_watched_files(&self, _: DidChangeWatchedFilesParams) {
        self.client
            .log_message(MessageType::INFO, "watched files have changed!")
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "Did change!")
            .await;
        self.state.apply_edit(params, &self.client).await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "Did open!")
            .await;
        self.state.open_doc(params, &self.client).await;
    }

    async fn did_save(&self, _: DidSaveTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "file saved!")
            .await;
    }

    async fn did_close(&self, _: DidCloseTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "file closed!")
            .await;
    }

    async fn shutdown(&self) -> tower_lsp::jsonrpc::Result<()> {
        Ok(())
    }

    async fn hover(&self, params: HoverParams) -> tower_lsp::jsonrpc::Result<Option<Hover>> {
        let hover = self.state.get_hover_tooltip(params, &self.client).await;

        Ok(hover)
    }

    async fn signature_help(
        &self,
        params: SignatureHelpParams,
    ) -> tower_lsp::jsonrpc::Result<Option<SignatureHelp>> {
        Ok(self.state.get_signature_help(params, &self.client).await)
    }
}
