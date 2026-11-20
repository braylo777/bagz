from __future__ import annotations

from typing import Literal

DecisionName = Literal["bag", "pass", "super"]
StoredDecision = Literal["bag", "pass"]
PlanName = Literal["free", "pro", "vip"]
RarityName = Literal["legendary", "rare", "common"]
PhaseName = Literal["Queued", "Applying", "Applied", "Reply"]

class Job:
    """One example role from ``jobs.json``."""

    id: str
    title: str
    company: str
    pay: str
    pay_score: int
    growth: int
    vibe: int
    chase: int
    area: str
    mode: str
    schedule: str
    posted: str
    applicants: str
    size: str
    stage: str
    match_pct: int
    reasons: list[str]
    skills: list[str]
    perks: list[str]
    reply: str
    sends: str

    def score(self) -> int:
        """Rounded mean of pay, fit, growth, and vibe."""

    def rarity(self) -> RarityName:
        """``legendary`` at 90, ``rare`` at 85, otherwise ``common``."""

    def sticker(self) -> str:
        """``HOT``, ``REMOTE``, ``NEW``, ``$$$``, or ``""`` when there is no sticker."""

    def __repr__(self) -> str: ...

class SkillMatch:
    """A job ranked by how many of its skills a seeker listed."""

    job: Job
    shared: list[str]
    overlap_pct: int

    def __repr__(self) -> str: ...

class Catalog:
    """The example job catalog, in file order."""

    @staticmethod
    def load(path: str) -> Catalog:
        """Load and validate a jobs JSON file."""

    @staticmethod
    def bundled() -> Catalog:
        """Catalog compiled from the repo's example ``jobs.json``."""

    def jobs(self) -> list[Job]: ...
    def get(self, job_id: str) -> Job | None: ...
    def filter(
        self,
        *,
        mode: str | None = None,
        area: str | None = None,
        skill: str | None = None,
        min_match: int | None = None,
    ) -> list[Job]:
        """Jobs matching every set filter, in file order.

        Mode and skill are case-insensitive exact matches. Area is a
        case-insensitive substring. ``min_match`` is inclusive and must be
        from 0 to 100.
        """

    def rank_for_skills(self, skills: list[str]) -> list[SkillMatch]:
        """Rank by skill overlap, then shipped match, then file order."""

    def __len__(self) -> int: ...
    def __repr__(self) -> str: ...

class Deck:
    """Swipe session: history, XP, streak, combo, and plan."""

    def __init__(self, catalog: Catalog) -> None: ...
    def swipe(self, job_id: str, decision: DecisionName, today: str | None = None) -> None:
        """Record ``bag``, ``pass``, or ``super``.

        ``today`` is the UTC day used for the streak (``YYYY-MM-DD``). When
        it is omitted, the streak uses today's UTC date. Unknown jobs and
        unknown decisions raise ``ValueError`` and leave the deck unchanged.
        """

    def undo(self) -> None:
        """Drop the last swipe. XP, streak, and combo stay."""

    def reset(self) -> None:
        """Clear swipe history. XP, streak, plan, and combo stay."""

    def applied(self) -> list[Job]:
        """Applied jobs, newest first."""

    def passed(self) -> list[Job]:
        """Passed jobs, oldest first."""

    def remaining(self) -> list[Job]:
        """Jobs not yet bagged or passed, in file order."""

    def queue(self, limit: int = 4) -> list[Job]:
        """Applied jobs first, then the rest of the catalog."""

    def watchers(self) -> list[Job]:
        """Last three jobs that are still in the deck."""

    def history(self) -> list[tuple[str, StoredDecision]]:
        """``(job_id, "bag" | "pass")``, oldest first. Super is stored as ``bag``."""

    @property
    def xp(self) -> int: ...
    @property
    def streak(self) -> int: ...
    @property
    def combo(self) -> int: ...
    @property
    def plan(self) -> PlanName: ...
    @property
    def last_day(self) -> str: ...
    @property
    def chased(self) -> int: ...
    @property
    def rank(self) -> str: ...
    def set_plan(self, plan: PlanName) -> None:
        """Set ``free``, ``pro``, or ``vip``. A saved ``agent`` plan is only accepted by ``import_state``."""

    def export_state(self) -> str:
        """JSON object in the demo's saved-session shape."""

    def import_state(self, raw: str) -> None:
        """Replace the session from a saved JSON object.

        Invalid JSON raises ``ValueError`` and leaves the deck alone. A parsed
        object is sanitized: unknown jobs and bad decisions are dropped, the
        first valid record for an id wins, and a legacy ``agent`` plan becomes ``vip``.
        """

    def __repr__(self) -> str: ...

def load_jobs(path: str) -> Catalog:
    """Load and validate a jobs JSON file. Same as ``Catalog.load``."""

def apply_phase(elapsed_ms: int, index: int) -> PhaseName:
    """Chat-row phase: ``Queued``, ``Applying``, ``Applied``, or ``Reply``."""
