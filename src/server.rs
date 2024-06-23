use std::io::ErrorKind;
use std::{fs, net::IpAddr};

use axum::extract::ws::WebSocket;
use axum::{
    body::Body,
    extract::{ws::Message, Request, WebSocketUpgrade},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    routing::get,
    Router,
};
use futures::{sink::SinkExt, stream::StreamExt};
use local_ip_address::local_ip;
use tokio::net::TcpListener;

use crate::{ADDR, HARD, ROOT, TX};

pub(crate) async fn serve(tcp_listener: TcpListener, router: Router) {
    axum::serve(tcp_listener, router).await.unwrap();
}

pub(crate) async fn create_listener(addr: String) -> Result<TcpListener, String> {
    match tokio::net::TcpListener::bind(&addr).await {
        Ok(listener) => {
            let port = listener.local_addr().unwrap().port();
            let host = listener.local_addr().unwrap().ip();
            let host = match host.is_unspecified() {
                true => match local_ip() {
                    Ok(addr) => addr,
                    Err(err) => {
                        log::warn!("Failed to get local IP address: {}", err);
                        host
                    }
                },
                false => host,
            };

            let addr = match host {
                IpAddr::V4(host) => format!("{host}:{port}"),
                IpAddr::V6(host) => format!("[{host}]:{port}"),
            };
            log::info!("Listening on http://{addr}/");
            ADDR.set(addr).unwrap();
            Ok(listener)
        }
        Err(err) => {
            let err_msg = if let std::io::ErrorKind::AddrInUse = err.kind() {
                format!("Address {} is already in use", &addr)
            } else {
                format!("Failed to listen on {}: {}", addr, err)
            };
            log::error!("{err_msg}");
            Err(err_msg)
        }
    }
}

pub(crate) fn create_server() -> Router {
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

async fn static_assets(req: Request<Body>) -> (StatusCode, HeaderMap, Body) {
    let addr = ADDR.get().unwrap();
    let root = ROOT.get().unwrap();

    let is_reload = req.uri().query().is_some_and(|x| x == "reload");

    // Get the path and mime of the static file.
    let uri_path = req.uri().path();
    let mut path = root.join(&uri_path[1..]);
    if path.is_dir() {
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

    // Read the file.
    let mut file = match fs::read(&path) {
        Ok(file) => file,
        Err(err) => {
            match path.to_str() {
                Some(path) => log::warn!("Failed to read \"{}\": {}", path, err),
                None => log::warn!("Failed to read file with invalid path: {}", err),
            }
            let status_code = match err.kind() {
                ErrorKind::NotFound => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            if mime == "text/html" {
                let script = format_script(addr, is_reload, true);
                let html = format!(include_str!("templates/error.html"), script, err);
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
        let script = format_script(addr, is_reload, false);
        file = format!("{text}{script}").into_bytes();
    } else if !HARD.get().copied().unwrap_or(false) {
        // allow client to cache assets for a smoother reload.
        // client handles preloading to refresh cache before reloading.
        headers.append(
            header::CACHE_CONTROL,
            HeaderValue::from_str("max-age=30").unwrap(),
        );
    }

    (StatusCode::OK, headers, Body::from(file))
}

/// JS script containing a function that takes in the address and connects to the websocket.
const WEBSOCKET_FUNCTION: &str = include_str!("templates/websocket.js");

/// JS script to inject to the HTML on reload so the client
/// knows it's a successful reload.
const RELOAD_PAYLOAD: &str = include_str!("templates/reload.js");

/// Inject the address into the websocket script and wrap it in a script tag
fn format_script(addr: &str, is_reload: bool, is_error: bool) -> String {
    match (is_reload, is_error) {
        // successful reload, inject the reload payload
        (true, false) => format!("<script>{}</script>", RELOAD_PAYLOAD),
        // failed reload, don't inject anything so the client polls again
        (true, true) => String::new(),
        // normal connection, inject the websocket client
        _ => {
            let hard = if HARD.get().copied().unwrap_or(false) {
                "true"
            } else {
                "false"
            };
            format!(
                r#"<script>{}("{}", {})</script>"#,
                WEBSOCKET_FUNCTION, addr, hard
            )
        }
    }
}
