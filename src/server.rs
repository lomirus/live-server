use crate::{HOST, PORT};
use async_std::{prelude::*, sync::Mutex};
use html_editor::{parse, Editable, Htmlifiable, Node, Selector};
use once_cell::sync::Lazy;
use std::{collections::HashMap, fs};
use tide::{prelude::*, Request, Response, StatusCode};
use tide_websockets::{WebSocket, WebSocketConnection};
use uuid::Uuid;

const BASE_URL: &str = "./public";
const SCRIPT: &str = r#"
    const ws = new WebSocket("ws://localhost:8080/live-server-ws");
    ws.onopen = () => console.log("[Live Server] Connection Established");
    ws.onmessage = () => location.reload();
    ws.onclose = () => console.log("[Live Server] Connection Closed");
"#;

static WS_CLIENTS: Lazy<Mutex<HashMap<Uuid, WebSocketConnection>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub async fn serve() {
    let host = HOST.get().unwrap();
    let port = PORT.get().unwrap();
    let mut app = tide::new();
    app.at("/").get(static_assets);
    app.at("/*").get(static_assets);
    app.at("/live-server-ws")
        .get(WebSocket::new(|_request, mut stream| async move {
            let uuid = Uuid::new_v4();
            // Add the connection to clients when opening a new connection
            WS_CLIENTS.lock().await.insert(uuid, stream.clone());
            // Waiting for the connection to be closed
            while let Some(Ok(_)) = stream.next().await {}
            // Remove the connection from clients when it is closed
            WS_CLIENTS.lock().await.remove(&uuid);
            Ok(())
        }));
    let mut listener = app
        .bind(format!("{}:{}", host, port))
        .await
        .expect("Failed to bind host and port");
    println!("Server listening on http://{}:{}/", host, port);
    listener.accept().await.unwrap();
}

async fn static_assets(req: Request<()>) -> tide::Result {
    let path = req.url().path();
    let path = match path {
        "/" => "/index.html",
        _ => path,
    };
    let path = format!("{}{}", BASE_URL, path);

    let head_selector = Selector::from("head");
    let script = Node::new_element("script", vec![], vec![Node::Text(SCRIPT.to_string())]);

    let file = match fs::read(&path) {
        Ok(file) => file,
        Err(err) => {
            eprintln!(r#"[ERROR] Not Found: "{}""#, path);
            return Err(tide::Error::new(StatusCode::NotFound, err));
        }
    };
    let html: String = String::from_utf8_lossy(&file).parse::<String>()?;
    let html = parse(html.as_str())
        .insert_to(&head_selector, script)
        .html();
    let mut response: Response = format!("{}\n", html).into();
    response.set_content_type("text/html; charset=utf-8");
    Ok(response)
}
