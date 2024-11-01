//! Launch a local network server with live reload feature for static pages.
//!
//! ## Create live server
//! ```
//! use live_server::listen;
//!
//! async fn serve() -> Result<(), Box<dyn std::error::Error>> {
//!     listen("127.0.0.1:8080", "./", false).await?.start().await
//! }
//! ```
//!
//! ## Enable logs (Optional)
//! ```rust
//! env_logger::init();
//! ```

mod server;
mod watcher;

use std::{error::Error, net::IpAddr, path::PathBuf};

use axum::Router;
use local_ip_address::local_ip;
use notify::RecommendedWatcher;
use notify_debouncer_full::{DebouncedEvent, Debouncer, FileIdMap};
use server::{create_listener, create_server};
use tokio::{
    net::TcpListener,
    sync::{broadcast, mpsc::Receiver, OnceCell},
};
use watcher::create_watcher;

static ADDR: OnceCell<String> = OnceCell::const_new();
static ROOT: OnceCell<PathBuf> = OnceCell::const_new();
static INDEX: OnceCell<bool> = OnceCell::const_new();
static HARD: OnceCell<bool> = OnceCell::const_new();
static TX: OnceCell<broadcast::Sender<()>> = OnceCell::const_new();

pub struct Listener {
    tcp_listener: TcpListener,
    router: Router,
    root_path: PathBuf,
    debouncer: Debouncer<RecommendedWatcher, FileIdMap>,
    rx: Receiver<Result<Vec<DebouncedEvent>, Vec<notify::Error>>>,
    hard: bool,
    index: bool,
}

impl Listener {
    /// Start live-server.
    ///
    /// ```
    /// use live_server::listen;
    ///
    /// async fn serve() -> Result<(), Box<dyn std::error::Error>> {
    ///     listen("127.0.0.1:8080", "./", false).await?.start().await
    /// }
    /// ```
    pub async fn start(self) -> Result<(), Box<dyn Error>> {
        HARD.set(self.hard)?;
        ROOT.set(self.root_path.clone())?;
        INDEX.set(self.index)?;
        let (tx, _) = broadcast::channel(16);
        TX.set(tx)?;

        let watcher_future = tokio::spawn(watcher::watch(self.root_path, self.debouncer, self.rx));
        let server_future = tokio::spawn(server::serve(self.tcp_listener, self.router));

        tokio::try_join!(watcher_future, server_future)?;

        Ok(())
    }

    /// Always hard reload the page instead of hot-reload
    /// ```
    /// use live_server::listen;
    ///
    /// async fn serve_hard() -> Result<(), Box<dyn std::error::Error>> {
    ///     listen("127.0.0.1:8080", "./", false).await?.hard_reload().start().await
    /// }
    /// ```
    pub fn hard_reload(mut self) -> Self {
        self.hard = true;
        self
    }

    /// Return the link of the server, like `http://127.0.0.1:8080`.
    ///
    /// ```
    /// use live_server::listen;
    ///
    /// async fn serve() {
    ///     let listener = listen("127.0.0.1:8080", "./", false).await.unwrap();
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
/// use live_server::listen;
///
/// async fn serve() -> Result<(), Box<dyn std::error::Error>> {
///     listen("127.0.0.1:8080", "./", false).await?.start().await
/// }
/// ```
pub async fn listen<A: Into<String>, R: Into<PathBuf>, I: Into<bool>>(
    addr: A,
    root: R,
    index: I,
) -> Result<Listener, String> {
    let tcp_listener = create_listener(addr.into()).await?;
    let router = create_server();
    let (debouncer, root_path, rx) = create_watcher(root.into()).await?;

    Ok(Listener {
        tcp_listener,
        router,
        debouncer,
        root_path,
        index: index.into(),
        rx,
        hard: false,
    })
}
