"""The Python module loads the example jobs and records swipes."""

import json
from pathlib import Path

import pytest

import bagz

ROOT = Path(__file__).resolve().parents[3]
JOBS = ROOT / "src" / "data" / "jobs.json"
DAY = "2026-11-20"


def test_example_catalog_matches_the_file() -> None:
    loaded = bagz.load_example_jobs()
    from_path = bagz.load_jobs(str(JOBS))
    assert [job.id for job in loaded] == [job.id for job in from_path]
    assert len(loaded) == 12
    assert len({job.id for job in loaded}) == 12
    froth = loaded[0]
    assert froth.id == "froth"
    assert froth.title == "Barista"
    assert froth.company == "Froth Club"
    assert froth.pay == "$17.85/hr"
    assert froth.area == "East Austin"
    assert froth.match_pct == 91
    assert "Espresso" in froth.skills
    assert froth.score() == 70
    assert froth.rarity() == "legendary"
    assert froth.sticker() == "HOT"
    carton = next(job for job in loaded if job.id == "carton")
    assert carton.score() == 82
    assert carton.rarity() == "rare"
    assert carton.sticker() == "$$$"
    clip = next(job for job in loaded if job.id == "clip")
    assert clip.sticker() == "REMOTE"
    assert "–" in froth.sends


def test_missing_and_invalid_files(tmp_path: Path) -> None:
    with pytest.raises(FileNotFoundError):
        bagz.load_jobs("/no/such/bagz-jobs.json")

    broken = tmp_path / "jobs.json"
    broken.write_text("{", encoding="utf-8")
    with pytest.raises(ValueError, match="JSON"):
        bagz.load_jobs(str(broken))

    jobs = json.loads(JOBS.read_text(encoding="utf-8"))
    jobs.append(jobs[0])
    duplicate = tmp_path / "dup.json"
    duplicate.write_text(json.dumps(jobs), encoding="utf-8")
    with pytest.raises(ValueError, match="duplicate"):
        bagz.load_jobs(str(duplicate))


def test_swipe_applied_queue_and_undo() -> None:
    deck = bagz.Deck.example()
    assert [job.id for job in deck.remaining()][0] == "froth"
    deck.swipe("froth", "like", day=DAY)
    deck.swipe("quad", "super", day=DAY)
    assert [job.id for job in deck.applied()] == ["quad", "froth"]
    assert deck.xp == 35

    deck.swipe("froth", "pass", day=DAY)
    assert [job.id for job in deck.applied()] == ["quad"]
    assert [job.id for job in deck.passed()] == ["froth"]
    assert deck.xp == 35
    assert "froth" not in {job.id for job in deck.remaining()}

    assert deck.undo() == "froth"
    # Passing replaced the earlier bag, so undo only removes the pass.
    assert [job.id for job in deck.applied()] == ["quad"]
    assert deck.passed() == []
    assert "froth" in {job.id for job in deck.remaining()}
    assert deck.xp == 35

    deck.reset()
    assert deck.applied() == []
    assert len(deck.remaining()) == 12
    assert deck.xp == 35
    assert deck.undo() is None


def test_swipe_rejects_unknown_input() -> None:
    deck = bagz.Deck.load(str(JOBS))
    with pytest.raises(ValueError, match="unknown job"):
        deck.swipe("nope", "bag", day=DAY)
    with pytest.raises(ValueError, match="decision"):
        deck.swipe("froth", "nope", day=DAY)
    with pytest.raises(ValueError, match="YYYY-MM-DD"):
        deck.swipe("froth", "apply", day="tomorrow")
    assert deck.applied() == []


def test_filter_jobs_is_keyword_only_and_ordered() -> None:
    jobs = bagz.load_example_jobs()
    remote = bagz.filter_jobs(jobs, mode="remote")
    assert [job.id for job in remote] == ["clip"]
    assert len(jobs) == 12

    hot = bagz.filter_jobs(jobs, min_match=90)
    assert [job.id for job in hot] == ["froth", "poster", "quad"]
    react = bagz.filter_jobs(jobs, skill="React")
    assert [job.id for job in react] == ["carton"]

    with pytest.raises(TypeError):
        bagz.filter_jobs(jobs, "Remote")  # type: ignore[call-arg]
    with pytest.raises(ValueError, match="min_match"):
        bagz.filter_jobs(jobs, min_match=101)
    with pytest.raises(ValueError, match="min_match"):
        bagz.filter_jobs(jobs, min_match=-1)
