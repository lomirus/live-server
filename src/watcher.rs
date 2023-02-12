use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{mpsc::channel, Arc},
    time::Duration,
};

use async_std::sync::Mutex;
use notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};
use tide_websockets::{Message, WebSocketConnection};
use uuid::Uuid;

async fn broadcast(connections: &Arc<Mutex<HashMap<Uuid, WebSocketConnection>>>) {
    for (_, conn) in connections.lock().await.iter() {
        conn.send(Message::Text(String::new())).await.unwrap();
    }
}

pub async fn watch(root: String, connections: &Arc<Mutex<HashMap<Uuid, WebSocketConnection>>>) {
    let abs_root = match fs::canonicalize(root.clone()) {
        Ok(path) => path,
        Err(err) => {
            log::error!("Failed to get absolute path of `{}`: {}", root, err);
            return;
        }
    };
    let abs_root_str = match abs_root.clone().into_os_string().into_string() {
        Ok(path_str) => path_str,
        Err(_) => {
            log::error!("Failed to parse path to string for `{:?}`", abs_root);
            return;
        }
    };
    log::info!("Listening on {}", abs_root_str);
    let (tx, rx) = channel();
    let mut watcher = watcher(tx, Duration::from_millis(100)).unwrap();
    match watcher.watch(abs_root.clone(), RecursiveMode::Recursive) {
        Ok(_) => {}
        Err(err) => log::warn!("Watcher: {}", err),
    }

    loop {
        use DebouncedEvent::*;
        let recv = rx.recv();
        match recv {
            Ok(event) => match event {
                Create(path) => {
                    log::debug!("[CREATE] {}", strip_prefix(path, &abs_root));
                    broadcast(connections).await;
                }
                Write(path) => {
                    log::debug!("[UPDATE] {}", strip_prefix(path, &abs_root));
                    broadcast(connections).await;
                }
                Remove(path) => {
                    log::debug!("[REMOVE] {}", strip_prefix(path, &abs_root));
                    broadcast(connections).await;
                }
                Rename(from, to) => {
                    log::debug!(
                        "[RENAME] {} -> {}",
                        strip_prefix(from, &abs_root),
                        strip_prefix(to, &abs_root)
                    );
                    broadcast(connections).await;
                }
                Error(err, _) => log::error!("{}", err),
                _ => {}
            },
            Err(err) => log::error!("{}", err),
        }
    }
}

fn strip_prefix(path: PathBuf, prefix: &Path) -> String {
    path.strip_prefix(prefix)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
}
