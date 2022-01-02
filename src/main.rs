use std::{
    collections::HashMap,
    env::current_dir,
    fs,
    path::{Path, PathBuf},
    sync::mpsc::channel,
    thread,
    time::Duration,
};

use async_std::{prelude::*, sync::Mutex, task::block_on};
use html_editor::{parse, Editable, Htmlifiable, Node, Selector};
use notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};
use once_cell::sync::{Lazy, OnceCell};
use tide::{prelude::*, Request, Response, StatusCode};
use tide_websockets::{Message, WebSocket, WebSocketConnection};
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
static PORT: OnceCell<u16> = OnceCell::new();
static HOST: OnceCell<&str> = OnceCell::new();

fn get_rltv_path(path: PathBuf) -> String {
    let prefix_len = current_dir()
        .expect("Failed to get current directory")
        .to_str()
        .expect("Failed to convert current directory to string")
        .len()
        + 3;
    let path = path
        .to_str()
        .expect("Failed to convert the changed file/folder path to string");
    let path = path[prefix_len..].to_string();

    path
}

async fn broadcast() {
    for (_, conn) in WS_CLIENTS.lock().await.iter() {
        conn.send(Message::Text(String::new())).await.unwrap();
    }
}

async fn watch_files() {
    println!("Watching files...");
    let (tx, rx) = channel();
    let mut watcher = watcher(tx, Duration::from_millis(100)).expect("Failed to create watcher");
    watcher
        .watch(Path::new(BASE_URL), RecursiveMode::Recursive)
        .unwrap();

    loop {
        use DebouncedEvent::*;
        let recv = rx.recv();
        match recv {
            Ok(event) => match event {
                Create(path) => {
                    let path = get_rltv_path(path);
                    println!("[CREATE] {:?}", path);
                    broadcast().await;
                }
                Write(path) => {
                    let path = get_rltv_path(path);
                    println!("[UPDATE] {:?}", path);
                    broadcast().await;
                }
                Remove(path) => {
                    let path = get_rltv_path(path);
                    println!("[REMOVE] {:?}", path);
                    broadcast().await;
                }
                Rename(from, to) => {
                    let from = get_rltv_path(from);
                    let to = get_rltv_path(to);
                    println!("[RENAME] {:?} -> {:?}", from, to);
                    broadcast().await;
                }
                Error(err, _) => println!("{}", err),
                _ => {}
            },
            Err(err) => println!("{}", err),
        }
    }
}

async fn create_server() {
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

#[async_std::main]
async fn main() {
    HOST.set("127.0.0.1").unwrap();
    PORT.set(8080).unwrap();

    thread::spawn(|| block_on(watch_files()));
    create_server().await;
}
