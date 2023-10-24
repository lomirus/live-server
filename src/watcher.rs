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
        let mut files_changed = false;
        match result {
            Ok(events) => {
                for e in events {
                    use notify::EventKind::*;
                    match e.event.kind {
                        Create(_) => {
                            let path = e.event.paths[0].to_str().unwrap();
                            log::debug!("[CREATE] {}", path);
                            files_changed = true;
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
                                        files_changed = true;
                                    }
                                }
                                _ => {
                                    let paths = e.event.paths[0].to_str().unwrap();
                                    log::debug!("[UPDATE] {}", paths);
                                    files_changed = true;
                                }
                            }
                        }
                        Remove(_) => {
                            let paths = e.event.paths[0].to_str().unwrap();
                            log::debug!("[REMOVE] {}", paths);
                            files_changed = true;
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
        if files_changed {
            broadcast(connections).await;
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
