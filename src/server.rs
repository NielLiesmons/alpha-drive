use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;

use crate::store::BlobStore;

#[derive(Clone)]
struct App {
    store: Arc<BlobStore>,
    token: String,
}

/// Build the HTTP router (also used by integration tests).
pub fn router(data: PathBuf, token: String) -> Result<Router, Box<dyn std::error::Error>> {
    let store = Arc::new(BlobStore::open(data)?);
    let app = App { store, token };
    Ok(Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/v1/blocks/{hash}", get(get_block).put(put_block))
        .route("/v1/tip", get(get_tip).put(put_tip))
        .layer(CorsLayer::permissive())
        .with_state(app))
}

pub async fn serve(
    listen: SocketAddr,
    data: PathBuf,
    token: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let router = router(data, token)?;
    tracing::info!("alpha-drive listening on http://{listen} (ciphertext only)");
    let listener = tokio::net::TcpListener::bind(listen).await?;
    axum::serve(listener, router).await?;
    Ok(())
}

fn require_token(headers: &HeaderMap, expected: &str) -> Result<(), StatusCode> {
    let auth = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let got = auth.strip_prefix("Bearer ").unwrap_or("");
    if got == expected && !expected.is_empty() {
        Ok(())
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

async fn get_block(
    State(app): State<App>,
    Path(hash): Path<String>,
    headers: HeaderMap,
) -> Result<Vec<u8>, StatusCode> {
    require_token(&headers, &app.token)?;
    app.store.get(&hash).map_err(|_| StatusCode::NOT_FOUND)
}

async fn put_block(
    State(app): State<App>,
    Path(hash): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, StatusCode> {
    require_token(&headers, &app.token)?;
    app.store
        .put_verified(&hash, &body)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize, Deserialize)]
struct TipBody {
    root_hash: String,
}

async fn get_tip(State(app): State<App>, headers: HeaderMap) -> Result<Json<TipBody>, StatusCode> {
    require_token(&headers, &app.token)?;
    let root_hash = app.store.get_tip().map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json(TipBody { root_hash }))
}

async fn put_tip(
    State(app): State<App>,
    headers: HeaderMap,
    Json(body): Json<TipBody>,
) -> Result<StatusCode, StatusCode> {
    require_token(&headers, &app.token)?;
    if !app.store.exists(&body.root_hash) {
        return Err(StatusCode::BAD_REQUEST);
    }
    app.store
        .set_tip(&body.root_hash)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}
