[![Crates.io](https://img.shields.io/crates/v/live-server)](https://crates.io/crates/live-server)
![License](https://img.shields.io/crates/l/live-server)

# Live Server

Launch a local network server with live reload feature for static pages.

## Install

```console
$ cargo install live-server
```

## Usage

```console
$ live-server --help
live-server 0.3.0
Launch a local network server with live reload feature for static pages

USAGE:
    live-server [OPTIONS]

OPTIONS:
    -h, --host <HOST>    Set the listener host, otherwise it will be set to the local IP address
        --help           Print help information
    -p, --port <PORT>    Set the listener port [default: 8000]
    -V, --version        Print version information
```

## Example

```console
$ live-server
Watcher listening on /home/lomirus/demo
 Server listening on http://192.168.0.105:8000/
[UPDATE] "index.html"
[UPDATE] "script.js"
```