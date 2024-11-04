use std::net::IpAddr;

use local_ip_address::local_ip;
use tokio::net::TcpListener;

pub(crate) async fn create_listener(addr: String) -> Result<TcpListener, String> {
    match tokio::net::TcpListener::bind(&addr).await {
        Ok(listener) => {
            let port = listener.local_addr().unwrap().port();
            let host = listener.local_addr().unwrap().ip();
            let host = match host.is_unspecified() {
                true => match local_ip() {
                    Ok(addr) => addr,
                    Err(err) => {
                        log::warn!("Failed to get local IP address: {}", err);
                        host
                    }
                },
                false => host,
            };

            let addr = match host {
                IpAddr::V4(host) => format!("{host}:{port}"),
                IpAddr::V6(host) => format!("[{host}]:{port}"),
            };
            log::info!("Listening on http://{addr}/");
            Ok(listener)
        }
        Err(err) => {
            let err_msg = if let std::io::ErrorKind::AddrInUse = err.kind() {
                format!("Address {} is already in use", &addr)
            } else {
                format!("Failed to listen on {}: {}", addr, err)
            };
            log::error!("{err_msg}");
            Err(err_msg)
        }
    }
}
