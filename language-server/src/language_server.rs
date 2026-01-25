use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use tokio::sync::RwLock;
use tower_lsp::{
    Client, LanguageServer as LanguageServerLike,
    lsp_types::{
        CompletionParams, CompletionResponse, Diagnostic as LspDiagnostic, DiagnosticSeverity,
        DidCloseTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams, Hover,
        HoverContents, HoverParams, HoverProviderCapability, InitializeParams, InitializeResult,
        InitializedParams, MarkedString, MessageType, Position as LspPosition, Range as LspRange,
        SaveOptions, ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind,
        TextDocumentSyncOptions, TextDocumentSyncSaveOptions, Url,
    },
};

use moss_interpreter::{
    interpreter::{
        Id, Interpreter, InterpreterLike, Node, SRC_FILE_EXTENSION, UntypedNode,
        diagnose::Diagnostic,
        file::FileId,
        scope::Scope,
        value::{self, Value},
    },
    utils::{contexted::WithContext as _, erase, erase_mut},
};
use type_sitter::TreeCursor;
use walkdir::WalkDir;

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
            ls: &'a LanguageServer,
            ip: &'a Interpreter,
            lsp_diagnostics: &'a mut Vec<LspDiagnostic>,
            cursor: TreeCursor<'static>,
        }
        impl<'a> Context<'a> {
            fn grammar(&mut self) {
                loop {
                    let node = self.cursor.node();

                    if node.is_extra() {
                        self.lsp_diagnostics.push(self.ls.make_diagnostic(
                            node,
                            format!("grammar error: extra {}", node.kind()),
                            DiagnosticSeverity::ERROR,
                        ));
                    }

                    if node.is_error() {
                        self.lsp_diagnostics.push(self.ls.make_diagnostic(
                            node,
                            format!("grammar error: error token"),
                            DiagnosticSeverity::ERROR,
                        ));
                    }

                    if node.is_missing() {
                        self.lsp_diagnostics.push(self.ls.make_diagnostic(
                            node,
                            format!("grammar error: missing {}", node.kind()),
                            DiagnosticSeverity::ERROR,
                        ));
                    }

                    if self.cursor.goto_first_child() {
                        self.grammar();
                        self.cursor.goto_parent();
                    }

                    if !self.cursor.goto_next_sibling() {
                        break;
                    }
                }
            }
            fn traverse(&mut self, scope_id: Id<Scope>) {
                let scope_local = unsafe { self.ip.get_local(scope_id) };
                let scope = self.ip.get(scope_id);
                if let Some(authored) = scope.authored {
                    let source = authored.source.source();
                    for diagnostic in &scope_local.diagnoistics {
                        self.lsp_diagnostics.push(self.ls.make_diagnostic(
                            source,
                            format!("{}", diagnostic.with_ctx(self.ip)),
                            DiagnosticSeverity::ERROR,
                        ));
                    }
                }
                for element_id in scope
                    .elements
                    .values()
                    .chain(scope.temp_elements.iter())
                    .copied()
                {
                    let element_local = unsafe { self.ip.get_local(element_id) };
                    let element = self.ip.get(element_id);
                    if let Some(source) = &element.source {
                        for diagnostic in &element_local.diagnoistics {
                            let source = if diagnostic.is_key() {
                                source.key_source.unwrap().upcast()
                            } else {
                                source.value_source.upcast()
                            };
                            self.lsp_diagnostics.push(self.ls.make_diagnostic(
                                source,
                                format!("{}", diagnostic.with_ctx(self.ip)),
                                DiagnosticSeverity::ERROR,
                            ));
                        }
                        if let Some(key_node) = source.key_source {
                            self.lsp_diagnostics.push(self.ls.make_diagnostic(
                                key_node.upcast(),
                                format!(
                                        "{}",
                                        element_local
                                            .value
                                            .unwrap_or(Value::Error(value::Error))
                                            .with_ctx(self.ip)
                                    ),
                                DiagnosticSeverity::HINT,
                            ));
                        }
                    }
                }
                for child_id in scope_local.children.iter().copied() {
                    let child = self.ip.get(child_id);
                    if let Some(file) = child.get_file()
                        && file == self.file_id
                    {
                        self.traverse(child_id);
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
        let scope_id = interpreter
            .get_element_value(module.root_scope.unwrap())
            .unwrap()
            .as_scope()
            .unwrap()
            .0;

        let mut context = Context {
            file_id,
            ls: self,
            ip: interpreter,
            lsp_diagnostics: &mut lsp_diagnostics,
            cursor: erase(file).tree.walk(),
        };

        context.grammar();
        context.traverse(scope_id);

        self.client
            .publish_diagnostics(uri, lsp_diagnostics, None)
            .await;
    }
    pub async fn run(&self) {
        {
            let Some(interpreter) = self.interpreter.get() else {
                return;
            };
            let mut interpreter = interpreter.write().await;
            interpreter.clear();
            interpreter.init();
            for entry in WalkDir::new(interpreter.get_src_path())
                .into_iter()
                .filter_map(Result::ok)
            {
                let path = entry.path();
                if path.is_file() {
                    if let Some(extension) = path.extension() {
                        if extension == SRC_FILE_EXTENSION {
                            let path = path
                                .strip_prefix(interpreter.get_worksapce_path())
                                .unwrap()
                                .to_path_buf();
                            interpreter.add_module(Some(path));
                        }
                    }
                }
            }
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

    async fn initialized(&self, _params: InitializedParams) {
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
            let file = erase_mut(interpreter).get_file_mut(file);
            file.update(interpreter);
        }
        self.run().await;
    }

    async fn hover(&self, _params: HoverParams) -> tower_lsp::jsonrpc::Result<Option<Hover>> {
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
