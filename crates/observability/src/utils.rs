//! Helpers with no home of their own.

use external_services::metrics_service::Period;
use time::{Duration, OffsetDateTime};

/// How long an evaluation waits for a completed CloudWatch period to become readable.
///
/// Metric producers publish asynchronously. Evaluating the newest completed minute can therefore
/// reconstruct both sides of a transition before a late datapoint exists, then observe only the
/// recovery after it arrives. Two minutes is a temporary watermark until transition state is
/// persisted.
const METRIC_PUBLICATION_DELAY: Duration = Duration::minutes(2);

/// How often CloudWatch re-evaluates a rule of this period, which is also how far apart two
/// consecutive evaluations are.
///
/// A minute for anything sampled per minute or slower: a 300-second window is re-evaluated every
/// minute and slides with it rather than stepping five minutes at a time. Below a minute the
/// cadence is the period itself.
pub fn evaluation_cadence(period: Period) -> Duration {
    Duration::seconds(i64::from(period.seconds()).clamp(1, 60))
}

/// The latest instant at or before `now` on which an evaluation of `period` could have run.
pub fn latest_completed_period(now: OffsetDateTime, period: Period) -> OffsetDateTime {
    let cadence = evaluation_cadence(period).whole_seconds().max(1);
    let elapsed = now.unix_timestamp().rem_euclid(cadence);

    (now - Duration::seconds(elapsed))
        .replace_nanosecond(0)
        .unwrap_or(now)
}

/// The latest evaluation boundary whose metric data has had time to arrive.
pub fn latest_settled_period(now: OffsetDateTime, period: Period) -> OffsetDateTime {
    latest_completed_period(now - METRIC_PUBLICATION_DELAY, period)
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
    fn an_evaluation_waits_two_minutes_for_metrics_to_settle() {
        assert_eq!(
            latest_settled_period(NOW, Period::ONE_MINUTE),
            datetime!(2026-09-11 12:05:00 UTC)
        );
    }

    #[test]
    fn an_instant_already_on_a_boundary_stays_there() {
        assert_eq!(
            latest_completed_period(datetime!(2026-09-11 12:07:00 UTC), Period::ONE_MINUTE),
            datetime!(2026-09-11 12:07:00 UTC)
        );
    }

    /// A five-minute rule is re-evaluated every minute, so its previous evaluation is a minute
    /// back — not five.
    #[test]
    fn the_cadence_is_a_minute_unless_the_period_is_shorter() {
        assert_eq!(
            evaluation_cadence(Period::FIVE_MINUTES),
            Duration::minutes(1)
        );
        assert_eq!(evaluation_cadence(Period::ONE_MINUTE), Duration::minutes(1));
        assert_eq!(
            evaluation_cadence(Period::from_seconds(10)),
            Duration::seconds(10)
        );
    }
}
