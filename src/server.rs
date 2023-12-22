use std::io::ErrorKind;
use std::{fs, net::IpAddr};

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

use crate::{ADDR, ROOT, TX};

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
                .on_upgrade(|socket| async move {
                    let (mut sender, mut receiver) = socket.split();
                    let tx = TX.get().unwrap();
                    let mut rx = tx.subscribe();
                    let mut send_task = tokio::spawn(async move {
                        while rx.recv().await.is_ok() {
                            sender.send(Message::Text(String::new())).await.unwrap();
                        }
                    });
                    let mut recv_task =
                        tokio::spawn(
                            async move { while let Some(Ok(_)) = receiver.next().await {} },
                        );
                    tokio::select! {
                        _ = (&mut send_task) => recv_task.abort(),
                        _ = (&mut recv_task) => send_task.abort(),
                    };
                })
            }),
        )
}

async fn static_assets(req: Request<Body>) -> (StatusCode, HeaderMap, Body) {
    let addr = ADDR.get().unwrap();
    let root = ROOT.get().unwrap();

    // Get the path and mime of the static file.
    let mut path = req.uri().path().to_string();
    path.remove(0);
    let mut path = root.join(path);
    if path.is_dir() {
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
                let script = format!(include_str!("templates/websocket.html"), addr);
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
        let script = format!(include_str!("templates/websocket.html"), addr);
        file = format!("{text}{script}").into_bytes();
    }

    (StatusCode::OK, headers, Body::from(file))
}
