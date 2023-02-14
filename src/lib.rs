mod server;
mod watcher;

use std::{collections::HashMap, sync::Arc};

use async_std::{path::PathBuf, sync::Mutex, task};

pub async fn run<R: Into<PathBuf>>(root: R, host: &str, port: u16) {
    let connections1 = Arc::new(Mutex::new(HashMap::new()));
    let connections2 = Arc::clone(&connections1);
    let root1: PathBuf = root.into();
    let root2: PathBuf = root1.clone();

    task::spawn(async move { watcher::watch(root1, &connections1).await });
    server::serve(host, port, root2, connections2).await;
}
