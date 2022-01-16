use async_std::prelude::*;
use html_editor::{parse, Editable, Htmlifiable, Selector};
use std::fs;
use tide::{prelude::*, Request, Response, StatusCode};
use tide_websockets::WebSocket;
use uuid::Uuid;
use local_ip_address::local_ip;

use crate::{PORT, SCRIPT, WS_CLIENTS};

pub async fn serve() {
    let host = local_ip().unwrap().to_string();
    let port = PORT.get().unwrap();
    let mut app = tide::new();
    app.at("/").get(static_assets);
    app.at("/*").get(static_assets);
    app.at("/live-server-ws")
        .get(WebSocket::new(|_request, mut stream| async move {
            let uuid = Uuid::new_v4();
            // Add the connection to clients when opening a new connection
            WS_CLIENTS.lock().await.insert(uuid, stream.clone());
            // Waiting for the connection to be closed
            while let Some(Ok(_)) = stream.next().await {}
            // Remove the connection from clients when it is closed
            WS_CLIENTS.lock().await.remove(&uuid);
            Ok(())
        }));
    let mut listener = app
        .bind(format!("{}:{}", host, port))
        .await
        .expect("Failed to bind host and port");
    println!(" Server listening on http://{}:{}/", host, port);
    listener.accept().await.unwrap();
}

async fn static_assets(req: Request<()>) -> tide::Result {
    // Get the path and mime of the static file.
    let mut path = req.url().path().to_string();
    path = if path.ends_with("/") {
        format!(".{}index.html", path)
    } else {
        format!(".{}", path)
    };
    let mime = mime_guess::from_path(&path).first_or_text_plain();

    // Read the file.
    let file = match fs::read(&path) {
        Ok(file) => file,
        Err(err) => {
            eprintln!(r#"[ERROR] Not Found: "{}""#, path);
            return Err(tide::Error::new(StatusCode::NotFound, err));
        }
    };
    let mut file: String = String::from_utf8_lossy(&file).parse()?;

    // Construct the response.
    let mut response: Response;
    if mime == "text/html" {
        let head_selector = Selector::from("head");
        let script = SCRIPT.get().unwrap().clone();
        file = parse(file.as_str())
            .insert_to(&head_selector, script)
            .html();
    }
    response = file.into();
    response.set_content_type(mime.to_string().as_str());

    Ok(response)
}
