//! Helpers with no home of their own.

use external_services::metrics_service::Period;
use time::OffsetDateTime;

/// The latest instant at or before `now` on which a period of `period` completed.
///
/// Capped at a minute because that is how often CloudWatch evaluates anything sampled per minute
/// or slower: a 300-second window ends on the latest completed *minute*, not the latest completed
/// five minutes, and slides with it. Below a minute the cadence is the period itself, so a
/// ten-second rule is not left up to fifty-nine seconds stale.
pub fn latest_completed_period(now: OffsetDateTime, period: Period) -> OffsetDateTime {
    let cadence = i64::from(period.seconds()).clamp(1, 60);
    let elapsed = now.unix_timestamp().rem_euclid(cadence);

    (now - time::Duration::seconds(elapsed))
        .replace_nanosecond(0)
        .unwrap_or(now)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use time::macros::datetime;

    use super::*;

    const NOW: OffsetDateTime = datetime!(2026-09-11 12:07:42.5 UTC);

    #[test]
    fn a_minute_or_slower_ends_on_the_last_completed_minute() {
        for period in [Period::ONE_MINUTE, Period::FIVE_MINUTES, Period::ONE_HOUR] {
            assert_eq!(
                latest_completed_period(NOW, period),
                datetime!(2026-09-11 12:07:00 UTC),
                "{period:?}"
            );
        }
    }

    /// Flooring a high-resolution rule to the minute would throw away completed periods.
    #[test]
    fn a_high_resolution_period_ends_on_its_own_boundary() {
        assert_eq!(
            latest_completed_period(NOW, Period::from_seconds(10)),
            datetime!(2026-09-11 12:07:40 UTC)
        );
        assert_eq!(
            latest_completed_period(NOW, Period::from_seconds(1)),
            datetime!(2026-09-11 12:07:42 UTC)
        );
    }

    #[test]
    fn an_instant_already_on_a_boundary_stays_there() {
        assert_eq!(
            latest_completed_period(datetime!(2026-09-11 12:07:00 UTC), Period::ONE_MINUTE),
            datetime!(2026-09-11 12:07:00 UTC)
        );
    }
}
