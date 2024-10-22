use clap::Parser;
use env_logger::Env;
use live_server::listen;

/// Launch a local network server with live reload feature for static pages.
#[derive(Parser)]
#[clap(version)]
struct Args {
    /// Set the root path of the static assets
    #[clap(default_value = ".")]
    root: String,
    // Whether to show directory listings if there is no index.html
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
    /// Hard reload the page on update instead of hot reload
    ///
    /// Try using this if the reload is not working as expected
    #[clap(long)]
    hard: bool,
}

#[tokio::main]
async fn main() {
    let env = Env::new().default_filter_or("info");
    env_logger::init_from_env(env);

    let Args {
        host,
        port,
        root,
        index,
        open,
        hard,
    } = Args::parse();

    let addr = format!("{}:{}", host, port);
    let mut listener = listen(addr, root, index).await.unwrap();

    if let Some(page) = open {
        let origin = listener.link().unwrap();
        let path = page.unwrap_or_default();
        open::that(format!("{origin}/{path}")).unwrap();
    }

    if hard {
        listener = listener.hard_reload();
    }

    listener.start().await.unwrap();
}
