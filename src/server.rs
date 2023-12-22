use std::fs;
use std::io::ErrorKind;

use axum::{
    body::Body,
    extract::{ws::Message, Request, WebSocketUpgrade},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    routing::get,
    Router,
};
use futures::{sink::SinkExt, stream::StreamExt};
use tokio::net::TcpListener;

use crate::{HOST, PORT, ROOT, TX};

pub async fn serve(port: u16) -> Result<(), String> {
    let listener = create_listener(port).await?;
    let app = create_server();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

async fn create_listener(port: u16) -> Result<TcpListener, String> {
    let host = HOST.get().unwrap();
    match tokio::net::TcpListener::bind(format!("{host}:{port}")).await {
        Ok(listener) => {
            let port = listener.local_addr().unwrap().port();
            log::info!("Listening on http://{}:{}/", host, port);
            PORT.set(port).unwrap();
            Ok(listener)
        }
        Err(err) => {
            let err_msg = if let std::io::ErrorKind::AddrInUse = err.kind() {
                format!("Port {} is already in use", port)
            } else {
                format!("Failed to listen on {}:{}: {}", host, port, err)
            };
            log::error!("{err_msg}");
            Err(err_msg)
        }
    }
}

fn create_server() -> Router {
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
    let host = HOST.get().unwrap();
    let port = PORT.get().unwrap();
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
                let script = format!(include_str!("templates/websocket.html"), host, port);
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
        let script = format!(include_str!("templates/websocket.html"), host, port);
        file = format!("{text}{script}").into_bytes();
    }

    (StatusCode::OK, headers, Body::from(file))
}
