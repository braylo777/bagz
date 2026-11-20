use std::path::PathBuf;

use bagz_core::{
    apply_phase, utc_date, ApplyPhase, Catalog, Decision, Deck, JobFilter, Plan, Rarity, Swipe,
    CHAT_QUEUE_LIMIT,
};

fn catalog() -> Catalog {
    Catalog::bundled().expect("example jobs.json should validate")
}

fn ids<'a>(jobs: &'a [&'a bagz_core::Job]) -> Vec<&'a str> {
    jobs.iter().map(|job| job.id.as_str()).collect()
}

fn sample_job(id: &str) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "title": "Barista",
        "company": "Froth Club",
        "pay": "$17.85/hr",
        "payScore": 62,
        "growth": 40,
        "vibe": 88,
        "chase": 100,
        "area": "East Austin",
        "mode": "Onsite",
        "schedule": "Mornings",
        "posted": "2d ago",
        "applicants": "14 applied",
        "size": "18 people",
        "stage": "One shop",
        "matchPct": 91,
        "reasons": ["Near"],
        "skills": ["Espresso"],
        "perks": ["Tips"],
        "reply": "Replies in 1 day",
        "sends": "Mornings"
    })
}

#[test]
fn bundled_catalog_is_the_twelve_example_jobs() {
    let catalog = catalog();
    assert_eq!(catalog.len(), 12);
    assert!(!catalog.is_empty());
    assert_eq!(
        ids(&catalog.jobs().iter().collect::<Vec<_>>()),
        vec![
            "froth", "rack", "ping", "poster", "spoke", "latebird", "clip", "dayglow", "salsa",
            "lace", "quad", "carton",
        ]
    );

    let froth = catalog.get("froth").unwrap();
    assert_eq!(froth.title, "Barista");
    assert_eq!(froth.company, "Froth Club");
    assert_eq!(froth.pay, "$17.85/hr");
    assert_eq!(froth.chase, 27_846);
    assert_eq!(froth.skills, ["Espresso", "Register", "Rush"]);

    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src/data/jobs.json");
    let loaded = Catalog::load_path(&path).unwrap();
    assert_eq!(
        ids(&loaded.jobs().iter().collect::<Vec<_>>()),
        ids(&catalog.jobs().iter().collect::<Vec<_>>())
    );
}

#[test]
fn scores_rarities_and_stickers_follow_the_card() {
    let catalog = catalog();
    let froth = catalog.get("froth").unwrap();
    let quad = catalog.get("quad").unwrap();
    let carton = catalog.get("carton").unwrap();
    let clip = catalog.get("clip").unwrap();
    let spoke = catalog.get("spoke").unwrap();
    let dayglow = catalog.get("dayglow").unwrap();

    assert_eq!(froth.score(), 70);
    assert_eq!(quad.score(), 82);
    assert_eq!(carton.score(), 82);

    assert_eq!(quad.rarity(), Rarity::Legendary);
    assert_eq!(quad.sticker(), "HOT");
    assert_eq!(clip.rarity(), Rarity::Rare);
    assert_eq!(clip.sticker(), "REMOTE");
    assert_eq!(spoke.sticker(), "NEW");
    assert_eq!(carton.sticker(), "$$$");
    assert_eq!(dayglow.rarity(), Rarity::Common);
    assert_eq!(dayglow.sticker(), "");

    for job in catalog.jobs() {
        assert!(job.score() <= 100);
        match job.match_pct {
            90..=100 => {
                assert_eq!(job.rarity(), Rarity::Legendary);
                assert_eq!(job.sticker(), "HOT");
            }
            85..=89 => assert_eq!(job.rarity(), Rarity::Rare),
            _ => assert_eq!(job.rarity(), Rarity::Common),
        }
    }
}

#[test]
fn filters_and_skill_rank_use_the_example_catalog() {
    let catalog = catalog();

    let remote = catalog.filter(&JobFilter {
        mode: Some("remote".to_string()),
        ..JobFilter::default()
    });
    assert_eq!(ids(&remote), vec!["clip"]);

    let east = catalog.filter(&JobFilter {
        area: Some("EAST".to_string()),
        ..JobFilter::default()
    });
    assert_eq!(ids(&east), vec!["froth", "poster", "salsa"]);

    let react = catalog.filter(&JobFilter {
        skill: Some("react".to_string()),
        ..JobFilter::default()
    });
    assert_eq!(ids(&react), vec!["carton"]);

    let hot = catalog.filter(&JobFilter {
        min_match: Some(90),
        ..JobFilter::default()
    });
    assert_eq!(ids(&hot), vec!["froth", "poster", "quad"]);

    let espresso_east = catalog.filter(&JobFilter {
        mode: Some("Onsite".to_string()),
        area: Some("east".to_string()),
        skill: Some("espresso".to_string()),
        min_match: Some(90),
    });
    assert_eq!(ids(&espresso_east), vec!["froth"]);

    let blank = catalog.filter(&JobFilter {
        mode: Some("  ".to_string()),
        area: Some(String::new()),
        skill: None,
        min_match: None,
    });
    assert_eq!(blank.len(), 12);

    let ranked = catalog.rank_for_skills(&["React", "Espresso", "Food handler"]);
    assert_eq!(ranked[0].job.id, "froth");
    assert_eq!(ranked[0].shared, vec!["Espresso".to_string()]);
    assert_eq!(ranked[0].overlap_pct, 33);
    assert_eq!(ranked[1].job.id, "carton");
    assert_eq!(ranked[1].shared, vec!["React".to_string()]);
    assert_eq!(ranked[1].overlap_pct, 33);
    assert!(ranked[2].overlap_pct == 0);
    assert!(ranked[2].job.match_pct >= ranked.last().unwrap().job.match_pct);
}

#[test]
fn swipe_apply_pass_and_super_match_the_store() {
    let mut deck = Deck::new(catalog());
    let day = "2026-11-20";

    deck.swipe("froth", Swipe::Bag, day).unwrap();
    deck.swipe("rack", Swipe::Pass, day).unwrap();
    deck.swipe("carton", Swipe::Super, day).unwrap();

    assert_eq!(ids(&deck.applied()), vec!["carton", "froth"]);
    assert_eq!(ids(&deck.passed()), vec!["rack"]);
    assert!(!ids(&deck.remaining()).contains(&"froth"));
    assert!(!ids(&deck.remaining()).contains(&"rack"));
    assert!(!ids(&deck.remaining()).contains(&"carton"));
    assert_eq!(deck.xp(), 35);
    assert_eq!(deck.streak(), 1);
    assert_eq!(deck.last_day(), day);
    assert_eq!(deck.combo(), 1);
    assert_eq!(deck.history().last().unwrap().decision, Decision::Bag);
    assert_eq!(deck.chased(), 67_850 + 27_846);
    assert_eq!(deck.rank(), "Rookie");

    let queue = deck.queue(CHAT_QUEUE_LIMIT);
    assert_eq!(ids(&queue), vec!["carton", "froth", "rack", "ping"]);
    assert_eq!(queue.len(), 4);
}

#[test]
fn streak_combo_undo_and_reset_follow_the_demo() {
    let mut deck = Deck::new(catalog());

    deck.swipe("froth", Swipe::Bag, "2026-11-20").unwrap();
    assert_eq!(deck.streak(), 1);
    assert_eq!(deck.combo(), 1);

    deck.swipe("rack", Swipe::Pass, "2026-11-20").unwrap();
    assert_eq!(deck.streak(), 1);
    assert_eq!(deck.xp(), 10);
    assert_eq!(deck.combo(), 0);

    deck.swipe("ping", Swipe::Bag, "2026-11-20").unwrap();
    assert_eq!(deck.streak(), 1);
    assert_eq!(deck.xp(), 20);
    assert_eq!(deck.combo(), 1);

    deck.swipe("poster", Swipe::Bag, "2026-11-21").unwrap();
    assert_eq!(deck.streak(), 2);
    assert_eq!(deck.xp(), 30);
    assert_eq!(deck.last_day(), "2026-11-21");

    deck.undo();
    assert_eq!(ids(&deck.applied()), vec!["ping", "froth"]);
    assert_eq!(deck.xp(), 30);
    assert_eq!(deck.streak(), 2);
    assert_eq!(deck.combo(), 2);

    deck.swipe("froth", Swipe::Pass, "2026-11-21").unwrap();
    assert_eq!(ids(&deck.applied()), vec!["ping"]);
    assert_eq!(ids(&deck.passed()), vec!["rack", "froth"]);
    assert_eq!(deck.xp(), 30);

    deck.reset();
    assert!(deck.applied().is_empty());
    assert!(deck.passed().is_empty());
    assert_eq!(deck.remaining().len(), 12);
    assert_eq!(deck.xp(), 30);
    assert_eq!(deck.streak(), 2);
}

#[test]
fn unknown_swipe_does_not_change_the_deck() {
    let mut deck = Deck::new(catalog());
    let error = deck.swipe("nope", Swipe::Bag, "2026-11-20").unwrap_err();
    assert!(error.to_string().contains("unknown job id `nope`"));
    assert!(deck.history().is_empty());
    assert_eq!(deck.xp(), 0);
    assert!(Swipe::parse("like").is_err());
    assert!(matches!(Swipe::parse("super"), Ok(Swipe::Super)));
}

#[test]
fn watchers_are_the_last_three_remaining_jobs() {
    let deck = Deck::new(catalog());
    assert_eq!(ids(&deck.watchers()), vec!["lace", "quad", "carton"]);

    let mut deck = Deck::new(catalog());
    deck.swipe("carton", Swipe::Bag, "2026-11-20").unwrap();
    assert_eq!(ids(&deck.watchers()), vec!["salsa", "lace", "quad"]);
}

#[test]
fn saved_state_round_trips_and_sanitizes_like_the_page_load() {
    let mut deck = Deck::new(catalog());
    deck.swipe("froth", Swipe::Bag, "2026-11-20").unwrap();
    deck.swipe("rack", Swipe::Pass, "2026-11-20").unwrap();
    deck.swipe("poster", Swipe::Super, "2026-11-20").unwrap();
    assert_eq!(deck.combo(), 1);

    let raw = deck.export_state().unwrap();
    let mut restored = Deck::new(catalog());
    restored.import_state(&raw).unwrap();
    assert_eq!(ids(&restored.applied()), vec!["poster", "froth"]);
    assert_eq!(ids(&restored.passed()), vec!["rack"]);
    assert_eq!(restored.xp(), 35);
    assert_eq!(restored.streak(), 1);
    assert_eq!(restored.last_day(), "2026-11-20");
    assert_eq!(restored.plan(), Plan::Free);
    assert_eq!(restored.combo(), 0);

    let messy = r#"{
        "plan": "agent",
        "history": [
            {"id": "froth", "decision": "bag"},
            {"id": "nope", "decision": "bag"},
            {"id": "froth", "decision": "pass"},
            {"id": "rack", "decision": "nope"},
            "bad",
            {"id": "rack", "decision": "pass"}
        ],
        "xp": 12.9,
        "streak": 3,
        "lastDay": "2026-11-20"
    }"#;
    restored.import_state(messy).unwrap();
    assert_eq!(restored.plan(), Plan::Vip);
    assert_eq!(
        restored
            .history()
            .iter()
            .map(|record| (record.id.as_str(), record.decision))
            .collect::<Vec<_>>(),
        vec![("froth", Decision::Bag), ("rack", Decision::Pass)]
    );
    assert_eq!(restored.xp(), 12);
    assert_eq!(restored.streak(), 3);
    assert_eq!(restored.combo(), 0);

    restored
        .import_state(r#"{"xp":"10","plan":"VIP"}"#)
        .unwrap();
    assert_eq!(restored.xp(), 0);
    assert_eq!(restored.plan(), Plan::Free);
    assert!(restored.history().is_empty());

    restored.import_state("null").unwrap();
    assert_eq!(restored.plan(), Plan::Free);
    assert_eq!(restored.xp(), 0);

    let error = restored.import_state("{").unwrap_err();
    assert!(error.to_string().contains("not valid"));
    assert_eq!(restored.xp(), 0);

    restored.set_plan(Plan::parse("pro").unwrap());
    assert_eq!(restored.plan(), Plan::Pro);
    assert!(Plan::parse("agent").is_err());
    restored
        .import_state(r#"{"plan":"agent","xp":120}"#)
        .unwrap();
    assert_eq!(restored.plan(), Plan::Vip);
    assert_eq!(restored.rank(), "Bag Secured");
    restored.import_state(r#"{"xp":40}"#).unwrap();
    assert_eq!(restored.rank(), "Closer");
    restored.import_state(r#"{"xp":39}"#).unwrap();
    assert_eq!(restored.rank(), "Rookie");
    restored.import_state(r#"{"xp":-5}"#).unwrap();
    assert_eq!(restored.xp(), -5);
    assert_eq!(restored.rank(), "Rookie");
}

#[test]
fn apply_phase_matches_the_chat_timeline() {
    let cases = [
        (-1, 0, ApplyPhase::Queued),
        (449, 0, ApplyPhase::Queued),
        (450, 0, ApplyPhase::Applying),
        (1099, 1, ApplyPhase::Applying),
        (1100, 0, ApplyPhase::Applied),
        (1899, 1, ApplyPhase::Applied),
        (1900, 0, ApplyPhase::Reply),
        (1900, 1, ApplyPhase::Reply),
        (1900, 2, ApplyPhase::Applied),
        (8000, 3, ApplyPhase::Applied),
    ];
    for (elapsed, index, expected) in cases {
        assert_eq!(
            apply_phase(elapsed, index),
            expected,
            "elapsed {elapsed} index {index}"
        );
    }
}

#[test]
fn utc_dates_match_known_unix_days() {
    assert_eq!(utc_date(0), "1970-01-01");
    assert_eq!(utc_date(86_400), "1970-01-02");
    assert_eq!(utc_date(1_732_060_800), "2024-11-20");
    assert_eq!(utc_date(1_700_000_000), "2023-11-14");
}

#[test]
fn validation_rejects_a_broken_catalog() {
    let mut duplicate = serde_json::json!([sample_job("froth"), sample_job("froth")]);
    let error = Catalog::from_json(&duplicate.to_string()).unwrap_err();
    assert!(
        error.to_string().contains("duplicate id `froth`"),
        "{error}"
    );

    duplicate[0]["payScore"] = serde_json::json!(101);
    duplicate[1]["id"] = serde_json::json!("rack");
    let error = Catalog::from_json(&duplicate.to_string()).unwrap_err();
    assert!(error.to_string().contains("payScore"), "{error}");

    let mut empty_skills = serde_json::json!([sample_job("froth")]);
    empty_skills[0]["skills"] = serde_json::json!([]);
    let error = Catalog::from_json(&empty_skills.to_string()).unwrap_err();
    assert!(error.to_string().contains("skills"), "{error}");

    let mut blank = serde_json::json!([sample_job("froth")]);
    blank[0]["reasons"] = serde_json::json!(["ok", " "]);
    let error = Catalog::from_json(&blank.to_string()).unwrap_err();
    assert!(error.to_string().contains("reasons[1]"), "{error}");

    let mut missing = sample_job("froth");
    missing.as_object_mut().unwrap().remove("title");
    let error = Catalog::from_json(&serde_json::json!([missing]).to_string()).unwrap_err();
    assert!(error.to_string().contains("title"), "{error}");

    let error = Catalog::from_json("[]").unwrap_err();
    assert!(error.to_string().contains("at least one job"), "{error}");

    let error = Catalog::from_json("{}").unwrap_err();
    assert!(error.to_string().contains("not valid"), "{error}");

    let error = Catalog::load_path("/no/such/bagz-jobs.json").unwrap_err();
    assert!(
        error.to_string().contains("could not read jobs file"),
        "{error}"
    );
}
