[![Crates.io](https://img.shields.io/crates/v/html_editor)](https://crates.io/crates/live-server)
![License](https://img.shields.io/crates/l/live-server)

# Live Server

Launch a local network server with live reload feature for static pages.

## Usage

```console
$ live-server --help
live-server 
Launch a local network server with live reload feature for static pages

USAGE:
    live-server [OPTIONS]

OPTIONS:
    -h, --help           Print help information
    -p, --port <PORT>    [default: 8000]
```

## Example

```console
$ live-server
Watcher listening on /home/lomirus/demo
 Server listening on http://192.168.0.105:8000/
[UPDATE] "index.html"
[UPDATE] "script.js"
```