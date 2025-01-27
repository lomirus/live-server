use live_server::{listen, Options};
use reqwest::StatusCode;

#[tokio::test]
async fn request() {
    const HOST: &str = "127.0.0.1:8000";

    let listener = listen(HOST, "./tests/page").await.unwrap();
    tokio::spawn(async {
        listener.start(Options::default()).await.unwrap();
    });

    // Test requesting index.html
    let response = reqwest::get(format!("http://{HOST}")).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let content_type = response.headers().get("content-type").unwrap();
    assert_eq!(content_type, "text/html");

    let text = response.text().await.unwrap().replace("\r\n", "\n");
    let target_text = format!(
        r#"{}<script>{}(false)</script>"#,
        include_str!("./page/index.html"),
        include_str!("../src/templates/websocket.js"),
    )
    .replace("\r\n", "\n");
    assert_eq!(text, target_text);
    assert!(text.contains("<script>"));

    // Test requesting index.js
    let response = reqwest::get(format!("http://{HOST}/index.js"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let content_type = response.headers().get("content-type").unwrap();
    assert_eq!(content_type, "text/javascript");

    let text = response.text().await.unwrap().replace("\r\n", "\n");
    let target_text = include_str!("./page/index.js").replace("\r\n", "\n");
    assert_eq!(text, target_text);

    // Test requesting non-existent html file
    let response = reqwest::get(format!("http://{HOST}/404.html"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let content_type = response.headers().get("content-type").unwrap();
    assert_eq!(content_type, "text/html");

    let text = response.text().await.unwrap();
    assert!(text.starts_with("<!DOCTYPE html>"));

    // Test requesting non-existent asset
    let response = reqwest::get(format!("http://{HOST}/favicon.ico"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let content_type = response.headers().get("content-type").unwrap();
    assert_eq!(content_type, "image/x-icon");

    // Test requesting with reload query
    let response = reqwest::get(format!("http://{HOST}?reload")).await.unwrap();

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
    let response = reqwest::get(format!("http://{HOST}/404.html?reload"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let content_type = response.headers().get("content-type").unwrap();
    assert_eq!(content_type, "text/html");

    let text = response.text().await.unwrap();
    assert!(!text.contains("<script>"));
}

#[tokio::test]
async fn disable_index_listing() {
    const HOST: &str = "127.0.0.1:8001";

    let listener = listen(HOST, "./tests/empty_index").await.unwrap();
    tokio::spawn(async {
        listener
            .start(Options {
                hard_reload: true,
                index_listing: false,
                auto_ignore: false,
            })
            .await
            .unwrap();
    });

    // Test requesting index.html
    let response = reqwest::get(format!("http://{HOST}")).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let content_type = response.headers().get("content-type").unwrap();
    assert_eq!(content_type, "text/html");

    let text = response.text().await.unwrap().replace("\r\n", "\n");
    assert!(text.starts_with("<!DOCTYPE html>"));
    assert!(text.contains("<script>"));
}

#[tokio::test]
async fn enable_index_listing() {
    const HOST: &str = "127.0.0.1:8002";

    let listener = listen(HOST, "./tests/empty_index").await.unwrap();
    tokio::spawn(async {
        listener
            .start(Options {
                hard_reload: true,
                index_listing: true,
                auto_ignore: false,
            })
            .await
            .unwrap();
    });

    // Test requesting index.html
    let response = reqwest::get(format!("http://{HOST}")).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let content_type = response.headers().get("content-type").unwrap();
    assert_eq!(content_type, "text/html");

    let text = response.text().await.unwrap().replace("\r\n", "\n");
    assert!(text.ends_with(
        "<body><ul><li><a href=\"not_index.html\">not_index.html</a></li></ul></body>\n</html>\n"
    ));
    assert!(text.contains("<script>"));
}
