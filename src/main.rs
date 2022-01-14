mod server;
mod watcher;

use async_std::task::block_on;
use clap::Parser;
use once_cell::sync::OnceCell;
use std::thread;

#[derive(Parser)]
struct Args {
    #[clap(short, long, default_value_t = 8000)]
    port: u16,
}

pub static PORT: OnceCell<u16> = OnceCell::new();
pub static HOST: OnceCell<&str> = OnceCell::new();

#[async_std::main]
async fn main() {
    let args = Args::parse();

    HOST.set("127.0.0.1").unwrap();
    PORT.set(args.port).unwrap();

    thread::spawn(|| block_on(watcher::watch()));
    server::serve().await;
}
