use std::{
    collections::{HashMap, HashSet},
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
        InitializedParams, InlayHint, InlayHintKind, InlayHintLabel, InlayHintParams, MarkedString,
        MessageType, OneOf, Position as LspPosition, Range as LspRange, SaveOptions,
        ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind,
        TextDocumentSyncOptions, TextDocumentSyncSaveOptions, Url, request,
    },
};

use moss_interpreter::{
    interpreter::{
        Id, Interpreter, InterpreterLike, Managed, Node, SRC_FILE_EXTENSION, UntypedNode,
        error::Error,
        expr::Expr,
        scope::Scope,
        value::{self, ValueStorage},
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
    pub fn make_error_diagnostic<T: InterpreterLike>(
        &self,
        error: Error,
        ip: &T,
    ) -> Option<LspDiagnostic> {
        let source = match error.location {
            moss_interpreter::interpreter::Location::Element(mut id) => loop {
                let element = ip.get(id);
                if let Some(source) = element.source {
                    if error.kind.is_key() {
                        break source.key_source.map(|x| x.upcast());
                    } else {
                        break Some(source.value_source.upcast());
                    }
                }
                let element_local = unsafe { ip.get_local(id) };
                if let Some(expr) = element_local.expr
                    && let Expr::Ref(r#ref) = expr
                {
                    id = r#ref.element;
                } else {
                    log::error!(
                        "make_error_diagnostic: {}, expr: {:?}, value: {}",
                        ValueStorage::Element(value::Element(id)).with_ctx(ip),
                        element_local.expr,
                        element_local.value.unwrap().with_ctx(ip)
                    );
                    break None;
                }
            },
            moss_interpreter::interpreter::Location::Scope(id) => {
                let scope = ip.get(id);
                scope.source.map(|x| x.upcast())
            }
        }?;
        Some(self.make_diagnostic(
            source.upcast(),
            format!("{}", error.kind.with_ctx(ip)),
            DiagnosticSeverity::ERROR,
        ))
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
    pub fn make_inlay_hint(
        &self,
        source: UntypedNode<'static>,
        message: impl Into<String>,
    ) -> InlayHint {
        let source_end = source.end_position();
        InlayHint {
            position: LspPosition::new(source_end.row as u32, source_end.column as u32),
            label: InlayHintLabel::String(message.into()),
            kind: Some(InlayHintKind::TYPE),
            text_edits: None,
            tooltip: None,
            padding_left: Some(false),
            padding_right: Some(false),
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
            ls: &'a LanguageServer,
            ip: &'a Interpreter,
            lsp_diagnostics: &'a mut Vec<LspDiagnostic>,
            cursor: TreeCursor<'static>,
            error_set: HashSet<Id<Error>>,
            scope_set: HashSet<Id<Scope>>,
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
            fn traverse(&mut self, scope: Id<Scope>) {
                let scope = self.ip.get(scope);
                for element in scope.visible_elements() {
                    let Some(value) = self.ip.get_element_value(element) else {
                        continue;
                    };
                    match value {
                        ValueStorage::Error(value::Error(error)) => {
                            if self.error_set.insert(error) {
                                let error = *self.ip.get(error);
                                if let Some(diagnostic) =
                                    self.ls.make_error_diagnostic(error, self.ip)
                                {
                                    self.lsp_diagnostics.push(diagnostic);
                                }
                            }
                        }
                        ValueStorage::Scope(value::Scope(id)) => {
                            if self.scope_set.insert(id) {
                                self.traverse(id);
                            }
                        }
                        ValueStorage::Function(value::Function(id)) => {
                            let function = self.ip.get(id);
                            self.traverse(function.scope);
                        }
                        _ => (),
                    }
                }
            }
        }
        let Some(file_id) = interpreter.find_file(path) else {
            return;
        };

        let file = interpreter.get_file(file_id);
        let Some(module_id) = file.module else {
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
            ls: self,
            ip: interpreter,
            lsp_diagnostics: &mut lsp_diagnostics,
            cursor: erase(file).tree.walk(),
            error_set: Default::default(),
            scope_set: Default::default(),
        };

        context.grammar();
        context.traverse(scope_id);
        let module_local = unsafe { interpreter.get_module_local(module_id) };
        for error in module_local.errors.iter().copied() {
            if let Some(diagnosic) = self.make_error_diagnostic(error, interpreter) {
                lsp_diagnostics.push(diagnosic);
            }
        }

        self.client
            .publish_diagnostics(uri, lsp_diagnostics, None)
            .await;
    }
    pub fn inlay_hint(
        &self,
        path: impl AsRef<Path>,
        interpreter: &Interpreter,
    ) -> Option<Vec<InlayHint>> {
        let mut inlay_hints = Vec::<InlayHint>::new();
        let Some(file_id) = interpreter.find_file(path) else {
            return None;
        };

        let file = interpreter.get_file(file_id);
        let Some(module_id) = file.module else {
            return None;
        };
        let module = unsafe { interpreter.get_module_local(module_id) };

        for element in module.pools.elements.iter() {
            let Some(source) = element.source else {
                continue;
            };
            let Some(key_source) = source.key_source else {
                continue;
            };
            inlay_hints.push(self.make_inlay_hint(
                key_source.upcast(),
                format!(
                        "{}",
                        interpreter
                            .get_element_value(element.get_id())
                            .unwrap_or(ValueStorage::Unkown(value::Unkown))
                            .with_ctx(interpreter)
                    ),
            ));
        }
        Some(inlay_hints)
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
        self.client
            .send_request::<request::InlayHintRefreshRequest>(())
            .await
            .ok();
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
                inlay_hint_provider: Some(OneOf::Left(true)),
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

    async fn inlay_hint(
        &self,
        params: InlayHintParams,
    ) -> tower_lsp::jsonrpc::Result<Option<Vec<InlayHint>>> {
        let mut interpreter = self.interpreter.get().unwrap().write().await;
        let interpreter = &mut *interpreter;
        let uri = params.text_document.uri;
        let Some(path) = self.uri2path(&uri, interpreter) else {
            return Ok(None);
        };
        let hints = self.inlay_hint(path, interpreter);
        Ok(hints)
    }
}
