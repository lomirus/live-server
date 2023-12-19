//! Launch a local network server with live reload feature for static pages.
//!
//! ## Create live server
//! ```
//! use live_server::listen;
//! 
//! async fn serve() {
//!     listen("127.0.0.1", 8080, "./").await.unwrap();
//! }
//! ```
//!
//! ## Enable logs (Optional)
//! ```rust
//! env_logger::init();
//! ```

mod server;
mod watcher;

use std::{error::Error, path::PathBuf};

use tokio::sync::{broadcast, OnceCell};

static HOST: OnceCell<String> = OnceCell::const_new();
static PORT: OnceCell<u16> = OnceCell::const_new();
static ROOT: OnceCell<PathBuf> = OnceCell::const_new();
static TX: OnceCell<broadcast::Sender<()>> = OnceCell::const_new();

/// Watch the directory and create a static server.
/// ```
/// use live_server::listen;
/// 
/// async fn serve() {
///     listen("127.0.0.1", 8080, "./").await.unwrap();
/// }
/// ```
pub async fn listen<R: Into<PathBuf>>(
    host: &str,
    port: u16,
    root: R,
) -> Result<(), Box<dyn Error>> {
    HOST.set(host.to_string()).unwrap();
    ROOT.set(root.into()).unwrap();
    let (tx, _) = broadcast::channel(16);
    TX.set(tx).unwrap();

    let watcher_future = tokio::spawn(watcher::watch());
    let server_future = tokio::spawn(server::serve(port));

    tokio::try_join!(watcher_future, server_future)?;

    Ok(())
}
