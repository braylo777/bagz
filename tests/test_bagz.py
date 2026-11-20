"""Bindings over the example catalog. No extra jobs, no personal data."""

from __future__ import annotations

from pathlib import Path

import pytest

import bagz

ROOT = Path(__file__).resolve().parents[1]
JOBS = ROOT / "src" / "data" / "jobs.json"
DAY = "2026-11-20"


def ids(jobs: list[bagz.Job]) -> list[str]:
    return [job.id for job in jobs]


def test_load_jobs_reads_the_example_file() -> None:
    catalog = bagz.load_jobs(str(JOBS))
    bundled = bagz.Catalog.bundled()
    assert isinstance(catalog, bagz.Catalog)
    assert len(catalog) == 12
    assert ids(catalog.jobs()) == ids(bundled.jobs())
    assert ids(catalog.jobs())[0] == "froth"
    assert catalog.get("missing") is None

    froth = catalog.get("froth")
    assert froth is not None
    assert froth.title == "Barista"
    assert froth.company == "Froth Club"
    assert froth.pay == "$17.85/hr"
    assert froth.pay_score == 62
    assert froth.match_pct == 91
    assert froth.chase == 27846
    assert froth.area == "East Austin"
    assert froth.mode == "Onsite"
    assert froth.skills == ["Espresso", "Register", "Rush"]
    assert froth.reasons[0].startswith("You already open")
    assert froth.score() == 70
    assert froth.rarity() == "legendary"
    assert froth.sticker() == "HOT"
    assert "Job(" in repr(froth)


def test_missing_file_is_an_os_error() -> None:
    with pytest.raises(OSError, match="could not read jobs file"):
        bagz.load_jobs("/no/such/bagz-jobs.json")


def test_filter_score_and_skill_rank() -> None:
    catalog = bagz.Catalog.bundled()
    assert ids(catalog.filter(mode="remote")) == ["clip"]
    assert ids(catalog.filter(area="east")) == ["froth", "poster", "salsa"]
    assert ids(catalog.filter(skill="React")) == ["carton"]
    assert ids(catalog.filter(min_match=90)) == ["froth", "poster", "quad"]

    clip = catalog.get("clip")
    carton = catalog.get("carton")
    spoke = catalog.get("spoke")
    assert clip is not None and clip.sticker() == "REMOTE" and clip.rarity() == "rare"
    assert carton is not None and carton.sticker() == "$$$" and carton.score() == 82
    assert spoke is not None and spoke.sticker() == "NEW"

    ranked = catalog.rank_for_skills(["React", "Espresso"])
    assert ranked[0].job.id == "froth"
    assert ranked[0].shared == ["Espresso"]
    assert ranked[0].overlap_pct == 33
    assert ranked[1].job.id == "carton"

    with pytest.raises(ValueError, match="min_match"):
        catalog.filter(min_match=101)


def test_swipe_lists_applied_newest_first() -> None:
    deck = bagz.Deck(bagz.Catalog.bundled())
    deck.swipe("froth", "bag", today=DAY)
    deck.swipe("rack", "pass", today=DAY)
    deck.swipe("carton", "super", today=DAY)

    applied = deck.applied()
    assert ids(applied) == ["carton", "froth"]
    assert applied[0].title == "Junior Developer"
    assert applied[0].company == "Carton Labs"
    assert ids(deck.passed()) == ["rack"]
    assert "rack" not in ids(deck.remaining())
    assert deck.history() == [("froth", "bag"), ("rack", "pass"), ("carton", "bag")]
    assert deck.xp == 35
    assert deck.streak == 1
    assert deck.combo == 1
    assert deck.last_day == DAY
    assert deck.plan == "free"
    assert deck.chased == 67850 + 27846
    assert deck.rank == "Rookie"
    assert ids(deck.queue()) == ["carton", "froth", "rack", "ping"]
    assert ids(deck.watchers())[-1] != "carton"


def test_rejected_swipe_leaves_the_deck_alone() -> None:
    deck = bagz.Deck(bagz.Catalog.bundled())
    with pytest.raises(ValueError, match="unknown job"):
        deck.swipe("nope", "bag", today=DAY)
    with pytest.raises(ValueError, match="decision"):
        deck.swipe("froth", "like", today=DAY)
    assert deck.applied() == []
    assert deck.xp == 0
    assert deck.history() == []


def test_undo_reset_and_saved_state() -> None:
    deck = bagz.Deck(bagz.Catalog.bundled())
    deck.swipe("froth", "bag", today=DAY)
    deck.swipe("ping", "bag", today="2026-11-21")
    assert deck.streak == 2
    deck.undo()
    assert ids(deck.applied()) == ["froth"]
    assert deck.xp == 20
    assert deck.streak == 2

    raw = deck.export_state()
    other = bagz.Deck(bagz.Catalog.bundled())
    other.import_state(raw)
    assert ids(other.applied()) == ["froth"]
    assert other.xp == 20
    assert other.streak == 2
    assert other.combo == 0

    other.import_state(
        '{"plan":"agent","history":[{"id":"quad","decision":"bag"},{"id":"quad","decision":"pass"}],"xp":120}'
    )
    assert other.plan == "vip"
    assert ids(other.applied()) == ["quad"]
    assert other.rank == "Bag Secured"

    with pytest.raises(ValueError, match="not valid"):
        other.import_state("{")
    assert ids(other.applied()) == ["quad"]

    other.set_plan("pro")
    assert other.plan == "pro"
    with pytest.raises(ValueError, match="plan"):
        other.set_plan("agent")

    other.reset()
    assert other.applied() == []
    assert other.xp == 120


def test_apply_phase_labels() -> None:
    assert bagz.apply_phase(0, 0) == "Queued"
    assert bagz.apply_phase(450, 0) == "Applying"
    assert bagz.apply_phase(1100, 3) == "Applied"
    assert bagz.apply_phase(1900, 1) == "Reply"
    assert bagz.apply_phase(1900, 2) == "Applied"


def test_type_stub_is_installed_beside_the_module() -> None:
    stub = Path(bagz.__file__).with_suffix(".pyi")
    assert stub.is_file()
    text = stub.read_text(encoding="utf-8")
    assert "def load_jobs" in text
    assert "def swipe" in text
    assert "def applied" in text
