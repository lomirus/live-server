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

pub use http_layer::server::Options;

use file_layer::watcher::{create_watcher, watch};
use http_layer::{
    listener::create_listener,
    server::{create_server, serve, AppState},
};
use local_ip_address::local_ip;
use notify::RecommendedWatcher;
use notify_debouncer_full::{DebouncedEvent, Debouncer, FileIdMap};
use std::{error::Error, net::IpAddr, path::PathBuf, sync::Arc};
use tokio::{
    net::TcpListener,
    sync::{broadcast, mpsc::Receiver, OnceCell},
};

static ADDR: OnceCell<String> = OnceCell::const_new();
static ROOT: OnceCell<PathBuf> = OnceCell::const_new();

pub struct Listener {
    tcp_listener: TcpListener,
    root_path: PathBuf,
    debouncer: Debouncer<RecommendedWatcher, FileIdMap>,
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
        ROOT.set(self.root_path.clone())?;
        let (tx, _) = broadcast::channel(16);

        let arc_tx = Arc::new(tx);
        let app_state = AppState {
            hard_reload: options.hard_reload,
            index_listing: options.index_listing,
            tx: arc_tx.clone(),
        };

        let watcher_future = tokio::spawn(watch(self.root_path, self.debouncer, self.rx, arc_tx));
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
pub async fn listen<A: Into<String>, R: Into<PathBuf>>(
    addr: A,
    root: R,
) -> Result<Listener, String> {
    let tcp_listener = create_listener(addr.into()).await?;
    let (debouncer, root_path, rx) = create_watcher(root.into()).await?;

    Ok(Listener {
        tcp_listener,
        debouncer,
        root_path,
        rx,
    })
}
