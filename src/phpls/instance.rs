use crate::codetree::codetree::CallbackProgress;
use crate::codetree::codetree::CodeTree;
use crate::phpparser::phpfile::PHPFile;
use php_tree_sitter::analysis::state::AnalysisState;
use php_tree_sitter::autonodes::any::AnyNodeRef;
use php_tree_sitter::issue::Issue;
use php_tree_sitter::issue::Severity;

use php_tree_sitter::symboldata::ArcedSymbolAccess;
use php_tree_sitter::symboldata::SymbolData;

// use php_tree_sitter::symboldata::SymbolData;
use rust_lsp::jsonrpc::method_types::MethodError;
use rust_lsp::jsonrpc::Endpoint;
use rust_lsp::jsonrpc::MethodCompletable;
use rust_lsp::lsp::client_rpc_handle;
use rust_lsp::lsp::LanguageServerHandling;
use rust_lsp::lsp::LspClientRpc;
use rust_lsp::lsp::LspClientRpc_;
use rust_lsp::lsp_types::request::GotoDeclaration;

use rust_lsp::lsp_types::request::GotoDeclarationParams;
use rust_lsp::lsp_types::request::GotoTypeDefinition;

use rust_lsp::lsp_types::request::GotoTypeDefinitionParams;
use rust_lsp::lsp_types::*;
use std::convert::TryInto;
use std::sync::Arc;
// use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::RwLock;

use rust_lsp::lsp_types::request::Request;

use super::goto_declaration::goto_declaration;
use super::goto_type_definition::goto_type_definition;
use crate::phpls::goto_definition::goto_definition;
use crate::phpls::hover::hover;
/*
struct DiagnosticsEmitter {
    issues: RwLock<Vec<Diagnostic>>,
}

impl DiagnosticsEmitter {
    pub fn new() -> Self {
        Self {
            issues: RwLock::new(vec![]),
        }
    }
}
*/
trait FromIssue {
    fn from_issue(issue: &Issue) -> Diagnostic;
}

impl FromIssue for Diagnostic {
    fn from_issue(issue: &Issue) -> Self {
        let ts_range = issue.range();

        let range = Range {
            start: Position {
                line: ts_range.start_point.row.try_into().unwrap(),
                character: ts_range.start_point.column.try_into().unwrap(),
            },
            end: Position {
                line: ts_range.end_point.row.try_into().unwrap(),
                character: ts_range.end_point.column.try_into().unwrap(),
            },
        };
        let severity = Some(match issue.severity() {
            Severity::Hint => DiagnosticSeverity::Hint,
            Severity::Error => DiagnosticSeverity::Error,
            Severity::Warning => DiagnosticSeverity::Warning,
            Severity::Information => DiagnosticSeverity::Information,
        });
        Self {
            range,
            severity,
            code: None,
            code_description: None,
            source: Some(format!("phidelity")),
            message: issue.as_string(),
            related_information: None,
            tags: issue.get_tags(),
            data: None,
        }
    }
}
/*
impl IssueEmitter for DiagnosticsEmitter {
    fn emit(&self, issue: Issue) {}
}
*/
trait GetTagsFromIssue {
    fn get_tags(&self) -> Option<Vec<DiagnosticTag>>;
}

impl GetTagsFromIssue for Issue {
    fn get_tags(&self) -> Option<Vec<DiagnosticTag>> {
        match self {
            Issue::UnreachableCode(_) => Some(vec![DiagnosticTag::Unnecessary]),
            _ => None,
        }
    }
}

struct InAnalysisState {
    request_reanalysis: bool,
    on_completion: Vec<Box<dyn FnOnce(&mut PHPLanguageServerInstance, Arc<CodeTree>) -> ()>>,
}

impl InAnalysisState {
    fn new() -> Self {
        Self {
            request_reanalysis: false,
            on_completion: vec![],
        }
    }
}

#[derive(Clone)]
pub struct PHPLanguageServerInstanceClient {
    endpoint: Endpoint,
}

impl PHPLanguageServerInstanceClient {
    pub fn new(endpoint: Endpoint) -> Self {
        Self { endpoint }
    }

    pub fn client<'a>(&'a mut self) -> LspClientRpc_<'a> {
        client_rpc_handle(&mut self.endpoint)
    }
}

pub struct PHPLanguageServerInstance {
    endpoint: Endpoint,
    codetrees: Vec<Arc<CodeTree>>,
    in_analyzing: RwLock<Option<InAnalysisState>>,
    progress_registered: AtomicBool,
}

impl PHPLanguageServerInstance {
    pub fn new(endpoint: Endpoint) -> Self {
        PHPLanguageServerInstance {
            endpoint,
            codetrees: vec![],
            in_analyzing: RwLock::new(None),
            progress_registered: AtomicBool::new(false),
        }
    }

    pub fn client(&self) -> PHPLanguageServerInstanceClient {
        PHPLanguageServerInstanceClient::new(self.endpoint.clone())
    }
    pub fn error_not_available<DATA>(data: DATA) -> MethodError<DATA> {
        let msg = "Functionality not implemented.".to_string();
        MethodError::<DATA> {
            code: 1,
            message: msg,
            data: data,
        }
    }

    fn get_codetree_for_uri(&self, uri: &Url) -> Option<Arc<CodeTree>> {
        for ct in &self.codetrees {
            if ct.contains_file(uri) {
                return Some(ct.clone());
            }
        }
        None
    }

    pub fn when_completed_analysis(
        &mut self,
        uri: Url,
        callback: Box<dyn FnOnce(&mut PHPLanguageServerInstance, Arc<CodeTree>) -> ()>,
    ) {
        {
            let mut write = self.in_analyzing.write().unwrap();
            if let Some(analysing) = &mut *write {
                // If we're analyzing, pending for after
                analysing.on_completion.push(callback);
                return;
            }
        }
        if let Some(ct) = self.get_codetree_for_uri(&uri) {
            let read_handle = ct.symbol_data.read().unwrap();
            if let Some(_) = &*read_handle {
                callback(self, ct.clone());
                return;
            }
        }

        panic!("");
        // self.pending_analysis.push(callback);
        // void
    }

    pub fn republish_diagnostics(&mut self, uri: Url) {
        let code_tree = match self.get_codetree_for_uri(&uri) {
            Some(ct) => ct,
            _ => return,
        };
        let issues = code_tree.get_issues_for_uri(&uri);
        let diagnostics = issues
            .iter()
            .map(|i| Diagnostic::from_issue(i))
            .collect::<Vec<Diagnostic>>();
        let diag_cnt = diagnostics.len();
        let mut client_handle = self.client();
        let res = client_handle
            .client()
            .publish_diagnostics(PublishDiagnosticsParams {
                uri,
                diagnostics,
                version: None,
            });

        eprintln!("published {} diagnostics: {:?}", diag_cnt, res);

        /*
                for ct in &self.codetrees {
                    if !ct.contains_file(&uri) {
                        continue;
                    }
                }
                let diagnostics = if let Some(file) = self.get_phpfile_for_uri(&uri) {
                    let emitter = DiagnosticsEmitter::new();
                    let data = if let Some(data) = &self.symbol_data {
                        data.clone()
                    } else {
                        Arc::new(SymbolData::new())
                    };
                    file.analyze_with_symbol_data(&emitter, data);
                    let x = emitter.issues.read().unwrap();
                    x.clone()
                } else {
                    vec![]
                };
        */
    }

    pub(crate) fn reanalyze(
        &mut self,
        _uri: Option<Url>,
        mut then: Option<Box<dyn FnOnce(&mut Self, Arc<CodeTree>) -> ()>>,
    ) -> Option<()> {
        {
            let mut write = self.in_analyzing.write().unwrap();
            if let Some(in_analysis) = &mut *write {
                in_analysis.request_reanalysis = true;
                if let Some(then) = then.take() {
                    in_analysis.on_completion.push(then);
                }
                return None;
            }
            let mut state = InAnalysisState::new();
            if let Some(then) = then.take() {
                state.on_completion.push(then);
            }
            (*write) = Some(state);
            // end of write lock
        }
        let mut client_handle = self.client();
        let progress_token = NumberOrString::String("oh_by_the_way".to_string());

        if !self.progress_registered.swap(true, Ordering::Relaxed) {
            client_handle
                .client()
                .window_work_done_progress_create(WorkDoneProgressCreateParams {
                    token: progress_token.clone(),
                });
        }
        client_handle.client().progress(ProgressParams {
            token: progress_token.clone(),
            value: ProgressParamsValue::WorkDone(WorkDoneProgress::Begin(WorkDoneProgressBegin {
                title: "Analyserer".to_string(),
                cancellable: Some(false),
                message: Some("message...".into()),
                percentage: Some(0),
            })),
        });

        let thread_count = 8;
        let cb_client_handle = client_handle.clone();
        let cb_token = progress_token.clone();
        let status = Arc::new(CallbackProgress::new(Box::new(move |percent, ident| {
            let mut handle = cb_client_handle.clone();
            handle.client().progress(ProgressParams {
                token: cb_token.clone(),
                value: ProgressParamsValue::WorkDone(WorkDoneProgress::Report(
                    WorkDoneProgressReport {
                        cancellable: Some(false),
                        message: Some(ident),
                        percentage: Some(percent.try_into().ok().unwrap_or(0)),
                    },
                )),
            });
        })));
        for ct in &self.codetrees {
            match ct.run_analysis(thread_count, status.clone()) {
                Ok(_) => (),
                Err(e) => eprintln!("Error: {}", e),
            }
        }

        let mut state = {
            let mut write = self.in_analyzing.write().unwrap();
            write
                .take()
                .expect("This must exist, or we're in a broken state")
            // End of write lock
        };

        client_handle.client().progress(ProgressParams {
            token: progress_token,
            value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(WorkDoneProgressEnd {
                message: Some("Done.".into()),
            })),
        });

        for cb in state.on_completion.drain(..) {
            let ct = self.codetrees.get(0).unwrap().clone();
            cb(self, ct);
            /*             for ct in &self.codetrees {
                cb(self, ct);
                break;
            }*/
        }

        if state.request_reanalysis {
            eprintln!("Skipping reanalysis");
            // self.reanalyze(None, None);
        }

        None
    }

    pub fn at_position<T>(
        &self,
        position: TextDocumentPositionParams,
        callback: Box<
            dyn FnOnce(AnyNodeRef, &mut AnalysisState, &Vec<AnyNodeRef>) -> T + Send + Sync,
        >,
    ) -> Result<(Option<Arc<SymbolData>>, Option<T>), &'static str>
    where
        T: 'static + Clone + Send + Sync,
    {
        let uri = position.text_document.uri.clone();

        if uri.scheme() != "file" {
            return Err("uri-scheme not file");
        }

        let codetree =
            if let Some(codetree) = self.codetrees.iter().find(|&ct| ct.contains_file(&uri)) {
                codetree
            } else {
                return Err("file not found");
            };

        let file = codetree.analyze_file_uri(&uri);

        let symbol_data = codetree.get_symbol_data();

        let line: Result<usize, _> = position.position.line.try_into();
        let charpos: Result<usize, _> = position.position.character.try_into();
        match (file, line, charpos) {
            (Some(file), Ok(line), Ok(character)) => {
                match file.analyze_with_callback_at_position(
                    line,
                    character,
                    symbol_data.clone(),
                    callback,
                ) {
                    Ok(res) => Ok((symbol_data, res)),
                    Err(e) => Err(e),
                }
            }

            _ => Ok((symbol_data, None)),
        }
    }
}

impl LanguageServerHandling for PHPLanguageServerInstance {
    fn initialize(
        &mut self,
        params: InitializeParams,
        completable: MethodCompletable<InitializeResult, InitializeError>,
    ) {
        eprintln!("Her er vi i initialize med");
        if let Some(process_id) = params.process_id {
            eprintln!("  params.process_id: {}", process_id);
        }
        if let Some(client_info) = params.client_info {
            eprintln!("  params.client_info: {:?}", client_info);
        }
        if let Some(init_options) = params.initialization_options {
            eprintln!("  params.initialization_options: {}", init_options);
        }
        if let Some(locale) = params.locale {
            eprintln!("  params.locale: {}", locale);
        }
        /*if let Some(root_path) = params.root_path {
            eprintln!("  params.root_path: {} [deprecated]", root_path);
        }*/
        if let Some(root_uri) = params.root_uri {
            eprintln!("  params.root_uri: {} [deprecated]", root_uri);
        }
        if let Some(trace) = params.trace {
            eprintln!("  params.trace: {:?}", trace);
        }
        if let Some(ws_folders) = params.workspace_folders {
            eprintln!("  params.workspace_folders: {:?}", ws_folders);
            for folder in ws_folders {
                if let Some(tree) = CodeTree::new_for_url(folder.uri, folder.name) {
                    self.codetrees.push(Arc::new(tree));
                }
            }
        }

        let mut capabilities = ServerCapabilities::default();
        capabilities.text_document_sync = Some(TextDocumentSyncCapability::Options(
            TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(TextDocumentSyncKind::None),
                will_save: None,
                will_save_wait_until: None,
                save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                    include_text: Some(false),
                })),
            },
        ));

        // provide hover-text
        capabilities.hover_provider = Some(HoverProviderCapability::Simple(true));

        capabilities.references_provider = Some(OneOf::Left(true));
        // provide goto definition
        capabilities.definition_provider = Some(OneOf::Right(DefinitionOptions {
            work_done_progress_options: WorkDoneProgressOptions {
                work_done_progress: Some(false),
            },
        }));

        capabilities.declaration_provider = Some(DeclarationCapability::Simple(true));

        capabilities.type_definition_provider =
            Some(TypeDefinitionProviderCapability::Simple(true));

        capabilities.implementation_provider = Some(ImplementationProviderCapability::Simple(true));

        // provide goto type declaration
        capabilities.type_definition_provider =
            Some(TypeDefinitionProviderCapability::Simple(true));

        //         capabilities.
        let server_info = ServerInfo {
            name: String::from("phplint"),
            version: Some(String::from("0.0.1")),
        };
        /*  let x = ServerCapabilities {
            call_hierarchy_provider: None,
            code_lens_provider: None,
            code_action_provider: None,
            color_provider: None,
            completion_provider: None,
            declaration_provider: None,
            document_highlight_provider: None,
            document_link_provider: None,
            document_on_type_formatting_provider: None,
            document_range_formatting_provider: None,
            document_symbol_provider: None,
            document_formatting_provider: None,
            definition_provider: None,
            execute_command_provider: None,
            experimental: None,
            folding_range_provider: None,
            hover_provider: None,
            implementation_provider: None,
            linked_editing_range_provider: None,
            moniker_provider: None,
            references_provider: None,
            rename_provider: None,
            selection_range_provider: None,
            semantic_tokens_provider: None,
            signature_help_provider: None,
            text_document_sync: None,
            type_definition_provider: None,
            workspace: None,
            workspace_symbol_provider: None,
        };*/

        completable.complete(Ok(InitializeResult {
            capabilities: capabilities,
            server_info: Some(server_info),
            // offset_encoding: None,
        }));

        self.reanalyze(None, None);
    }

    fn shutdown(&mut self, _: (), completable: MethodCompletable<(), ()>) {
        completable.complete(Ok(()));
    }

    fn exit(&mut self, _: ()) {
        self.endpoint.request_shutdown();
    }

    fn workspace_change_configuration(&mut self, _params: DidChangeConfigurationParams) {
        eprintln!("workspace_change_configuration");
    }

    fn did_open_text_document(&mut self, params: DidOpenTextDocumentParams) {
        eprintln!("did_open_text_document");
        let uri = params.text_document.uri;
        self.when_completed_analysis(
            uri.clone(),
            Box::new(|server, _codetree| {
                server.republish_diagnostics(uri);
            }),
        );
    }

    fn did_change_text_document(&mut self, _params: DidChangeTextDocumentParams) {
        eprintln!("did_change_text_document")
    }

    fn did_close_text_document(&mut self, _params: DidCloseTextDocumentParams) {
        eprintln!("did_close_text_document")
    }

    fn did_save_text_document(&mut self, params: DidSaveTextDocumentParams) {
        eprintln!("did_save_text_document");
        let uri = params.text_document.uri.clone();
        self.reanalyze(
            Some(uri.clone()),
            Some(Box::new(|server, _codetree| {
                server.republish_diagnostics(uri)
            })),
        );
    }

    fn did_change_watched_files(&mut self, _params: DidChangeWatchedFilesParams) {
        eprintln!("did_change_watched_files");
        /*for ct in &self.codetrees {
            for evnt in &params.changes {
                if ct.contains_file(&evnt.uri) {
                    self.reanalyze(Some(evnt.uri.clone()), None);
                }
            }
        }*/
    }

    fn completion(
        &mut self,
        _params: TextDocumentPositionParams,
        completable: MethodCompletable<CompletionList, ()>,
    ) {
        eprintln!("completion");
        completable.complete(Err(Self::error_not_available(())));
    }

    fn resolve_completion_item(
        &mut self,
        _item: CompletionItem,
        completable: MethodCompletable<CompletionItem, ()>,
    ) {
        eprintln!("resolve_completion_item");
        completable.complete(Err(Self::error_not_available(())));
    }

    fn hover(
        &mut self,
        params: TextDocumentPositionParams,
        completable: MethodCompletable<Hover, ()>,
    ) {
        hover(self, params, completable);
    }

    fn signature_help(
        &mut self,
        _params: TextDocumentPositionParams,
        completable: MethodCompletable<SignatureHelp, ()>,
    ) {
        eprintln!("signature_help");
        completable.complete(Err(Self::error_not_available(())));
    }

    fn goto_definition(
        &mut self,
        params: TextDocumentPositionParams,
        completable: MethodCompletable<std::vec::Vec<Location>, ()>,
    ) {
        eprintln!("goto_definition");

        goto_definition(self, params, completable);
    }

    fn references(
        &mut self,
        _params: ReferenceParams,
        completable: MethodCompletable<std::vec::Vec<Location>, ()>,
    ) {
        eprintln!("references");
        completable.complete(Err(Self::error_not_available(())));
    }

    fn document_highlight(
        &mut self,
        _params: TextDocumentPositionParams,
        completable: MethodCompletable<std::vec::Vec<DocumentHighlight>, ()>,
    ) {
        eprintln!("document_highlight");
        completable.complete(Err(Self::error_not_available(())));
    }

    fn document_symbols(
        &mut self,
        _params: DocumentSymbolParams,
        completable: MethodCompletable<std::vec::Vec<SymbolInformation>, ()>,
    ) {
        eprintln!("document_symbols");
        completable.complete(Err(Self::error_not_available(())));
    }

    fn workspace_symbols(
        &mut self,
        _params: WorkspaceSymbolParams,
        completable: MethodCompletable<std::vec::Vec<SymbolInformation>, ()>,
    ) {
        eprintln!("workspace_symbols");
        completable.complete(Err(Self::error_not_available(())));
    }

    fn code_action(
        &mut self,
        _params: CodeActionParams,
        completable: MethodCompletable<std::vec::Vec<Command>, ()>,
    ) {
        eprintln!("code_action");
        completable.complete(Err(Self::error_not_available(())));
    }

    fn code_lens(
        &mut self,
        _params: CodeLensParams,
        completable: MethodCompletable<std::vec::Vec<CodeLens>, ()>,
    ) {
        eprintln!("code_lens");
        completable.complete(Err(Self::error_not_available(())));
    }

    fn code_lens_resolve(&mut self, _: CodeLens, completable: MethodCompletable<CodeLens, ()>) {
        eprintln!("code_lens_resolve");
        completable.complete(Err(Self::error_not_available(())));
    }

    fn document_link(
        &mut self,
        _params: DocumentLinkParams,
        completable: MethodCompletable<std::vec::Vec<DocumentLink>, ()>,
    ) {
        eprintln!("document_link");
        completable.complete(Err(Self::error_not_available(())));
    }

    fn document_link_resolve(
        &mut self,
        _: DocumentLink,
        completable: MethodCompletable<DocumentLink, ()>,
    ) {
        eprintln!("document_link_resolve");
        completable.complete(Err(Self::error_not_available(())));
    }

    fn formatting(
        &mut self,
        _params: DocumentFormattingParams,
        completable: MethodCompletable<std::vec::Vec<TextEdit>, ()>,
    ) {
        eprintln!("formatting");
        completable.complete(Err(Self::error_not_available(())));
    }

    fn range_formatting(
        &mut self,
        _params: DocumentRangeFormattingParams,
        completable: MethodCompletable<std::vec::Vec<TextEdit>, ()>,
    ) {
        eprintln!("range_formatting");
        completable.complete(Err(Self::error_not_available(())));
    }

    fn on_type_formatting(
        &mut self,
        _params: DocumentOnTypeFormattingParams,
        completable: MethodCompletable<std::vec::Vec<TextEdit>, ()>,
    ) {
        eprintln!("on_type_formatting");
        completable.complete(Err(Self::error_not_available(())));
    }

    fn rename(&mut self, _params: RenameParams, completable: MethodCompletable<WorkspaceEdit, ()>) {
        eprintln!("rename");
        completable.complete(Err(Self::error_not_available(())));
    }

    fn execute_command(
        &mut self,
        _params: ExecuteCommandParams,
        completable: MethodCompletable<Option<serde_json::value::Value>, ()>,
    ) {
        eprintln!("execute_command");
        completable.complete(Err(Self::error_not_available(())));
    }

    fn handle_other_method(
        &mut self,
        method_name: &str,
        params: rust_lsp::jsonrpc::jsonrpc_request::RequestParams,
        completable: rust_lsp::jsonrpc::ResponseCompletable,
    ) {
        match method_name {
            GotoDeclaration::METHOD => completable.handle_request_with(
                params,
                |params: GotoDeclarationParams, completable| {
                    goto_declaration(self, params, completable)
                },
            ),

            GotoTypeDefinition::METHOD => completable.handle_request_with(
                params,
                |params: GotoTypeDefinitionParams, completable| {
                    goto_type_definition(self, params, completable)
                },
            ),

            // Other
            _ => completable.complete_with_error(
                rust_lsp::jsonrpc::jsonrpc_common::error_JSON_RPC_MethodNotFound(),
            ),
        }
    }
}
