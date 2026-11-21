//! Offline walkthrough of the whole pipeline:
//!
//! ```text
//! cargo run --example demo
//! ```
//!
//! It reads the recorded fixtures (no network), ranks the jobs for the
//! fictional example profile, runs all six agents on the deterministic mock
//! model, and shows each proposal waiting for approval. The demo plays the
//! human with the name `demo-reviewer` so it can finish; a real run waits
//! for a person. Nothing is submitted or sent. Files go to `target/demo/`.

use std::path::PathBuf;

use bagz_engine::agents::receipt::Tracker;
use bagz_engine::agents::{cover_letter, follow_up, prefill, scorer, tailor, REGISTRY};
use bagz_engine::docx;
use bagz_engine::http::FixtureClient;
use bagz_engine::ingest::{parse_board_list, run};
use bagz_engine::llm::MockLlm;
use bagz_engine::rank::rank;
use bagz_engine::text::days_from_civil;
use bagz_engine::Profile;

const REVIEWER: &str = "demo-reviewer";

fn main() -> bagz_engine::Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = root.join("tests/fixtures/http");
    let out = root.ancestors().nth(2).unwrap_or(&root).join("target/demo");
    std::fs::create_dir_all(&out)?;
    let today = days_from_civil(2026, 11, 20);
    let llm = MockLlm::new();
    let profile = Profile::example();

    println!("== 1. Ingest (recorded HTTP, no network)");
    let boards = parse_board_list(&std::fs::read_to_string(fixtures.join("boards.txt"))?)?;
    let (postings, report) = run(&FixtureClient::from_dir(&fixtures)?, &boards);
    println!(
        "   {} boards, {} fetched, {} after dedupe, {} failed (on purpose: not recorded)",
        report.boards.len(),
        report.fetched,
        report.indexed,
        report.boards.iter().filter(|b| b.error.is_some()).count()
    );

    println!("== 2. Rank for {}", profile.full_name());
    let ranked = rank(&postings, &profile, today);
    for (p, s) in ranked.iter().take(5) {
        println!(
            "   {:5.1}  {} at {} ({})",
            s.total, p.title, p.company, p.source
        );
    }
    let best = ranked[0].0;

    println!(
        "== 3. Agents ({} registered). Each output waits for a human.",
        REGISTRY.len()
    );
    let card = scorer::run(&llm, best, &profile, today)?;
    println!("   [{}] {}", card.agent, card.summary);
    let card = card.approve(REVIEWER)?;

    let resume = tailor::run(&llm, best, &profile)?;
    println!("   [{}] {}", resume.agent, resume.summary);
    let resume = resume.approve(REVIEWER)?;

    let letter = cover_letter::run(&llm, best, &profile)?;
    println!("   [{}] {}", letter.agent, letter.summary);
    let letter = letter.approve(REVIEWER)?;

    let form = prefill::run(best, &profile);
    println!("   [{}] {}", form.agent, form.summary);
    for f in form.payload.needs_human() {
        println!("        you answer: {}", f.label);
    }
    let form = form.approve(REVIEWER)?;

    println!("== 4. Export ATS-safe .docx (approved values only)");
    let rp = out.join("resume.docx");
    let lp = out.join("cover-letter.docx");
    docx::export_resume(&resume, &rp)?;
    docx::export_letter(&letter, &profile, &lp)?;
    let issues = docx::ats_violations(&std::fs::read(&rp)?)?;
    println!("   {} ({} ATS issues)", rp.display(), issues.len());
    println!("   {}", lp.display());

    println!(
        "== 5. You submit the form yourself at {}",
        form.payload().apply_url
    );
    let mut tracker = Tracker::default();
    tracker.record_submission(&form, &best.company, &best.title, "2026-11-10")?;

    let mail_subject = format!("Thank you for applying to {}", best.company);
    let from = match best.source {
        bagz_engine::Source::Lever => "no-reply@hire.lever.co",
        bagz_engine::Source::Ashby => "no-reply@ashbyhq.com",
        bagz_engine::Source::Workable => "noreply@candidates.workablemail.com",
        _ => "no-reply@us.greenhouse-mail.io",
    };
    if let Some(update) = tracker.propose(
        from,
        &mail_subject,
        "We received your application.",
        "2026-11-11",
    ) {
        println!("   [receipt_tracker] {}", update.summary);
        tracker.apply(update.approve(REVIEWER)?)?;
    }

    println!("== 6. Follow-ups due (7 days, no news)");
    for app in follow_up::due(&tracker, today, follow_up::DEFAULT_WAIT_DAYS) {
        let f = follow_up::run(&llm, app, &profile)?;
        println!("   [{}] {}", f.agent, f.summary);
        println!("        Subject: {}", f.payload.subject);
    }

    println!(
        "== Done. Score card verdict: {:?}. Model calls made: {}.",
        card.payload().verdict,
        llm.calls().len()
    );
    Ok(())
}
