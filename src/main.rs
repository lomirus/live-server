mod server;
mod watcher;

use std::thread;
use async_std::task::block_on;
use once_cell::sync::OnceCell;

pub static PORT: OnceCell<u16> = OnceCell::new();
pub static HOST: OnceCell<&str> = OnceCell::new();

#[async_std::main]
async fn main() {
    HOST.set("127.0.0.1").unwrap();
    PORT.set(8080).unwrap();

    thread::spawn(|| block_on(watcher::watch()));
    server::serve().await;
}
