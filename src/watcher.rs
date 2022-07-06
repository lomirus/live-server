use colored::Colorize;
use notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};
use std::{env::current_dir, path::PathBuf, sync::mpsc::channel, time::Duration};
use tide_websockets::Message;

use crate::{log, WS_CLIENTS};

fn get_rltv_path(path: PathBuf) -> String {
    let prefix_len = current_dir()
        .expect("Failed to get current directory")
        .to_str()
        .expect("Failed to convert current directory to string")
        .len()
        + 1;
    let path = path
        .to_str()
        .expect("Failed to convert the changed file/folder path to string");

    path[prefix_len..].to_string()
}

async fn broadcast() {
    for (_, conn) in WS_CLIENTS.lock().await.iter() {
        conn.send(Message::Text(String::new())).await.unwrap();
    }
}

pub async fn watch() {
    println!(
        "Watcher listening on {}",
        current_dir()
            .unwrap()
            .into_os_string()
            .to_str()
            .unwrap()
            .blue()
    );
    let (tx, rx) = channel();
    let mut watcher = watcher(tx, Duration::from_millis(100)).unwrap();
    match watcher.watch(current_dir().unwrap(), RecursiveMode::Recursive) {
        Ok(_) => {}
        Err(err) => log::error!("Watcher: {}", err),
    }

    loop {
        use DebouncedEvent::*;
        let recv = rx.recv();
        match recv {
            Ok(event) => match event {
                Create(path) => {
                    let path = get_rltv_path(path);
                    log::info!("[CREATE] {:?}", path);
                    broadcast().await;
                }
                Write(path) => {
                    let path = get_rltv_path(path);
                    log::info!("[UPDATE] {:?}", path);
                    broadcast().await;
                }
                Remove(path) => {
                    let path = get_rltv_path(path);
                    log::info!("[REMOVE] {:?}", path);
                    broadcast().await;
                }
                Rename(from, to) => {
                    let from = get_rltv_path(from);
                    let to = get_rltv_path(to);
                    log::info!("[RENAME] {:?} -> {:?}", from, to);
                    broadcast().await;
                }
                Error(err, _) => log::error!("{}", err),
                _ => {}
            },
            Err(err) => log::error!("{}", err),
        }
    }
}
