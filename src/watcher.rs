use std::time::Duration;

use notify::{Error, RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, DebounceEventResult, DebouncedEvent};
use tokio::{fs, runtime::Handle, sync::mpsc::channel};

use crate::{ROOT, TX};

async fn broadcast() {
    let tx = TX.get().unwrap();
    let _ = tx.send(());
}

pub async fn watch() {
    let root = ROOT.get().unwrap();
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
    let rt = Handle::current();
    let (tx, mut rx) = channel::<Result<Vec<DebouncedEvent>, Vec<Error>>>(16);
    let mut debouncer = new_debouncer(
        Duration::from_millis(200),
        None,
        move |result: DebounceEventResult| {
            let tx = tx.clone();
            rt.spawn(async move {
                if let Err(err) = tx.send(result).await {
                    log::error!("Failed to send event result: {}", err);
                }
            });
        },
    )
    .unwrap();
    debouncer
        .watcher()
        .watch(&abs_root, RecursiveMode::Recursive)
        .unwrap();
    debouncer
        .cache()
        .add_root(&abs_root, RecursiveMode::Recursive);

    while let Some(result) = rx.recv().await {
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
                                            strip_prefix(source_name, &abs_root),
                                            strip_prefix(target_name, &abs_root)
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
            broadcast().await;
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
