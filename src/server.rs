use std::collections::HashMap;
use std::sync::Arc;

use async_std::{fs, prelude::*, sync::Mutex};
use once_cell::sync::OnceCell;
use tide::{listener::Listener, Body, Request, Response, StatusCode};
use tide_websockets::{WebSocket, WebSocketConnection};
use uuid::Uuid;

use crate::PATH;

pub static SCRIPT: OnceCell<String> = OnceCell::new();

pub async fn serve(
    host: String,
    port: u16,
    connections: &Arc<Mutex<HashMap<Uuid, WebSocketConnection>>>,
) {
    let mut listener = create_listener(&host, port, connections).await;
    init_ws_script(host, port);

    listener.accept().await.unwrap();
}

async fn create_listener(
    host: &String,
    port: u16,
    connections: &Arc<Mutex<HashMap<Uuid, WebSocketConnection>>>,
) -> impl Listener<()> {
    let mut port = port;
    // Loop until the port is available
    loop {
        let app = create_server(Arc::clone(connections));
        match app.bind(format!("{host}:{port}")).await {
            Ok(listener) => {
                log::info!("Listening on http://{}:{}/", host, port);
                break listener;
            }
            Err(err) => {
                if let std::io::ErrorKind::AddrInUse = err.kind() {
                    log::warn!("Port {} is already in use", port);
                    port += 1;
                } else {
                    log::error!("Failed to listen on {}:{}: {}", host, port, err);
                }
            }
        }
    }
}

fn create_server(connections: Arc<Mutex<HashMap<Uuid, WebSocketConnection>>>) -> tide::Server<()> {
    let mut app = tide::new();
    app.at("/").get(static_assets);
    app.at("/*").get(static_assets);
    app.at("/asd").get(|_| async {
        let response: Response = Body::from("bytes").into();
        Ok(response)
    });
    app.at("/live-server-ws")
        .get(WebSocket::new(move |_request, mut stream| {
            let connections = Arc::clone(&connections);
            async move {
                let uuid = Uuid::new_v4();

                // Add the connection to clients when opening a new connection
                connections.lock().await.insert(uuid, stream.clone());

                // Waiting for the connection to be closed
                while let Some(Ok(_)) = stream.next().await {}

                // Remove the connection from clients when it is closed
                connections.lock().await.remove(&uuid);
                
                Ok(())
            }
        }));
    app
}

fn init_ws_script(host: String, port: u16) {
    let script = format!(include_str!("scripts/websocket.html"), host, port);
    SCRIPT.set(script).unwrap();
}

async fn static_assets(req: Request<()>) -> tide::Result {
    // Get the path and mime of the static file.
    let mut path = req.url().path().to_string();
    path = if path.ends_with('/') {
        format!("{}{}index.html", PATH.get().unwrap().display(), path)
    } else {
        format!("{}{}", PATH.get().unwrap().display(), path)
    };
    let mime = mime_guess::from_path(&path).first_or_text_plain();

    // Read the file.
    let mut file = match fs::read(&path).await {
        Ok(file) => file,
        Err(err) => {
            log::error!("{}", err);
            return Err(tide::Error::new(StatusCode::NotFound, err));
        }
    };

    // Construct the response.
    if mime == "text/html" {
        let text = match String::from_utf8(file) {
            Ok(text) => text,
            Err(err) => {
                log::error!("{}", err);
                return Err(tide::Error::from_str(StatusCode::InternalServerError, err));
            }
        };
        let script = SCRIPT.get().unwrap();
        file = format!("{text}{script}").into_bytes();
    }
    let mut response: Response = Body::from_bytes(file).into();
    response.set_content_type(mime.to_string().as_str());

    Ok(response)
}
