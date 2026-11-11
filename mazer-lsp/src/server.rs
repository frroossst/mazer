//! The protocol loop: lifecycle, document sync and request dispatch.
//!
//! Single-threaded and synchronous. Each message is handled to completion
//! before the next is read. Every handler runs under `catch_unwind`, so a bug
//! answers one request with an error instead of killing the editor's
//! connection; malformed parameters get `InvalidParams`, unknown requests
//! `MethodNotFound`, unknown notifications are ignored, and positions outside
//! the note clamp.

use std::collections::HashMap;
use std::error::Error;
use std::panic::{AssertUnwindSafe, catch_unwind};

use lsp_server::{Connection, ErrorCode, Message, Notification, ProtocolError, Request, RequestId, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification as _,
    PublishDiagnostics, ShowMessage,
};
use lsp_types::request::{
    Completion, DocumentSymbolRequest, ExecuteCommand, GotoDefinition, HoverRequest, References, Request as _,
    SemanticTokensFullRequest, SignatureHelpRequest,
};
use lsp_types::{
    CompletionOptions, CompletionParams, CompletionResponse, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DocumentSymbolParams, DocumentSymbolResponse, ExecuteCommandOptions,
    ExecuteCommandParams, GotoDefinitionParams, GotoDefinitionResponse, HoverParams, HoverProviderCapability,
    InitializeParams, MessageType, OneOf, Position, PositionEncodingKind, PublishDiagnosticsParams, ReferenceParams,
    SemanticTokensFullOptions, SemanticTokensOptions, SemanticTokensParams, SemanticTokensResult,
    SemanticTokensServerCapabilities, ServerCapabilities, ServerInfo, ShowMessageParams, SignatureHelpOptions,
    SignatureHelpParams, TextDocumentContentChangeEvent, TextDocumentSyncCapability, TextDocumentSyncKind,
    TextDocumentSyncOptions, TextDocumentSyncSaveOptions, Uri,
};
use mazer_doc::{Analysis, ByteOffset, Diag, Page, SourceMap, analyze};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::preview::Preview;
use crate::{completion, convert, diagnostics, hover, navigation, semantic, signature, symbols};

/// The one command the server offers.
pub const PREVIEW_COMMAND: &str = "mazer.preview";

/// Runs the server over stdin/stdout until `exit`. Returns whether the
/// client shut it down properly (`shutdown` before `exit`).
///
/// # Errors
/// If the connection breaks during initialization or the I/O threads fail.
pub fn run_stdio() -> Result<bool, Box<dyn Error + Send + Sync>> {
    let (connection, io_threads) = Connection::stdio();
    let clean = run(&connection)?;
    drop(connection);
    io_threads.join()?;
    Ok(clean)
}

/// The capabilities the server implements, exactly.
#[must_use]
pub fn capabilities() -> ServerCapabilities {
    ServerCapabilities {
        position_encoding: Some(PositionEncodingKind::UTF16),
        text_document_sync: Some(TextDocumentSyncCapability::Options(TextDocumentSyncOptions {
            open_close: Some(true),
            change: Some(TextDocumentSyncKind::FULL),
            will_save: None,
            will_save_wait_until: None,
            save: Some(TextDocumentSyncSaveOptions::Supported(true)),
        })),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(["(", ":", "'", " "].map(str::to_owned).to_vec()),
            resolve_provider: Some(false),
            ..CompletionOptions::default()
        }),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        signature_help_provider: Some(SignatureHelpOptions {
            trigger_characters: Some([" ", "("].map(str::to_owned).to_vec()),
            retrigger_characters: None,
            work_done_progress_options: lsp_types::WorkDoneProgressOptions::default(),
        }),
        definition_provider: Some(OneOf::Left(true)),
        references_provider: Some(OneOf::Left(true)),
        document_symbol_provider: Some(OneOf::Left(true)),
        semantic_tokens_provider: Some(SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
            legend: semantic::legend(),
            full: Some(SemanticTokensFullOptions::Bool(true)),
            range: None,
            work_done_progress_options: lsp_types::WorkDoneProgressOptions::default(),
        })),
        execute_command_provider: Some(ExecuteCommandOptions {
            commands: vec![PREVIEW_COMMAND.to_owned()],
            work_done_progress_options: lsp_types::WorkDoneProgressOptions::default(),
        }),
        ..ServerCapabilities::default()
    }
}

/// Initializes over `connection` and serves until `exit` (or until the
/// client goes away). Returns whether `shutdown` came before `exit`.
///
/// # Errors
/// If the connection breaks during initialization.
pub fn run(connection: &Connection) -> Result<bool, ProtocolError> {
    let (id, params) = connection.initialize_start()?;
    let params: InitializeParams = serde_json::from_value(params).unwrap_or_default();
    let snippets = params
        .capabilities
        .text_document
        .as_ref()
        .and_then(|t| t.completion.as_ref())
        .and_then(|c| c.completion_item.as_ref())
        .and_then(|i| i.snippet_support)
        .unwrap_or(false);
    let result = serde_json::json!({
        "capabilities": capabilities(),
        "serverInfo": ServerInfo { name: "mazer-lsp".to_owned(), version: Some(env!("CARGO_PKG_VERSION").to_owned()) },
    });
    connection.initialize_finish(id, result)?;

    let mut server = Server {
        connection,
        notes: HashMap::new(),
        completion: completion::Config { snippets },
        preview: None,
        preview_ids: HashMap::new(),
    };
    let clean = server.main_loop();
    if let Some(preview) = server.preview.take() {
        preview.stop();
    }
    Ok(clean)
}

/// One open note.
pub struct OpenNote {
    pub version: i32,
    pub analysis: Analysis,
    /// Problems only rendering finds.
    pub render_diags: Vec<Diag>,
}

struct Server<'c> {
    connection: &'c Connection,
    notes: HashMap<Uri, OpenNote>,
    completion: completion::Config,
    preview: Option<Preview>,
    /// The preview page id of each note that has been previewed.
    preview_ids: HashMap<Uri, u32>,
}

/// A handler's failure: an LSP error code and message.
type Failure = (ErrorCode, String);

impl Server<'_> {
    fn main_loop(&mut self) -> bool {
        let mut shut_down = false;
        while let Ok(message) = self.connection.receiver.recv() {
            match message {
                Message::Request(req) => {
                    let id = req.id.clone();
                    if shut_down {
                        self.reply_err(id, ErrorCode::InvalidRequest, "the server is shutting down".to_owned());
                        continue;
                    }
                    if req.method == "shutdown" {
                        shut_down = true;
                        self.send(Response::new_ok(id, ()).into());
                        continue;
                    }
                    let method = req.method.clone();
                    let response = match catch_unwind(AssertUnwindSafe(|| self.request(req))) {
                        Ok(Ok(value)) => Response::new_ok(id, value),
                        Ok(Err((code, message))) => Response::new_err(id, code as i32, message),
                        Err(_) => Response::new_err(
                            id,
                            ErrorCode::InternalError as i32,
                            format!("mazer-lsp hit a bug while answering {method}"),
                        ),
                    };
                    self.send(response.into());
                }
                Message::Notification(note) => {
                    if note.method == "exit" {
                        return shut_down;
                    }
                    let method = note.method.clone();
                    if catch_unwind(AssertUnwindSafe(|| self.notification(note))).is_err() {
                        eprintln!("mazer-lsp: a bug while handling {method}");
                    }
                }
                Message::Response(_) => {}
            }
        }
        false
    }

    fn send(&self, message: Message) {
        let _ = self.connection.sender.send(message);
    }

    fn reply_err(&self, id: RequestId, code: ErrorCode, message: String) {
        self.send(Response::new_err(id, code as i32, message).into());
    }

    fn notify<N: lsp_types::notification::Notification>(&self, params: N::Params) {
        self.send(Notification::new(N::METHOD.to_owned(), params).into());
    }

    // ---- notifications ----

    fn notification(&mut self, note: Notification) {
        match note.method.as_str() {
            DidOpenTextDocument::METHOD => {
                if let Ok(p) = serde_json::from_value::<DidOpenTextDocumentParams>(note.params) {
                    let doc = p.text_document;
                    self.update(doc.uri, doc.version, &doc.text);
                }
            }
            DidChangeTextDocument::METHOD => {
                if let Ok(p) = serde_json::from_value::<DidChangeTextDocumentParams>(note.params) {
                    let uri = p.text_document.uri;
                    let Some(text) = self.notes.get(&uri).map(|n| current_text(&n.analysis)) else {
                        return;
                    };
                    let text = apply_changes(text, p.content_changes);
                    self.update(uri, p.text_document.version, &text);
                }
            }
            DidCloseTextDocument::METHOD => {
                if let Ok(p) = serde_json::from_value::<DidCloseTextDocumentParams>(note.params) {
                    let uri = p.text_document.uri;
                    self.notes.remove(&uri);
                    self.notify::<PublishDiagnostics>(PublishDiagnosticsParams::new(uri, Vec::new(), None));
                }
            }
            // didSave: the text is already known from didOpen / didChange.
            // Anything else is ignored.
            _ => {}
        }
    }

    /// Re-analyzes a note from its full text and publishes its diagnostics
    /// (and refreshes its preview, if it has one).
    fn update(&mut self, uri: Uri, version: i32, text: &str) {
        let name = convert::note_name(&uri);
        let analysis = analyze(text, &name);
        let previewed = self.preview_ids.get(&uri).copied();
        let page = match previewed {
            Some(_) => Page::Full {
                title: convert::note_title(&uri),
            },
            None => Page::Fragment,
        };
        let (html, render_diags) = diagnostics::render_problems(&analysis, &name, page);
        if let (Some(id), Some(preview)) = (previewed, &self.preview) {
            preview.update(id, &convert::note_title(&uri), html);
        }
        let diags = diagnostics::collect(&analysis, &uri, &render_diags);
        self.notify::<PublishDiagnostics>(PublishDiagnosticsParams::new(uri.clone(), diags, Some(version)));
        self.notes.insert(
            uri,
            OpenNote {
                version,
                analysis,
                render_diags,
            },
        );
    }

    // ---- requests ----

    fn request(&mut self, req: Request) -> Result<Value, Failure> {
        let params = req.params;
        match req.method.as_str() {
            Completion::METHOD => {
                let p: CompletionParams = parse(params)?;
                let pos = p.text_document_position;
                let items = self.with_note(&pos.text_document.uri, pos.position, |a, at, config| {
                    completion::complete(a, at, config)
                });
                to_value(items.map(CompletionResponse::Array))
            }
            HoverRequest::METHOD => {
                let p: HoverParams = parse(params)?;
                let pos = p.text_document_position_params;
                to_value(self.with_note(&pos.text_document.uri, pos.position, |a, at, _| hover::hover(a, at)).flatten())
            }
            SignatureHelpRequest::METHOD => {
                let p: SignatureHelpParams = parse(params)?;
                let pos = p.text_document_position_params;
                to_value(
                    self.with_note(&pos.text_document.uri, pos.position, |a, at, _| signature::signature_help(a, at))
                        .flatten(),
                )
            }
            GotoDefinition::METHOD => {
                let p: GotoDefinitionParams = parse(params)?;
                let pos = p.text_document_position_params;
                let uri = pos.text_document.uri.clone();
                let found = self
                    .with_note(&uri, pos.position, |a, at, _| navigation::definition(a, &uri, at))
                    .flatten();
                to_value(found.map(GotoDefinitionResponse::Scalar))
            }
            References::METHOD => {
                let p: ReferenceParams = parse(params)?;
                let pos = p.text_document_position;
                let uri = pos.text_document.uri.clone();
                let decl = p.context.include_declaration;
                to_value(self.with_note(&uri, pos.position, |a, at, _| navigation::references(a, &uri, at, decl)))
            }
            DocumentSymbolRequest::METHOD => {
                let p: DocumentSymbolParams = parse(params)?;
                let found = self.notes.get(&p.text_document.uri).map(|n| symbols::document_symbols(&n.analysis));
                to_value(found.map(DocumentSymbolResponse::Nested))
            }
            SemanticTokensFullRequest::METHOD => {
                let p: SemanticTokensParams = parse(params)?;
                let found = self.notes.get(&p.text_document.uri).map(|n| semantic::semantic_tokens(&n.analysis));
                to_value(found.map(SemanticTokensResult::Tokens))
            }
            ExecuteCommand::METHOD => {
                let p: ExecuteCommandParams = parse(params)?;
                self.execute(&p)
            }
            other => Err((ErrorCode::MethodNotFound, format!("mazer-lsp does not handle {other}"))),
        }
    }

    /// Runs `f` on the analysis of an open note at a position; `None` if the
    /// note is not open.
    fn with_note<T>(
        &self,
        uri: &Uri,
        pos: Position,
        f: impl FnOnce(&Analysis, ByteOffset, completion::Config) -> T,
    ) -> Option<T> {
        let note = self.notes.get(uri)?;
        let at = convert::offset_at(&note.analysis.sources, pos);
        Some(f(&note.analysis, at, self.completion))
    }

    fn execute(&mut self, p: &ExecuteCommandParams) -> Result<Value, Failure> {
        if p.command != PREVIEW_COMMAND {
            return Err((ErrorCode::InvalidParams, format!("unknown command {}", p.command)));
        }
        let uri: Uri = p
            .arguments
            .first()
            .and_then(Value::as_str)
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| (ErrorCode::InvalidParams, format!("{PREVIEW_COMMAND} takes the document URI")))?;
        let Some(note) = self.notes.get(&uri) else {
            return Err((ErrorCode::InvalidParams, format!("{} is not open", uri.as_str())));
        };
        if self.preview.is_none() {
            let started = Preview::start()
                .map_err(|e| (ErrorCode::RequestFailed, format!("cannot start the preview server: {e}")))?;
            self.preview = Some(started);
        }
        let next = u32::try_from(self.preview_ids.len() + 1).unwrap_or(u32::MAX);
        let id = *self.preview_ids.entry(uri.clone()).or_insert(next);
        let title = convert::note_title(&uri);
        let name = convert::note_name(&uri);
        let (html, _) = diagnostics::render_problems(&note.analysis, &name, Page::Full { title: title.clone() });
        let Some(preview) = &self.preview else {
            return Err((ErrorCode::InternalError, "no preview server".to_owned()));
        };
        preview.update(id, &title, html);
        let url = preview.url(id);
        self.notify::<ShowMessage>(ShowMessageParams {
            typ: MessageType::INFO,
            message: format!("mazer preview of {name}: {url}"),
        });
        Ok(Value::String(url))
    }
}

fn parse<P: DeserializeOwned>(params: Value) -> Result<P, Failure> {
    serde_json::from_value(params).map_err(|e| (ErrorCode::InvalidParams, format!("invalid parameters: {e}")))
}

fn to_value<T: serde::Serialize>(value: T) -> Result<Value, Failure> {
    serde_json::to_value(value).map_err(|e| (ErrorCode::InternalError, e.to_string()))
}

/// The current text of a note.
fn current_text(analysis: &Analysis) -> String {
    convert::note_text(&analysis.sources).to_owned()
}

/// Applies content changes in order.
///
/// A change without a range replaces the whole text (the sync kind the
/// server asks for); one with a range is applied there too, in case a client
/// sends incremental changes anyway.
#[must_use]
pub fn apply_changes(mut text: String, changes: Vec<TextDocumentContentChangeEvent>) -> String {
    for change in changes {
        match change.range {
            None => text = change.text,
            Some(range) => {
                let mut sources = SourceMap::new();
                sources.add(String::new(), std::mem::take(&mut text));
                let start = convert::offset_at(&sources, range.start).to_usize();
                let end = convert::offset_at(&sources, range.end).to_usize();
                convert::note_text(&sources).clone_into(&mut text);
                let (start, end) = (start.min(end), start.max(end));
                text.replace_range(start..end, &change.text);
            }
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use lsp_types::Range;

    use super::*;

    #[test]
    fn incremental_changes_apply_anyway() {
        let change = |range: Option<Range>, text: &str| TextDocumentContentChangeEvent {
            range,
            range_length: None,
            text: text.to_owned(),
        };
        let text = apply_changes(
            "τ (show x)".to_owned(),
            vec![change(Some(Range::new(Position::new(0, 8), Position::new(0, 9))), "y")],
        );
        assert_eq!(text, "τ (show y)");
        let text = apply_changes(text, vec![change(None, "new"), change(Some(Range::new(Position::new(9, 9), Position::new(0, 0))), "!")]);
        assert_eq!(text, "!", "a reversed range is the same range");
    }

    #[test]
    fn capabilities_are_utf16_and_full_sync() {
        let caps = serde_json::to_value(capabilities()).unwrap();
        assert_eq!(caps["positionEncoding"], "utf-16");
        assert_eq!(caps["textDocumentSync"]["change"], 1);
        assert_eq!(caps["executeCommandProvider"]["commands"][0], PREVIEW_COMMAND);
    }
}
