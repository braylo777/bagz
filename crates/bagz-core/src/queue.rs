//! Agent timeline math from `src/components/Chat.tsx`.

/// Status of one row on the agent timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Waiting to be filed.
    Queued,
    /// Filing in progress.
    Applying,
    /// Application recorded.
    Applied,
    /// A reply arrived. Only the first two rows reach this.
    Reply,
}

impl Phase {
    /// Label shown on the timeline.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "Queued",
            Self::Applying => "Applying",
            Self::Applied => "Applied",
            Self::Reply => "Reply",
        }
    }
}

/// Phase for a row `index` after `elapsed_ms` on that row's own clock.
///
/// The chat screen staggers rows by 700ms before calling this. Negative
/// elapsed time is still queued.
#[must_use]
pub fn phase_for(elapsed_ms: i64, index: usize) -> Phase {
    if elapsed_ms < 450 {
        Phase::Queued
    } else if elapsed_ms < 1_100 {
        Phase::Applying
    } else if index < 2 && elapsed_ms >= 1_900 {
        Phase::Reply
    } else {
        Phase::Applied
    }
}

/// Clock label `9:MM:SS`, starting at 9:41:00, matching the demo's `stamp`.
///
/// Minutes wrap at 60 and the hour stays at 9, which is what the TypeScript
/// version does.
#[must_use]
pub fn stamp_label(elapsed_ms: i64) -> String {
    let extra_secs = elapsed_ms.max(0).div_euclid(1_000);
    let total = 9 * 3_600 + 41 * 60 + extra_secs;
    let minute = total.div_euclid(60).rem_euclid(60);
    let second = total.rem_euclid(60);
    format!("9:{minute:02}:{second:02}")
}

/// Hours-saved label for `sent` filed applications.
///
/// This is `(sent * 22 / 60)` shown to one decimal place, the same values as
/// `Number#toFixed(1)` for the demo's sent counts.
#[must_use]
pub fn hours_saved_label(sent: u32) -> String {
    let tenths = (u64::from(sent) * 220 + 30) / 60;
    format!("{}.{}", tenths / 10, tenths % 10)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phases_follow_the_chat_clock() {
        assert_eq!(phase_for(-1, 0), Phase::Queued);
        assert_eq!(phase_for(449, 0), Phase::Queued);
        assert_eq!(phase_for(450, 0), Phase::Applying);
        assert_eq!(phase_for(1_099, 1), Phase::Applying);
        assert_eq!(phase_for(1_100, 0), Phase::Applied);
        assert_eq!(phase_for(1_899, 0), Phase::Applied);
        assert_eq!(phase_for(1_900, 0), Phase::Reply);
        assert_eq!(phase_for(1_900, 1), Phase::Reply);
        assert_eq!(phase_for(5_000, 2), Phase::Applied);
        assert_eq!(Phase::Reply.as_str(), "Reply");
    }

    #[test]
    fn stamp_starts_at_nine_forty_one() {
        assert_eq!(stamp_label(0), "9:41:00");
        assert_eq!(stamp_label(1_500), "9:41:01");
        assert_eq!(stamp_label(-20), "9:41:00");
        assert_eq!(stamp_label(60_000), "9:42:00");
    }

    #[test]
    fn hours_saved_matches_the_demo_labels() {
        let labels = [
            "0.0", "0.4", "0.7", "1.1", "1.5", "1.8", "2.2", "2.6", "2.9", "3.3", "3.7", "4.0",
            "4.4",
        ];
        for (sent, label) in labels.iter().enumerate() {
            assert_eq!(hours_saved_label(u32::try_from(sent).unwrap()), *label);
        }
    }
}
