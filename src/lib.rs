//! Launch a local network server with live reload feature for static pages.
//!
//! ## Create live server
//! ```
//! use live_server::listen;
//! listen("127.0.0.1", 8080, "./").await.unwrap();
//! ```
//! 
//! ## Enable logs (Optional)
//! ```rust
//! env_logger::init();
//! ```

mod server;
mod watcher;

use std::{collections::HashMap, sync::Arc};

use async_std::{path::PathBuf, sync::Mutex, task};

/// Watch the directory and create a static server.
/// ```
/// use live_server::listen;
/// listen("127.0.0.1", 8080, "./").await.unwrap();
/// ```
pub async fn listen<R: Into<PathBuf>>(
    host: &str,
    port: u16,
    root: R,
) -> Result<(), std::io::Error> {
    let connections1 = Arc::new(Mutex::new(HashMap::new()));
    let connections2 = Arc::clone(&connections1);
    let root1: PathBuf = root.into();
    let root2: PathBuf = root1.clone();

    task::spawn(async move { watcher::watch(root1, &connections1).await });
    server::serve(host, port, root2, connections2).await
}
