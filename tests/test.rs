use live_server::{listen, Options};
use reqwest::StatusCode;

#[tokio::test]
async fn request() {
    let listener = listen("127.0.0.1:8000", "./tests/page").await.unwrap();
    tokio::spawn(async {
        listener.start(Options::default()).await.unwrap();
    });

    // Test requesting index.html
    let response = reqwest::get("http://127.0.0.1:8000").await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let content_type = response.headers().get("content-type").unwrap();
    assert_eq!(content_type, "text/html");

    let text = response.text().await.unwrap().replace("\r\n", "\n");
    let target_text = format!(
        r#"{}<script>{}("{}", false)</script>"#,
        include_str!("./page/index.html"),
        include_str!("../src/templates/websocket.js"),
        "127.0.0.1:8000"
    )
    .replace("\r\n", "\n");
    assert_eq!(text, target_text);
    assert!(text.contains("<script>"));

    // Test requesting index.js
    let response = reqwest::get("http://127.0.0.1:8000/index.js")
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let content_type = response.headers().get("content-type").unwrap();
    assert_eq!(content_type, "text/javascript");

    let text = response.text().await.unwrap().replace("\r\n", "\n");
    let target_text = include_str!("./page/index.js").replace("\r\n", "\n");
    assert_eq!(text, target_text);

    // Test requesting non-existent html file
    let response = reqwest::get("http://127.0.0.1:8000/404.html")
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let content_type = response.headers().get("content-type").unwrap();
    assert_eq!(content_type, "text/html");

    let text = response.text().await.unwrap();
    assert!(text.starts_with("<!DOCTYPE html>"));

    // Test requesting non-existent asset
    let response = reqwest::get("http://127.0.0.1:8000/favicon.ico")
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let content_type = response.headers().get("content-type").unwrap();
    assert_eq!(content_type, "image/x-icon");

    // Test requesting with reload query
    let response = reqwest::get("http://127.0.0.1:8000?reload").await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let content_type = response.headers().get("content-type").unwrap();
    assert_eq!(content_type, "text/html");

    let text = response.text().await.unwrap().replace("\r\n", "\n");
    let target_text = format!(
        r#"{}<script>{}</script>"#,
        include_str!("./page/index.html"),
        include_str!("../src/templates/reload.js"),
    )
    .replace("\r\n", "\n");
    assert_eq!(text, target_text);

    // Test requesting non-existent html file with reload query does not inject script
    let response = reqwest::get("http://127.0.0.1:8000/404.html?reload")
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let content_type = response.headers().get("content-type").unwrap();
    assert_eq!(content_type, "text/html");

    let text = response.text().await.unwrap();
    assert!(!text.contains("<script>"));
}
