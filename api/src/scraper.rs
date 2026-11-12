use anyhow::{bail, ensure, Context, Result};
use chrono::{Datelike, NaiveDate};
use dorm_shared::Entry;
use regex::Regex;
use scraper::{ElementRef, Html, Node};
use url::Url;
pub const PAGE: &str = "https://bajaionkormanyzat.hu/index.php/kozerdeku/szocialis-intezmenyek/intezmenyi-es-szunidei-gyermeketkeztetes";
pub const HOST: &str = "bajaionkormanyzat.hu";
fn norm(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_uppercase()
}
pub fn trusted(url: &Url) -> bool {
    url.host_str() == Some(HOST)
        && url.scheme() == "https"
        && url.port_or_known_default() == Some(443)
        && url.username().is_empty()
        && url.password().is_none()
}
pub fn discover(html: &str, today: NaiveDate) -> Result<Vec<Entry>> {
    let doc = Html::parse_document(html);
    let mut active = false;
    let mut found = 0;
    let mut rows = vec![];
    let mut text = String::new();
    let mut link: Option<String> = None;
    for node in doc.root_element().descendants() {
        match node.value() {
            Node::Element(e) if ["strong", "h2", "h3", "h4"].contains(&e.name()) => {
                let heading = norm(&ElementRef::wrap(node).unwrap().text().collect::<String>());
                if heading.contains("RADNÓTI MIKLÓS")
                    && heading.contains("III. BÉLA")
                    && heading.contains("ÉTLAPOK")
                {
                    found += 1;
                    active = true;
                    text.clear();
                    link = None;
                } else if active && !heading.is_empty() {
                    rows.push((std::mem::take(&mut text), link.take()));
                    active = false;
                }
            }
            Node::Element(e) if active && ["br", "p"].contains(&e.name()) => {
                rows.push((std::mem::take(&mut text), link.take()));
            }
            Node::Element(e) if active && e.name() == "a" => {
                if let Some(href) = e.attr("href") {
                    let mut url = Url::parse(PAGE)?.join(href)?;
                    // Upgrade links on the verified HTTPS municipality host only.
                    if url.host_str() == Some(HOST) && url.scheme() == "http" {
                        url.set_scheme("https").ok();
                    }
                    ensure!(trusted(&url), "untrusted source link");
                    ensure!(
                        url.path().to_ascii_lowercase().ends_with(".pdf"),
                        "unexpected menu link type"
                    );
                    ensure!(link.is_none(), "multiple links in one menu row");
                    link = Some(url.to_string());
                }
            }
            Node::Text(t) if active => {
                let inside_heading = node.ancestors().any(|n| {
                    n.value()
                        .as_element()
                        .is_some_and(|e| ["strong", "h2", "h3", "h4"].contains(&e.name()))
                });
                if !inside_heading {
                    text.push_str(t);
                }
            }
            _ => {}
        }
    }
    if active {
        rows.push((text, link));
    }
    ensure!(
        found == 1,
        "expected exactly one institution section; layout changed"
    );
    let re = Regex::new(
        r"(?i)(\d{1,2})\.\s*heti\s+men[^\(]*\(\s*(\d{1,2})\.(\d{1,2})\.\s*-\s*(\d{1,2})\.(\d{1,2})\.\s*\)",
    )?;
    let year_re = Regex::new(r"(?i)Kollegium(20\d{2})(\d{2})het\.pdf$")?;
    let mut out = vec![];
    for (label, source_url) in rows {
        let label = label.split_whitespace().collect::<Vec<_>>().join(" ");
        if label.is_empty() {
            continue;
        }
        let c = re
            .captures(&label)
            .with_context(|| format!("unrecognized institution row: {label}"))?;
        let n = |i: usize| c[i].parse::<u32>().unwrap();
        let week = n(1);
        let hint = source_url.as_ref().and_then(|u| year_re.captures(u));
        if let Some(h) = &hint {
            ensure!(
                h[2].parse::<u32>()? == week,
                "filename/page week disagreement"
            );
        }
        let mut candidates = vec![];
        for year in today.year() - 1..=today.year() + 1 {
            let Some(a) = NaiveDate::from_ymd_opt(year, n(2), n(3)) else {
                continue;
            };
            let Some(b) = NaiveDate::from_ymd_opt(year + i32::from(n(4) < n(2)), n(4), n(5)) else {
                continue;
            };
            if hint.as_ref().is_some_and(|h| {
                let filename_year = h[1].parse::<i32>().ok();
                filename_year != Some(a.year()) && filename_year != Some(a.iso_week().year())
            }) {
                continue;
            }
            if a.iso_week().week() == week
                && b >= a
                && (b - a).num_days() <= 14
                && (hint.is_some() || (a - today).num_days().abs() <= 180)
            {
                candidates.push((a, b));
            }
        }
        ensure!(
            candidates.len() == 1,
            "ambiguous or inconsistent year/date/week: {label}"
        );
        let (a, b) = candidates[0];
        let has_year_hint = hint.is_some();
        out.push(Entry {
            label,
            week_number: week,
            valid_from: Some(a),
            valid_until: Some(b),
            source_url,
            date_evidence: if has_year_hint {
                "page month/day + filename calendar/ISO week year; ISO week cross-checked"
            } else {
                "page month/day + nearest unique year within 180 days; ISO week cross-checked"
            }
            .into(),
        });
    }
    if out.is_empty() {
        bail!("institution section contains no recognized menu rows");
    }
    ensure!(out.len() <= 20, "too many menu entries");
    Ok(out)
}
