mod server;
mod watcher;

use async_std::{sync::Mutex, task::block_on};
use clap::Parser;
use local_ip_address::local_ip;
use once_cell::sync::{Lazy, OnceCell};
use std::{collections::HashMap, path::PathBuf, thread};
use tide_websockets::WebSocketConnection;
use uuid::Uuid;

/// Launch a local network server with live reload feature for static pages.
#[derive(Parser)]
#[clap(version)]
struct Args {
    /// Set the listener port
    #[clap(short, long, default_value_t = 8000)]
    port: u16,
    /// Set the listener host [default: LAN IP address]
    #[clap(short, long)]
    host: Option<String>,
    /// Set the path of the static assets
    #[clap(default_value = ".")]
    path: String,
}

pub(crate) static HOST: OnceCell<String> = OnceCell::new();
pub(crate) static PATH: OnceCell<PathBuf> = OnceCell::new();
pub(crate) static WS_CLIENTS: Lazy<Mutex<HashMap<Uuid, WebSocketConnection>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[async_std::main]
async fn main() {
    let args = Args::parse();
    HOST.set({
        match args.host {
            Some(host) => host,
            None => match local_ip() {
                Err(err) => {
                    log::error!(
                        "Failed to get local IP address: {}. Using \"localhost\" by default",
                        err
                    );
                    "localhost".to_string()
                }
                Ok(addr) => addr.to_string(),
            },
        }
    })
    .unwrap();

    thread::spawn(|| block_on(watcher::watch(args.path)));
    server::serve(args.port).await;
}
