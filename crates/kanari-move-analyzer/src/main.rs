// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use clap::Parser;
use crossbeam::channel::{bounded, select};
use lsp_server::{Connection, Message, Notification, Request, Response};
use lsp_types::{
    CompletionOptions, Diagnostic, HoverProviderCapability, OneOf, SaveOptions,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions,
    TypeDefinitionProviderCapability, WorkDoneProgressOptions, notification::Notification as _,
    request::Request as _,
};
use move_compiler::linters::LintLevel;
use std::{
    collections::BTreeMap,
    io::Read,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use move_analyzer::{
    completion::on_completion_request,
    context::Context,
    symbols,
    utils::{path_to_uri, uri_to_file_path},
    vfs::on_text_document_sync_notification,
};
use vfs::{VfsPath, impls::memory::MemoryFS};

const LINT_NONE: &str = "none";
const LINT_DEFAULT: &str = "default";
const LINT_ALL: &str = "all";

#[derive(Parser)]
#[clap(author, version, about)]
struct Options {}

fn main() {
    // For now, move-analyzer only responds to options built-in to clap,
    // such as `--help` or `--version`.
    Options::parse();

    // stdio is used to communicate Language Server Protocol requests and responses.
    // stderr is used for logging (and, when Visual Studio Code is used to communicate with this
    // server, it captures this output in a dedicated "output channel").
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .to_string();
    eprintln!(
        "Starting language server '{}' communicating via stdio...",
        exe
    );

    // Kanari: repair ~/.move caches before symbolication so a stale
    // index.lock does not suppress all diagnostics.
    let (repaired, unrepairable) = kanari_move_analyzer::deps::repair_dependency_caches();
    if repaired > 0 || unrepairable > 0 {
        eprintln!(
            "kanari-move-analyzer: dependency cache repair repaired={repaired} unrepairable={unrepairable}"
        );
    }

    let (connection, io_threads) = Connection::stdio();
    let symbols = Arc::new(Mutex::new(symbols::empty_symbols()));
    let pkg_deps = Arc::new(Mutex::new(
        BTreeMap::<PathBuf, symbols::PrecompiledPkgDeps>::new(),
    ));
    let ide_files_root: VfsPath = MemoryFS::new().into();
    let context = Context {
        connection,
        symbols: symbols.clone(),
    };

    let (id, client_response) = context
        .connection
        .initialize_start()
        .expect("could not start connection initialization");

    let capabilities = serde_json::to_value(lsp_types::ServerCapabilities {
        // The server receives notifications from the client as users open, close,
        // and modify documents.
        text_document_sync: Some(TextDocumentSyncCapability::Options(
            TextDocumentSyncOptions {
                open_close: Some(true),
                // TODO: We request that the language server client send us the entire text of any
                // files that are modified. We ought to use the "incremental" sync kind, which would
                // have clients only send us what has changed and where, thereby requiring far less
                // data be sent "over the wire." However, to do so, our language server would need
                // to be capable of applying deltas to its view of the client's open files. See the
                // 'move_analyzer::vfs' module for details.
                change: Some(TextDocumentSyncKind::FULL),
                will_save: None,
                will_save_wait_until: None,
                save: Some(
                    SaveOptions {
                        include_text: Some(true),
                    }
                    .into(),
                ),
            },
        )),
        selection_range_provider: None,
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        // The server provides completions as a user is typing.
        completion_provider: Some(CompletionOptions {
            resolve_provider: None,
            // In Move, `foo::` and `foo.` should trigger completion suggestions for after
            // the `:` or `.`
            // (Trigger characters are just that: characters, such as `:`, and not sequences of
            // characters, such as `::`. So when the language server encounters a completion
            // request, it checks whether completions are being requested for `foo:`, and returns no
            // completions in that case.)
            trigger_characters: Some(vec![":".to_string(), ".".to_string(), "{".to_string()]),
            all_commit_characters: None,
            work_done_progress_options: WorkDoneProgressOptions {
                work_done_progress: None,
            },
            completion_item: None,
        }),
        definition_provider: Some(OneOf::Left(symbols::DEFS_AND_REFS_SUPPORT)),
        type_definition_provider: Some(TypeDefinitionProviderCapability::Simple(
            symbols::DEFS_AND_REFS_SUPPORT,
        )),
        references_provider: Some(OneOf::Left(symbols::DEFS_AND_REFS_SUPPORT)),
        document_symbol_provider: Some(OneOf::Left(true)),
        code_action_provider: Some(lsp_types::CodeActionProviderCapability::Simple(true)),
        document_formatting_provider: Some(lsp_types::OneOf::Left(true)),
        code_lens_provider: Some(lsp_types::CodeLensOptions {
            resolve_provider: None,
        }),
        ..Default::default()
    })
    .expect("could not serialize server capabilities");

    let (diag_sender, diag_receiver) = bounded::<Result<BTreeMap<PathBuf, Vec<Diagnostic>>>>(0);
    let mut symbolicator_runner = symbols::SymbolicatorRunner::idle();
    if symbols::DEFS_AND_REFS_SUPPORT {
        let initialize_params: lsp_types::InitializeParams =
            serde_json::from_value(client_response)
                .expect("could not deserialize client capabilities");

        // determine if linting is on or off based on what the editor requested
        let lint = {
            let lint_level = initialize_params
                .initialization_options
                .as_ref()
                .and_then(|init_options| init_options.get("lintLevel"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or(LINT_DEFAULT);
            if lint_level == LINT_ALL {
                LintLevel::All
            } else if lint_level == LINT_NONE {
                LintLevel::None
            } else {
                LintLevel::Default
            }
        };
        eprintln!("linting level {:?}", lint);

        symbolicator_runner = symbols::SymbolicatorRunner::new(
            ide_files_root.clone(),
            symbols.clone(),
            pkg_deps.clone(),
            diag_sender,
            lint,
        );

        // If initialization information from the client contains a path to the directory being
        // opened, try to initialize symbols before sending response to the client. Do not bother
        // with diagnostics as they will be recomputed whenever the first source file is opened. The
        // main reason for this is to enable unit tests that rely on the symbolication information
        // to be available right after the client is initialized.
        let root_uri = initialize_params
            .workspace_folders
            .as_ref()
            .and_then(|folders| folders.first())
            .map(|folder| folder.uri.clone())
            .or_else(|| {
                #[allow(deprecated)]
                initialize_params.root_uri.clone()
            });
        if let Some(uri) = root_uri
            && let Some(p) = symbols::SymbolicatorRunner::root_dir(&uri_to_file_path(&uri).unwrap())
            && let Ok((Some(new_symbols), _)) = symbols::get_symbols(
                Arc::new(Mutex::new(BTreeMap::new())),
                ide_files_root.clone(),
                p.as_path(),
                lint,
            )
        {
            let mut old_symbols = symbols.lock().unwrap();
            (*old_symbols).merge(new_symbols);
        }
    };

    context
        .connection
        .initialize_finish(
            id,
            serde_json::json!({
                "capabilities": capabilities,
            }),
        )
        .expect("could not finish connection initialization");

    let mut shutdown_req_received = false;
    loop {
        select! {
            recv(diag_receiver) -> message => {
                match message {
                    Ok(result) => {
                        match result {
                            Ok(diags) => {
                                for (k, v) in diags {
                                    let file_text = read_doc_text(&ide_files_root, &k);
                                    let v: Vec<_> = v
                                        .into_iter()
                                        .filter(|d| {
                                            !kanari_move_analyzer::diagnostics::is_kanari_false_positive_with_file(
                                                &d.message,
                                                &file_text,
                                                d.range.start.line as usize,
                                            )
                                        })
                                        .map(|mut d| {
                                            if kanari_move_analyzer::diagnostics::is_kanari_warning(
                                                &d.message,
                                            ) {
                                                d.severity =
                                                    Some(lsp_types::DiagnosticSeverity::WARNING);
                                            }
                                            d
                                        })
                                        .collect();
                                    let uri = path_to_uri(&k).unwrap();
                                    let params = lsp_types::PublishDiagnosticsParams::new(uri, v, None);
                                    let notification = Notification::new(lsp_types::notification::PublishDiagnostics::METHOD.to_string(), params);
                                    if let Err(err) = context
                                        .connection
                                        .sender
                                        .send(lsp_server::Message::Notification(notification)) {
                                            eprintln!("could not send diagnostics response: {:?}", err);
                                        };
                                }
                            },
                            Err(err) => {
                                    let typ = lsp_types::MessageType::ERROR;
                                let message = format!("{err}");
                                    // report missing manifest only once to avoid re-generating
                                    // user-visible error in cases when the developer decides to
                                    // keep editing a file that does not belong to a packages
                                    let params = lsp_types::ShowMessageParams { typ, message };
                                let notification = Notification::new(lsp_types::notification::ShowMessage::METHOD.to_string(), params);
                                if let Err(err) = context
                                    .connection
                                    .sender
                                    .send(lsp_server::Message::Notification(notification)) {
                                        eprintln!("could not send compiler error response: {:?}", err);
                                    };
                            },
                        }
                    },
                    Err(error) =>
                        eprintln!("symbolicator message error: {:?}", error),
                }
            },
            recv(context.connection.receiver) -> message => {
                match message {
                    Ok(Message::Request(request)) => {
                        // the server should not quit after receiving the shutdown request to give itself
                        // a chance of completing pending requests (but should not accept new requests
                        // either which is handled inside on_requst) - instead it quits after receiving
                        // the exit notification from the client, which is handled below
                        shutdown_req_received = on_request(&context, &request, ide_files_root.clone(), pkg_deps.clone(), shutdown_req_received);
                    }
                    Ok(Message::Response(response)) => on_response(&context, &response),
                    Ok(Message::Notification(notification)) => {
                        match notification.method.as_str() {
                            lsp_types::notification::Exit::METHOD => break,
                            lsp_types::notification::Cancel::METHOD => {
                                // TODO: Currently the server does not implement request cancellation.
                                // It ought to, especially once it begins processing requests that may
                                // take a long time to respond to.
                            }
                            _ => on_notification(ide_files_root.clone(), &symbolicator_runner, &notification),
                        }
                    }
                    Err(error) => eprintln!("IDE message error: {:?}", error),
                }
            }
        };
    }

    io_threads.join().expect("I/O threads could not finish");
    symbolicator_runner.quit();
    eprintln!("Shut down language server '{}'.", exe);
}

/// Read the *current* (possibly unsaved) text for `path` from the VFS,
/// falling back to disk. Unsaved edits live only in the VFS — reading
/// disk here would make code actions, formatting and the entry-param
/// filter act on stale text (wrong lines / missing `_` etc.).
fn read_doc_text(ide_files_root: &VfsPath, path: &Path) -> String {
    if let Ok(vfs_path) = ide_files_root.join(path.to_string_lossy())
        && let Ok(mut f) = vfs_path.open_file()
    {
        let mut buf = String::new();
        if f.read_to_string(&mut buf).is_ok() {
            return buf;
        }
    }
    std::fs::read_to_string(path).unwrap_or_default()
}

fn read_uri_text(ide_files_root: &VfsPath, uri: &lsp_types::Uri) -> String {
    match move_analyzer::utils::uri_to_file_path(uri) {
        Ok(p) => read_doc_text(ide_files_root, &p),
        Err(_) => String::new(),
    }
}

/// This function returns `true` if shutdown request has been received, and `false` otherwise.
/// The reason why this information is also passed as an argument is that according to the LSP
/// spec, if any additional requests are received after shutdownd then the LSP implementation
/// should respond with a particular type of error.
fn on_request(
    context: &Context,
    request: &Request,
    ide_files_root: VfsPath,
    pkg_dependencies: Arc<Mutex<BTreeMap<PathBuf, symbols::PrecompiledPkgDeps>>>,
    shutdown_request_received: bool,
) -> bool {
    if shutdown_request_received {
        let response = lsp_server::Response::new_err(
            request.id.clone(),
            lsp_server::ErrorCode::InvalidRequest as i32,
            "a shutdown request already received by the server".to_string(),
        );
        if let Err(err) = context
            .connection
            .sender
            .send(lsp_server::Message::Response(response))
        {
            eprintln!("could not send shutdown response: {:?}", err);
        }
        return true;
    }
    match request.method.as_str() {
        lsp_types::request::Completion::METHOD => {
            on_completion_request(context, request, ide_files_root.clone(), pkg_dependencies)
        }
        lsp_types::request::GotoDefinition::METHOD => {
            symbols::on_go_to_def_request(context, request, &context.symbols.lock().unwrap());
        }
        lsp_types::request::GotoTypeDefinition::METHOD => {
            symbols::on_go_to_type_def_request(context, request, &context.symbols.lock().unwrap());
        }
        lsp_types::request::References::METHOD => {
            symbols::on_references_request(context, request, &context.symbols.lock().unwrap());
        }
        lsp_types::request::HoverRequest::METHOD => {
            symbols::on_hover_request(context, request, &context.symbols.lock().unwrap());
        }
        lsp_types::request::DocumentSymbolRequest::METHOD => {
            symbols::on_document_symbol_request(context, request, &context.symbols.lock().unwrap());
        }
        lsp_types::request::CodeActionRequest::METHOD => {
            eprintln!(
                "kanari-move-analyzer: codeAction request {}",
                request.params
            );
            let params: lsp_types::CodeActionParams =
                serde_json::from_value(request.params.clone()).unwrap_or_else(|_| {
                    lsp_types::CodeActionParams {
                        work_done_progress_params: Default::default(),
                        partial_result_params: Default::default(),
                        text_document: lsp_types::TextDocumentIdentifier {
                            uri: "file:///".parse().unwrap(),
                        },
                        range: lsp_types::Range::new(
                            lsp_types::Position::new(0, 0),
                            lsp_types::Position::new(0, 0),
                        ),
                        context: lsp_types::CodeActionContext {
                            diagnostics: Vec::new(),
                            only: None,
                            trigger_kind: None,
                        },
                    }
                });
            let current_text = read_uri_text(&ide_files_root, &params.text_document.uri);
            let actions = kanari_move_analyzer::code_action::on_code_action_request_with_text(
                &params,
                Some(current_text),
            );
            let response = lsp_server::Response::new_ok(
                request.id.clone(),
                serde_json::to_value(actions).unwrap_or(serde_json::Value::Null),
            );
            let _ = context
                .connection
                .sender
                .send(lsp_server::Message::Response(response));
        }
        lsp_types::request::Formatting::METHOD => {
            let params: lsp_types::DocumentFormattingParams =
                serde_json::from_value(request.params.clone()).unwrap_or_else(|_| {
                    lsp_types::DocumentFormattingParams {
                        text_document: lsp_types::TextDocumentIdentifier {
                            uri: "file:///".parse().unwrap(),
                        },
                        options: lsp_types::FormattingOptions {
                            tab_size: 4,
                            insert_spaces: true,
                            properties: Default::default(),
                            trim_trailing_whitespace: None,
                            insert_final_newline: None,
                            trim_final_newlines: None,
                        },
                        work_done_progress_params: Default::default(),
                    }
                });
            let text = read_uri_text(&ide_files_root, &params.text_document.uri);
            let edits = kanari_move_analyzer::code_action::format_document(&params, &text);
            let response = lsp_server::Response::new_ok(
                request.id.clone(),
                serde_json::to_value(edits).unwrap_or(serde_json::Value::Null),
            );
            let _ = context
                .connection
                .sender
                .send(lsp_server::Message::Response(response));
        }
        lsp_types::request::CodeLensRequest::METHOD => {
            let params: lsp_types::CodeLensParams = serde_json::from_value(request.params.clone())
                .unwrap_or_else(|_| lsp_types::CodeLensParams {
                    text_document: lsp_types::TextDocumentIdentifier {
                        uri: "file:///".parse().unwrap(),
                    },
                    work_done_progress_params: Default::default(),
                    partial_result_params: Default::default(),
                });
            let text = read_uri_text(&ide_files_root, &params.text_document.uri);
            let lenses = kanari_move_analyzer::code_action::code_lenses(&params, &text);
            let response = lsp_server::Response::new_ok(
                request.id.clone(),
                serde_json::to_value(lenses).unwrap_or(serde_json::Value::Null),
            );
            let _ = context
                .connection
                .sender
                .send(lsp_server::Message::Response(response));
        }
        lsp_types::request::Shutdown::METHOD => {
            eprintln!("Shutdown request received");
            let response =
                lsp_server::Response::new_ok(request.id.clone(), serde_json::Value::Null);
            if let Err(err) = context
                .connection
                .sender
                .send(lsp_server::Message::Response(response))
            {
                eprintln!("could not send shutdown response: {:?}", err);
            }
            return true;
        }
        _ => eprintln!("handle request '{}' from client", request.method),
    }
    false
}

// (is_kanari_false_positive moved to kanari_move_analyzer::diagnostics)

fn on_response(_context: &Context, _response: &Response) {
    eprintln!("handle response from client");
}

fn on_notification(
    ide_files_root: VfsPath,
    symbolicator_runner: &symbols::SymbolicatorRunner,
    notification: &Notification,
) {
    match notification.method.as_str() {
        lsp_types::notification::DidOpenTextDocument::METHOD
        | lsp_types::notification::DidChangeTextDocument::METHOD
        | lsp_types::notification::DidSaveTextDocument::METHOD
        | lsp_types::notification::DidCloseTextDocument::METHOD => {
            on_text_document_sync_notification(ide_files_root, symbolicator_runner, notification)
        }
        _ => eprintln!("handle notification '{}' from client", notification.method),
    }
}
