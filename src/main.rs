use clap::Parser;
use env_logger::Env;
use live_server::{Listener, Options, listen, listen_poll};
use notify::Watcher;

/// Launch a local network server with live reload feature for static pages.
#[derive(Parser)]
#[clap(version)]
struct Args {
    /// Set the root path of the static assets
    #[clap(default_value = ".")]
    root: String,
    /// Show directory listings if there is no index.html
    #[clap(long)]
    index: bool,
    /// Set the listener host
    #[clap(short = 'H', long, default_value = "0.0.0.0")]
    host: String,
    /// Set the listener port
    #[clap(short, long, default_value = "0")]
    port: u16,
    /// Open the page in browser automatically
    #[clap(short, long, value_name = "PAGE")]
    open: Option<Option<String>>,
    /// Specify a particular browser to open the page with
    #[clap(long, value_name = "PATH")]
    browser: Option<String>,
    /// Hard reload the page on update instead of hot reload
    ///
    /// Try using this if the reload is not working as expected.
    #[clap(long)]
    hard: bool,
    /// Ignore hidden and ignored files
    ///
    /// Ignored files will be invisible and unaccessible for user, and their
    /// changes will not trigger reload events. The "ignored" files only follow
    /// the `.gitignore` in the root directory, which means `.gitignore`s in the
    /// sub-directories won't work.
    #[clap(short = 'I', long)]
    ignore: bool,
    /// Create listener using `PollWatcher`
    ///
    /// `PollWatcher` is a fallback that manually checks file paths for changes at a regular interval.
    /// It is useful for cases where real-time OS notifications fail, such as when a symbolic link is
    /// atomically replaced, or when the monitored directory itself is moved or renamed.
    #[clap(long)]
    poll: bool,
}

// Workaround for https://github.com/rust-lang/rust/issues/63065
async fn run_listener<W: Watcher + Send + 'static>(listener: Listener<W>, args: &Args) {
    if let Some(page) = &args.open {
        let origin = listener.link().unwrap();
        let path = page.clone().unwrap_or_default();
        let url = format!("{origin}/{path}");
        match &args.browser {
            Some(browser) => open::with(url, browser).unwrap(),
            None => open::that(url).unwrap(),
        }
    }

    listener
        .start(Options {
            hard_reload: args.hard,
            index_listing: args.index,
            auto_ignore: args.ignore,
        })
        .await
        .unwrap()
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let env = Env::new().default_filter_or("info");
    env_logger::init_from_env(env);

    let args = Args::parse();
    let Args {
        host, port, root, ..
    } = &args;

    let addr = format!("{host}:{port}");
    if args.poll {
        let listener = listen_poll(addr, root).await?;
        run_listener(listener, &args).await;
    } else {
        let listener = listen(addr, root).await?;
        run_listener(listener, &args).await;
    };
    Ok(())
}
