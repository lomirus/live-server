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

Usage: live-server.exe [OPTIONS] [ROOT]

Arguments:
  [ROOT]  Set the root path of the static assets [default: .]

Options:
  -s, --host <HOST>  Set the listener host [default: LAN IP address]
  -p, --port <PORT>  Set the listener port [default: 8000]
  -h, --help         Print help
  -V, --version      Print version
```

### Log Level

You can set different [`RUST_LOG` environment variable](https://rust-lang-nursery.github.io/rust-cookbook/development_tools/debugging/config_log.html) to filter the log. The default log level is `info,tide=error`.

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