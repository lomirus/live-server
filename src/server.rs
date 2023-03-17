use std::collections::HashMap;
use std::sync::Arc;

use async_std::{fs, path::PathBuf, prelude::*, sync::Mutex};
use tide::{listener::Listener, Body, Request, Response, StatusCode};
use tide_websockets::{WebSocket, WebSocketConnection};
use uuid::Uuid;

macro_rules! static_assets_service {
    ($app: expr, $route: expr, $host: ident, $port: ident, $root: ident) => {
        let host_clone = $host.to_string();
        let port_clone = $port;
        let root_clone = $root.clone();
        $app.at($route).get(move |req: Request<()>| {
            let host = host_clone.clone();
            let port = port_clone.clone();
            let root = root_clone.clone();
            static_assets(req, host, port, root)
        });
    };
}

pub async fn serve(
    host: &str,
    port: u16,
    root: PathBuf,
    connections: Arc<Mutex<HashMap<Uuid, WebSocketConnection>>>,
) -> Result<(), std::io::Error> {
    let mut listener = create_listener(host, port, &root, connections).await;
    listener.accept().await
}

async fn create_listener(
    host: &str,
    port: u16,
    root: &PathBuf,
    connections: Arc<Mutex<HashMap<Uuid, WebSocketConnection>>>,
) -> impl Listener<()> {
    let mut port = port;
    // Loop until the port is available
    loop {
        let app = create_server(host, port, root, Arc::clone(&connections));
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

fn create_server(
    host: &str,
    port: u16,
    root: &PathBuf,
    connections: Arc<Mutex<HashMap<Uuid, WebSocketConnection>>>,
) -> tide::Server<()> {
    let mut app = tide::new();

    static_assets_service!(app, "/", host, port, root);
    static_assets_service!(app, "/*", host, port, root);

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

async fn static_assets(
    req: Request<()>,
    host: String,
    port: u16,
    root: PathBuf,
) -> Result<Response, tide::Error> {
    // Get the path and mime of the static file.
    let mut path = req.url().path().to_string();
    path.remove(0);
    let mut path = root.join(path);
    if path.is_dir().await {
        path.push("index.html");
    }
    let mime = mime_guess::from_path(&path).first_or_text_plain();

    // Read the file.
    let mut file = match fs::read(&path).await {
        Ok(file) => file,
        Err(err) => {
            log::warn!("{}", err);
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
        let script = format!(include_str!("scripts/websocket.html"), host, port);
        file = format!("{text}{script}").into_bytes();
    }
    let mut response: Response = Body::from_bytes(file).into();
    response.set_content_type(mime.to_string().as_str());

    Ok(response)
}
