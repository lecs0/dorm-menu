use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
    pub label: String,
    pub week_number: u32,
    pub valid_from: Option<NaiveDate>,
    pub valid_until: Option<NaiveDate>,
    pub source_url: Option<String>,
    pub date_evidence: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Meal {
    pub category: String,
    pub original_text: String,
    pub components: Vec<String>,
    pub allergen_status: String,
    pub source_allergen_codes: Vec<String>,
    pub salt_text: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Day {
    pub date: NaiveDate,
    pub day_name: String,
    pub meals: Vec<Meal>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Menu {
    pub menu_id: String,
    pub version: String,
    pub institution: String,
    pub entry: Entry,
    pub source_page: String,
    pub retrieved_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub days: Vec<Day>,
    pub notes: Vec<String>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Store {
    pub menus: Vec<Menu>,
    pub entries: Vec<Entry>,
    pub last_source_check: Option<DateTime<Utc>>,
    pub last_successful_update: Option<DateTime<Utc>>,
    pub source_error: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MenuView {
    #[serde(flatten)]
    pub menu: Menu,
    pub status: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Response {
    pub schema_version: u32,
    pub as_of: NaiveDate,
    pub last_source_check: Option<DateTime<Utc>>,
    pub last_successful_update: Option<DateTime<Utc>>,
    pub source_status: String,
    pub current_menu: Option<MenuView>,
    pub next_menu: Option<MenuView>,
    pub menus: Vec<MenuView>,
    pub unpublished: Vec<Entry>,
}
pub fn status(e: &Entry, today: NaiveDate) -> &'static str {
    match (e.valid_from, e.valid_until) {
        (Some(a), Some(b)) if a <= today && today <= b => "current",
        (Some(a), Some(_)) if a > today => "upcoming",
        (Some(_), Some(b)) if b < today => "expired",
        _ => "unknown_validity",
    }
}
impl Store {
    pub fn response(&self, today: NaiveDate) -> Response {
        let mut menus: Vec<_> = self
            .menus
            .iter()
            .map(|m| MenuView {
                menu: m.clone(),
                status: status(&m.entry, today).into(),
            })
            .collect();
        menus.sort_by_key(|m| m.menu.entry.valid_from);
        let current = menus
            .iter()
            .rev()
            .find(|m| m.status == "current")
            .or_else(|| menus.iter().rev().find(|m| m.status == "expired"))
            .or_else(|| menus.iter().find(|m| m.status == "unknown_validity"))
            .cloned();
        let next = menus.iter().find(|m| m.status == "upcoming").cloned();
        Response {
            schema_version: 1,
            as_of: today,
            last_source_check: self.last_source_check,
            last_successful_update: self.last_successful_update,
            source_status: if self.source_error.is_some() {
                "error"
            } else if self.last_successful_update.is_some() {
                "ok"
            } else {
                "unavailable"
            }
            .into(),
            current_menu: current,
            next_menu: next,
            menus,
            unpublished: self
                .entries
                .iter()
                .filter(|e| e.source_url.is_none())
                .cloned()
                .collect(),
        }
    }
}
/// Unicode lowercase + NFC normalization is applied in the app before this call.
/// Substring matching is intentional: `hal` also matches `halászlé`.
pub fn matches(text: &str, keywords: &[String]) -> Vec<String> {
    let text = text.to_lowercase();
    keywords
        .iter()
        .filter(|k| !k.trim().is_empty() && text.contains(&k.trim().to_lowercase()))
        .cloned()
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dates_and_weekends() {
        let mut e = Entry {
            label: "sample".into(),
            week_number: 46,
            valid_from: Some(NaiveDate::from_ymd_opt(2026, 11, 9).unwrap()),
            valid_until: Some(NaiveDate::from_ymd_opt(2026, 11, 13).unwrap()),
            source_url: None,
            date_evidence: "synthetic".into(),
        };
        assert_eq!(
            status(&e, NaiveDate::from_ymd_opt(2026, 11, 11).unwrap()),
            "current"
        );
        assert_eq!(
            status(&e, NaiveDate::from_ymd_opt(2026, 11, 14).unwrap()),
            "expired"
        );
        assert_eq!(
            status(&e, NaiveDate::from_ymd_opt(2026, 11, 8).unwrap()),
            "upcoming"
        );
        e.valid_from = None;
        assert_eq!(status(&e, Utc::now().date_naive()), "unknown_validity");
    }
    #[test]
    fn hungarian() {
        assert_eq!(
            matches("GOMBÁS szendvics", &["gombá".into()]),
            vec!["gombá"]
        );
    }
}
