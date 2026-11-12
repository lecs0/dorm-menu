use anyhow::{ensure, Context, Result};
use chrono::{Duration, Utc};
use dorm_shared::Store;
use reqwest::Client;
use std::{path::Path, time::Duration as StdDuration};
use tokio::{io::AsyncReadExt, process::Command};
use url::Url;
pub fn client() -> Result<Client> {
    Ok(Client::builder()
        .user_agent("DormMenu/0.1 (personal dorm menu reader; daily checks)")
        .connect_timeout(StdDuration::from_secs(10))
        .timeout(StdDuration::from_secs(40))
        .redirect(reqwest::redirect::Policy::custom(|a| {
            if a.previous().len() >= 3 || !crate::scraper::trusted(a.url()) {
                a.error("untrusted redirect")
            } else {
                a.follow()
            }
        }))
        .build()?)
}
async fn download(client: &Client, url: &str, max: usize, pdf: bool) -> Result<Vec<u8>> {
    ensure!(crate::scraper::trusted(&Url::parse(url)?), "untrusted URL");
    let mut res = client.get(url).send().await?.error_for_status()?;
    let ct = res
        .headers()
        .get("content-type")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    ensure!(
        if pdf {
            ct.starts_with("application/pdf") || ct.starts_with("application/octet-stream")
        } else {
            ct.starts_with("text/html")
        },
        "unexpected source content type"
    );
    ensure!(
        res.content_length().unwrap_or(0) <= max as u64,
        "source too large"
    );
    let mut bytes = vec![];
    while let Some(chunk) = res.chunk().await? {
        ensure!(bytes.len() + chunk.len() <= max, "source too large");
        bytes.extend_from_slice(&chunk);
    }
    if pdf {
        ensure!(bytes.starts_with(b"%PDF-"), "not a PDF");
    }
    Ok(bytes)
}
async fn extract(dir: &Path, bytes: &[u8]) -> Result<String> {
    // No shell. One bounded subprocess; container memory/CPU/pid limits also apply.
    let path = dir.join("extract.pdf");
    tokio::fs::write(&path, bytes).await?;
    let result = async {
        let mut child = Command::new("pdftotext")
            .args(["-bbox-layout", "-enc", "UTF-8"])
            .arg(&path)
            .arg("-")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()?;
        let mut stdout = child.stdout.take().unwrap();
        let mut buf = Vec::new();
        let run = async {
            (&mut stdout).take(2_000_001).read_to_end(&mut buf).await?;
            ensure!(buf.len() <= 2_000_000, "extracted text too large");
            ensure!(child.wait().await?.success(), "PDF extraction failed");
            Ok::<_, anyhow::Error>(())
        };
        tokio::time::timeout(StdDuration::from_secs(20), run)
            .await
            .context("PDF extraction timeout")??;
        Ok::<_, anyhow::Error>(String::from_utf8(buf)?)
    }
    .await;
    let _ = tokio::fs::remove_file(path).await;
    result
}
pub async fn refresh(client: &Client, dir: &Path, old: &Store) -> Result<Store> {
    let today = Utc::now()
        .with_timezone(&chrono_tz::Europe::Budapest)
        .date_naive();
    let html = download(client, crate::scraper::PAGE, 2_000_000, false).await?;
    let entries = crate::scraper::discover(std::str::from_utf8(&html)?, today)?;
    let mut next = old.clone();
    next.entries = entries.clone();
    tokio::fs::create_dir_all(dir).await?;
    for entry in entries.iter().filter(|e| {
        e.source_url.is_some()
            && e.valid_until
                .is_some_and(|d| d >= today - Duration::days(21))
    }) {
        // Download even unchanged URLs: content hashes detect replacements without relying on server headers.
        let bytes = download(client, entry.source_url.as_ref().unwrap(), 5_000_000, true).await?;
        let xml = extract(dir, &bytes).await?;
        let mut menu = crate::parser::parse_bbox(&xml, entry, &bytes)?;
        if let Some(previous) = old
            .menus
            .iter()
            .find(|m| m.menu_id == menu.menu_id && m.version == menu.version)
        {
            menu.retrieved_at = previous.retrieved_at;
            menu.updated_at = previous.updated_at;
        }
        next.menus.retain(|m| m.menu_id != menu.menu_id);
        next.menus.push(menu);
    }
    ensure!(!next.menus.is_empty(), "no validated published menus");
    // Keep one latest expired menu plus current/future; never drop fallback on failed refresh.
    next.menus.sort_by_key(|m| m.entry.valid_from);
    let latest_expired = next
        .menus
        .iter()
        .rposition(|m| m.entry.valid_until.is_some_and(|d| d < today));
    next.menus = next
        .menus
        .into_iter()
        .enumerate()
        .filter(|(i, m)| {
            Some(*i) == latest_expired || m.entry.valid_until.is_none_or(|d| d >= today)
        })
        .map(|(_, m)| m)
        .collect();
    ensure!(next.menus.len() <= 20, "too many stored menus");
    next.last_source_check = Some(Utc::now());
    next.last_successful_update = Some(Utc::now());
    next.source_error = None;
    crate::storage::save(dir, &next)?;
    Ok(next)
}
