use crate::{HOST, PORT, SCRIPT};
use async_std::{prelude::*, sync::Mutex};
use html_editor::{parse, Editable, Htmlifiable, Node, Selector};
use once_cell::sync::Lazy;
use std::{collections::HashMap, fs};
use tide::{prelude::*, Request, Response, StatusCode};
use tide_websockets::{WebSocket, WebSocketConnection};
use uuid::Uuid;

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
    println!(" Server listening on http://{}:{}/", host, port);
    listener.accept().await.unwrap();
}

async fn static_assets(req: Request<()>) -> tide::Result {
    let mut path = req.url().path().to_string();
    if path.ends_with("/") {
        path = format!(".{}index.html", path);
    } else {
        path = format!(".{}", path);
    }

    let head_selector = Selector::from("head");
    let script = SCRIPT.get().unwrap().to_string();
    let script = Node::new_element("script", vec![], vec![Node::Text(script)]);

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
