use colored::Colorize;
use notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};
use std::{env::current_dir, path::PathBuf, sync::mpsc::channel, time::Duration};
use tide_websockets::Message;

use crate::WS_CLIENTS;

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
    let path = path[prefix_len..].to_string();

    path
}

async fn broadcast() {
    for (_, conn) in WS_CLIENTS.lock().await.iter() {
        conn.send(Message::Text(String::new())).await.unwrap();
    }
}

pub async fn watch() {
    println!(
        "Watcher listening on {}",
        current_dir().unwrap().into_os_string().to_str().unwrap().blue()
    );
    let (tx, rx) = channel();
    let mut watcher = watcher(tx, Duration::from_millis(100)).unwrap();
    watcher
        .watch(current_dir().unwrap(), RecursiveMode::Recursive)
        .unwrap();

    loop {
        use DebouncedEvent::*;
        let recv = rx.recv();
        match recv {
            Ok(event) => match event {
                Create(path) => {
                    let path = get_rltv_path(path);
                    let info = format!("[CREATE] {:?}", path);
                    println!("{}", info.bright_black());
                    broadcast().await;
                }
                Write(path) => {
                    let path = get_rltv_path(path);
                    let info = format!("[UPDATE] {:?}", path);
                    println!("{}", info.bright_black());
                    broadcast().await;
                }
                Remove(path) => {
                    let path = get_rltv_path(path);
                    let info = format!("[REMOVE] {:?}", path);
                    println!("{}", info.bright_black());
                    broadcast().await;
                }
                Rename(from, to) => {
                    let from = get_rltv_path(from);
                    let to = get_rltv_path(to);
                    let info = format!("[RENAME] {:?} -> {:?}", from, to);
                    println!("{}", info.bright_black());
                    broadcast().await;
                }
                Error(err, _) => println!("{}", err.to_string().red()),
                _ => {}
            },
            Err(err) => println!("{}", err.to_string().red()),
        }
    }
}
