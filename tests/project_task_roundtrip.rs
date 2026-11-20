//! Project + Task + Note encrypted round-trip; wrong key cannot read.

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
    tokio::time::sleep(Duration::from_millis(20)).await;
    (format!("http://{addr}"), token, dir)
}

#[tokio::test]
async fn project_and_task_round_trip() {
    let (url, token, _dir) = spawn_drive().await;
    let root = hex::encode(crypto::generate_root_key());

    entities::init_space(&url, &token, &root, "Demo Space")
        .await
        .expect("init");

    let project = entities::create_project(
        &url,
        &token,
        &root,
        "Demo Space",
        Some("demo-space"),
    )
    .await
    .expect("create project");
    assert_eq!(project.schema, "alpha.project/1");
    assert_eq!(project.slug, "demo-space");
    assert!(project
        .drive
        .as_ref()
        .and_then(|d| d.home_prefix.as_ref())
        .unwrap()
        .ends_with("/home"));

    let other = entities::create_project(&url, &token, &root, "Website", Some("website"))
        .await
        .expect("create project 2");

    let got = entities::get_project(&url, &token, &root, &project.id)
        .await
        .expect("get project");
    assert_eq!(got.title, "Demo Space");

    let task = entities::create_task(
        &url,
        &token,
        &root,
        &[project.id.clone(), other.id.clone()],
        "Order CSA boxes",
    )
    .await
    .expect("create task");
    assert_eq!(task.schema, "alpha.task/3");
    assert_eq!(task.project_ids.len(), 2);
    assert!(task.links_project(&project.id));
    assert!(task.links_project(&other.id));
    assert_eq!(task.status, "open");

    let got_task = entities::get_task(&url, &token, &root, &task.id)
        .await
        .expect("get task");
    assert_eq!(got_task.title, "Order CSA boxes");

    let moment = entities::create_moment(
        &url,
        &token,
        &root,
        Some(&task.id),
        None,
        "2026-10-21T08:00:00Z",
        Some("2026-10-21T09:00:00Z"),
    )
    .await
    .expect("create moment");
    assert_eq!(moment.schema, "alpha.moment/1");
    assert_eq!(moment.task_id.as_deref(), Some(task.id.as_str()));

    let free = entities::create_moment(
        &url,
        &token,
        &root,
        None,
        Some("Vet visit"),
        "2026-10-22T14:00:00Z",
        Some("2026-10-22T15:00:00Z"),
    )
    .await
    .expect("create free calendar moment");
    assert!(free.task_id.is_none());

    let moments = entities::list_moments(&url, &token, &root, Some(&task.id))
        .await
        .expect("list moments for task");
    assert_eq!(moments.len(), 1);

    let all_moments = entities::list_moments(&url, &token, &root, None)
        .await
        .expect("list all moments");
    assert_eq!(all_moments.len(), 2);

    let projects = entities::list_projects(&url, &token, &root)
        .await
        .expect("list projects");
    assert_eq!(projects.len(), 2);

    let tasks = entities::list_tasks(&url, &token, &root, Some(&project.id))
        .await
        .expect("list tasks");
    assert_eq!(tasks.len(), 1);

    let tasks_other = entities::list_tasks(&url, &token, &root, Some(&other.id))
        .await
        .expect("list by other");
    assert_eq!(tasks_other.len(), 1);

    let note = entities::create_note(
        &url,
        &token,
        &root,
        &[project.id.clone()],
        "Drop-off planning",
        "## This week\n\n- Wednesday: Guingamp\n",
    )
    .await
    .expect("create note");
    assert_eq!(note.schema, "alpha.note/1");
    assert!(note.links_project(&project.id));
    assert!(note.body.contains("Guingamp"));

    let got_note = entities::get_note(&url, &token, &root, &note.id)
        .await
        .expect("get note");
    assert_eq!(got_note.title, "Drop-off planning");

    let notes = entities::list_notes(&url, &token, &root, Some(&project.id))
        .await
        .expect("list notes");
    assert_eq!(notes.len(), 1);

    let notes_other = entities::list_notes(&url, &token, &root, Some(&other.id))
        .await
        .expect("list notes other");
    assert!(notes_other.is_empty());
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
