use anyhow::{Context, Result};
use dorm_api::{auth, http, storage, update};
use rand::RngCore;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::RwLock;
// Retry transient source errors promptly, then back off without exceeding the
// regular polling interval. A successful update resets the failure count.
fn source_delay_seconds(normal: u64, failures: u32, random: u16) -> u64 {
    if failures == 0 {
        return normal.saturating_add(u64::from(random) % 301);
    }
    let retry = 300u64.saturating_mul(1u64 << failures.saturating_sub(1).min(4)).min(3600);
    // Keep the first retry within 5–6 minutes and cap all retries at normal.
    retry.saturating_add(u64::from(random) % 61).min(normal)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let args: Vec<_> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("token") => {
            let mut bytes = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut bytes);
            let token = hex::encode(bytes);
            println!(
                "Token (show once): {token}\nSHA-256 verifier: {}",
                auth::digest(&token)
            );
            return Ok(());
        }
        Some("clock-check") => {
            let before = std::time::Instant::now();
            let (lock, condition) = (Mutex::new(false), std::sync::Condvar::new());
            let (_guard, timeout) = condition
                .wait_timeout(lock.lock().unwrap(), Duration::from_millis(20))
                .unwrap();
            anyhow::ensure!(
                timeout.timed_out() && before.elapsed() >= Duration::from_millis(10),
                "timed wait did not work"
            );
            println!(
                "Clock, monotonic timer and timed wait OK: {}",
                chrono::Utc::now().to_rfc3339()
            );
            return Ok(());
        }
        Some("inspect") => {
            let html = std::fs::read_to_string(args.get(2).context("inspect HTML_PATH")?)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&dorm_api::scraper::discover(
                    &html,
                    chrono::Utc::now().date_naive()
                )?)?
            );
            return Ok(());
        }
        _ => {}
    }
    let dir = PathBuf::from(std::env::var("DATA_DIR").unwrap_or("data".into()));
    std::fs::create_dir_all(&dir)?;
    // Advisory lock survives crashes without stale PID-file cleanup.
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(dir.join("writer.lock"))?;
    fs2::FileExt::try_lock_exclusive(&lock)
        .context("another server/check owns this data directory; trigger its SIGUSR1 instead")?;
    let store = storage::load(&dir)?;
    let client = update::client()?;
    if args.get(1).map(String::as_str) == Some("check") {
        let next = update::refresh(&client, &dir, &store).await?;
        println!("validated {} menus", next.menus.len());
        return Ok(());
    }
    let tokens = PathBuf::from(std::env::var("TOKEN_FILE").unwrap_or("data/tokens.json".into()));
    anyhow::ensure!(
        tokens.exists(),
        "configure TOKEN_FILE before starting (see README)"
    );
    let shared = Arc::new(RwLock::new(store));
    let rpm = std::env::var("RATE_LIMIT_RPM")
        .unwrap_or("30".into())
        .parse::<u32>()?
        .clamp(1, 1000);
    let state = http::AppState {
        store: shared.clone(),
        tokens,
        rates: Arc::new(Mutex::new(HashMap::new())),
        rpm,
    };
    let base = std::env::var("CHECK_INTERVAL_SECONDS")
        .unwrap_or("86400".into())
        .parse::<u64>()?
        .max(3600);
    tokio::spawn(async move {
        let mut signal =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::user_defined1())
                .expect("SIGUSR1 registration");
        let mut failures = 0u32;
        loop {
            let old = shared.read().await.clone();
            match update::refresh(&client, &dir, &old).await {
                Ok(next) => {
                    *shared.write().await = next;
                    failures = 0;
                    tracing::info!(event = "source_update_ok");
                }
                Err(e) => {
                    failures = failures.saturating_add(1);
                    tracing::warn!(event="source_update_failed",error=%e);
                    let mut s = shared.write().await;
                    s.last_source_check = Some(chrono::Utc::now());
                    s.source_error =
                        Some("Source update failed; retaining last validated menu".into());
                    if let Err(e) = storage::save(&dir, &s) {
                        tracing::error!(event="state_save_failed",error=%e);
                    }
                }
            }
            let today = chrono::Utc::now()
                .with_timezone(&chrono_tz::Europe::Budapest)
                .date_naive();
            let response = shared.read().await.response(today);
            let near_end = response.next_menu.is_none()
                && response.current_menu.as_ref().is_none_or(|m| {
                    m.menu
                        .entry
                        .valid_until
                        .is_none_or(|d| (d - today).num_days() <= 2)
                });
            let normal = if near_end { base.min(43200) } else { base };
            let delay = source_delay_seconds(normal, failures, rand::random());
            tracing::info!(event = "source_next_check", delay_seconds = delay, consecutive_failures = failures);
            tokio::select! {_=tokio::time::sleep(Duration::from_secs(delay))=>{},_=signal.recv()=>{}}
        }
    });
    let addr = std::env::var("BIND").unwrap_or("127.0.0.1:3000".into());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(event = "server_started");
    axum::serve(listener, http::router(state))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}

#[cfg(test)]
mod scheduling_tests {
    use super::source_delay_seconds;

    #[test]
    fn failures_retry_quickly_then_back_off_and_cap() {
        assert_eq!(source_delay_seconds(86400, 1, 0), 300);
        assert_eq!(source_delay_seconds(86400, 1, 60), 360);
        assert_eq!(source_delay_seconds(86400, 2, 0), 600);
        assert_eq!(source_delay_seconds(86400, 3, 0), 1200);
        assert_eq!(source_delay_seconds(86400, 100, 60), 3660);
        assert_eq!(source_delay_seconds(3600, 100, 60), 3600);
        assert_eq!(source_delay_seconds(86400, 0, 0), 86400);
    }
}
