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
    /// Set the listener host
    #[clap(short = 'H', long, default_value = "0.0.0.0")]
    host: String,
    /// Set the listener port
    #[clap(short, long, default_value = "0")]
    port: u16,
    /// Open the page in browser automatically
    #[clap(short, long)]
    open: bool,
}

#[tokio::main]
async fn main() {
    let env = Env::new().default_filter_or("info");
    env_logger::init_from_env(env);

    let Args {
        host,
        port,
        root,
        open,
    } = Args::parse();

    let addr = format!("{}:{}", host, port);
    let listener = listen(addr, root).await.unwrap();

    if open {
        let link = listener.link().unwrap();
        open::that(link).unwrap();
    }

    listener.start().await.unwrap();
}
