mod server;
mod watcher;

use async_std::{sync::Mutex, task::block_on};
use clap::Parser;
use once_cell::sync::{Lazy, OnceCell};
use std::{collections::HashMap, thread};
use tide_websockets::WebSocketConnection;
use uuid::Uuid;

/// Launch a local network server with live reload feature for static pages.
#[derive(Parser)]
#[clap(version)]
struct Args {
    /// Set the listener port 
    #[clap(short, long, default_value_t = 8000)]
    port: u16,
    /// Set the listener host, otherwise it will be set to the local IP address
    #[clap(short, long)]
    host: Option<String>,
}

pub static PORT: OnceCell<u16> = OnceCell::new();
pub static HOST: OnceCell<Option<String>> = OnceCell::new();
pub static WS_CLIENTS: Lazy<Mutex<HashMap<Uuid, WebSocketConnection>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[async_std::main]
async fn main() {
    let args = Args::parse();

    PORT.set(args.port).unwrap();
    HOST.set(args.host).unwrap();

    thread::spawn(|| block_on(watcher::watch()));
    server::serve().await;
}
