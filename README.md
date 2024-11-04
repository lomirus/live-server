[![crate-badge]][crate-link]
![License](https://img.shields.io/crates/l/live-server)
[![check-badge]][check-link]

[crate-badge]: https://img.shields.io/crates/v/live-server
[crate-link]: https://crates.io/crates/live-server
[check-badge]: https://github.com/lomirus/live-server/workflows/check/badge.svg
[check-link]: https://github.com/lomirus/live-server/actions/workflows/check.yaml

# Live Server

Launch a local network server with live reload feature for static pages.

## Binary

You can use it as a CLI program.

### Install

```console
$ cargo install live-server
```

### Usage

```console
$ live-server --help
Launch a local network server with live reload feature for static pages

Usage: live-server [OPTIONS] [ROOT]

Arguments:
  [ROOT]  Set the root path of the static assets [default: .]

Options:
      --index          Whether to show directory listings if there is no index.html
  -H, --host <HOST>    Set the listener host [default: 0.0.0.0]
  -p, --port <PORT>    Set the listener port [default: 0]
  -o, --open [<PAGE>]  Open the page in browser automatically
      --hard           Hard reload the page on update instead of hot reload
  -h, --help           Print help (see more with '--help')
  -V, --version        Print version
```

```console
$ live-server
[2023-12-22T15:16:04Z INFO  live_server::server] Listening on http://10.17.95.220:6634/
[2023-12-22T15:16:04Z INFO  live_server::watcher] Listening on /home/mirus/html-demo
```

### Log Level

You can set different [`RUST_LOG` environment variable](https://rust-lang-nursery.github.io/rust-cookbook/development_tools/debugging/config_log.html) to filter the log. The default log level is `info`.

## Package

You can also import it as a library in your project.

### Create live server

```rust
use live_server::{listen, Options};

listen("127.0.0.1:8080", "./").await?.start(Options::default()).await;
```

### Enable logs (Optional)

```rust
env_logger::init();
```