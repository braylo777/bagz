//! Property tests: invariants that must hold for any input.

use std::collections::BTreeSet;

use bagz_engine::agents::receipt::AppStatus;
use bagz_engine::approval::{Pending, Queue, Status};
use bagz_engine::dedupe::dedupe;
use bagz_engine::docx::{self, Para, Style};
use bagz_engine::ingest::{self, salary_from_text, Board};
use bagz_engine::model::{Posting, Source, Workplace};
use bagz_engine::rank::rank;
use bagz_engine::text::*;
use bagz_engine::usage::{UsageState, DAY_WINDOW};
use bagz_engine::Profile;
use proptest::prelude::*;

fn source() -> impl Strategy<Value = Source> {
    prop::sample::select(Source::ALL.to_vec())
}

fn workplace() -> impl Strategy<Value = Workplace> {
    prop::sample::select(vec![
        Workplace::Remote,
        Workplace::Hybrid,
        Workplace::Onsite,
        Workplace::Unknown,
    ])
}

prop_compose! {
    fn posting()(
        n in 0u32..1_000_000,
        source in source(),
        company in prop::sample::select(vec!["Acme", "Acme Inc.", "Globex", "Initech LLC", "Hooli"]),
        title in prop::sample::select(vec!["Senior Backend Engineer", "Sr. Backend Eng", "Data Engineer", "Designer", "Account Executive"]),
        location in prop::sample::select(vec!["Austin, TX", "Remote", "Berlin", "New York, NY • Remote"]),
        workplace in workplace(),
        desc in "[a-zA-Z ,.]{0,80}",
        posted in prop::option::of((2025i64..2027, 1u32..13, 1u32..29)),
    ) -> Posting {
        Posting {
            id: format!("{source}:b:{n}"),
            source,
            board: "b".into(),
            company: company.into(),
            title: title.into(),
            location: location.into(),
            workplace,
            department: None,
            employment_type: None,
            url: format!("https://jobs.test/{source}/{n}"),
            apply_url: None,
            posted: posted.map(|(y, m, d)| format!("{y:04}-{m:02}-{d:02}")),
            salary: None,
            description: desc,
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn decode_entities_never_panics(s in "\\PC{0,200}") {
        let _ = decode_entities(&s);
    }

    #[test]
    fn decode_is_identity_without_ampersand(s in "[^&]{0,100}") {
        prop_assert_eq!(decode_entities(&s), s);
    }

    #[test]
    fn html_to_text_has_no_tags_or_blank_lines(s in "[a-z <>/p&;]{0,120}") {
        let t = html_to_text(&format!("<p>{}</p>", s.replace(['<', '>'], "")));
        prop_assert!(!t.contains("<p>"));
        prop_assert!(t.lines().all(|l| !l.trim().is_empty() && l == l.trim()));
    }

    #[test]
    fn truncate_is_a_prefix_within_limit(s in "\\PC{0,100}", max in 0usize..120) {
        let t = truncate_chars(&s, max);
        prop_assert!(t.chars().count() <= max);
        prop_assert!(s.starts_with(&t));
    }

    #[test]
    fn collapse_spaces_is_idempotent(s in "\\PC{0,100}") {
        let once = collapse_spaces(&s);
        prop_assert_eq!(collapse_spaces(&once), once.clone());
        prop_assert!(!once.contains("  "));
    }

    #[test]
    fn keys_are_idempotent(s in "[A-Za-z .,&-]{0,60}") {
        prop_assert_eq!(title_key(&title_key(&s)), title_key(&s));
        prop_assert_eq!(company_key(&company_key(&s)), company_key(&s));
    }

    #[test]
    fn civil_date_round_trip(days in -100_000i64..200_000) {
        let (y, m, d) = civil_from_days(days);
        prop_assert_eq!(days_from_civil(y, m, d), days);
        prop_assert_eq!(days_from_iso(&format!("{y:04}-{m:02}-{d:02}")), Some(days));
    }

    #[test]
    fn epoch_ms_date_is_iso(ms in 0i64..4_000_000_000_000) {
        let s = date_from_epoch_ms(ms);
        prop_assert_eq!(iso_date(&s), Some(s.clone()));
    }

    #[test]
    fn rfc822_parser_never_panics(s in "\\PC{0,60}") {
        let _ = rfc822_date(&s);
    }

    #[test]
    fn salary_parser_is_sane(s in "[$£€0-9,.kK to–-]{0,40}") {
        if let Some(sal) = salary_from_text(&s) {
            prop_assert!(sal.min <= sal.max);
            prop_assert!(sal.min >= 10_000);
        }
    }

    #[test]
    fn salary_reads_any_written_range(a in 10u64..400, extra in 0u64..200) {
        let b = a + extra;
        let s = salary_from_text(&format!("Pay is ${a},000 - ${b},000 per year")).unwrap();
        prop_assert_eq!((s.min, s.max), (a * 1000, b * 1000));
    }

    #[test]
    fn parsers_never_panic_on_garbage(s in "\\PC{0,300}") {
        for src in Source::ALL {
            let _ = Board::new(src, "x").parse(&s);
        }
    }

    #[test]
    fn parsers_never_panic_on_json_shapes(n in 0usize..5, title in "\\PC{0,20}") {
        let job = serde_json::json!({"id": n, "title": title, "text": title, "shortcode": "S", "location": {"name": title}});
        let gh = serde_json::json!({"jobs": vec![job.clone(); n]}).to_string();
        let lv = serde_json::json!(vec![job.clone(); n]).to_string();
        let _ = ingest::greenhouse::parse(&gh, "x");
        let _ = ingest::ashby::parse(&gh, "x");
        let _ = ingest::workable::parse(&gh, "x");
        let _ = ingest::lever::parse(&lv, "x");
    }

    #[test]
    fn board_spec_round_trips(src in source(), token in "[a-z0-9-]{1,20}") {
        let b = Board::new(src, token);
        prop_assert_eq!(b.to_string().parse::<Board>().unwrap(), b);
    }

    #[test]
    fn dedupe_is_idempotent_and_shrinks(ps in prop::collection::vec(posting(), 0..40)) {
        let once = dedupe(ps.clone());
        prop_assert!(once.len() <= ps.len());
        prop_assert_eq!(dedupe(once.clone()), once);
    }

    #[test]
    fn dedupe_keeps_order_independent_winners(ps in prop::collection::vec(posting(), 0..30)) {
        let mut ids = BTreeSet::new();
        let mut uniq = Vec::new();
        for p in ps { if ids.insert(p.id.clone()) { uniq.push(p); } }
        let mut rev = uniq.clone();
        rev.reverse();
        let a: BTreeSet<_> = dedupe(uniq).into_iter().map(|p| p.id).collect();
        let b: BTreeSet<_> = dedupe(rev).into_iter().map(|p| p.id).collect();
        prop_assert_eq!(a, b);
    }

    #[test]
    fn dedupe_output_keys_are_unique(ps in prop::collection::vec(posting(), 0..40)) {
        let out = dedupe(ps);
        let keys: BTreeSet<_> = out.iter().map(bagz_engine::dedupe::job_key).collect();
        prop_assert_eq!(keys.len(), out.len());
    }

    #[test]
    fn rank_is_sorted_bounded_and_complete(ps in prop::collection::vec(posting(), 0..30), today in 20_000i64..21_000) {
        let ranked = rank(&ps, &Profile::example(), today);
        prop_assert_eq!(ranked.len(), ps.len());
        for w in ranked.windows(2) { prop_assert!(w[0].1.total >= w[1].1.total); }
        for (_, s) in &ranked { prop_assert!((0.0..=100.0).contains(&s.total)); }
    }

    #[test]
    fn docx_text_round_trips(lines in prop::collection::vec("[ -~À-ÿ]{0,60}", 0..20)) {
        let paras: Vec<Para> = lines.iter().map(|l| Para::new(Style::Normal, l.clone())).collect();
        let bytes = docx::build(&paras, "t", "a");
        prop_assert_eq!(docx::extract_text(&bytes).unwrap(), lines);
        prop_assert!(docx::ats_violations(&bytes).unwrap().is_empty());
    }

    #[test]
    fn zip_round_trips(files in prop::collection::vec(("[a-z]{1,8}", prop::collection::vec(any::<u8>(), 0..200)), 1..8)) {
        let mut z = docx::ZipWriter::default();
        for (n, d) in &files { z.add(n, d); }
        let back = docx::read_entries(&z.finish()).unwrap();
        prop_assert_eq!(back, files);
    }

    #[test]
    fn crc32_detects_single_byte_change(data in prop::collection::vec(any::<u8>(), 1..100), i in any::<prop::sample::Index>(), x in 1u8..=255) {
        let mut other = data.clone();
        let k = i.index(data.len());
        other[k] ^= x;
        prop_assert_ne!(docx::crc32(&data), docx::crc32(&other));
    }

    #[test]
    fn queue_decides_each_proposal_once(ops in prop::collection::vec((0u64..6, any::<bool>()), 0..30)) {
        let mut q = Queue::new();
        for i in 0..5 { q.push(Pending::new("x", "", i)); }
        let mut decided = BTreeSet::new();
        for (id, approve) in ops {
            let r = if approve { q.approve(id, "Avery").map(|_| ()) } else { q.reject(id, "Avery", "") };
            let first = (1..=5).contains(&id) && decided.insert(id);
            prop_assert_eq!(r.is_ok(), first);
        }
        prop_assert_eq!(q.log().len(), decided.len());
        prop_assert_eq!(q.pending_ids().len(), 5 - decided.len());
        for id in decided { prop_assert_ne!(q.status(id), Some(Status::Pending)); }
    }

    #[test]
    fn final_statuses_never_move(to in prop::sample::select(vec![
        AppStatus::Submitted, AppStatus::Received, AppStatus::Interview,
        AppStatus::Offer, AppStatus::Rejected, AppStatus::Withdrawn])) {
        for from in [AppStatus::Offer, AppStatus::Rejected, AppStatus::Withdrawn] {
            prop_assert!(!from.can_move_to(to));
        }
        prop_assert!(!to.can_move_to(AppStatus::Submitted));
    }

    #[test]
    fn usage_never_records_while_off(days in prop::collection::vec("20[0-9]{2}-[01][0-9]-[0-3][0-9]", 0..20)) {
        let mut s = UsageState::default();
        for d in &days { s.record(d); }
        prop_assert_eq!(s, UsageState::default());
    }

    #[test]
    fn usage_months_match_days_when_on(offsets in prop::collection::vec(0i64..40, 1..80)) {
        let mut s = UsageState::default();
        s.enable(7);
        for o in &offsets {
            let (y, m, d) = civil_from_days(20_400 + o);
            s.record(&format!("{y:04}-{m:02}-{d:02}"));
        }
        prop_assert!(s.days.len() <= DAY_WINDOW);
        for d in &s.days { prop_assert!(s.months.contains(&d[..7])); }
        prop_assert_eq!(s.runs as usize, offsets.len());
    }

    #[test]
    fn workplace_labels_never_panic(s in "\\PC{0,40}") {
        let _ = Workplace::from_label(&s);
    }
}
