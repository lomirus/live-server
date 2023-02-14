mod server;
mod watcher;

use std::{collections::HashMap, sync::Arc};

use async_std::{path::PathBuf, sync::Mutex, task};
use local_ip_address::local_ip;

pub async fn listen<R: Into<PathBuf>>(host: &str, port: u16, root: R) {
    let connections1 = Arc::new(Mutex::new(HashMap::new()));
    let connections2 = Arc::clone(&connections1);
    let root1: PathBuf = root.into();
    let root2: PathBuf = root1.clone();

    task::spawn(async move { watcher::watch(root1, &connections1).await });
    server::serve(host, port, root2, connections2).await;
}

pub async fn listen_local<R: Into<PathBuf>>(port: u16, root: R) {
    match local_ip() {
        Err(err) => {
            log::error!(
                "Failed to get local IP address: {}. Using \"localhost\" by default",
                err
            );
        }
        Ok(addr) => {
            listen(&addr.to_string(), port, root).await;
        }
    };
}
