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
live-server 0.5.0
Launch a local network server with live reload feature for static pages

USAGE:
    live-server [OPTIONS] [PATH]

ARGS:
    <PATH>    Set the path of the static assets [default: .]

OPTIONS:
    -h, --host <HOST>    Set the listener host [default: LAN IP address]
        --help           Print help information
    -p, --port <PORT>    Set the listener port [default: 8000]
    -V, --version        Print version information
```

### Log Level

You can set different [`RUST_LOG` environment variable](https://rust-lang-nursery.github.io/rust-cookbook/development_tools/debugging/config_log.html) to filter the log. The default log level is `warn`. So if you want to get more detailed information, it is recommended to use:

```console
$ RUST_LOG=debug,tide=warn live-server
[2023-02-17T09:18:56Z INFO  live_server::server] Listening on http://192.168.0.166:8000/
[2023-02-17T09:18:56Z INFO  live_server::watcher] Listening on /tmp/live_server_test/
[2023-02-17T09:19:06Z DEBUG live_server::watcher] [UPDATE] index.html
```

## Package

You can also import it as a library in your project.

### Create live server

```rust
use live_server::listen;
listen("127.0.0.1", 8080, "./").await.unwrap();
```

### Enable logs (Optional)

```rust
env_logger::init();
```