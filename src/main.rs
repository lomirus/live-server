mod server;
mod watcher;

use async_std::{task::block_on, sync::Mutex};
use clap::Parser;
use once_cell::sync::{OnceCell, Lazy};
use tide_websockets::WebSocketConnection;
use uuid::Uuid;
use std::{thread, collections::HashMap};

#[derive(Parser)]
struct Args {
    #[clap(short, long, default_value_t = 8000)]
    port: u16,
}

pub static SCRIPT: OnceCell<String> = OnceCell::new();
pub static PORT: OnceCell<u16> = OnceCell::new();
pub static HOST: OnceCell<&str> = OnceCell::new();
pub static WS_CLIENTS: Lazy<Mutex<HashMap<Uuid, WebSocketConnection>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[async_std::main]
async fn main() {
    let args = Args::parse();

    HOST.set("127.0.0.1").unwrap();
    PORT.set(args.port).unwrap();
    SCRIPT.set(format!(r#"
        const ws = new WebSocket("ws://localhost:{}/live-server-ws");
        ws.onopen = () => console.log("[Live Server] Connection Established");
        ws.onmessage = () => location.reload();
        ws.onclose = () => console.log("[Live Server] Connection Closed");
    "#, PORT.get().unwrap())).unwrap();

    thread::spawn(|| block_on(watcher::watch()));
    server::serve().await;
}
