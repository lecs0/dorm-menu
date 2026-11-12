use chrono::{Duration, NaiveDate};
use dorm_shared::{MenuView, Response};
use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;
#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Preferences {
    pub keywords: Vec<String>,
    pub reminders: bool,
    pub reminder_hour: u32,
    pub reminder_minute: u32,
}
pub fn keyword_matches(text: &str, keywords: &[String]) -> Vec<String> {
    let text: String = text.nfc().collect();
    let keys: Vec<_> = keywords.iter().map(|k| k.nfc().collect()).collect();
    dorm_shared::matches(&text, &keys)
}
pub fn decode_cache(json: &str) -> Result<Response, String> {
    let r: Response = serde_json::from_str(json).map_err(|_| "Invalid cached menu".to_string())?;
    if r.schema_version != 1 {
        return Err("Unsupported schema; update the app".into());
    }
    Ok(r)
}
/// Recompute status from calendar dates while offline; the server snapshot may be old.
pub fn localize(mut r: Response, today: NaiveDate) -> Response {
    for m in &mut r.menus {
        m.status = dorm_shared::status(&m.menu.entry, today).into();
    }
    r.current_menu = r
        .menus
        .iter()
        .rev()
        .find(|m| m.status == "current")
        .or_else(|| r.menus.iter().rev().find(|m| m.status == "expired"))
        .or_else(|| r.menus.iter().find(|m| m.status == "unknown_validity"))
        .cloned();
    r.next_menu = r.menus.iter().find(|m| m.status == "upcoming").cloned();
    r.as_of = today;
    r
}
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Reminder {
    pub id: String,
    pub date: NaiveDate,
    pub message: String,
}
pub fn reminders(r: &Response, p: &Preferences, today: NaiveDate) -> Vec<Reminder> {
    if !p.reminders {
        return vec![];
    }
    let tomorrow = today + Duration::days(1);
    let mut out = vec![];
    for MenuView { menu, .. } in &r.menus {
        for day in menu.days.iter().filter(|d| d.date == tomorrow) {
            for meal in &day.meals {
                let hits = keyword_matches(&meal.original_text, &p.keywords);
                if !hits.is_empty() {
                    out.push(Reminder{id:format!("{}:{}",day.date,meal.category),date:day.date,message:format!("Tomorrow's {} includes “{}”. Check whether you need to cancel your order.",meal.category,hits.join(", "))});
                }
            }
        }
    }
    out
}
/// JNI surface is one JSON request/response to keep the Android bridge small.
pub fn dispatch(input: &str) -> Result<String, String> {
    let v: serde_json::Value = serde_json::from_str(input).map_err(|e| e.to_string())?;
    let today = NaiveDate::parse_from_str(v["today"].as_str().ok_or("missing today")?, "%Y-%m-%d")
        .map_err(|e| e.to_string())?;
    let r = localize(
        decode_cache(v["cache"].as_str().ok_or("missing cache")?)?,
        today,
    );
    let p: Preferences =
        serde_json::from_value(v["preferences"].clone()).map_err(|e| e.to_string())?;
    let plans = reminders(&r, &p, today);
    let mut value = serde_json::to_value(&r).map_err(|e| e.to_string())?;
    value["reminders"] = serde_json::to_value(plans).unwrap();
    for m in value["menus"].as_array_mut().unwrap() {
        for d in m["days"].as_array_mut().unwrap() {
            for meal in d["meals"].as_array_mut().unwrap() {
                meal["keyword_matches"] = serde_json::json!(keyword_matches(
                    meal["original_text"].as_str().unwrap_or(""),
                    &p.keywords
                ));
            }
        }
    }
    serde_json::to_string(&value).map_err(|e| e.to_string())
}
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_hu_dormmenu_MainActivity_process(
    mut env: jni::JNIEnv,
    _obj: jni::objects::JObject,
    input: jni::objects::JString,
) -> jni::sys::jstring {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let s: String = env.get_string(&input).map_err(|e| e.to_string())?.into();
        dispatch(&s)
    }));
    let text = match result {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => serde_json::json!({"error":e}).to_string(),
        Err(_) => "{\"error\":\"Internal app error\"}".into(),
    };
    env.new_string(text)
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accents() {
        assert_eq!(
            keyword_matches("TEJFÖL gomba", &["tejfo\u{308}l".into()]),
            vec!["tejföl"]
        );
    }
    #[test]
    fn cache_schema() {
        assert!(decode_cache("{}").is_err());
    }
}
#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    fn fixture() -> Response {
        let d = NaiveDate::from_ymd_opt(2026, 11, 9).unwrap();
        let entry = dorm_shared::Entry {
            label: "synthetic".into(),
            week_number: 46,
            valid_from: Some(d),
            valid_until: Some(d + Duration::days(4)),
            source_url: None,
            date_evidence: "synthetic".into(),
        };
        let menu = dorm_shared::Menu {
            menu_id: "synthetic".into(),
            version: "one".into(),
            institution: "synthetic".into(),
            entry,
            source_page: "synthetic".into(),
            retrieved_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            days: vec![dorm_shared::Day {
                date: d + Duration::days(1),
                day_name: "KEDD".into(),
                meals: vec![dorm_shared::Meal {
                    category: "breakfast".into(),
                    original_text: "Gombás szendvics".into(),
                    components: vec![],
                    allergen_status: "unknown".into(),
                    source_allergen_codes: vec![],
                    salt_text: None,
                }],
            }],
            notes: vec![],
        };
        dorm_shared::Store {
            menus: vec![menu],
            ..Default::default()
        }
        .response(d)
    }
    #[test]
    fn offline_rollover_and_schema() {
        let r = fixture();
        let json = serde_json::to_string(&r).unwrap();
        let cached = decode_cache(&json).unwrap();
        let weekend = NaiveDate::from_ymd_opt(2026, 11, 14).unwrap();
        assert_eq!(
            localize(cached, weekend).current_menu.unwrap().status,
            "expired"
        );
        let mut v = serde_json::to_value(r).unwrap();
        v["schema_version"] = 2.into();
        assert!(decode_cache(&v.to_string()).is_err());
    }
    #[test]
    fn reminders_stable_and_updated() {
        let r = fixture();
        let d = NaiveDate::from_ymd_opt(2026, 11, 9).unwrap();
        let mut p = Preferences {
            keywords: vec!["SZENDVICS".into()],
            reminders: true,
            reminder_hour: 18,
            reminder_minute: 0,
        };
        let plans = reminders(&r, &p, d);
        assert_eq!(plans.len(), 1);
        assert_eq!(plans, reminders(&r, &p, d));
        assert!(plans[0].message.contains("Check whether"));
        p.keywords = vec!["hal".into()];
        assert!(reminders(&r, &p, d).is_empty());
        p.reminders = false;
        assert!(reminders(&r, &p, d).is_empty());
    }
}
#[cfg(test)]
mod unknown_tests {
    use super::*;
    #[test]
    fn unknown_validity_remains_visible_offline() {
        let day = NaiveDate::from_ymd_opt(2026, 11, 11).unwrap();
        let entry = dorm_shared::Entry {
            label: "synthetic unknown".into(),
            week_number: 46,
            valid_from: None,
            valid_until: None,
            source_url: None,
            date_evidence: "unknown".into(),
        };
        let menu = dorm_shared::Menu {
            menu_id: "unknown".into(),
            version: "unknown".into(),
            institution: "synthetic".into(),
            entry,
            source_page: "synthetic".into(),
            retrieved_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            days: vec![],
            notes: vec![],
        };
        let snapshot = dorm_shared::Store {
            menus: vec![menu],
            ..Default::default()
        }
        .response(day);
        assert_eq!(
            localize(snapshot, day).current_menu.unwrap().status,
            "unknown_validity"
        );
    }
}
