use std::{
    collections::HashMap,
    sync::{mpsc::channel, Arc},
    time::Duration,
};

use async_std::{fs, path::PathBuf, sync::Mutex};
use notify::{RecursiveMode, Watcher};
use notify_debouncer_full::new_debouncer;
use tide_websockets::{Message, WebSocketConnection};
use uuid::Uuid;

async fn broadcast(connections: &Arc<Mutex<HashMap<Uuid, WebSocketConnection>>>) {
    for (_, conn) in connections.lock().await.iter() {
        conn.send(Message::Text(String::new())).await.unwrap();
    }
}

pub async fn watch(root: PathBuf, connections: &Arc<Mutex<HashMap<Uuid, WebSocketConnection>>>) {
    let abs_root = match fs::canonicalize(&root).await {
        Ok(path) => path,
        Err(err) => {
            log::error!("Failed to get absolute path of {:?}: {}", root, err);
            return;
        }
    };
    match abs_root.clone().into_os_string().into_string() {
        Ok(path_str) => {
            log::info!("Listening on {}", path_str);
        }
        Err(_) => {
            log::error!("Failed to parse path to string for `{:?}`", abs_root);
            return;
        }
    };

    let (tx, rx) = channel();
    let mut debouncer = new_debouncer(Duration::from_millis(200), None, tx).unwrap();
    let watched_path: std::path::PathBuf = abs_root.into();
    debouncer
        .watcher()
        .watch(watched_path.as_path(), RecursiveMode::Recursive)
        .unwrap();
    debouncer
        .cache()
        .add_root(watched_path.as_path(), RecursiveMode::Recursive);

    for result in rx {
        match result {
            Ok(events) => {
                println!("test");
                for e in events {
                    use notify::EventKind::*;
                    match e.event.kind {
                        Create(_) => {
                            let path = e.event.paths[0].to_str().unwrap();
                            log::debug!("[CREATE] {}", path);
                            broadcast(connections).await;
                        }
                        Modify(kind) => {
                            use notify::event::ModifyKind::*;
                            match kind {
                                Name(kind) => {
                                    use notify::event::RenameMode::*;
                                    if let Both = kind {
                                        let source_name = &e.event.paths[0];
                                        let target_name = &e.event.paths[1];
                                        log::debug!(
                                            "[RENAME] {} -> {}",
                                            strip_prefix(source_name, &watched_path),
                                            strip_prefix(target_name, &watched_path)
                                        );
                                        broadcast(connections).await;
                                    }
                                }
                                _ => {
                                    let paths = e.event.paths[0].to_str().unwrap();
                                    log::debug!("[UPDATE] {}", paths);
                                    broadcast(connections).await;
                                }
                            }
                        }
                        Remove(_) => {
                            let paths = e.event.paths[0].to_str().unwrap();
                            log::debug!("[REMOVE] {}", paths);
                            broadcast(connections).await;
                        }
                        _ => {}
                    }
                }
            }
            Err(errors) => {
                for err in errors {
                    log::error!("{}", err);
                }
            }
        }
    }
}

fn strip_prefix(path: &std::path::Path, prefix: &std::path::PathBuf) -> String {
    path.strip_prefix(prefix)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
}
