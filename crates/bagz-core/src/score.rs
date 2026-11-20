//! Card score, rarity, and sticker. These match `src/components/CardArt.tsx`.

use crate::job::Job;

/// Rarity class derived from `matchPct`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rarity {
    /// Fit below 85.
    Common,
    /// Fit from 85 up to, but not including, 90.
    Rare,
    /// Fit of 90 or more.
    Legendary,
}

impl Rarity {
    /// Class name used by the card CSS: `common`, `rare`, or `legendary`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Common => "common",
            Self::Rare => "rare",
            Self::Legendary => "legendary",
        }
    }
}

/// Corner sticker, in the same priority order as the demo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sticker {
    /// `matchPct` is at least 90.
    Hot,
    /// Mode is exactly `Remote` and the role is not hot.
    Remote,
    /// `posted` ends with `h ago`.
    New,
    /// `payScore` is at least 80.
    Pay,
}

impl Sticker {
    /// Label drawn on the card: `HOT`, `REMOTE`, `NEW`, or `$$$`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Hot => "HOT",
            Self::Remote => "REMOTE",
            Self::New => "NEW",
            Self::Pay => "$$$",
        }
    }
}

/// Rarity for a fit percentage. Values above 100 still classify as legendary.
#[must_use]
pub fn rarity_of(match_pct: u8) -> Rarity {
    if match_pct >= 90 {
        Rarity::Legendary
    } else if match_pct >= 85 {
        Rarity::Rare
    } else {
        Rarity::Common
    }
}

/// Sticker for a job, if the card would show one.
///
/// Priority is hot, then remote, then posted within the hour-count window,
/// then high pay. Mode is compared exactly, as in the TypeScript helper.
#[must_use]
pub fn sticker_for(job: &Job) -> Option<Sticker> {
    if job.match_pct >= 90 {
        Some(Sticker::Hot)
    } else if job.mode == "Remote" {
        Some(Sticker::Remote)
    } else if job.posted.ends_with("h ago") {
        Some(Sticker::New)
    } else if job.pay_score >= 80 {
        Some(Sticker::Pay)
    } else {
        None
    }
}

/// Rounded mean of pay, fit, growth, and vibe.
///
/// The rounding matches `Math.round` for these non-negative sums: half away
/// from zero.
#[must_use]
pub fn score_of(job: &Job) -> u8 {
    let sum = u16::from(job.pay_score)
        + u16::from(job.match_pct)
        + u16::from(job.growth)
        + u16::from(job.vibe);
    ((sum + 2) / 4) as u8
}
