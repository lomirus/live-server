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
use std::{fs, io::ErrorKind, sync::Arc};
use tokio::net::TcpListener;

use crate::{ADDR, ROOT, TX};

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
}

impl Default for Options {
    fn default() -> Self {
        Self {
            hard_reload: false,
            index_listing: true,
        }
    }
}

pub(crate) fn create_server(options: Options) -> Router {
    Router::new()
        .route("/", get(static_assets))
        .route("/*path", get(static_assets))
        .route(
            "/live-server-ws",
            get(|ws: WebSocketUpgrade| async move {
                ws.on_failed_upgrade(|error| {
                    log::error!("Failed to upgrade websocket: {}", error);
                })
                .on_upgrade(on_websocket_upgrade)
            }),
        )
        .with_state(Arc::new(options))
}

async fn on_websocket_upgrade(socket: WebSocket) {
    let (mut sender, mut receiver) = socket.split();
    let tx = TX.get().unwrap();
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

fn get_index_listing(uri_path: &str) -> String {
    let is_root = uri_path == "/";
    let path = ROOT.get().unwrap().join(&uri_path[1..]);
    let entries = fs::read_dir(path).unwrap();
    let mut entry_names = entries
        .into_iter()
        .filter_map(|e| {
            e.ok().and_then(|entry| {
                let is_dir = entry.metadata().ok()?.is_dir();
                let trailing = if is_dir { "/" } else { "" };
                entry
                    .file_name()
                    .to_str()
                    .map(|name| format!("{name}{trailing}"))
            })
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
    state: State<Arc<Options>>,
    req: Request<Body>,
) -> (StatusCode, HeaderMap, Body) {
    let addr = ADDR.get().unwrap();
    let root = ROOT.get().unwrap();

    let is_reload = req.uri().query().is_some_and(|x| x == "reload");

    // Get the path and mime of the static file.
    let uri_path = req.uri().path();
    let mut path = root.join(&uri_path[1..]);
    let mut reading_index = false;
    if path.is_dir() {
        if !uri_path.ends_with('/') {
            // redirect so parent links work correctly
            let redirect = format!("{}/", uri_path);
            let mut headers = HeaderMap::new();
            headers.append(header::LOCATION, HeaderValue::from_str(&redirect).unwrap());
            return (StatusCode::TEMPORARY_REDIRECT, headers, Body::empty());
        }
        path.push("index.html");
        reading_index = true;
    }
    let mime = mime_guess::from_path(&path).first_or_text_plain();

    let mut headers = HeaderMap::new();
    headers.append(
        header::CONTENT_TYPE,
        HeaderValue::from_str(mime.as_ref()).unwrap(),
    );

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
                    if state.index_listing && reading_index {
                        let script = format_script(addr, state.hard_reload, is_reload, false);
                        let html = format!(
                            include_str!("../templates/index.html"),
                            uri_path,
                            script,
                            get_index_listing(uri_path)
                        );
                        let body = Body::from(html);
                        return (StatusCode::OK, headers, body);
                    }
                    StatusCode::NOT_FOUND
                }
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            if mime == "text/html" {
                let script = format_script(addr, state.hard_reload, is_reload, true);
                let html = format!(include_str!("../templates/error.html"), script, err);
                let body = Body::from(html);

                return (status_code, headers, body);
            }
            return (status_code, headers, Body::empty());
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
        let script = format_script(addr, state.hard_reload, is_reload, false);
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
fn format_script(addr: &str, hard_reload: bool, is_reload: bool, is_error: bool) -> String {
    match (is_reload, is_error) {
        // successful reload, inject the reload payload
        (true, false) => format!("<script>{}</script>", RELOAD_PAYLOAD),
        // failed reload, don't inject anything so the client polls again
        (true, true) => String::new(),
        // normal connection, inject the websocket client
        _ => {
            let hard = if hard_reload { "true" } else { "false" };
            format!(
                r#"<script>{}("{}", {})</script>"#,
                WEBSOCKET_FUNCTION, addr, hard
            )
        }
    }
}
