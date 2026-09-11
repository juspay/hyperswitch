//! Evaluating a [`Rule`] the way CloudWatch evaluates the alarm it was ported from.
//!
//! A transcription, not a design. The one thing to know before changing it: `treat_missing_data`
//! is a *last* resort. Gaps take real readings from further back first, so filling them straight
//! from the policy — the obvious implementation — answers `ALARM` where AWS answers `OK`.
//!
//! <https://docs.aws.amazon.com/AmazonCloudWatch/latest/monitoring/alarms-and-missing-data.html>

use crate::settings::cloudwatch::{MissingDataPolicy, Rule};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Ok,
    Alarm,
    InsufficientData,
}

/// AWS does not publish the width: it "depends on the length of the alarm period and whether it is
/// based on a metric with standard resolution or high resolution". Both worked examples pair a
/// three-period window with a range of five, and the catalogue is all standard resolution.
const RANGE_MARGIN: u32 = 2;

pub fn evaluation_range(rule: &Rule) -> u32 {
    rule.evaluation_periods.saturating_add(RANGE_MARGIN)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reading {
    Breaching,
    NotBreaching,
    Absent,
}

/// Evaluate `rule` over [`evaluation_range`] readings, oldest first.
pub fn evaluate(rule: &Rule, range: &[Option<f64>]) -> State {
    let window_width = usize::try_from(rule.evaluation_periods).unwrap_or(usize::MAX);
    let to_alarm = usize::try_from(rule.datapoints_to_alarm.unwrap_or(rule.evaluation_periods))
        .unwrap_or(usize::MAX);

    let window = window(rule, range, window_width);
    let breaching = window
        .iter()
        .filter(|reading| **reading == Reading::Breaching)
        .count();

    if breaching >= to_alarm || is_premature_alarm(&window, to_alarm) {
        State::Alarm
    } else if window.iter().all(|reading| *reading == Reading::Absent) {
        State::InsufficientData
    } else {
        State::Ok
    }
}

/// The `width` most recent periods. Readings older than the window are spare parts: a gap takes
/// the nearest one still unused, and the policy fills only what is left — "CloudWatch uses missing
/// data points only as few times as possible".
fn window(rule: &Rule, range: &[Option<f64>], width: usize) -> Vec<Reading> {
    let (older, periods) = range.split_at(range.len().saturating_sub(width));
    let mut spare = older.iter().flatten().rev();

    let leading_gaps = width.saturating_sub(periods.len());
    let mut window: Vec<Reading> = std::iter::repeat_n(Reading::Absent, leading_gaps)
        .chain(periods.iter().map(|reading| match reading {
            Some(value) => rule.judge(*value),
            None => Reading::Absent,
        }))
        .collect();

    for reading in &mut window {
        if *reading == Reading::Absent {
            match spare.next() {
                Some(value) => *reading = rule.judge(*value),
                None => break,
            }
        }
    }

    if let Some(filler) = policy_reading(rule.treat_missing_data) {
        for reading in &mut window {
            if *reading == Reading::Absent {
                *reading = filler;
            }
        }
    }

    window
}

/// `Ignore` asks for the rule's previous state, which a stateless evaluator does not have, so it
/// falls through to `Missing`. See `an_ignored_gap_lands_where_missing_does`.
fn policy_reading(policy: MissingDataPolicy) -> Option<Reading> {
    match policy {
        MissingDataPolicy::Breaching => Some(Reading::Breaching),
        MissingDataPolicy::NotBreaching => Some(Reading::NotBreaching),
        MissingDataPolicy::Missing | MissingDataPolicy::Ignore => None,
    }
}

/// AWS's guard against alarming on a metric that has only just stopped reporting: `- - - - X`
/// waits for the next reading, `- - X - -` does not.
fn is_premature_alarm(window: &[Reading], to_alarm: usize) -> bool {
    window
        .iter()
        .position(|reading| *reading == Reading::Breaching)
        .is_some_and(|oldest| {
            window.len() - oldest >= to_alarm
                && window
                    .iter()
                    .skip(oldest)
                    .all(|reading| matches!(reading, Reading::Breaching | Reading::Absent))
        })
}

impl Rule {
    fn judge(&self, value: f64) -> Reading {
        if self.comparison_operator.breaches(value, self.threshold) {
            Reading::Breaching
        } else {
            Reading::NotBreaching
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::{State::*, *};
    use crate::settings::cloudwatch::ComparisonOperator;

    const BREACHING: f64 = 95.0;
    const WITHIN: f64 = 50.0;

    fn rule(evaluation_periods: u32, datapoints_to_alarm: u32, policy: MissingDataPolicy) -> Rule {
        Rule {
            threshold: 90.0,
            comparison_operator: ComparisonOperator::GreaterThanOrEqualToThreshold,
            evaluation_periods,
            datapoints_to_alarm: Some(datapoints_to_alarm),
            treat_missing_data: policy,
            description: "SEV1: something is wrong.".to_owned(),
        }
    }

    /// AWS's own notation: `X` breaching, `0` within the threshold, `-` missing, oldest first.
    fn readings(pattern: &str) -> Vec<Option<f64>> {
        pattern
            .split_whitespace()
            .map(|period| match period {
                "X" => Some(BREACHING),
                "0" => Some(WITHIN),
                "-" => None,
                other => panic!("`{other}` is not a datapoint"),
            })
            .collect()
    }

    /// One row of a table in
    /// <https://docs.aws.amazon.com/AmazonCloudWatch/latest/monitoring/alarms-and-missing-data.html>.
    struct Row {
        datapoints: &'static str,
        missing: State,
        /// `None` where the table says "Retain current state".
        ignore: Option<State>,
        breaching: State,
        not_breaching: State,
    }

    /// `Datapoints to Alarm` and `Evaluation Periods` both 3.
    fn three_of_three() -> Vec<Row> {
        vec![
            Row {
                datapoints: "0 - X - X",
                missing: Ok,
                ignore: Some(Ok),
                breaching: Ok,
                not_breaching: Ok,
            },
            Row {
                datapoints: "0 - - - -",
                missing: Ok,
                ignore: Some(Ok),
                breaching: Ok,
                not_breaching: Ok,
            },
            Row {
                datapoints: "- - - - -",
                missing: InsufficientData,
                ignore: None,
                breaching: Alarm,
                not_breaching: Ok,
            },
            Row {
                datapoints: "0 X X - X",
                missing: Alarm,
                ignore: Some(Alarm),
                breaching: Alarm,
                not_breaching: Alarm,
            },
            Row {
                datapoints: "- - X - -",
                missing: Alarm,
                ignore: None,
                breaching: Alarm,
                not_breaching: Ok,
            },
        ]
    }

    /// 2 out of 3.
    fn two_of_three() -> Vec<Row> {
        vec![
            Row {
                datapoints: "0 - X - X",
                missing: Alarm,
                ignore: Some(Alarm),
                breaching: Alarm,
                not_breaching: Alarm,
            },
            Row {
                datapoints: "0 0 X 0 X",
                missing: Alarm,
                ignore: Some(Alarm),
                breaching: Alarm,
                not_breaching: Alarm,
            },
            Row {
                datapoints: "0 - X - -",
                missing: Ok,
                ignore: Some(Ok),
                breaching: Alarm,
                not_breaching: Ok,
            },
            Row {
                datapoints: "- - - - 0",
                missing: Ok,
                ignore: Some(Ok),
                breaching: Alarm,
                not_breaching: Ok,
            },
            Row {
                datapoints: "- - - X -",
                missing: Alarm,
                ignore: None,
                breaching: Alarm,
                not_breaching: Ok,
            },
        ]
    }

    fn check(rows: &[Row], evaluation_periods: u32, datapoints_to_alarm: u32) {
        for row in rows {
            let range = readings(row.datapoints);

            for (policy, expected) in [
                (MissingDataPolicy::Missing, row.missing),
                (MissingDataPolicy::Breaching, row.breaching),
                (MissingDataPolicy::NotBreaching, row.not_breaching),
            ] {
                assert_eq!(
                    evaluate(
                        &rule(evaluation_periods, datapoints_to_alarm, policy),
                        &range
                    ),
                    expected,
                    "`{}` with {policy:?}, {datapoints_to_alarm} of {evaluation_periods}",
                    row.datapoints
                );
            }
        }
    }

    #[test]
    fn three_of_three_matches_the_published_table() {
        check(&three_of_three(), 3, 3);
    }

    #[test]
    fn two_of_three_matches_the_published_table() {
        check(&two_of_three(), 3, 2);
    }

    /// Wherever AWS commits to a state for `ignore`, it is the one `missing` gives — which is what
    /// makes the fallthrough safe rather than merely convenient.
    #[test]
    fn an_ignored_gap_lands_where_missing_does() {
        for (rows, to_alarm) in [(three_of_three(), 3), (two_of_three(), 2)] {
            for row in rows {
                let range = readings(row.datapoints);
                assert_eq!(
                    evaluate(&rule(3, to_alarm, MissingDataPolicy::Ignore), &range),
                    row.missing,
                    "`{}` with ignore",
                    row.datapoints
                );

                if let Some(ignore) = row.ignore {
                    assert_eq!(ignore, row.missing, "`{}` is documented", row.datapoints);
                }
            }
        }
    }

    #[test]
    fn a_breach_with_only_gaps_after_it_waits_for_the_next_reading() {
        let state = |pattern| evaluate(&rule(3, 3, MissingDataPolicy::Missing), &readings(pattern));

        assert_eq!(state("- - - - X"), Ok);
        assert_eq!(state("- - - X -"), Ok);
        assert_eq!(state("- - X - -"), Alarm);
    }

    /// The whole reason the range is wider than the window: one good reading four periods back
    /// outvotes the policy, so late ingestion does not read as an outage.
    #[test]
    fn a_real_reading_is_preferred_to_the_policy() {
        let state = |pattern| {
            evaluate(
                &rule(3, 3, MissingDataPolicy::Breaching),
                &readings(pattern),
            )
        };

        assert_eq!(state("0 - - - -"), Ok);
        assert_eq!(state("- - - - -"), Alarm);
    }

    /// `datapoints_to_alarm` is unset on every rule in the RDS catalogue.
    #[test]
    fn an_unset_datapoints_to_alarm_demands_the_whole_window() {
        let mut all_of_three = rule(3, 3, MissingDataPolicy::NotBreaching);
        all_of_three.datapoints_to_alarm = None;

        assert_eq!(evaluate(&all_of_three, &readings("0 0 X 0 X")), Ok);
        assert_eq!(evaluate(&all_of_three, &readings("0 0 X X X")), Alarm);
    }

    #[test]
    fn a_rule_reads_two_periods_past_its_window() {
        assert_eq!(evaluation_range(&rule(1, 1, MissingDataPolicy::Missing)), 3);
        assert_eq!(evaluation_range(&rule(5, 5, MissingDataPolicy::Missing)), 7);
    }

    /// A provider returning less than was asked for must not be read as a shorter window.
    #[test]
    fn a_short_range_is_padded_with_gaps_rather_than_shrinking_the_window() {
        assert_eq!(
            evaluate(&rule(3, 3, MissingDataPolicy::Breaching), &readings("X")),
            Alarm
        );
        assert_eq!(
            evaluate(&rule(3, 3, MissingDataPolicy::NotBreaching), &readings("X")),
            Ok
        );
        assert_eq!(
            evaluate(&rule(3, 3, MissingDataPolicy::Missing), &[]),
            InsufficientData
        );
    }

    #[test]
    fn a_rule_reading_below_a_threshold_breaches_downwards() {
        let mut memory = rule(1, 1, MissingDataPolicy::NotBreaching);
        memory.comparison_operator = ComparisonOperator::LessThanOrEqualToThreshold;
        memory.threshold = 858_993_459.0;

        assert_eq!(evaluate(&memory, &[Some(800_000_000.0)]), Alarm);
        assert_eq!(evaluate(&memory, &[Some(900_000_000.0)]), Ok);
    }
}
