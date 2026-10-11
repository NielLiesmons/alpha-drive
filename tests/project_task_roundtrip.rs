//! Project + Task encrypted round-trip; wrong key cannot read.

use std::net::SocketAddr;
use std::time::Duration;

use alpha_drive::{crypto, entities, server};

async fn spawn_drive() -> (String, String, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let token = "test-token".to_string();
    let router = server::router(dir.path().to_path_buf(), token.clone()).expect("router");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr: SocketAddr = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("serve");
    });
    // Tiny settle for accept loop
    tokio::time::sleep(Duration::from_millis(20)).await;
    (format!("http://{addr}"), token, dir)
}

#[tokio::test]
async fn project_and_task_round_trip() {
    let (url, token, _dir) = spawn_drive().await;
    let root = hex::encode(crypto::generate_root_key());

    entities::init_space(&url, &token, &root, "Bley Family")
        .await
        .expect("init");

    let project = entities::create_project(
        &url,
        &token,
        &root,
        "Ferme de Bley",
        Some("ferme-de-bley"),
    )
    .await
    .expect("create project");
    assert_eq!(project.schema, "alpha.project/1");
    assert_eq!(project.slug, "ferme-de-bley");

    let got = entities::get_project(&url, &token, &root, &project.id)
        .await
        .expect("get project");
    assert_eq!(got.title, "Ferme de Bley");

    let task = entities::create_task(&url, &token, &root, &project.id, "Order CSA boxes")
        .await
        .expect("create task");
    assert_eq!(task.project_id, project.id);
    assert_eq!(task.status, "todo");

    let got_task = entities::get_task(&url, &token, &root, &task.id)
        .await
        .expect("get task");
    assert_eq!(got_task.title, "Order CSA boxes");

    let projects = entities::list_projects(&url, &token, &root)
        .await
        .expect("list projects");
    assert_eq!(projects.len(), 1);

    let tasks = entities::list_tasks(&url, &token, &root, Some(&project.id))
        .await
        .expect("list tasks");
    assert_eq!(tasks.len(), 1);
}

#[tokio::test]
async fn wrong_root_key_cannot_read() {
    let (url, token, _dir) = spawn_drive().await;
    let root = hex::encode(crypto::generate_root_key());
    let other = hex::encode(crypto::generate_root_key());

    let project = entities::create_project(&url, &token, &root, "Secret farm", None)
        .await
        .expect("create");

    let err = entities::get_project(&url, &token, &other, &project.id)
        .await
        .expect_err("wrong key must fail");
    let msg = err.to_string();
    assert!(
        msg.contains("path not found")
            || msg.contains("decrypt")
            || msg.contains("aead")
            || msg.contains("missing"),
        "unexpected error: {msg}"
    );
}
