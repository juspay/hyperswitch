//! Turning an evaluation into the message an operator reads.
//!
//! The catalogue's per-severity `description` fields were written for people on call, and they are
//! reproduced verbatim: this module frames them, it does not paraphrase them. Everything added
//! around a description is there because it is the next thing asked after "what fired" — which
//! instance, what the number actually was, and what it was compared against.
//!
//! Rendering is separated from delivery so that a dry run can produce exactly the message a real
//! run would send. If a dry run built its preview differently, it would be a preview of something
//! else.

use std::fmt::Write;

use crate::domain::alarm::{
    catalogue::{AlarmTarget, Reading},
    Evaluation, MissingDataPolicy, Transition,
};

/// The message announcing one state transition.
pub fn announcement(
    target: &AlarmTarget,
    reading: &Reading,
    transition: Transition,
    evaluation: &Evaluation,
    evaluated_at: time::PrimitiveDateTime,
) -> String {
    let severity = target.severity.to_uppercase();
    let mut message = format!(
        "*{severity} · {} · {} → {}*\n{}\n",
        target.definition,
        transition.from.label(),
        transition.to.label(),
        target.description.trim(),
    );

    let period = reading.period.seconds();
    let _ = writeln!(
        message,
        "• Metric: {} ({:?} over {period}s)",
        reading.describe(),
        reading.aggregation,
    );
    let _ = writeln!(message, "• Dimensions: {}", dimensions(reading));
    let _ = writeln!(message, "• {}", observation(target, evaluation));
    let _ = writeln!(message, "• {}", window(target, evaluation));

    if evaluation.filled > 0 {
        let _ = writeln!(message, "• {}", gaps(target, evaluation));
    }

    let _ = write!(message, "• Evaluated through {}", timestamp(evaluated_at));

    message
}

/// The message reporting that a run could not read some or all of its metrics.
///
/// Separate from an alarm announcement on purpose. A CloudWatch outage is a fact about this
/// service, not about the database, and rendering it as an alarm would put an estate-shaped
/// message in front of someone about something that is not happening to the estate.
pub fn failure_summary(
    total: usize,
    failed: &[String],
    reason: Option<&str>,
    evaluated_at: time::PrimitiveDateTime,
) -> String {
    let at = timestamp(evaluated_at);
    let mut message = if failed.len() >= total {
        format!(
            "*Observability · CloudWatch evaluation failed*\nNo metric readings could be \
             retrieved for the evaluation ending {at}, so no alarm was evaluated this run.\n"
        )
    } else {
        format!(
            "*Observability · CloudWatch evaluation degraded*\n{} of {total} metric readings \
             could not be retrieved for the evaluation ending {at}. Everything else was evaluated \
             normally.\n",
            failed.len(),
        )
    };

    if let Some(reason) = reason {
        let _ = writeln!(message, "• Reason: {reason}");
    }

    for name in failed.iter().take(MAX_LISTED_FAILURES) {
        let _ = writeln!(message, "• Not evaluated: {name}");
    }

    if let Some(remaining) = failed.len().checked_sub(MAX_LISTED_FAILURES) {
        if remaining > 0 {
            let _ = writeln!(message, "• …and {remaining} more");
        }
    }

    let _ = write!(
        message,
        "This run is not retried; the next evaluation covers the same ground."
    );

    message
}

/// How many affected definitions a summary names before it starts counting instead.
///
/// A summary is read on a phone. Naming all seventy-six would be a wall that nobody reads, and the
/// count is the actionable part once the list is long enough to be an outage rather than a metric.
const MAX_LISTED_FAILURES: usize = 12;

fn dimensions(reading: &Reading) -> String {
    let pairs = reading
        .labels
        .iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>();

    pairs.join(", ")
}

fn observation(target: &AlarmTarget, evaluation: &Evaluation) -> String {
    let comparison = format!(
        "threshold {} {}",
        target.rule.comparison_operator.symbol(),
        number(target.rule.threshold),
    );

    match evaluation.observed {
        Some(value) => format!("Observed: {} ({comparison})", number(value)),
        // The state came entirely from the missing-data policy, so quoting a value would be
        // quoting one this run never saw.
        None => format!("Observed: no reading in the evaluation range ({comparison})"),
    }
}

fn window(target: &AlarmTarget, evaluation: &Evaluation) -> String {
    format!(
        "Window: {} of the last {} datapoints breaching, {} needed",
        evaluation.breaching, target.rule.evaluation_periods, target.rule.datapoints_to_alarm,
    )
}

fn gaps(target: &AlarmTarget, evaluation: &Evaluation) -> String {
    let treatment = match target.rule.treat_missing_data {
        MissingDataPolicy::Breaching => "counted as breaching",
        MissingDataPolicy::NotBreaching => "counted as within the threshold",
        MissingDataPolicy::Missing => "not enough data to decide on",
    };

    format!(
        "Missing: only {} real datapoints were available, so {} were {treatment}",
        evaluation.real, evaluation.filled,
    )
}

/// A number as an operator wrote it in the catalogue.
///
/// `f64`'s own formatting already drops a trailing `.0` and never reaches for scientific notation
/// at these magnitudes, so a threshold of `858993459` reads back as itself rather than as
/// `8.58993459e8`.
fn number(value: f64) -> String {
    format!("{value}")
}

/// A UTC instant, to the second.
///
/// Infallible on purpose. `time`'s well-known formatters return a `Result`, and there is nothing
/// sensible to do with an error while building an alert — a timestamp rendered as an error string
/// is worse than one rendered plainly. Every instant reaching here is minute-aligned, so no
/// subsecond is dropped; the response body carries the same instant through
/// `common_utils::custom_serde::iso8601`, which renders milliseconds as well.
pub fn timestamp(instant: time::PrimitiveDateTime) -> String {
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        instant.year(),
        u8::from(instant.month()),
        instant.day(),
        instant.hour(),
        instant.minute(),
        instant.second(),
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use external_services::metrics_service::{Aggregation, Period};
    use time::macros::datetime;

    use super::*;
    use crate::domain::alarm::{AlarmRule, AlarmState, ComparisonOperator};

    fn reading() -> Reading {
        Reading {
            namespace: "AWS/RDS".to_owned(),
            metric_name: "CPUUtilization".to_owned(),
            labels: [("DBInstanceIdentifier", "hyperswitchdb-primary")]
                .into_iter()
                .collect(),
            period: Period::ONE_MINUTE,
            aggregation: Aggregation::Average,
        }
    }

    fn target(policy: MissingDataPolicy) -> AlarmTarget {
        AlarmTarget {
            definition: "rds-primary-cpu".to_owned(),
            classification: "rds-alerts".to_owned(),
            severity: "sev1".to_owned(),
            description: "SEV1: RDS primary database CPU utilization is above 90% (Sev0 limit) \
                          for 5 minutes."
                .to_owned(),
            reading: 0,
            rule: AlarmRule {
                comparison_operator: ComparisonOperator::GreaterThanOrEqualToThreshold,
                threshold: 90.0,
                evaluation_periods: 3,
                datapoints_to_alarm: 3,
                treat_missing_data: policy,
            },
            destination: "smoke".to_owned(),
        }
    }

    fn evaluation(
        state: AlarmState,
        observed: Option<f64>,
        real: usize,
        filled: usize,
    ) -> Evaluation {
        Evaluation {
            state,
            breaching: 3,
            real,
            filled,
            observed,
        }
    }

    fn render(policy: MissingDataPolicy, evaluation: &Evaluation) -> String {
        announcement(
            &target(policy),
            &reading(),
            Transition {
                from: AlarmState::Ok,
                to: AlarmState::Alarm,
            },
            evaluation,
            datetime!(2026-09-10 12:00:00),
        )
    }

    /// Everything the ticket asks a message to carry: the description as written, the metric, the
    /// value, the threshold, and the dimension it fired on.
    #[test]
    fn an_announcement_carries_the_description_metric_value_threshold_and_dimension() {
        let message = render(
            MissingDataPolicy::NotBreaching,
            &evaluation(AlarmState::Alarm, Some(93.25), 3, 0),
        );

        assert!(message.contains("SEV1"), "{message}");
        assert!(message.contains("rds-primary-cpu"), "{message}");
        assert!(message.contains("OK → ALARM"), "{message}");
        assert!(
            message.contains("CPU utilization is above 90% (Sev0 limit)"),
            "{message}"
        );
        assert!(message.contains("AWS/RDS CPUUtilization"), "{message}");
        assert!(
            message.contains("DBInstanceIdentifier=hyperswitchdb-primary"),
            "{message}"
        );
        assert!(message.contains("Observed: 93.25"), "{message}");
        assert!(message.contains("threshold >= 90"), "{message}");
        assert!(message.contains("2026-09-10T12:00:00Z"), "{message}");
    }

    /// Recovery is announced the same way, so an operator who saw the alarm sees it close.
    #[test]
    fn a_recovery_reads_as_a_transition_back() {
        let message = announcement(
            &target(MissingDataPolicy::NotBreaching),
            &reading(),
            Transition {
                from: AlarmState::Alarm,
                to: AlarmState::Ok,
            },
            &evaluation(AlarmState::Ok, Some(12.0), 3, 0),
            datetime!(2026-09-10 12:00:00),
        );

        assert!(message.contains("ALARM → OK"), "{message}");
    }

    /// A state the policy decided has no observed value, and says so instead of quoting one.
    #[test]
    fn an_evaluation_with_no_readings_says_so_rather_than_inventing_a_value() {
        let message = render(
            MissingDataPolicy::Breaching,
            &evaluation(AlarmState::Alarm, None, 0, 3),
        );

        assert!(
            message.contains("no reading in the evaluation range"),
            "{message}"
        );
        assert!(message.contains("counted as breaching"), "{message}");
    }

    /// A window that was complete says nothing about gaps, rather than a line of zeroes.
    #[test]
    fn a_complete_window_mentions_no_gaps() {
        let message = render(
            MissingDataPolicy::NotBreaching,
            &evaluation(AlarmState::Alarm, Some(93.0), 3, 0),
        );

        assert!(!message.contains("Missing:"), "{message}");
    }

    /// Large thresholds come back as they were written, not in scientific notation.
    #[test]
    fn a_large_threshold_reads_as_the_catalogue_wrote_it() {
        assert_eq!(number(858993459.0), "858993459");
        assert_eq!(number(0.2), "0.2");
        assert_eq!(number(140625000.0), "140625000");
    }

    #[test]
    fn a_total_failure_and_a_partial_one_read_differently() {
        let total = failure_summary(
            3,
            &["a".to_owned(), "b".to_owned(), "c".to_owned()],
            Some("the credentials were rejected"),
            datetime!(2026-09-10 12:00:00),
        );
        assert!(total.contains("evaluation failed"), "{total}");
        assert!(total.contains("no alarm was evaluated"), "{total}");
        assert!(total.contains("credentials were rejected"), "{total}");

        let partial = failure_summary(
            20,
            &["rds-primary-cpu".to_owned()],
            None,
            datetime!(2026-09-10 12:00:00),
        );
        assert!(partial.contains("evaluation degraded"), "{partial}");
        assert!(partial.contains("1 of 20"), "{partial}");
        assert!(partial.contains("rds-primary-cpu"), "{partial}");
    }

    /// A long list becomes a count, because a summary is read on a phone.
    #[test]
    fn a_long_failure_list_is_truncated_with_a_count() {
        let failed = (0..20).map(|index| format!("d{index}")).collect::<Vec<_>>();
        let message = failure_summary(40, &failed, None, datetime!(2026-09-10 12:00:00));

        assert!(message.contains("…and 8 more"), "{message}");
    }

    #[test]
    fn a_timestamp_is_utc_to_the_second() {
        assert_eq!(
            timestamp(datetime!(2026-09-10 12:34:56)),
            "2026-09-10T12:34:56Z"
        );
    }
}
