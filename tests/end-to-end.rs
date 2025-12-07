use std::{
    collections::BTreeSet,
    ffi::OsStr,
    path::{Path, PathBuf},
    sync::{LazyLock, Mutex},
    time::Duration,
};

use chromiumoxide::{
    Browser, BrowserConfig, cdp::browser_protocol::page::EventFrameStoppedLoading,
};
use futures::StreamExt as _;
use get_port::{Ops, tcp::TcpPort};
use symlink::symlink_dir;
use tempfile::{self, TempDir, tempdir};
use tokio::{
    fs,
    process::{Child, Command},
};

async fn with_timeout<T>(
    future: impl Future<Output = T>,
) -> Result<T, tokio::time::error::Elapsed> {
    tokio::time::timeout(Duration::from_secs(6), future).await
}

fn subject_with(args: &[impl AsRef<OsStr>]) -> (Child, String) {
    static TAKEN_PORTS: LazyLock<Mutex<BTreeSet<u16>>> =
        LazyLock::new(|| Mutex::new(BTreeSet::new()));

    const HOST: &str = "127.0.0.1";
    let mut taken_ports = TAKEN_PORTS.lock().unwrap();
    let port = TcpPort::except(HOST, taken_ports.iter().copied().collect()).unwrap();
    taken_ports.insert(port);

    let subject = Command::new(env!("CARGO_BIN_EXE_live-server"))
        .kill_on_drop(true)
        .args(["--host", HOST])
        .args(["--port", &port.to_string()])
        .args(args)
        .spawn()
        .unwrap();

    (subject, format!("{HOST}:{port}"))
}

async fn fresh_browser() -> (Browser, TempDir) {
    let data_dir = tempdir().unwrap();
    let (browser, handler) = Browser::launch(
        BrowserConfig::builder()
            .user_data_dir(data_dir.path())
            .build()
            .unwrap(),
    )
    .await
    .unwrap();
    tokio::spawn(async move {
        handler.for_each(async |_| {}).await;
    });
    (browser, data_dir)
}

fn index_with(title: &str) -> String {
    format!(
        r#"<!DOCTYPE html><html><head><meta charset="UTF-8"><title>{title}</title></head><body></body></html>"#
    )
}

async fn fixture_with(title: &str) -> TempDir {
    let dir = tempdir().unwrap();
    let index_path: PathBuf = [dir.path(), Path::new("index.html")].iter().collect();
    fs::write(&index_path, index_with(title)).await.unwrap();
    dir
}

#[tokio::test]
async fn page_content_is_served() {
    let fixture = fixture_with("some page").await;
    let (_subject, authority) = subject_with(&[fixture.path()]);
    let (browser, _browser_dir) = fresh_browser().await;
    let title = browser
        .new_page(format!("http://{authority}/"))
        .await
        .unwrap()
        .wait_for_navigation()
        .await
        .unwrap()
        .get_title()
        .await
        .unwrap()
        .unwrap();
    assert_eq!(title, "some page");
}

#[tokio::test]
async fn browser_reloads_on_file_change() {
    let fixture = fixture_with("initial").await;
    let (_subject, authority) = subject_with(&[fixture.path()]);
    let (browser, _browser_dir) = fresh_browser().await;

    let page = browser
        .new_page(format!("http://{authority}/"))
        .await
        .unwrap();

    page.wait_for_navigation().await.unwrap();
    let title = page.get_title().await.unwrap().unwrap();
    assert_eq!(title, "initial");

    let mut frame_stopped_loading_event_stream = page
        .event_listener::<EventFrameStoppedLoading>()
        .await
        .unwrap();

    let index_path = [fixture.path(), Path::new("index.html")]
        .iter()
        .collect::<PathBuf>();
    fs::write(index_path, index_with("modified")).await.unwrap();

    with_timeout(frame_stopped_loading_event_stream.next())
        .await
        .unwrap()
        .unwrap();

    let title = page.get_title().await.unwrap().unwrap();
    assert_eq!(title, "modified");
}

#[tokio::test]
async fn browser_reloads_on_symlink_swap() {
    let fixture = fixture_with("initial").await;
    let symlink_parent = tempdir().unwrap();
    let symlink_path = [symlink_parent.path(), Path::new("symlink")]
        .iter()
        .collect::<PathBuf>();
    symlink_dir(&fixture, &symlink_path).unwrap();
    let (_subject, authority) =
        subject_with(&["--poll", symlink_path.as_os_str().to_str().unwrap()]);
    let (browser, _browser_dir) = fresh_browser().await;

    let page = browser
        .new_page(format!("http://{authority}/"))
        .await
        .unwrap();

    page.wait_for_navigation().await.unwrap();
    let title = page.get_title().await.unwrap().unwrap();
    assert_eq!(title, "initial");

    let mut frame_stopped_loading_event_stream = page
        .event_listener::<EventFrameStoppedLoading>()
        .await
        .unwrap();

    let temp_symlink_path = [symlink_parent.path(), Path::new("temp-symlink")]
        .iter()
        .collect::<PathBuf>();
    let fixture = fixture_with("modified").await;
    symlink_dir(&fixture, &temp_symlink_path).unwrap();
    fs::rename(&temp_symlink_path, &symlink_path).await.unwrap();

    with_timeout(frame_stopped_loading_event_stream.next())
        .await
        .unwrap()
        .unwrap();

    let title = page.get_title().await.unwrap().unwrap();
    assert_eq!(title, "modified");
}
