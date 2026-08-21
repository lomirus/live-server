use live_server::{Options, listen};
use reqwest::StatusCode;
use std::fs;

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
    assert_eq!(content_type, "text/html; charset=utf-8");

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
    assert_eq!(content_type, "text/javascript; charset=utf-8");

    let text = response.text().await.unwrap().replace("\r\n", "\n");
    let target_text = include_str!("./page/index.js").replace("\r\n", "\n");
    assert_eq!(text, target_text);

    // Test requesting non-existent html file
    let response = reqwest::get(format!("http://{HOST}/404.html"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let content_type = response.headers().get("content-type").unwrap();
    assert_eq!(content_type, "text/html; charset=utf-8");

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
    assert_eq!(content_type, "text/html; charset=utf-8");

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
    assert_eq!(content_type, "text/html; charset=utf-8");

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
    assert_eq!(content_type, "text/html; charset=utf-8");

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
    assert_eq!(content_type, "text/html; charset=utf-8");

    let text = response.text().await.unwrap().replace("\r\n", "\n");
    assert!(text.ends_with(
        "<body><ul><li><a href=\"not_index.html\">not_index.html</a></li></ul></body>\n</html>\n"
    ));
    assert!(text.contains("<script>"));
}

#[tokio::test]
async fn request_paths_with_spaces() {
    let temp_dir = tempfile::tempdir().unwrap();
    let root = temp_dir.path().join("root with spaces");
    let nested_dir = root.join("dir with spaces");
    fs::create_dir_all(&nested_dir).unwrap();
    fs::write(root.join("file with spaces.txt"), "file content").unwrap();
    fs::write(nested_dir.join("nested file.txt"), "nested content").unwrap();
    fs::write(temp_dir.path().join("outside.txt"), "outside content").unwrap();

    let listener = listen("127.0.0.1:0", &root).await.unwrap();
    let origin = listener.link().unwrap();
    tokio::spawn(async move {
        listener.start(Options::default()).await.unwrap();
    });

    // The generated listing must use URL-encoded links for names containing spaces.
    let response = reqwest::get(&origin).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let text = response.text().await.unwrap();
    assert!(text.contains("href=\"file%20with%20spaces.txt\""));
    assert!(text.contains("href=\"dir%20with%20spaces/\""));

    // URL-encoded spaces must resolve to spaces in the filesystem file name.
    let response = reqwest::get(format!("{origin}/file%20with%20spaces.txt"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.text().await.unwrap(), "file content");

    // Directory redirects must preserve the encoded path and add a trailing slash.
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let response = client
        .get(format!("{origin}/dir%20with%20spaces"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(
        response.headers().get("location").unwrap(),
        "/dir%20with%20spaces/"
    );

    let response = reqwest::get(format!("{origin}/dir%20with%20spaces/"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        response
            .text()
            .await
            .unwrap()
            .contains("href=\"nested%20file.txt\"")
    );

    let response = reqwest::get(format!("{origin}/dir%20with%20spaces/nested%20file.txt"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.text().await.unwrap(), "nested content");

    // Percent-decoding must not turn one URL segment into a traversal path.
    let response = reqwest::get(format!("{origin}/%2E%2E%2Foutside.txt"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_ne!(response.text().await.unwrap(), "outside content");
}
