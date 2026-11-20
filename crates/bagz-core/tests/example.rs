//! The embedded catalog is the demo's `jobs.json`, and the deck follows it.

use std::fs;
use std::path::PathBuf;

use bagz_core::{
    AGENT_QUEUE_LIMIT, BAG_XP, Catalog, Decision, Error, JobFilter, Rank, Rarity, SUPER_XP,
    Session, Sticker, Swipe, load_example_jobs, rank_for, rarity_of, score_of, sticker_for,
    utc_today,
};
use chrono::NaiveDate;
use serde_json::json;

const IDS: [&str; 12] = [
    "froth", "rack", "ping", "poster", "spoke", "latebird", "clip", "dayglow", "salsa", "lace",
    "quad", "carton",
];

fn example() -> Catalog {
    load_example_jobs().expect("example jobs")
}

#[test]
fn embedded_json_is_the_repo_file() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src/data/jobs.json");
    let disk = fs::read_to_string(&path).expect("read jobs.json");
    assert_eq!(disk, bagz_core::example_jobs_json());
    let catalog = example();
    let ids: Vec<_> = catalog.jobs().iter().map(|job| job.id.as_str()).collect();
    assert_eq!(ids, IDS);
}

#[test]
fn example_jobs_round_trip_and_score() {
    let catalog = example();
    let json = serde_json::to_string(catalog.jobs()).unwrap();
    let again = Catalog::parse(&json).unwrap();
    assert_eq!(catalog.jobs(), again.jobs());

    let froth = catalog.get("froth").unwrap();
    let carton = catalog.get("carton").unwrap();
    let quad = catalog.get("quad").unwrap();
    assert_eq!(score_of(froth), 70);
    assert_eq!(score_of(carton), 82);
    assert_eq!(score_of(quad), 82);
    assert!(froth.sends.contains('–'));

    let mut legendary = 0;
    let mut rare = 0;
    let mut common = 0;
    for job in catalog.jobs() {
        match rarity_of(job.match_pct) {
            Rarity::Legendary => legendary += 1,
            Rarity::Rare => rare += 1,
            Rarity::Common => common += 1,
        }
        assert!(score_of(job) <= 100);
    }
    assert_eq!((legendary, rare, common), (3, 6, 3));
}

#[test]
fn stickers_follow_card_priority() {
    let catalog = example();
    let expected = [
        ("froth", Some(Sticker::Hot)),
        ("rack", Some(Sticker::New)),
        ("ping", None),
        ("poster", Some(Sticker::Hot)),
        ("spoke", Some(Sticker::New)),
        ("latebird", None),
        ("clip", Some(Sticker::Remote)),
        ("dayglow", None),
        ("salsa", Some(Sticker::New)),
        ("lace", None),
        ("quad", Some(Sticker::Hot)),
        ("carton", Some(Sticker::Pay)),
    ];
    for (id, sticker) in expected {
        assert_eq!(sticker_for(catalog.get(id).unwrap()), sticker, "{id}");
    }
}

#[test]
fn filters_keep_catalog_order() {
    let catalog = example();
    let remote = JobFilter::try_new(Some("remote".into()), None, None, None, None).unwrap();
    assert_eq!(ids_of(&catalog.filter(&remote)), vec!["clip"]);

    let east = JobFilter::try_new(None, Some("  East Austin ".into()), None, None, None).unwrap();
    assert_eq!(ids_of(&catalog.filter(&east)), vec!["froth", "poster"]);

    let nights = JobFilter::try_new(None, None, Some("Nights".into()), None, None).unwrap();
    assert_eq!(ids_of(&catalog.filter(&nights)), vec!["latebird", "salsa"]);

    let rush = JobFilter::try_new(None, None, None, Some("rush".into()), None).unwrap();
    assert_eq!(
        ids_of(&catalog.filter(&rush)),
        vec!["froth", "spoke", "salsa"]
    );

    let hot = JobFilter::try_new(None, None, None, None, Some(90)).unwrap();
    assert_eq!(
        ids_of(&catalog.filter(&hot)),
        vec!["froth", "poster", "quad"]
    );
    assert!(JobFilter::try_new(None, None, None, None, Some(101)).is_err());
    assert!(matches!(
        JobFilter::try_new(None, None, None, None, Some(-1)),
        Err(Error::MatchOutOfRange(-1))
    ));
}

#[test]
fn swipe_queue_matches_the_demo_store() {
    let mut session = Session::new(example());
    let day = "2026-11-20";
    let next = "2026-11-21";

    assert_eq!(session.top().unwrap().id, "froth");
    assert_eq!(ids_of(&session.watchers()), vec!["lace", "quad", "carton"]);

    let bag = session.swipe("froth", Swipe::Bag, day).unwrap();
    assert_eq!(bag.xp_gain, BAG_XP);
    assert_eq!(bag.streak, 1);
    assert_eq!(bag.rank, Rank::Rookie);
    session.swipe("froth", Swipe::Bag, day).unwrap();
    assert_eq!(session.history().len(), 1);
    assert_eq!(session.xp(), 20);
    assert_eq!(session.streak(), 1);
    assert_eq!(session.combo(), 2);

    session.swipe("quad", Swipe::Super, next).unwrap();
    assert_eq!(session.xp(), 20 + SUPER_XP);
    assert_eq!(session.streak(), 2);
    assert_eq!(session.rank(), Rank::Closer);
    assert_eq!(ids_of(&session.applied()), vec!["quad", "froth"]);
    assert_eq!(session.chased(), 9_485 + 27_846);

    session.swipe("carton", Swipe::Pass, next).unwrap();
    assert_eq!(session.xp(), 45);
    assert_eq!(session.combo(), 0);
    assert_eq!(session.streak(), 2);
    assert_eq!(ids_of(&session.passed()), vec!["carton"]);
    assert_eq!(
        ids_of(&session.agent_queue(AGENT_QUEUE_LIMIT)),
        vec!["quad", "froth", "rack", "ping"]
    );
    assert!(session.agent_queue(0).is_empty());
    assert!(!ids_of(&session.remaining()).contains(&"froth".to_string()));

    let undone = session.undo().unwrap();
    assert_eq!(undone.id, "carton");
    assert_eq!(session.xp(), 45);
    assert!(session.passed().is_empty());

    session.reset_swipes();
    assert!(session.applied().is_empty());
    assert_eq!(session.remaining().len(), 12);
    assert_eq!(session.xp(), 45);
    assert_eq!(session.combo(), 0);

    let err = session.swipe("nope", Swipe::Bag, day).unwrap_err();
    assert!(matches!(err, Error::UnknownJob(_)));
    assert!(session.history().is_empty());
    assert!(session.swipe("froth", Swipe::Pass, "tomorrow").is_err());
    assert!(session.history().is_empty());
}

#[test]
fn restore_keeps_saved_progress_and_known_bags() {
    let raw = json!({
        "plan": "agent",
        "xp": 15,
        "streak": 3,
        "lastDay": "2026-11-20",
        "history": [
            {"id": "gone", "decision": "bag"},
            {"id": "salsa", "decision": "bag"},
            {"id": "rack", "decision": "pass"}
        ]
    });
    let mut session = Session::restore(example(), &raw);
    assert_eq!(ids_of(&session.applied()), vec!["salsa"]);
    assert_eq!(session.xp(), 15);
    assert_eq!(session.streak(), 3);
    session.swipe("ping", Swipe::Bag, "2026-11-20").unwrap();
    assert_eq!(session.streak(), 3);
    assert_eq!(session.xp(), 25);
    session.swipe("clip", Swipe::Bag, "2026-11-22").unwrap();
    assert_eq!(session.streak(), 4);
    assert_eq!(rank_for(120), Rank::BagSecured);
    assert_eq!(rank_for(119), Rank::Closer);
    assert_eq!(rank_for(39), Rank::Rookie);
    assert_eq!(session.history()[0].decision, Decision::Bag);
}

#[test]
fn rejects_bad_catalogs() {
    let err = Catalog::load("/no/such/bagz-jobs.json").unwrap_err();
    assert!(matches!(err, Error::Read { .. }));
    assert!(Catalog::parse("{\"id\":\"froth\"}").is_err());

    let mut jobs: Vec<serde_json::Value> =
        serde_json::from_str(bagz_core::example_jobs_json()).unwrap();
    jobs.push(jobs[0].clone());
    let err = Catalog::parse(&serde_json::to_string(&jobs).unwrap()).unwrap_err();
    assert!(matches!(err, Error::DuplicateId(id) if id == "froth"));

    jobs.pop();
    jobs[0]["matchPct"] = json!(101);
    assert!(Catalog::parse(&serde_json::to_string(&jobs).unwrap()).is_err());
    jobs[0]["matchPct"] = json!(91);
    jobs[0]["secret"] = json!(true);
    assert!(Catalog::parse(&serde_json::to_string(&jobs).unwrap()).is_err());
    jobs[0].as_object_mut().unwrap().remove("secret");
    jobs[0]["title"] = json!("  ");
    assert!(Catalog::parse(&serde_json::to_string(&jobs).unwrap()).is_err());
}

#[test]
fn utc_today_is_a_calendar_day() {
    let day = utc_today();
    assert!(NaiveDate::parse_from_str(&day, "%Y-%m-%d").is_ok(), "{day}");
}

fn ids_of(jobs: &[&bagz_core::Job]) -> Vec<String> {
    jobs.iter().map(|job| job.id.clone()).collect()
}
