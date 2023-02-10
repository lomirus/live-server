mod server;
mod watcher;

use async_std::{sync::Mutex, task};
use clap::Parser;
use local_ip_address::local_ip;
use once_cell::sync::OnceCell;
use std::{collections::HashMap, path::PathBuf, sync::Arc};

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

pub(crate) static PATH: OnceCell<PathBuf> = OnceCell::new();

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

    let connections1 = Arc::new(Mutex::new(HashMap::new()));
    let connections2 = Arc::clone(&connections1);

    task::spawn(async move { watcher::watch(args.path, &connections1).await });
    server::serve(host, args.port, &connections2).await;
}
