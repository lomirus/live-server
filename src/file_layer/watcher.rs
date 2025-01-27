use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use notify::{Error, RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{
    new_debouncer, DebounceEventResult, DebouncedEvent, Debouncer, RecommendedCache,
};
use tokio::{
    fs,
    runtime::Handle,
    sync::{
        broadcast,
        mpsc::{channel, Receiver},
    },
};

use crate::utils::is_ignored;

pub(crate) async fn create_watcher(
    root: &Path,
) -> Result<
    (
        Debouncer<RecommendedWatcher, RecommendedCache>,
        PathBuf,
        Receiver<Result<Vec<DebouncedEvent>, Vec<Error>>>,
    ),
    String,
> {
    let abs_root = match fs::canonicalize(&root).await {
        Ok(path) => path,
        Err(err) => {
            let err_msg = format!("Failed to get absolute path of {:?}: {}", root, err);
            log::error!("{err_msg}");
            return Err(err_msg);
        }
    };
    match abs_root.clone().into_os_string().into_string() {
        Ok(path_str) => {
            log::info!("Listening on {}", path_str);
        }
        Err(_) => {
            let err_msg = format!("Failed to parse path to string for `{:?}`", abs_root);
            log::error!("{err_msg}");
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
    mut debouncer: Debouncer<RecommendedWatcher, RecommendedCache>,
    mut rx: Receiver<Result<Vec<DebouncedEvent>, Vec<Error>>>,
    tx: Arc<broadcast::Sender<()>>,
    ignore_files: bool,
) {
    debouncer
        .watch(&root_path, RecursiveMode::Recursive)
        .unwrap();

    while let Some(result) = rx.recv().await {
        let mut files_changed = false;
        match result {
            Ok(events) => {
                for e in events {
                    if ignore_files {
                        match e
                            .paths
                            .iter()
                            .map(|p| is_ignored(&root_path, p, p.is_dir()))
                            .collect::<Result<Vec<_>, _>>()
                        {
                            Ok(ignored_list) => {
                                if ignored_list.iter().all(|ignored| *ignored) {
                                    log::debug!("Skipped ignored files: {:?}", e.paths);
                                    continue;
                                }
                            }
                            Err(err) => {
                                log::error!("Failed to check ignore files: {err}");
                                // Do nothing if we cannot know if it's an ignored entry
                                continue;
                            }
                        }
                    }
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
                                            strip_prefix(source_name, &root_path).display(),
                                            strip_prefix(target_name, &root_path).display(),
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

fn strip_prefix<'a>(path: &'a Path, prefix: &Path) -> &'a Path {
    path.strip_prefix(prefix).unwrap()
}
