//! Launch a local network server with live reload feature for static pages.
//!
//! ## Create live server
//! ```
//! use live_server::{listen, Options};
//!
//! async fn serve() -> Result<(), Box<dyn std::error::Error>> {
//!     listen("127.0.0.1:8080", "./").await?.start(Options::default()).await
//! }
//! ```
//!
//! ## Enable logs (Optional)
//! ```rust
//! env_logger::init();
//! ```

mod file_layer;
mod http_layer;
mod utils;

pub use http_layer::server::Options;

use file_layer::watcher::{create_watcher, watch};
use http_layer::{
    listener::create_listener,
    server::{AppState, create_server, serve},
};
use local_ip_address::local_ip;
use notify::RecommendedWatcher;
use notify_debouncer_full::{DebouncedEvent, Debouncer, RecommendedCache};
use path_absolutize::Absolutize;
use std::{
    error::Error,
    net::IpAddr,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{
    net::TcpListener,
    sync::{broadcast, mpsc::Receiver},
};

pub struct Listener {
    tcp_listener: TcpListener,
    root_path: PathBuf,
    debouncer: Debouncer<RecommendedWatcher, RecommendedCache>,
    rx: Receiver<Result<Vec<DebouncedEvent>, Vec<notify::Error>>>,
}

impl Listener {
    /// Start live-server.
    ///
    /// ```
    /// use live_server::{listen, Options};
    ///
    /// async fn serve() -> Result<(), Box<dyn std::error::Error>> {
    ///     listen("127.0.0.1:8080", "./").await?.start(Options::default()).await
    /// }
    /// ```
    pub async fn start(self, options: Options) -> Result<(), Box<dyn Error>> {
        let (tx, _) = broadcast::channel(16);

        let arc_tx = Arc::new(tx);
        let app_state = AppState {
            hard_reload: options.hard_reload,
            index_listing: options.index_listing,
            auto_ignore: options.auto_ignore,
            tx: arc_tx.clone(),
            root: self.root_path.clone(),
        };

        let watcher_future = tokio::spawn(watch(
            self.root_path,
            self.debouncer,
            self.rx,
            arc_tx,
            options.auto_ignore,
        ));
        let server_future = tokio::spawn(serve(self.tcp_listener, create_server(app_state)));

        tokio::try_join!(watcher_future, server_future)?;

        Ok(())
    }

    /// Return the link of the server, like `http://127.0.0.1:8080`.
    ///
    /// ```
    /// use live_server::{listen, Options};
    ///
    /// async fn serve() {
    ///     let listener = listen("127.0.0.1:8080", "./").await.unwrap();
    ///     let link = listener.link().unwrap();
    ///     assert_eq!(link, "http://127.0.0.1:8080");
    /// }
    /// ```
    ///
    /// This is useful when you did not specify the host or port (e.g. `listen("0.0.0.0:0", ".")`),
    /// because this method will return the specific address.
    pub fn link(&self) -> Result<String, Box<dyn Error>> {
        let addr = self.tcp_listener.local_addr()?;
        let port = addr.port();
        let host = addr.ip();
        let host = match host.is_unspecified() {
            true => local_ip()?,
            false => host,
        };

        Ok(match host {
            IpAddr::V4(host) => format!("http://{host}:{port}"),
            IpAddr::V6(host) => format!("http://[{host}]:{port}"),
        })
    }
}

/// Create live-server listener
///
/// ```
/// use live_server::{listen, Options};
///
/// async fn serve() -> Result<(), Box<dyn std::error::Error>> {
///     listen("127.0.0.1:8080", "./").await?.start(Options::default()).await
/// }
/// ```
pub async fn listen(addr: impl AsRef<str>, root: impl AsRef<Path>) -> Result<Listener, String> {
    let tcp_listener = create_listener(addr.as_ref()).await?;
    let (debouncer, rx) = create_watcher().await?;

    let abs_root = get_absolute_path(root.as_ref())?;
    print_listening_on_path(&abs_root)?;

    Ok(Listener {
        tcp_listener,
        debouncer,
        root_path: abs_root,
        rx,
    })
}

fn get_absolute_path(path: &Path) -> Result<PathBuf, String> {
    match path.absolutize() {
        Ok(path) => Ok(path.to_path_buf()),
        Err(err) => {
            let err_msg = format!("Failed to get absolute path of {path:?}: {err}");
            log::error!("{err_msg}");
            Err(err_msg)
        }
    }
}

fn print_listening_on_path(path: &PathBuf) -> Result<(), String> {
    match path.as_os_str().to_str() {
        Some(path_str) => {
            log::info!("Listening on {path_str}");
            Ok(())
        }
        None => {
            let err_msg = format!("Failed to parse path to string for `{path:?}`");
            log::error!("{err_msg}");
            Err(err_msg)
        }
    }
}
