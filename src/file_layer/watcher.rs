use std::{path::PathBuf, sync::Arc, time::Duration};

use notify::{Error, RecommendedWatcher, RecursiveMode, Watcher};
use notify_debouncer_full::{
    new_debouncer, DebounceEventResult, DebouncedEvent, Debouncer, FileIdMap,
};
use tokio::{
    fs,
    runtime::Handle,
    sync::{broadcast, mpsc::{channel, Receiver}},
};

pub(crate) async fn create_watcher(
    root: PathBuf,
) -> Result<
    (
        Debouncer<RecommendedWatcher, FileIdMap>,
        PathBuf,
        Receiver<Result<Vec<DebouncedEvent>, Vec<Error>>>,
    ),
    String,
> {
    let abs_root = match fs::canonicalize(&root).await {
        Ok(path) => path,
        Err(err) => {
            let err_msg = format!("Failed to get absolute path of {:?}: {}", root, err);
            log::error!("{}", err_msg);
            return Err(err_msg);
        }
    };
    match abs_root.clone().into_os_string().into_string() {
        Ok(path_str) => {
            log::info!("Listening on {}", path_str);
        }
        Err(_) => {
            let err_msg = format!("Failed to parse path to string for `{:?}`", abs_root);
            log::error!("{}", err_msg);
            return Err(err_msg);
        }
    };
    let rt = Handle::current();
    let (tx, rx) = channel::<Result<Vec<DebouncedEvent>, Vec<Error>>>(16);
    new_debouncer(
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
    .map(|d| (d, abs_root, rx))
    .map_err(|e| e.to_string())
}

pub async fn watch(
    root_path: PathBuf,
    mut debouncer: Debouncer<RecommendedWatcher, FileIdMap>,
    mut rx: Receiver<Result<Vec<DebouncedEvent>, Vec<Error>>>,
    tx: Arc<broadcast::Sender<()>>
) {
    debouncer
        .watcher()
        .watch(&root_path, RecursiveMode::Recursive)
        .unwrap();
    debouncer
        .cache()
        .add_root(&root_path, RecursiveMode::Recursive);

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
                                            strip_prefix(source_name, &root_path),
                                            strip_prefix(target_name, &root_path)
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
            if let Err(err) = tx.send(()) {
                log::error!("{:?}", err);
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
