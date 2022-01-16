mod server;
mod watcher;

use async_std::{sync::Mutex, task::block_on};
use clap::Parser;
use html_editor::Node;
use once_cell::sync::{Lazy, OnceCell};
use std::{collections::HashMap, thread};
use tide_websockets::WebSocketConnection;
use uuid::Uuid;

#[derive(Parser)]
struct Args {
    #[clap(short, long, default_value_t = 8000)]
    port: u16,
}

pub static SCRIPT: OnceCell<Node> = OnceCell::new();
pub static PORT: OnceCell<u16> = OnceCell::new();
pub static WS_CLIENTS: Lazy<Mutex<HashMap<Uuid, WebSocketConnection>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[async_std::main]
async fn main() {
    let args = Args::parse();

    PORT.set(args.port).unwrap();
    SCRIPT
        .set({
            let script = format!(
                r#"
                    const ws = new WebSocket("ws://localhost:{}/live-server-ws");
                    ws.onopen = () => console.log("[Live Server] Connection Established");
                    ws.onmessage = () => location.reload();
                    ws.onclose = () => console.log("[Live Server] Connection Closed");
                "#,
                PORT.get().unwrap()
            );
            Node::new_element("script", vec![], vec![Node::Text(script)])
        })
        .unwrap();

    thread::spawn(|| block_on(watcher::watch()));
    server::serve().await;
}
