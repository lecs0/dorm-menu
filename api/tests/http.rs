use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use dorm_api::{
    auth,
    http::{router, AppState},
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tower::ServiceExt;
#[tokio::test]
async fn authentication_revocation_etags_and_rate_limits() {
    let dir = std::env::temp_dir().join(format!("dorm-auth-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("tokens.json");
    let token = "a".repeat(64);
    std::fs::write(
        &path,
        serde_json::json!({"device":auth::digest(&token)}).to_string(),
    )
    .unwrap();
    let app = router(AppState {
        store: Arc::new(tokio::sync::RwLock::new(Default::default())),
        tokens: path.clone(),
        rates: Arc::new(Mutex::new(HashMap::new())),
        rpm: 3,
    });
    let request = |t: Option<&str>, etag: Option<&str>| {
        let mut r = Request::builder().uri("/api/v1/menus");
        if let Some(t) = t {
            r = r.header("Authorization", format!("Bearer {t}"));
        }
        if let Some(e) = etag {
            r = r.header("If-None-Match", e);
        }
        r.body(Body::empty()).unwrap()
    };
    assert_eq!(
        app.clone()
            .oneshot(request(None, None))
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let r = app
        .clone()
        .oneshot(request(Some(&token), None))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let etag = r.headers()["etag"].to_str().unwrap();
    assert_eq!(
        app.clone()
            .oneshot(request(Some(&token), Some(etag)))
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_MODIFIED
    );
    std::fs::write(&path, "{}").unwrap();
    assert_eq!(
        app.clone()
            .oneshot(request(Some(&token), None))
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    std::fs::write(
        &path,
        serde_json::json!({"device":auth::digest(&token)}).to_string(),
    )
    .unwrap();
    assert_eq!(
        app.clone()
            .oneshot(request(Some(&token), None))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        app.oneshot(request(Some(&token), None))
            .await
            .unwrap()
            .status(),
        StatusCode::TOO_MANY_REQUESTS
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn website_and_apk_are_public_while_menu_stays_private() {
    let app = router(AppState {
        store: Arc::new(tokio::sync::RwLock::new(Default::default())),
        tokens: std::env::temp_dir().join("dorm-no-such-token-file"),
        rates: Arc::new(Mutex::new(HashMap::new())),
        rpm: 30,
    });
    let get = |path: &str| Request::builder().uri(path).body(Body::empty()).unwrap();
    let page = app.clone().oneshot(get("/")).await.unwrap();
    assert_eq!(page.status(), StatusCode::OK);
    assert!(page.headers()["content-type"].to_str().unwrap().starts_with("text/html"));
    let apk = app.clone().oneshot(get("/Kollegiumi-menu.apk")).await.unwrap();
    assert_eq!(apk.status(), StatusCode::OK);
    assert_eq!(apk.headers()["content-type"], "application/vnd.android.package-archive");
    assert!(apk.headers()["content-disposition"].to_str().unwrap().contains("Kollegiumi-menu.apk"));
    let bytes = axum::body::to_bytes(apk.into_body(), 3_000_000).await.unwrap();
    assert_eq!(bytes.as_ref(), include_bytes!("../../website/dist/Kollegiumi-menu.apk"));
    assert_eq!(app.clone().oneshot(get("/api/v1/menus")).await.unwrap().status(), StatusCode::UNAUTHORIZED);
    assert_eq!(app.oneshot(get("/admin")).await.unwrap().status(), StatusCode::NOT_FOUND);
}
