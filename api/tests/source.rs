use chrono::NaiveDate;
use dorm_api::{parser, scraper};
fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 11).unwrap()
}
#[test]
fn real_section_and_missing_links() {
    let entries = scraper::discover(
        include_str!("../../fixtures/source/menu-page.html"),
        today(),
    )
    .unwrap();
    assert_eq!(entries.len(), 4);
    assert_eq!(entries.iter().filter(|e| e.source_url.is_some()).count(), 2);
    assert!(entries
        .iter()
        .filter_map(|e| e.source_url.as_ref())
        .all(|u| u.contains("Kollegium")));
    assert_eq!(
        entries[0].valid_from,
        Some(NaiveDate::from_ymd_opt(2026, 11, 2).unwrap())
    );
    assert_eq!(entries[2].week_number, 47);
    assert!(entries[2].source_url.is_none());
}
#[test]
fn real_pdf_tables() {
    let entries = scraper::discover(
        include_str!("../../fixtures/source/menu-page.html"),
        today(),
    )
    .unwrap();
    for (i, xml, pdf) in [
        (
            0,
            include_str!("../../fixtures/source/week45.bbox.html"),
            include_bytes!("../../fixtures/source/week45.pdf").as_slice(),
        ),
        (
            1,
            include_str!("../../fixtures/source/week46.bbox.html"),
            include_bytes!("../../fixtures/source/week46.pdf").as_slice(),
        ),
    ] {
        let menu = parser::parse_bbox(xml, &entries[i], pdf).unwrap();
        assert_eq!(menu.days.len(), 5);
        assert_eq!(menu.days[0].meals.len(), 3);
        assert_eq!(menu.days[4].meals.len(), 2);
        let breakfast = &menu.days[0].meals[0];
        assert!(
            breakfast.original_text.contains(if i == 0 {
                "Erdei tea"
            } else {
                "Vaníliás tej"
            }),
            "{}",
            breakfast.original_text
        );
        assert!(menu.days[0].meals[1].original_text.contains(if i == 0 {
            "Carbonara"
        } else {
            "Sertéspörkölt"
        }));
        assert!(!breakfast.source_allergen_codes.is_empty());
        let mut revised_pdf = pdf.to_vec();
        revised_pdf.extend_from_slice(b"\n% synthetic same-URL revision for hash test\n");
        let changed = parser::parse_bbox(xml, &entries[i], &revised_pdf).unwrap();
        assert_eq!(menu.entry.source_url, changed.entry.source_url);
        assert_ne!(menu.version, changed.version);
        assert_eq!(menu.menu_id, changed.menu_id);
        let view = dorm_shared::Store {
            menus: vec![menu],
            ..Default::default()
        }
        .response(today());
        assert!(view.current_menu.is_some());
    }
}
#[test]
fn reject_changes() {
    assert!(scraper::discover("<strong>Other institution</strong>", today()).is_err());
    let html = include_str!("../../fixtures/source/menu-page.html").replace(
        "/images/2026/10/28/Kollegium202645het.pdf",
        "https://evil.example/menu.pdf",
    );
    assert!(scraper::discover(&html, today()).is_err());
    let entries = scraper::discover(
        include_str!("../../fixtures/source/menu-page.html"),
        today(),
    )
    .unwrap();
    assert!(parser::parse_bbox("<doc/>", &entries[0], b"bad").is_err());
}
#[test]
fn year_boundary_synthetic() {
    let html="<p><strong>RADNÓTI MIKLÓS KOLLÉGIUMI ÉS III. BÉLA GIMNÁZIUMI ÉTLAPOK:</strong><br>53. heti menü (12.28. - 01.01.)<br><strong>OTHER SECTION</strong><a href='/other.pdf'>1. heti menü</a></p>";
    let e = scraper::discover(html, NaiveDate::from_ymd_opt(2026, 12, 30).unwrap()).unwrap();
    assert_eq!(
        e[0].valid_until,
        Some(NaiveDate::from_ymd_opt(2027, 1, 1).unwrap())
    );
}
#[test]
fn atomic_store_and_rejected_parse() {
    let dir = std::env::temp_dir().join(format!("dorm-store-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let s = dorm_shared::Store {
        source_error: Some("sentinel".into()),
        ..Default::default()
    };
    dorm_api::storage::save(&dir, &s).unwrap();
    let entries = scraper::discover(
        include_str!("../../fixtures/source/menu-page.html"),
        today(),
    )
    .unwrap();
    assert!(parser::parse_bbox("bad", &entries[0], b"bad").is_err());
    assert_eq!(
        dorm_api::storage::load(&dir).unwrap().source_error,
        s.source_error
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn failed_live_pipeline_preserves_real_store() {
    let entries = scraper::discover(
        include_str!("../../fixtures/source/menu-page.html"),
        today(),
    )
    .unwrap();
    let menu = parser::parse_bbox(
        include_str!("../../fixtures/source/week46.bbox.html"),
        &entries[1],
        include_bytes!("../../fixtures/source/week46.pdf"),
    )
    .unwrap();
    let original = dorm_shared::Store {
        menus: vec![menu],
        entries,
        ..Default::default()
    };
    let dir = std::env::temp_dir().join(format!("dorm-failure-test-{}", std::process::id()));
    dorm_api::storage::save(&dir, &original).unwrap();
    // A closed local proxy forces a transport failure without touching the municipality.
    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::all("http://127.0.0.1:9").unwrap())
        .timeout(std::time::Duration::from_secs(1))
        .build()
        .unwrap();
    assert!(dorm_api::update::refresh(&client, &dir, &original)
        .await
        .is_err());
    let after = dorm_api::storage::load(&dir).unwrap();
    assert_eq!(
        serde_json::to_value(after).unwrap(),
        serde_json::to_value(original).unwrap()
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn future_publication_does_not_replace_current() {
    let entries = scraper::discover(
        include_str!("../../fixtures/source/menu-page.html"),
        today(),
    )
    .unwrap();
    let current = parser::parse_bbox(
        include_str!("../../fixtures/source/week46.bbox.html"),
        &entries[1],
        include_bytes!("../../fixtures/source/week46.pdf"),
    )
    .unwrap();
    let mut future = current.clone();
    future.menu_id = "synthetic-future".into();
    future.entry.valid_from = Some(today() + chrono::Duration::days(5));
    future.entry.valid_until = Some(today() + chrono::Duration::days(9));
    let store = dorm_shared::Store {
        menus: vec![current.clone(), future],
        ..Default::default()
    };
    let r = store.response(today());
    assert_eq!(r.current_menu.unwrap().menu.menu_id, current.menu_id);
    assert_eq!(r.next_menu.unwrap().menu.menu_id, "synthetic-future");
    let weekend = store.response(today() + chrono::Duration::days(3));
    assert_eq!(weekend.current_menu.unwrap().status, "expired");
    assert!(weekend.next_menu.is_some());
    let monday = store.response(today() + chrono::Duration::days(5));
    assert_eq!(
        monday.current_menu.unwrap().menu.menu_id,
        "synthetic-future"
    );
    assert!(monday.next_menu.is_none());
}

#[test]
fn filename_iso_year_can_differ_from_calendar_year_synthetic() {
    let html="<p><strong>RADNÓTI MIKLÓS KOLLÉGIUMI ÉS III. BÉLA GIMNÁZIUMI ÉTLAPOK:</strong><br><a href='/synthetic/Kollegium202601het.pdf'>1. heti menü</a> (12.29. - 01.02.)<br><strong>OTHER</strong></p>";
    let entries = scraper::discover(html, NaiveDate::from_ymd_opt(2025, 12, 30).unwrap()).unwrap();
    assert_eq!(
        entries[0].valid_from,
        Some(NaiveDate::from_ymd_opt(2025, 12, 29).unwrap())
    );
    assert_eq!(
        entries[0].valid_until,
        Some(NaiveDate::from_ymd_opt(2026, 1, 2).unwrap())
    );
}
