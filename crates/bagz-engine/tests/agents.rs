//! The six agents and the approval gate, all on the deterministic mock.

use bagz_engine::agents::cover_letter;
use bagz_engine::agents::follow_up;
use bagz_engine::agents::prefill;
use bagz_engine::agents::receipt::{parse_email, AppStatus, Tracker};
use bagz_engine::agents::scorer::{self, Verdict};
use bagz_engine::agents::tailor;
use bagz_engine::approval::{Pending, Queue, Status};
use bagz_engine::error::Error;
use bagz_engine::llm::{Llm, MockLlm};
use bagz_engine::model::{Posting, Source, Workplace};
use bagz_engine::text::days_from_civil;
use bagz_engine::Profile;

fn today() -> i64 {
    days_from_civil(2026, 11, 20)
}

fn backend(source: Source) -> Posting {
    Posting {
        id: format!("{source}:acme:1"),
        source,
        board: "acme".into(),
        company: "Acme Robotics".into(),
        title: "Senior Backend Engineer".into(),
        location: "Austin, TX".into(),
        workplace: Workplace::Hybrid,
        department: Some("Engineering".into()),
        employment_type: Some("Full-time".into()),
        url: "https://boards.greenhouse.io/acme/jobs/1".into(),
        apply_url: Some("https://boards.greenhouse.io/acme/jobs/1#app".into()),
        posted: Some("2026-11-18".into()),
        salary: None,
        description: "Build Rust services with Kafka and PostgreSQL on AWS. Kubernetes on-call."
            .into(),
    }
}

fn sales() -> Posting {
    Posting {
        title: "Account Executive".into(),
        description: "Close enterprise deals.".into(),
        location: "Berlin".into(),
        workplace: Workplace::Onsite,
        posted: Some("2026-08-01".into()),
        ..backend(Source::Lever)
    }
}

// ---------- approval gate ----------

#[test]
fn pending_needs_a_named_approver() {
    let p = Pending::new("x", "s", 5);
    assert!(matches!(p.clone().approve("  "), Err(Error::Guard(_))));
    let a = p.approve("Avery").unwrap();
    assert_eq!(a.approver(), "Avery");
    assert_eq!(a.agent(), "x");
    assert_eq!(*a.payload(), 5);
    assert_eq!(a.into_payload(), 5);
}

#[test]
fn pending_edit_and_map() {
    let p = Pending::new("x", "s", 2).edit(|v| *v += 1).map(|v| v * 10);
    assert_eq!(p.payload, 30);
}

#[test]
fn queue_approve_reject_and_log() {
    let mut q = Queue::new();
    let a = q.push(Pending::new("scorer", "a", 1));
    let b = q.push(Pending::new("scorer", "b", 2));
    assert_eq!(q.pending_ids(), vec![a, b]);
    assert_eq!(q.peek(a).unwrap().summary, "a");
    let ok = q.approve(a, "Avery").unwrap();
    assert_eq!(*ok.payload(), 1);
    q.reject(b, "Avery", "not a fit").unwrap();
    assert_eq!(q.status(a), Some(Status::Approved));
    assert_eq!(q.status(b), Some(Status::Rejected));
    assert!(q.pending_ids().is_empty());
    assert_eq!(q.log().len(), 2);
    assert_eq!(q.log()[1].note, "not a fit");
    assert_eq!(q.len(), 2);
    assert!(!q.is_empty());
}

#[test]
fn queue_refuses_double_decisions_and_unknown_ids() {
    let mut q = Queue::new();
    let a = q.push(Pending::new("x", "a", ()));
    q.approve(a, "Avery").unwrap();
    assert!(matches!(q.approve(a, "Avery"), Err(Error::NotPending(_))));
    assert!(matches!(
        q.reject(a, "Avery", ""),
        Err(Error::NotPending(_))
    ));
    assert!(matches!(
        q.approve(99, "Avery"),
        Err(Error::UnknownProposal(99))
    ));
}

#[test]
fn queue_blank_approver_leaves_proposal_pending() {
    let mut q = Queue::new();
    let a = q.push(Pending::new("x", "a", 7));
    assert!(q.approve(a, "").is_err());
    assert_eq!(q.status(a), Some(Status::Pending));
    assert_eq!(*q.approve(a, "Avery").unwrap().payload(), 7);
}

// ---------- scorer ----------

#[test]
fn scorer_proposes_apply_for_strong_match() {
    let card = scorer::run(
        &MockLlm::new(),
        &backend(Source::Greenhouse),
        &Profile::example(),
        today(),
    )
    .unwrap()
    .payload;
    assert_eq!(card.verdict, Verdict::Apply, "{:?}", card.score);
    assert_eq!(
        card.matched_skills,
        vec!["Rust", "PostgreSQL", "Kafka", "AWS", "Kubernetes"]
    );
    assert!(card.rationale.starts_with("Fit summary"));
}

#[test]
fn scorer_proposes_skip_for_weak_match() {
    let card = scorer::run(&MockLlm::new(), &sales(), &Profile::example(), today()).unwrap();
    assert_eq!(card.payload.verdict, Verdict::Skip);
    assert!(card.summary.starts_with("Skip"));
}

#[test]
fn scorer_score_is_not_from_the_model() {
    let a = scorer::run(
        &MockLlm::scripted(["anything"]),
        &backend(Source::Lever),
        &Profile::example(),
        today(),
    )
    .unwrap();
    let b = scorer::run(
        &MockLlm::scripted(["else"]),
        &backend(Source::Lever),
        &Profile::example(),
        today(),
    )
    .unwrap();
    assert_eq!(a.payload.score, b.payload.score);
    assert_eq!(a.payload.rationale, "anything");
}

#[test]
fn scorer_prompt_carries_facts() {
    let llm = MockLlm::new();
    scorer::run(&llm, &backend(Source::Ashby), &Profile::example(), today()).unwrap();
    let call = &llm.calls()[0];
    assert_eq!(call.task, "rationale");
    assert!(call.user.contains("Company: Acme Robotics"));
}

// ---------- résumé tailor ----------

#[test]
fn tailor_moves_matching_skills_first_and_keeps_all() {
    let p = Profile::example();
    let r = tailor::run(&MockLlm::new(), &sales(), &p).unwrap().payload;
    let mut a = r.skills.clone();
    let mut b = p.skills.clone();
    a.sort();
    b.sort();
    assert_eq!(a, b, "no skill added or lost");
}

#[test]
fn tailor_orders_bullets_by_relevance() {
    let mut post = backend(Source::Greenhouse);
    post.description = "Kubernetes on-call rotation for services".into();
    let r = tailor::reorder(&Profile::example(), &post);
    assert!(r.experience[0].bullets[0].contains("Kubernetes"));
    assert_eq!(r.target.as_deref(), Some(post.id.as_str()));
}

#[test]
fn tailor_never_adds_bullets_or_roles() {
    let p = Profile::example();
    let r = tailor::reorder(&p, &backend(Source::Greenhouse));
    assert_eq!(r.experience.len(), p.experience.len());
    for (a, b) in r.experience.iter().zip(&p.experience) {
        let mut x = a.bullets.clone();
        let mut y = b.bullets.clone();
        x.sort();
        y.sort();
        assert_eq!(x, y);
    }
}

#[test]
fn tailor_refuses_invented_skill_in_summary() {
    let err = tailor::run(
        &MockLlm::scripted(["Expert in Java and Scala."]),
        &backend(Source::Lever),
        &Profile::example(),
    )
    .unwrap_err();
    assert!(err.to_string().contains("java"), "{err}");
}

#[test]
fn tailor_refuses_empty_long_or_placeholder_summary() {
    let p = Profile::example();
    let post = backend(Source::Lever);
    assert!(tailor::run(&MockLlm::scripted([""]), &post, &p).is_err());
    assert!(tailor::run(&MockLlm::scripted(["x".repeat(401)]), &post, &p).is_err());
    assert!(tailor::run(&MockLlm::scripted(["Engineer at [COMPANY]"]), &post, &p).is_err());
}

#[test]
fn tailor_from_profile_contact_lines() {
    let r = tailor::Resume::from_profile(&Profile::example());
    assert_eq!(r.name, "Avery Example");
    assert_eq!(r.contact.len(), 5);
}

// ---------- cover letter ----------

#[test]
fn cover_letter_mock_passes_guards() {
    let l = cover_letter::run(
        &MockLlm::new(),
        &backend(Source::Greenhouse),
        &Profile::example(),
    )
    .unwrap();
    assert!(l.payload.body.contains("Acme Robotics"));
    assert!(l.payload.body.contains("Senior Backend Engineer"));
    assert_eq!(l.agent, cover_letter::NAME);
}

#[test]
fn cover_letter_is_deterministic() {
    let a = cover_letter::run(
        &MockLlm::new(),
        &backend(Source::Greenhouse),
        &Profile::example(),
    )
    .unwrap();
    let b = cover_letter::run(
        &MockLlm::new(),
        &backend(Source::Greenhouse),
        &Profile::example(),
    )
    .unwrap();
    assert_eq!(a, b);
}

#[test]
fn cover_letter_rejects_short_drafts() {
    let e = cover_letter::run(
        &MockLlm::scripted(["Hire me, Acme Robotics."]),
        &backend(Source::Greenhouse),
        &Profile::example(),
    );
    assert!(e.unwrap_err().to_string().contains("words"));
}

#[test]
fn cover_letter_rejects_missing_company() {
    let body = "Dear team, ".to_string() + &"I build reliable Rust services. ".repeat(10);
    let e = cover_letter::check(&body, &backend(Source::Greenhouse), &Profile::example());
    assert!(e.unwrap_err().to_string().contains("company"));
}

#[test]
fn cover_letter_rejects_placeholders_and_invented_skills() {
    let filler = "I build reliable services for Acme Robotics. ".repeat(8);
    let p = Profile::example();
    let post = backend(Source::Greenhouse);
    assert!(cover_letter::check(&format!("Dear [Name], {filler}"), &post, &p).is_err());
    assert!(cover_letter::check(&format!("{filler} I am a Swift expert."), &post, &p).is_err());
    assert!(cover_letter::check(&filler, &post, &p).is_ok());
}

#[test]
fn cover_letter_rejects_overlong() {
    let body = "Acme Robotics ".repeat(300);
    assert!(cover_letter::check(&body, &backend(Source::Greenhouse), &Profile::example()).is_err());
}

// ---------- form pre-fill ----------

#[test]
fn prefill_greenhouse_field_names() {
    let d = prefill::run(&backend(Source::Greenhouse), &Profile::example()).payload;
    assert_eq!(d.get("first_name").unwrap().value.as_deref(), Some("Avery"));
    assert_eq!(
        d.get("email").unwrap().value.as_deref(),
        Some("avery@example.com")
    );
    assert!(d.get("urls[LinkedIn]").is_some());
    assert!(d.apply_url.ends_with("#app"));
}

#[test]
fn prefill_lever_ashby_workable_field_names() {
    let p = Profile::example();
    let lv = prefill::run(&backend(Source::Lever), &p).payload;
    assert_eq!(
        lv.get("name").unwrap().value.as_deref(),
        Some("Avery Example")
    );
    assert_eq!(
        lv.get("org").unwrap().value.as_deref(),
        Some("Northwind Data")
    );
    let ab = prefill::run(&backend(Source::Ashby), &p).payload;
    assert!(ab.get("_systemfield_email").is_some());
    let wk = prefill::run(&backend(Source::Workable), &p).payload;
    assert_eq!(wk.get("firstname").unwrap().value.as_deref(), Some("Avery"));
}

#[test]
fn prefill_flags_legal_and_demographic_questions_for_a_human() {
    for s in Source::ALL {
        let d = prefill::run(&backend(s), &Profile::example()).payload;
        for name in ["work_authorization", "sponsorship", "eeo"] {
            assert!(d.get(name).unwrap().needs_human, "{s} {name}");
        }
        assert!(
            d.get("eeo").unwrap().value.is_none(),
            "EEO is never pre-answered"
        );
    }
}

#[test]
fn prefill_suggests_but_does_not_decide_authorization() {
    let d = prefill::run(&backend(Source::Greenhouse), &Profile::example()).payload;
    let f = d.get("work_authorization").unwrap();
    assert_eq!(f.value.as_deref(), Some("Yes"));
    assert!(f.needs_human);
}

#[test]
fn prefill_empty_profile_fields_need_a_human() {
    let mut p = Profile::example();
    p.phone.clear();
    let d = prefill::run(&backend(Source::Greenhouse), &p).payload;
    let phone = d.get("phone").unwrap();
    assert!(phone.value.is_none() && phone.needs_human);
}

#[test]
fn prefill_rss_has_no_known_form() {
    let d = prefill::run(&backend(Source::Rss), &Profile::example()).payload;
    assert!(d.get("external_form").unwrap().needs_human);
    assert_eq!(d.fields.len(), 4);
}

#[test]
fn prefill_summary_counts_fields() {
    let p = prefill::run(&backend(Source::Greenhouse), &Profile::example());
    assert!(p.summary.contains("need you"));
    assert_eq!(p.payload.needs_human().len(), 5);
}

// ---------- receipt tracker ----------

fn tracked() -> Tracker {
    let mut t = Tracker::default();
    let form = prefill::run(&backend(Source::Greenhouse), &Profile::example())
        .approve("Avery")
        .unwrap();
    t.record_submission(
        &form,
        "Acme Robotics",
        "Senior Backend Engineer",
        "2026-11-10",
    )
    .unwrap();
    t
}

#[test]
fn status_transitions() {
    use AppStatus::*;
    assert!(Submitted.can_move_to(Received));
    assert!(Received.can_move_to(Interview));
    assert!(Interview.can_move_to(Offer));
    assert!(Submitted.can_move_to(Rejected));
    assert!(!Interview.can_move_to(Received));
    assert!(!Rejected.can_move_to(Interview));
    assert!(!Offer.can_move_to(Rejected));
    assert!(!Received.can_move_to(Submitted));
    assert!(Withdrawn.is_final());
}

#[test]
fn parse_email_kinds() {
    let r = parse_email(
        "no-reply@us.greenhouse-mail.io",
        "Thank you for applying to Acme Robotics",
        "We received your application.",
    )
    .unwrap();
    assert_eq!(
        (r.status, r.ats),
        (AppStatus::Received, Some(Source::Greenhouse))
    );
    assert_eq!(r.company.as_deref(), Some("Acme Robotics"));
    let i = parse_email(
        "hire@lever.co",
        "Next steps",
        "We would like to schedule an interview with you.",
    )
    .unwrap();
    assert_eq!(i.status, AppStatus::Interview);
    let x = parse_email(
        "x@ashbyhq.com",
        "Your application",
        "Unfortunately we are not moving forward.",
    )
    .unwrap();
    assert_eq!(x.status, AppStatus::Rejected);
    let o = parse_email(
        "a@workable.com",
        "Offer",
        "We are pleased to offer you the role.",
    )
    .unwrap();
    assert_eq!(
        (o.status, o.ats),
        (AppStatus::Offer, Some(Source::Workable))
    );
    assert!(parse_email("friend@example.com", "Lunch?", "Tacos at noon").is_none());
}

#[test]
fn tracker_records_submission_once() {
    let mut t = tracked();
    assert_eq!(
        t.get("greenhouse:acme:1").unwrap().status,
        AppStatus::Submitted
    );
    let form = prefill::run(&backend(Source::Greenhouse), &Profile::example())
        .approve("Avery")
        .unwrap();
    assert!(t
        .record_submission(&form, "Acme Robotics", "x", "2026-11-11")
        .is_err());
}

#[test]
fn tracker_proposes_and_applies_after_approval() {
    let mut t = tracked();
    let p = t
        .propose(
            "no-reply@greenhouse.io",
            "Thank you for applying to Acme Robotics",
            "",
            "2026-11-11",
        )
        .unwrap();
    assert_eq!(
        (p.payload.from, p.payload.to),
        (AppStatus::Submitted, AppStatus::Received)
    );
    // Nothing changed before approval.
    assert_eq!(
        t.get("greenhouse:acme:1").unwrap().status,
        AppStatus::Submitted
    );
    t.apply(p.approve("Avery").unwrap()).unwrap();
    let app = t.get("greenhouse:acme:1").unwrap();
    assert_eq!(app.status, AppStatus::Received);
    assert_eq!(app.history.len(), 2);
    assert_eq!(app.last_update, "2026-11-11");
}

#[test]
fn tracker_ignores_unmatched_or_backward_mail() {
    let mut t = tracked();
    assert!(t
        .propose(
            "x@greenhouse.io",
            "Thank you for applying to Globex",
            "",
            "d"
        )
        .is_none());
    assert!(
        t.propose(
            "x@lever.co",
            "Thank you for applying to Acme Robotics",
            "",
            "d"
        )
        .is_none(),
        "wrong ATS"
    );
    let i = t
        .propose(
            "x@greenhouse.io",
            "Interview with Acme Robotics",
            "Let's schedule an interview",
            "d",
        )
        .unwrap();
    t.apply(i.approve("Avery").unwrap()).unwrap();
    assert!(
        t.propose(
            "x@greenhouse.io",
            "Thank you for applying to Acme Robotics",
            "",
            "d"
        )
        .is_none(),
        "no going back"
    );
}

#[test]
fn tracker_refuses_stale_update() {
    let mut t = tracked();
    let a = t
        .propose(
            "x@greenhouse.io",
            "Thank you for applying to Acme Robotics",
            "",
            "d1",
        )
        .unwrap();
    let b = t
        .propose(
            "x@greenhouse.io",
            "Update from Acme Robotics",
            "Unfortunately we chose other candidates",
            "d2",
        )
        .unwrap();
    t.apply(b.approve("Avery").unwrap()).unwrap();
    assert!(t.apply(a.approve("Avery").unwrap()).is_err());
}

// ---------- follow-up ----------

#[test]
fn follow_up_due_after_wait() {
    let t = tracked();
    assert_eq!(follow_up::due(&t, today(), 7).len(), 1);
    assert!(follow_up::due(&t, today(), 11).is_empty());
}

#[test]
fn follow_up_not_due_after_interview_or_follow_up() {
    let mut t = tracked();
    let f = follow_up::run(
        &MockLlm::new(),
        &t.applications[0].clone(),
        &Profile::example(),
    )
    .unwrap();
    t.mark_followed_up(&f.approve("Avery").unwrap(), "2026-11-20")
        .unwrap();
    assert!(follow_up::due(&t, today(), 7).is_empty());
    let mut t2 = tracked();
    let i = t2
        .propose(
            "x@greenhouse.io",
            "Acme Robotics",
            "schedule an interview with Acme Robotics",
            "d",
        )
        .unwrap();
    t2.apply(i.approve("Avery").unwrap()).unwrap();
    assert!(follow_up::due(&t2, today(), 7).is_empty());
}

#[test]
fn follow_up_draft_splits_subject() {
    let t = tracked();
    let f = follow_up::run(&MockLlm::new(), &t.applications[0], &Profile::example())
        .unwrap()
        .payload;
    assert_eq!(
        f.subject,
        "Following up on my Senior Backend Engineer application"
    );
    assert!(f.body.starts_with("Hello Acme Robotics team"));
}

#[test]
fn follow_up_guards() {
    let t = tracked();
    let app = &t.applications[0];
    let p = Profile::example();
    assert!(follow_up::run(&MockLlm::scripted(["Subject: hi\n"]), app, &p).is_err());
    assert!(follow_up::run(&MockLlm::scripted(["word ".repeat(200)]), app, &p).is_err());
    assert!(follow_up::run(&MockLlm::scripted(["Hi {{name}}"]), app, &p).is_err());
    let ok = follow_up::run(
        &MockLlm::scripted(["Just checking in on my application."]),
        app,
        &p,
    )
    .unwrap();
    assert_eq!(ok.payload.subject, "Following up: Senior Backend Engineer");
}

#[test]
fn split_subject_helper() {
    assert_eq!(
        follow_up::split_subject("Subject: A\n\nBody", "F"),
        ("A".to_string(), "Body".to_string())
    );
    assert_eq!(
        follow_up::split_subject("Body only", "F"),
        ("F".to_string(), "Body only".to_string())
    );
}

#[test]
fn mark_followed_up_needs_tracked_posting() {
    let mut t = Tracker::default();
    let f = follow_up::FollowUp {
        posting_id: "nope".into(),
        company: "x".into(),
        subject: "s".into(),
        body: "b".into(),
    };
    assert!(t
        .mark_followed_up(
            &Pending::new("follow_up", "", f).approve("Avery").unwrap(),
            "d"
        )
        .is_err());
}

#[test]
fn mock_is_the_llm_name() {
    assert_eq!(MockLlm::new().name(), "mock");
}
