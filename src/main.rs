mod server;
mod watcher;

use async_std::{sync::Mutex, task};
use clap::Parser;
use local_ip_address::local_ip;
use std::{collections::HashMap, sync::Arc};

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
    /// Set the root path of the static assets
    #[clap(default_value = ".")]
    root: String,
}

#[async_std::main]
async fn main() {
    env_logger::init();

    let args = Args::parse();
    let host = match args.host {
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
    };

    run(args.root, host, args.port).await;
}

async fn run(root: String, host: String, port: u16) {
    let connections1 = Arc::new(Mutex::new(HashMap::new()));
    let connections2 = Arc::clone(&connections1);
    let root_clone = root.clone();

    task::spawn(async move { watcher::watch(root_clone, &connections1).await });
    server::serve(host, port, root, connections2).await;
}