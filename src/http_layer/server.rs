use axum::{
    body::Body,
    extract::{
        ws::{Message, WebSocket},
        Request, State, WebSocketUpgrade,
    },
    http::{header, HeaderMap, HeaderValue, StatusCode},
    routing::get,
    Router,
};
use futures::{sink::SinkExt, stream::StreamExt};
use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{net::TcpListener, sync::broadcast};

use crate::{http_layer::template::{error_html, index_html}, utils::is_ignored};

/// JS script containing a function that takes in the address and connects to the websocket.
const WEBSOCKET_FUNCTION: &str = include_str!("../templates/websocket.js");

/// JS script to inject to the HTML on reload so the client
/// knows it's a successful reload.
const RELOAD_PAYLOAD: &str = include_str!("../templates/reload.js");

pub(crate) async fn serve(tcp_listener: TcpListener, router: Router) {
    axum::serve(tcp_listener, router).await.unwrap();
}

pub struct Options {
    /// Always hard reload the page instead of hot-reload
    pub hard_reload: bool,
    /// Show page list of the current URL if `index.html` does not exist
    pub index_listing: bool,
    /// Ignore hidden and ignored files
    pub auto_ignore: bool,
}

pub(crate) struct AppState {
    /// Always hard reload the page instead of hot-reload
    pub(crate) hard_reload: bool,
    /// Show page list of the current URL if `index.html` does not exist
    pub(crate) index_listing: bool,
    /// Ignore hidden and ignored files
    pub(crate) auto_ignore: bool,
    pub(crate) tx: Arc<broadcast::Sender<()>>,
    pub(crate) root: PathBuf,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            hard_reload: false,
            index_listing: true,
            auto_ignore: false,
        }
    }
}

pub(crate) fn create_server(state: AppState) -> Router {
    let tx = state.tx.clone();
    Router::new()
        .route("/", get(static_assets))
        .route("/*path", get(static_assets))
        .route(
            "/live-server-ws",
            get(|ws: WebSocketUpgrade| async move {
                ws.on_failed_upgrade(|error| {
                    log::error!("Failed to upgrade websocket: {}", error);
                })
                .on_upgrade(|socket: WebSocket| on_websocket_upgrade(socket, tx))
            }),
        )
        .with_state(Arc::new(state))
}

async fn on_websocket_upgrade(socket: WebSocket, tx: Arc<broadcast::Sender<()>>) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = tx.subscribe();
    let mut send_task = tokio::spawn(async move {
        while rx.recv().await.is_ok() {
            sender.send(Message::Text(String::new())).await.unwrap();
        }
    });
    let mut recv_task =
        tokio::spawn(async move { while let Some(Ok(_)) = receiver.next().await {} });
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };
}

fn get_index_listing(uri_path: &str, root: &Path, auto_ignore: bool) -> String {
    let is_root = uri_path == "/";
    let path = root.join(&uri_path[1..]);
    let entries = fs::read_dir(path).unwrap();
    let mut entry_names = entries
        .into_iter()
        .filter_map(|e| {
            if let Ok(entry) = e {
                if auto_ignore {
                    match is_ignored(root, &entry.path()) {
                        Ok(ignored) => {
                            if ignored {
                                return None;
                            }
                        }
                        Err(err) => {
                            log::error!("Failed to check ignore files: {err}");
                            // Do nothing if we cannot know if it's an ignored entry
                            return None;
                        }
                    }
                }
                let is_dir = entry.metadata().ok()?.is_dir();
                let trailing = if is_dir { "/" } else { "" };
                entry
                    .file_name()
                    .to_str()
                    .map(|name| format!("{name}{trailing}"))
            } else {
                None
            }
        })
        .collect::<Vec<String>>();
    entry_names.sort();
    if !is_root {
        entry_names.insert(0, "..".to_string());
    }
    entry_names
        .into_iter()
        .map(|en| format!("<li><a href=\"{en}\">{en}</a></li>"))
        .collect::<Vec<String>>()
        .join("\n")
}

async fn static_assets(
    state: State<Arc<AppState>>,
    req: Request<Body>,
) -> (StatusCode, HeaderMap, Body) {
    let is_reload = req.uri().query().is_some_and(|x| x == "reload");

    // Get the path and mime of the static file.
    let uri_path = req.uri().path();
    let mut path = state.root.join(&uri_path[1..]);
    let is_accessing_dir = path.is_dir();
    if is_accessing_dir {
        if !uri_path.ends_with('/') {
            // redirect so parent links work correctly
            let redirect = format!("{}/", uri_path);
            let mut headers = HeaderMap::new();
            headers.append(header::LOCATION, HeaderValue::from_str(&redirect).unwrap());
            return (StatusCode::TEMPORARY_REDIRECT, headers, Body::empty());
        }
        path.push("index.html");
    }
    let mime = mime_guess::from_path(&path).first_or_text_plain();

    let mut headers = HeaderMap::new();
    headers.append(
        header::CONTENT_TYPE,
        HeaderValue::from_str(mime.as_ref()).unwrap(),
    );

    if state.auto_ignore {
        match is_ignored(&state.root, &path) {
            Ok(ignored) => {
                if ignored {
                    let err_msg =
                        "Unable to access ignored or hidden file, because `--ignore` is enabled";
                    let body = generate_error_body(err_msg, state.hard_reload, is_reload);

                    return (StatusCode::FORBIDDEN, HeaderMap::new(), body);
                }
            }
            Err(err) => {
                let err_msg = format!("Failed to check ignore files: {err}");
                let body = generate_error_body(&err_msg, state.hard_reload, is_reload);
                log::error!("{err_msg}");

                return (StatusCode::INTERNAL_SERVER_ERROR, HeaderMap::new(), body);
            }
        }
    }

    // Read the file.
    let mut file = match fs::read(&path) {
        Ok(file) => file,
        Err(err) => {
            match path.to_str() {
                Some(path) => log::warn!("Failed to read \"{}\": {}", path, err),
                None => log::warn!("Failed to read file with invalid path: {}", err),
            }
            let status_code = match err.kind() {
                ErrorKind::NotFound => {
                    if state.index_listing && is_accessing_dir {
                        let script = format_script(state.hard_reload, is_reload, false);
                        let html = index_html(
                            uri_path,
                            &script,
                            &get_index_listing(uri_path, &state.root, state.auto_ignore),
                        );
                        let body = Body::from(html);
                        return (StatusCode::OK, headers, body);
                    }
                    StatusCode::NOT_FOUND
                }
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            return (
                status_code,
                headers,
                if mime == "text/html" {
                    generate_error_body(&err.to_string(), state.hard_reload, is_reload)
                } else {
                    Body::empty()
                },
            );
        }
    };

    // Construct the response.
    if mime == "text/html" {
        let text = match String::from_utf8(file) {
            Ok(text) => text,
            Err(err) => {
                log::error!("{}", err);
                let body = Body::from(err.to_string());
                return (StatusCode::INTERNAL_SERVER_ERROR, headers, body);
            }
        };
        let script = format_script(state.hard_reload, is_reload, false);
        file = format!("{text}{script}").into_bytes();
    } else if state.hard_reload {
        // allow client to cache assets for a smoother reload.
        // client handles preloading to refresh cache before reloading.
        headers.append(
            header::CACHE_CONTROL,
            HeaderValue::from_str("max-age=30").unwrap(),
        );
    }

    (StatusCode::OK, headers, Body::from(file))
}

/// Inject the address into the websocket script and wrap it in a script tag
fn format_script(hard_reload: bool, is_reload: bool, is_error: bool) -> String {
    match (is_reload, is_error) {
        // successful reload, inject the reload payload
        (true, false) => format!("<script>{}</script>", RELOAD_PAYLOAD),
        // failed reload, don't inject anything so the client polls again
        (true, true) => String::new(),
        // normal connection, inject the websocket client
        _ => {
            let hard = if hard_reload { "true" } else { "false" };
            format!(r#"<script>{WEBSOCKET_FUNCTION}({hard})</script>"#)
        }
    }
}

fn generate_error_body(err_msg: &str, hard_reload: bool, is_reload: bool) -> Body {
    let script = format_script(hard_reload, is_reload, true);
    let html = error_html(&script, err_msg);
    Body::from(html)
}
