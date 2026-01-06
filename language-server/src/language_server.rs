use std::{
    cell::OnceCell,
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
    sync::OnceLock,
};

use tokio::sync::RwLock;
use tower_lsp::{
    Client, LanguageServer as LanguageServerLike,
    lsp_types::{
        CompletionParams, CompletionResponse, Diagnostic as LspDiagnostic, DiagnosticSeverity,
        DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
        DidSaveTextDocumentParams, Hover, HoverContents, HoverParams, HoverProviderCapability,
        InitializeParams, InitializeResult, InitializedParams, MarkedString, MessageType,
        Position as LspPosition, Range as LspRange, SaveOptions, ServerCapabilities,
        TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions,
        TextDocumentSyncSaveOptions, Url,
    },
};

use moss_lang_interpreter::interpreter::{
    Interpreter, InterpreterLike, InterpreterLikeMut, LocalId, Node, StringInterner, UntypedNode, diagnose::Diagnostic, file::FileId, module::ModuleAuthored, scope::LocalScopeId, value::ContextedValue
};

pub struct LanguageServer {
    pub client: Client,
    pub interpreter: OnceLock<RwLock<Interpreter>>,
    pub opened_files: RwLock<HashMap<Url, File>>,
}

pub struct File {
    pub path: PathBuf,
}

impl File {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl LanguageServer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            interpreter: Default::default(),
            opened_files: RwLock::new(Default::default()),
        }
    }
    pub fn make_diagnostic(
        &self,
        source: UntypedNode<'static>,
        message: impl Into<String>,
        severity: DiagnosticSeverity,
    ) -> LspDiagnostic {
        let source_start = source.start_position();
        let source_end = source.end_position();
        LspDiagnostic {
            range: LspRange::new(
                LspPosition::new(source_start.row as u32, source_start.column as u32),
                LspPosition::new(source_end.row as u32, source_end.column as u32),
            ),
            severity: Some(severity),
            code: None,
            code_description: None,
            source: None,
            message: message.into(),
            related_information: None,
            tags: None,
            data: None,
        }
    }
    pub fn uri2path(&self, uri: &Url, interpreter: &Interpreter) -> Option<PathBuf> {
        let raw_path = uri.to_file_path().unwrap();
        Some(
            raw_path
                .strip_prefix(&interpreter.workspace_path)
                .ok()?
                .to_path_buf(),
        )
    }
    pub async fn diagnose(&self, uri: Url, path: impl AsRef<Path>, interpreter: &Interpreter) {
        let mut lsp_diagnostics = Vec::<LspDiagnostic>::new();

        struct Context<'a> {
            file_id: FileId,
            language_server: &'a LanguageServer,
            interpreter: &'a Interpreter,
            lsp_diagnostics: &'a mut Vec<LspDiagnostic>,
        }
        impl<'a> Context<'a> {
            fn diagnose(&mut self, diagnostic: &Diagnostic) {
                match diagnostic {
                    Diagnostic::GrammarError { source } => {
                        self.lsp_diagnostics
                            .push(self.language_server.make_diagnostic(
                                *source,
                                "grammar error",
                                DiagnosticSeverity::ERROR,
                            ));
                    }
                    Diagnostic::ElementKeyRedundancy { source } => {
                        self.lsp_diagnostics
                            .push(self.language_server.make_diagnostic(
                                *source,
                                "element key redundancy",
                                DiagnosticSeverity::ERROR,
                            ));
                    }
                    Diagnostic::FailedFindElement { source } => {
                        self.lsp_diagnostics
                            .push(self.language_server.make_diagnostic(
                                *source,
                                "failed find element",
                                DiagnosticSeverity::ERROR,
                            ));
                    }
                    Diagnostic::FialedFindElementOrPrivateElement { source } => {
                        self.lsp_diagnostics
                            .push(self.language_server.make_diagnostic(
                                *source,
                                "failed find element or private element",
                                DiagnosticSeverity::ERROR,
                            ));
                    }
                    Diagnostic::CanNotFindIn { source, value } => {
                        self.lsp_diagnostics
                            .push(self.language_server.make_diagnostic(
                                *source,
                                format!(
                                    "can not find element in {}",
                                    ContextedValue {
                                        value: &value,
                                        ctx: self.interpreter
                                    }
                                ),
                                DiagnosticSeverity::ERROR,
                            ));
                    }
                    Diagnostic::CanNotCallOn { source, value } => {
                        self.lsp_diagnostics
                            .push(self.language_server.make_diagnostic(
                                *source,
                                format!(
                                    "can not call on {}",
                                    ContextedValue {
                                        value: &value,
                                        ctx: self.interpreter
                                    }
                                ),
                                DiagnosticSeverity::ERROR,
                            ));
                    }
                    Diagnostic::PathError { source } => {
                        self.lsp_diagnostics
                            .push(self.language_server.make_diagnostic(
                                *source,
                                "path error",
                                DiagnosticSeverity::ERROR,
                            ));
                    }
                };
            }
            fn traverse(&mut self, scope_id: LocalScopeId) {
                let scope = self.interpreter.get_scope(scope_id);
                for diagnostic in &scope.diagnoistics {
                    self.diagnose(diagnostic);
                }
                for element_id in scope.elements.values() {
                    let element = self
                        .interpreter
                        .get_element(element_id.global(scope_id.module));
                    for diagnoistic in &element.diagnoistics {
                        self.diagnose(diagnoistic);
                    }
                    if let Some(authored) = &element.authored {
                        if let Some(key_node) = authored.key_node {
                            self.lsp_diagnostics
                                .push(self.language_server.make_diagnostic(
                                    key_node.upcast(),
                                    format!(
                                        "{}: {}",
                                        ContextedValue {
                                            ctx: self.interpreter,
                                            value: &element.resolved_value.value
                                        },
                                        ContextedValue {
                                            ctx: self.interpreter,
                                            value: &element.resolved_value.r#type
                                        }
                                    ),
                                    DiagnosticSeverity::HINT,
                                ));
                        }
                    }
                }
                for child_id in &scope.children {
                    let child = self.interpreter.get_scope(child_id.global(scope_id.module));
                    if let Some(file) = child.get_file()
                        && file == self.file_id
                    {
                        self.traverse(child_id.global(scope_id.module));
                    }
                }
            }
        }
        let Some(file_id) = interpreter.find_file(path) else {
            return;
        };

        let file = interpreter.get_file(file_id);
        let Some(module_id) = file.is_module else {
            return;
        };
        let module = interpreter.get_module(module_id);
        let Some(scope_id) = module.root_scope.get() else {
            return;
        };

        let mut context = Context {
            file_id,
            language_server: self,
            interpreter,
            lsp_diagnostics: &mut lsp_diagnostics,
        };

        context.traverse(scope_id.global(module_id));

        self.client
            .publish_diagnostics(uri, lsp_diagnostics, None)
            .await;
    }
    pub async fn run(&self) {
        {
            let entry_path = PathBuf::from_str("src/_.moss").unwrap();
            if !entry_path.exists() {
                self.client
                    .log_message(
                        MessageType::ERROR,
                        format!("can't find entry file at {}", entry_path.display()),
                    )
                    .await;
                return;
            }
            let Some(interpreter) = self.interpreter.get() else {
                return;
            };
            let mut interpreter = interpreter.write().await;
            interpreter.clear();
            interpreter.add_module(ModuleAuthored::File { path: entry_path });
            interpreter.run().await;
        }
        {
            let interpreter = self.interpreter.get().unwrap().read().await;
            let files = self.opened_files.read().await;
            for (uri, file) in files.iter() {
                self.diagnose(uri.clone(), &file.path, &*interpreter).await
            }
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServerLike for LanguageServer {
    async fn initialize(
        &self,
        params: InitializeParams,
    ) -> tower_lsp::jsonrpc::Result<InitializeResult> {
        let uri = params.root_uri.as_ref().and_then(|_| {
            params
                .workspace_folders
                .as_ref()
                .map(|x| x.first().map(|x| &x.uri))
                .flatten()
        });
        if let Some(uri) = uri {
            let workspace_path = uri.to_file_path().unwrap_or_else(|_| {
                log::error!("error workspace: {uri}");
                "err".into()
            });
            if let Err(_) = self
                .interpreter
                .set(RwLock::new(Interpreter::new(workspace_path)))
            {
                log::error!("re-initialize interpreter");
            }
        } else {
            log::error!("no workspace");
        }

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::NONE),
                        will_save: Some(false),
                        will_save_wait_until: Some(false),
                        save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                            include_text: Some(false),
                        })),
                    },
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: None,
                ..ServerCapabilities::default()
            },
            server_info: None,
        })
    }

    async fn initialized(&self, params: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Moss Language Server initialized")
            .await;
        self.run().await;
    }

    async fn shutdown(&self) -> tower_lsp::jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let Some(interpreter) = self.interpreter.get() else {
            return;
        };
        let interpreter = interpreter.read().await;
        let interpreter = &*interpreter;
        let mut files = self.opened_files.write().await;

        let uri = params.text_document.uri;
        let Some(path) = self.uri2path(&uri, interpreter) else {
            return;
        };

        files.insert(uri.clone(), File::new(path.clone()));

        self.diagnose(uri, &path, &*interpreter).await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        {
            let mut interpreter = self.interpreter.get().unwrap().write().await;
            let interpreter = &mut *interpreter;
            let files = self.opened_files.read().await;

            let uri = params.text_document.uri;
            let file = files.get(&uri).unwrap();
            let path = &file.path;
            let Some(file) = interpreter.find_file(path) else {
                return;
            };
            let file = interpreter.get_file_mut(file);
            file.update();
        }
        self.run().await;
    }

    async fn hover(&self, params: HoverParams) -> tower_lsp::jsonrpc::Result<Option<Hover>> {
        let contents = HoverContents::Scalar(MarkedString::String("kkk".to_string()));
        Ok(Some(Hover {
            contents,
            range: None,
        }))
    }

    async fn completion(
        &self,
        _: CompletionParams,
    ) -> tower_lsp::jsonrpc::Result<Option<CompletionResponse>> {
        Ok(None)
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let mut files = self.opened_files.write().await;
        let uri = params.text_document.uri;
        files.remove(&uri);
    }
}
