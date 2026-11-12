use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::get,
    Json, Router,
};
use dorm_shared::Store;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::RwLock;
#[derive(Clone)]
pub struct AppState {
    pub store: Arc<RwLock<Store>>,
    pub tokens: PathBuf,
    pub rates: Arc<Mutex<HashMap<String, (Instant, u32)>>>,
    pub rpm: u32,
}
fn error(status: StatusCode, message: &str) -> Response {
    (status, Json(serde_json::json!({"error":message}))).into_response()
}
fn allow(state: &AppState, key: &str, limit: u32) -> bool {
    let mut rates = state.rates.lock().unwrap();
    rates.retain(|_, (t, _)| t.elapsed() < Duration::from_secs(60));
    if rates.len() > 1024 && !rates.contains_key(key) {
        return false;
    }
    let v = rates.entry(key.into()).or_insert((Instant::now(), 0));
    v.1 += 1;
    v.1 <= limit
}
async fn guard(State(s): State<AppState>, req: Request, next: Next) -> Response {
    if req.headers().contains_key(header::TRANSFER_ENCODING)
        || req
            .headers()
            .get(header::CONTENT_LENGTH)
            .is_some_and(|h| h != "0")
    {
        return error(StatusCode::PAYLOAD_TOO_LARGE, "body_not_allowed");
    }
    if !allow(&s, "global", s.rpm.saturating_mul(10)) {
        return error(StatusCode::TOO_MANY_REQUESTS, "rate_limited");
    }
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "));
    let Some(id) = token.and_then(|t| crate::auth::verify(&s.tokens, t).ok().flatten()) else {
        let mut r = error(StatusCode::UNAUTHORIZED, "invalid_token");
        r.headers_mut()
            .insert(header::WWW_AUTHENTICATE, "Bearer".parse().unwrap());
        return r;
    };
    if !allow(&s, &format!("token:{id}"), s.rpm) {
        let mut r = error(StatusCode::TOO_MANY_REQUESTS, "rate_limited");
        r.headers_mut()
            .insert(header::RETRY_AFTER, "60".parse().unwrap());
        return r;
    }
    match tokio::time::timeout(Duration::from_secs(10), next.run(req)).await {
        Ok(r) => r,
        Err(_) => error(StatusCode::REQUEST_TIMEOUT, "timeout"),
    }
}
async fn menus(State(s): State<AppState>, req: Request) -> Response {
    let today = chrono::Utc::now()
        .with_timezone(&chrono_tz::Europe::Budapest)
        .date_naive();
    let snapshot = s.store.read().await.response(today);
    let payload = match req.uri().path() {
        "/api/v1/menus/current" => match snapshot.current_menu {
            Some(m) => serde_json::to_value(m).unwrap(),
            None => return error(StatusCode::NOT_FOUND, "current_menu_unavailable"),
        },
        "/api/v1/menus/next" => match snapshot.next_menu {
            Some(m) => serde_json::to_value(m).unwrap(),
            None => return error(StatusCode::NOT_FOUND, "next_menu_unavailable"),
        },
        _ => serde_json::to_value(snapshot).unwrap(),
    };
    let body = serde_json::to_vec(&payload).unwrap();
    if body.len() > 4_000_000 {
        return error(StatusCode::SERVICE_UNAVAILABLE, "data_unavailable");
    }
    let etag = format!("\"{}\"", hex::encode(Sha256::digest(&body)));
    let matches = req
        .headers()
        .get(header::IF_NONE_MATCH)
        .and_then(|h| h.to_str().ok())
        .is_some_and(|h| {
            h.split(',').any(|t| {
                t.trim() == etag || t.trim().strip_prefix("W/") == Some(&etag) || t.trim() == "*"
            })
        });
    let mut response = if matches {
        StatusCode::NOT_MODIFIED.into_response()
    } else {
        (
            [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
            body,
        )
            .into_response()
    };
    response
        .headers_mut()
        .insert(header::ETAG, etag.parse().unwrap());
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, "private, no-cache".parse().unwrap());
    response
        .headers_mut()
        .insert(header::VARY, "Authorization".parse().unwrap());
    response
}
async fn public_guard(State(s): State<AppState>, req: Request, next: Next) -> Response {
    if req.headers().contains_key(header::TRANSFER_ENCODING)
        || req.headers().get(header::CONTENT_LENGTH).is_some_and(|h| h != "0")
    {
        return error(StatusCode::PAYLOAD_TOO_LARGE, "body_not_allowed");
    }
    if !allow(&s, "public-download", 60) {
        return error(StatusCode::TOO_MANY_REQUESTS, "rate_limited");
    }
    next.run(req).await
}

async fn download_page() -> Response {
    let mut response = Html(include_str!("../../website/dist/index.html")).into_response();
    response.headers_mut().insert(header::CONTENT_SECURITY_POLICY,
        "default-src 'none'; style-src 'unsafe-inline'; base-uri 'none'; frame-ancestors 'none'".parse().unwrap());
    response.headers_mut().insert(header::X_CONTENT_TYPE_OPTIONS, "nosniff".parse().unwrap());
    response
}

async fn download_apk() -> Response {
    let bytes: &'static [u8] = include_bytes!("../../website/dist/Kollegiumi-menu.apk");
    let mut response = Response::new(axum::body::Body::from(bytes));
    response.headers_mut().insert(header::CONTENT_TYPE, "application/vnd.android.package-archive".parse().unwrap());
    response.headers_mut().insert(header::CONTENT_DISPOSITION, "attachment; filename=\"Kollegiumi-menu.apk\"".parse().unwrap());
    response.headers_mut().insert(header::CACHE_CONTROL, "public, max-age=300".parse().unwrap());
    response.headers_mut().insert(header::X_CONTENT_TYPE_OPTIONS, "nosniff".parse().unwrap());
    response
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route(
            "/health",
            get(|| async { Json(serde_json::json!({"status":"ok"})) }),
        )
        .merge(
            Router::new()
                .route("/", get(download_page))
                .route("/Kollegiumi-menu.apk", get(download_apk))
                .route_layer(middleware::from_fn_with_state(state.clone(), public_guard)),
        )
        .merge(
            Router::new()
                .route("/api/v1/menus", get(menus))
                .route("/api/v1/menus/current", get(menus))
                .route("/api/v1/menus/next", get(menus))
                .route_layer(middleware::from_fn_with_state(state.clone(), guard)),
        )
        .layer(axum::extract::DefaultBodyLimit::max(1024))
        .with_state(state)
}
