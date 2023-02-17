//! Launch a local network server with live reload feature for static pages.
//!
//! - Listening custom host
//! ```
//! use live_server::listen;
//! listen("127.0.0.1", 8080, "./").await.unwrap();
//! ```
//!
//! - Listening local network IP address
//! ```
//! use live_server::listen_local;
//! listen_local(8080, "./").await.unwrap();
//! ```

mod server;
mod watcher;

use std::{collections::HashMap, sync::Arc};

use async_std::{path::PathBuf, sync::Mutex, task};
use local_ip_address::local_ip;

#[derive(Debug)]
pub enum Error {
    /// Returned when `local_ip` is unable to find the system's local IP address
    /// in the collection of network interfaces
    LocalIpAddressNotFound,
    /// Returned when an error occurs in the strategy level.
    /// The error message may include any internal strategy error if available
    StrategyError(String),
    /// Returned when the current platform is not yet supported
    PlatformNotSupported(String),
    /// Returned when the listener returns errors when accepting incoming connections
    ListenerError(std::io::Error),
}

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

/// Watch the directory and create a static server, which
/// will listen on the local network IP address.
///
/// ```
/// use live_server::listen_local;
/// listen_local(8080, "./").await.unwrap();
/// ```
pub async fn listen_local<R: Into<PathBuf>>(port: u16, root: R) -> Result<(), Error> {
    return match local_ip() {
        Err(err) => match err {
            local_ip_address::Error::LocalIpAddressNotFound => Err(Error::LocalIpAddressNotFound),
            local_ip_address::Error::StrategyError(err) => Err(Error::StrategyError(err)),
            local_ip_address::Error::PlatformNotSupported(err) => {
                Err(Error::PlatformNotSupported(err))
            }
        },
        Ok(addr) => match listen(&addr.to_string(), port, root).await {
            Ok(_) => Ok(()),
            Err(err) => Err(Error::ListenerError(err)),
        },
    };
}
