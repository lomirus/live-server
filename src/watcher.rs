use std::{
    collections::HashMap,
    env::current_dir,
    path::{Path, PathBuf},
    sync::mpsc::channel,
    time::Duration,
};

use async_std::sync::Mutex;
use notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};
use once_cell::sync::Lazy;
use tide_websockets::{Message, WebSocketConnection};
use uuid::Uuid;

const BASE_URL: &str = "./public";

static WS_CLIENTS: Lazy<Mutex<HashMap<Uuid, WebSocketConnection>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn get_rltv_path(path: PathBuf) -> String {
    let prefix_len = current_dir()
        .expect("Failed to get current directory")
        .to_str()
        .expect("Failed to convert current directory to string")
        .len()
        + 3;
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
    println!("Watching files...");
    let (tx, rx) = channel();
    let mut watcher = watcher(tx, Duration::from_millis(100)).expect("Failed to create watcher");
    watcher
        .watch(Path::new(BASE_URL), RecursiveMode::Recursive)
        .unwrap();

    loop {
        use DebouncedEvent::*;
        let recv = rx.recv();
        match recv {
            Ok(event) => match event {
                Create(path) => {
                    let path = get_rltv_path(path);
                    println!("[CREATE] {:?}", path);
                    broadcast().await;
                }
                Write(path) => {
                    let path = get_rltv_path(path);
                    println!("[UPDATE] {:?}", path);
                    broadcast().await;
                }
                Remove(path) => {
                    let path = get_rltv_path(path);
                    println!("[REMOVE] {:?}", path);
                    broadcast().await;
                }
                Rename(from, to) => {
                    let from = get_rltv_path(from);
                    let to = get_rltv_path(to);
                    println!("[RENAME] {:?} -> {:?}", from, to);
                    broadcast().await;
                }
                Error(err, _) => println!("{}", err),
                _ => {}
            },
            Err(err) => println!("{}", err),
        }
    }
}
