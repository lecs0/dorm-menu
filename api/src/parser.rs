//! Layout adapter for the two inspected, single-page landscape tables.
//! Keep raw meal text; dish boundaries and allergen-code meanings are not inferred.
use anyhow::{ensure, Result};
use chrono::{Duration, Utc};
use dorm_shared::{Day, Entry, Meal, Menu};
use regex::Regex;
use sha2::{Digest, Sha256};
#[derive(Clone, Debug)]
struct Word {
    x: f64,
    y: f64,
    text: String,
}
pub fn parse_bbox(xml: &str, entry: &Entry, pdf: &[u8]) -> Result<Menu> {
    let doc = roxmltree::Document::parse_with_options(
        xml,
        roxmltree::ParsingOptions {
            allow_dtd: true,
            ..Default::default()
        },
    )?;
    ensure!(
        doc.descendants().filter(|n| n.has_tag_name("page")).count() == 1,
        "unsupported PDF page count"
    );
    let mut words = vec![];
    for n in doc.descendants().filter(|n| n.has_tag_name("word")) {
        words.push(Word {
            x: (n.attribute("xMin").unwrap_or("").parse::<f64>()?
                + n.attribute("xMax").unwrap_or("").parse::<f64>()?)
                / 2.,
            y: n.attribute("yMin").unwrap_or("").parse()?,
            text: n.text().unwrap_or("").into(),
        });
    }
    ensure!(
        words.len() > 60 && words.len() < 5000,
        "suspicious PDF text size"
    );
    let names = ["HÉTFŐ", "KEDD", "SZERDA", "CSÜTÖRTÖK", "PÉNTEK"];
    let mut headers = vec![];
    for name in names {
        let h: Vec<_> = words.iter().filter(|w| w.text == name).collect();
        ensure!(h.len() == 1, "weekday header missing/duplicated");
        headers.push(h[0].clone());
    }
    ensure!(
        headers
            .windows(2)
            .all(|w| w[0].x < w[1].x && (w[0].y - w[1].y).abs() < 4.),
        "weekday columns changed"
    );
    let left = headers[0].x - (headers[1].x - headers[0].x) / 2.;
    let label = words
        .iter()
        .filter(|w| w.x < left)
        .map(|w| w.text.as_str())
        .collect::<Vec<_>>()
        .join("");
    ensure!(
        (label.contains("REGGELI") || label.contains("REGGELUI")) && label.contains("VACSORA"),
        "meal labels changed"
    );
    // Salt value lines bound each meal, including wrapped content below its label.
    let salt = Regex::new(r"^\d+(?:[,\.]\d+)?\s*g$")?;
    let mut salt_ys: Vec<f64> = words
        .iter()
        .filter(|w| w.x >= left && (salt.is_match(&w.text) || w.text == "g"))
        .map(|w| w.y)
        .collect();
    salt_ys.sort_by(f64::total_cmp);
    let mut bands: Vec<f64> = vec![];
    for y in salt_ys {
        if bands.last().is_none_or(|last| y - last > 20.) {
            bands.push(y);
        }
    }
    ensure!(bands.len() == 3, "expected three salt rows");
    let from = entry
        .valid_from
        .ok_or_else(|| anyhow::anyhow!("unknown calendar dates"))?;
    ensure!(
        from.weekday() == chrono::Weekday::Mon
            && entry.valid_until == Some(from + Duration::days(4)),
        "this layout adapter requires five published Mon–Fri dates"
    );
    let mut days = vec![];
    let code = Regex::new(r"(?:^|[^\d])((?:1[0-4]|[1-9])(?:\s*,\s*(?:1[0-4]|[1-9]))*),?")?;
    for i in 0..5 {
        let low = if i == 0 {
            left
        } else {
            (headers[i - 1].x + headers[i].x) / 2.
        };
        let high = if i == 4 {
            f64::MAX
        } else {
            (headers[i].x + headers[i + 1].x) / 2.
        };
        let mut meals = vec![];
        for (j, category) in ["breakfast", "lunch", "dinner"].iter().enumerate() {
            let start = if j == 0 {
                headers[0].y + 10.
            } else {
                bands[j - 1] + 12.
            };
            let end = bands[j];
            let mut selected: Vec<_> = words
                .iter()
                .filter(|w| w.x >= low && w.x < high && w.y > start && w.y < end - 3.)
                .cloned()
                .collect();
            selected.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)));
            let mut lines: Vec<(f64, Vec<Word>)> = vec![];
            for w in selected {
                if let Some(l) = lines.last_mut().filter(|l| (w.y - l.0).abs() < 3.) {
                    l.1.push(w);
                } else {
                    lines.push((w.y, vec![w]));
                }
            }
            let components: Vec<String> = lines
                .into_iter()
                .map(|(_, mut ws)| {
                    ws.sort_by(|a, b| a.x.total_cmp(&b.x));
                    ws.iter()
                        .map(|w| w.text.as_str())
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .collect();
            let original_text = components.join("\n");
            if original_text.is_empty() {
                ensure!(i == 4 && j == 2, "unexpected empty meal");
                continue;
            }
            let mut codes = vec![];
            for c in code.captures_iter(&original_text) {
                codes.push(c[1].into());
            }
            let salt_text = words
                .iter()
                .filter(|w| w.x >= low && w.x < high && (w.y - end).abs() < 20.)
                .map(|w| w.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            meals.push(Meal {
                category: (*category).into(),
                original_text,
                components,
                allergen_status: if codes.is_empty() {
                    "unknown"
                } else {
                    "source_codes_only"
                }
                .into(),
                source_allergen_codes: codes,
                salt_text: Some(salt_text),
            });
        }
        days.push(Day {
            date: from + Duration::days(i as i64),
            day_name: names[i].into(),
            meals,
        });
    }
    let now = Utc::now();
    Ok(Menu {menu_id:format!("{}-{}",from,entry.week_number),version:hex::encode(Sha256::digest(pdf)),institution:"Radnóti Miklós Kollégium és III. Béla Gimnázium (page attribution)".into(),entry:entry.clone(),source_page:crate::scraper::PAGE.into(),retrieved_at:now,updated_at:now,days,
        notes:vec!["The PDF header uses graphical text not returned by pdftotext. Calendar dates and institution are attributed from the page entry; automatic PDF-header date cross-check is unavailable. Inspected fixtures visually agree with the page. No allergen legend was found.".into(),"Components preserve visual lines, not guaranteed dish boundaries. Numeric allergen codes are source text only; their meanings and completeness are unknown. Friday dinner is blank in the source.".into()]})
}
use chrono::Datelike;
