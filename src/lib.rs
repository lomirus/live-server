mod server;
mod watcher;

use std::{collections::HashMap, sync::Arc};

use async_std::{sync::Mutex, task};

pub async fn run(root: String, host: String, port: u16) {
    let connections1 = Arc::new(Mutex::new(HashMap::new()));
    let connections2 = Arc::clone(&connections1);
    let root_clone = root.clone();

    task::spawn(async move { watcher::watch(root_clone, &connections1).await });
    server::serve(host, port, root, connections2).await;
}
