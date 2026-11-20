//! Chunked media blob round-trip + per-chunk fetch (Player stream-on-play).

use std::net::SocketAddr;
use std::time::Duration;

use alpha_drive::{crypto, media, server};

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
async fn media_blob_chunk_round_trip() {
    let (url, token, _dir) = spawn_drive().await;
    let root = hex::encode(crypto::generate_root_key());

    // ~3 chunks at 64 KiB
    let mut plaintext = vec![0u8; 64 * 1024 * 3 - 100];
    for (i, b) in plaintext.iter_mut().enumerate() {
        *b = (i % 251) as u8;
    }

    let path = "media/audio/tracks/demo/media.json";
    let (manifest, _key) = media::put_media_blob(
        &url,
        &token,
        &root,
        path,
        "audio/mpeg",
        &plaintext,
        Some(64 * 1024),
    )
    .await
    .expect("put media");

    assert_eq!(manifest.schema, "alpha.media_blob/1");
    assert_eq!(manifest.plaintext_len, plaintext.len() as u64);
    assert_eq!(manifest.chunks.len(), 3);

    let chunk0 = media::get_media_chunk(&url, &token, &root, path, 0)
        .await
        .expect("chunk 0");
    assert_eq!(chunk0.len(), 64 * 1024);
    assert_eq!(&chunk0[..], &plaintext[..64 * 1024]);

    let (_m, full) = media::get_media_blob(&url, &token, &root, path)
        .await
        .expect("full");
    assert_eq!(full, plaintext);
}

#[tokio::test]
async fn wrong_chunk_index_aad_fails() {
    let key = crypto::generate_root_key();
    let pt = b"hello media chunk";
    let ct = crypto::encrypt_chunk(&key, 0, pt).expect("enc");
    let err = crypto::decrypt_chunk(&key, 1, &ct).expect_err("aad must bind index");
    assert!(err.contains("decrypt"), "{err}");
}
