//! Deciding whether a run of metric readings is alarming.
//!
//! Everything in here is pure: readings in, a state out. No clock, no provider, no chat. That is
//! what makes the parity claims below testable against AWS's own published tables rather than
//! against a live account, and it is why [`crate::core::alarm`] — which has all three — holds no
//! evaluation logic of its own.
//!
//! ## What is being reproduced
//!
//! CloudWatch's own alarm evaluation, closely enough that porting the `rds-alerts` catalogue does
//! not change when it fires. Four properties matter and each is implemented here:
//!
//! * **A window of N periods, M of which must breach.** `evaluation_periods` is N,
//!   `datapoints_to_alarm` is M, and the breaching datapoints need not be consecutive.
//! * **A wider *evaluation range* than the window.** CloudWatch retrieves more datapoints than N
//!   so that gaps in the recent ones can be covered by real readings from further back. See
//!   [`AlarmRule::evaluate`].
//! * **A missing-data policy**, consulted only when the range cannot supply N real readings.
//! * **Evaluation every minute**, with the window sliding by a minute rather than by a period.
//!   That one lives in [`crate::core::alarm`], because it is about clocks.
//!
//! ## Where parity stops, deliberately
//!
//! Two divergences, both named rather than hidden:
//!
//! 1. **`ignore` is not implementable here.** It means "keep the state the alarm already had",
//!    which is a memory of a previous decision. This service reconstructs both windows from
//!    CloudWatch on every request and remembers nothing, so it cannot honour `ignore`. Rather than
//!    approximate it, [`MissingDataPolicy`] has no such variant and
//!    [`settings::cloudwatch`](crate::settings::cloudwatch) refuses a catalogue that asks for it.
//!    No `rds-alerts` definition does.
//! 2. **The *premature alarm state* rule is not implemented.** AWS documents a special case that
//!    forces `ALARM` when the oldest breaching datapoint in the window is at least as old as M and
//!    everything more recent is breaching or missing. Its published examples are not consistent
//!    enough to reimplement from — two rows of the tables in
//!    <https://docs.aws.amazon.com/AmazonCloudWatch/latest/monitoring/alarms-and-missing-data.html>
//!    satisfy the stated condition and disagree on the outcome — and it only ever changes the
//!    result under `missing` or `ignore`. Neither is used by `rds-alerts`. The tests below encode
//!    all twenty cells of both AWS tables, including the two this divergence gets wrong, so the
//!    gap is visible rather than assumed away.

pub mod catalogue;
pub mod message;

/// What state a set of readings puts an alarm in.
///
/// Three, not two. `INSUFFICIENT_DATA` is not "we had a problem fetching" — a retrieval failure
/// never reaches this module at all, precisely so that a CloudWatch outage cannot be rendered as a
/// metric that stopped reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AlarmState {
    /// Every reading the window needed was within the threshold.
    Ok,
    /// At least `datapoints_to_alarm` of the window's readings breached.
    Alarm,
    /// The evaluation range held no real reading at all, under a policy that says so.
    InsufficientData,
}

impl AlarmState {
    /// The name this state is announced under, matching CloudWatch's own spelling.
    pub fn label(self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::Alarm => "ALARM",
            Self::InsufficientData => "INSUFFICIENT_DATA",
        }
    }
}

/// What to do with the periods of a window that carry no reading.
///
/// CloudWatch's four minus `ignore`; see the module docs for why that one cannot exist here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissingDataPolicy {
    /// A gap counts as a breach.
    Breaching,
    /// A gap counts as within the threshold.
    NotBreaching,
    /// A window of nothing but gaps is [`AlarmState::InsufficientData`].
    Missing,
}

/// Which side of the threshold breaches.
///
/// The four static comparisons the catalogue uses. CloudWatch's anomaly-detection operators
/// (`LessThanLowerOrGreaterThanUpperThreshold` and friends) compare against a band rather than a
/// number and would need a second metric to fetch, so they are not modelled as if they were more
/// of the same.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonOperator {
    /// Breaches above the threshold.
    GreaterThanThreshold,
    /// Breaches at or above the threshold.
    GreaterThanOrEqualToThreshold,
    /// Breaches below the threshold.
    LessThanThreshold,
    /// Breaches at or below the threshold.
    LessThanOrEqualToThreshold,
}

impl ComparisonOperator {
    /// Whether `value` is on the breaching side.
    ///
    /// A `NaN` reading breaches nothing, which falls out of IEEE comparison rather than being
    /// special-cased: every one of these is false against `NaN`, and treating an unrepresentable
    /// aggregate as a breach would page someone over a provider's arithmetic.
    fn breaches(self, value: f64, threshold: f64) -> bool {
        match self {
            Self::GreaterThanThreshold => value > threshold,
            Self::GreaterThanOrEqualToThreshold => value >= threshold,
            Self::LessThanThreshold => value < threshold,
            Self::LessThanOrEqualToThreshold => value <= threshold,
        }
    }

    /// How the comparison reads in an announcement, next to the threshold.
    pub fn symbol(self) -> &'static str {
        match self {
            Self::GreaterThanThreshold => ">",
            Self::GreaterThanOrEqualToThreshold => ">=",
            Self::LessThanThreshold => "<",
            Self::LessThanOrEqualToThreshold => "<=",
        }
    }
}

/// One severity's threshold and window: everything needed to turn readings into a state.
///
/// A severity rather than a definition, because the catalogue's severities are independent
/// generation rules that happen to share a metric. Two severities of one definition differ only in
/// their threshold and are evaluated separately against the same readings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlarmRule {
    /// Which side of [`Self::threshold`] breaches.
    pub comparison_operator: ComparisonOperator,
    /// The number a reading is compared against.
    pub threshold: f64,
    /// N — how many of the most recent periods form the window.
    pub evaluation_periods: u32,
    /// M — how many of the window's readings must breach. Equal to N unless the catalogue says
    /// otherwise, which is how CloudWatch itself defaults it.
    pub datapoints_to_alarm: u32,
    /// What a period with no reading counts as.
    pub treat_missing_data: MissingDataPolicy,
}

impl AlarmRule {
    /// How many datapoints this rule wants fetched — CloudWatch's *evaluation range*.
    ///
    /// Wider than the window on purpose: when recent periods have no reading, CloudWatch prefers a
    /// real reading from further back over a fabricated one, and only fills what it still cannot
    /// cover. `lookback` is how much further back to reach.
    ///
    /// **The size of that widening is not documented.** AWS says only that it "depends on the
    /// length of the alarm period and whether it is based on a metric with standard resolution or
    /// high resolution", and publishes no formula. Both of its worked examples use a
    /// standard-resolution five-minute metric with N = 3 and an evaluation range of 5, so N + 2 is
    /// the only figure it has ever put a number to — which is why `lookback` is configuration with
    /// that default rather than a constant compiled in here. Getting it wrong changes the outcome
    /// only for a window that has some readings but not N of them.
    pub fn evaluation_range(&self, lookback: u32) -> usize {
        usize::try_from(self.evaluation_periods.saturating_add(lookback)).unwrap_or(usize::MAX)
    }

    /// Evaluate `range`, oldest reading first, `None` where a period had no datapoint.
    ///
    /// `range` is the evaluation range, not the window: it is expected to be
    /// [`Self::evaluation_range`] long, and the window is the most recent N *readings* within it.
    ///
    /// The three-branch shape is CloudWatch's, and the order matters:
    ///
    /// 1. **N real readings are available.** The most recent N of them decide, the gaps between
    ///    them are simply stepped over, and the missing-data policy is never consulted. This is
    ///    the branch the wider range exists to reach.
    /// 2. **Some real readings, but fewer than N.** Every one of them counts, and only the
    ///    shortfall is filled according to the policy — "as few times as possible", as AWS puts it.
    /// 3. **None at all.** The policy alone decides, and it is the only branch that can produce
    ///    [`AlarmState::InsufficientData`].
    pub fn evaluate(&self, range: &[Option<f64>]) -> Evaluation {
        let window = usize::try_from(self.evaluation_periods).unwrap_or(usize::MAX);
        let to_alarm = usize::try_from(self.datapoints_to_alarm).unwrap_or(usize::MAX);

        // Flattening away the gaps is the whole trick: what CloudWatch calls "the most recent data
        // points collected" is a count of readings, not a span of periods.
        let readings = range.iter().flatten().copied().collect::<Vec<_>>();
        let observed = readings.last().copied();

        if readings.len() >= window {
            let breaching = readings
                .iter()
                .rev()
                .take(window)
                .filter(|value| self.breaches(**value))
                .count();

            return Evaluation {
                state: self.state_for(breaching, to_alarm),
                breaching,
                real: window,
                filled: 0,
                observed,
            };
        }

        let breaching = readings
            .iter()
            .filter(|value| self.breaches(**value))
            .count();
        let filled = window.saturating_sub(readings.len());

        let state = match self.treat_missing_data {
            MissingDataPolicy::Breaching => {
                self.state_for(breaching.saturating_add(filled), to_alarm)
            }
            MissingDataPolicy::NotBreaching => self.state_for(breaching, to_alarm),
            MissingDataPolicy::Missing if readings.is_empty() => AlarmState::InsufficientData,
            MissingDataPolicy::Missing => self.state_for(breaching, to_alarm),
        };

        Evaluation {
            state,
            breaching,
            real: readings.len(),
            filled,
            observed,
        }
    }

    fn breaches(&self, value: f64) -> bool {
        self.comparison_operator.breaches(value, self.threshold)
    }

    fn state_for(&self, breaching: usize, to_alarm: usize) -> AlarmState {
        if breaching >= to_alarm {
            AlarmState::Alarm
        } else {
            AlarmState::Ok
        }
    }
}

/// What one evaluation found, beyond the state itself.
///
/// The extra counts exist for the announcement and for the response body: "3 of 3 breaching" and
/// "2 of 3 readings were missing" are the two sentences an operator asks for first, and neither is
/// recoverable from [`AlarmState`] alone.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Evaluation {
    /// The state these readings put the alarm in.
    pub state: AlarmState,
    /// How many of the evaluated readings breached.
    pub breaching: usize,
    /// How many real readings the window was decided on.
    pub real: usize,
    /// How many periods the missing-data policy had to stand in for.
    pub filled: usize,
    /// The most recent real reading in the range, if there was one.
    ///
    /// The number that goes in the message. `None` means the range held nothing real — in which
    /// case the state came entirely from the policy and there is no observed value to quote.
    pub observed: Option<f64>,
}

/// A change of state between two consecutive evaluations.
///
/// Announcing is driven by this and nothing else: an alarm that stays breaching produces no
/// transition and therefore no second message, which is what makes a stateless evaluator quiet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transition {
    /// Where the previous evaluation left it.
    pub from: AlarmState,
    /// Where this evaluation puts it.
    pub to: AlarmState,
}

impl Transition {
    /// The transition between two evaluations, or `None` if the state did not move.
    pub fn between(previous: AlarmState, current: AlarmState) -> Option<Self> {
        (previous != current).then_some(Self {
            from: previous,
            to: current,
        })
    }
}

/// The `count` readings of `grid` ending `back` slots before its end.
///
/// `None` when the grid is too short to supply them, which a caller must treat as "cannot
/// evaluate" rather than as a window of gaps: a short grid means we did not fetch what we meant
/// to, and inventing missing readings from it would put the fabrication this module refuses back
/// in through the other side.
pub fn window(grid: &[Option<f64>], count: usize, back: usize) -> Option<&[Option<f64>]> {
    let end = grid.len().checked_sub(back)?;
    let start = end.checked_sub(count)?;

    grid.get(start..end)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    /// Read AWS's own notation: `0` a reading within the threshold, `X` a breaching one, `-` a
    /// period with no reading. Thresholded at 1.0 with `>`, so `X` is 2.0 and `0` is 0.0.
    fn readings(pattern: &str) -> Vec<Option<f64>> {
        pattern
            .chars()
            .filter(|character| !character.is_whitespace())
            .map(|character| match character {
                'X' => Some(2.0),
                '0' => Some(0.0),
                _ => None,
            })
            .collect()
    }

    fn rule(window: u32, to_alarm: u32, policy: MissingDataPolicy) -> AlarmRule {
        AlarmRule {
            comparison_operator: ComparisonOperator::GreaterThanThreshold,
            threshold: 1.0,
            evaluation_periods: window,
            datapoints_to_alarm: to_alarm,
            treat_missing_data: policy,
        }
    }

    fn state(pattern: &str, window: u32, to_alarm: u32, policy: MissingDataPolicy) -> AlarmState {
        rule(window, to_alarm, policy)
            .evaluate(&readings(pattern))
            .state
    }

    /// The first table from AWS's missing-data documentation: N = 3, M = 3, evaluation range 5.
    ///
    /// Every cell of the `breaching` and `notBreaching` columns, which are the only two policies
    /// `rds-alerts` uses, plus the `missing` column so the one divergence stays visible.
    #[test]
    fn the_published_three_of_three_table_is_reproduced() {
        use MissingDataPolicy::{Breaching, Missing, NotBreaching};

        for (pattern, breaching, not_breaching, missing) in [
            ("0 - X - X", AlarmState::Ok, AlarmState::Ok, AlarmState::Ok),
            ("0 - - - -", AlarmState::Ok, AlarmState::Ok, AlarmState::Ok),
            (
                "- - - - -",
                AlarmState::Alarm,
                AlarmState::Ok,
                AlarmState::InsufficientData,
            ),
            (
                "0 X X - X",
                AlarmState::Alarm,
                AlarmState::Alarm,
                AlarmState::Alarm,
            ),
            // Row 5 is AWS's "premature alarm state" case. It publishes `ALARM` for `missing`;
            // this evaluator says `OK`, which is the divergence the module docs name. The
            // `breaching` and `notBreaching` cells still match, and they are the ones in use.
            (
                "- - X - -",
                AlarmState::Alarm,
                AlarmState::Ok,
                AlarmState::Ok,
            ),
        ] {
            assert_eq!(
                state(pattern, 3, 3, Breaching),
                breaching,
                "{pattern} breaching"
            );
            assert_eq!(
                state(pattern, 3, 3, NotBreaching),
                not_breaching,
                "{pattern} notBreaching"
            );
            assert_eq!(state(pattern, 3, 3, Missing), missing, "{pattern} missing");
        }
    }

    /// The second published table: N = 3, M = 2, evaluation range 5 — an M-out-of-N alarm.
    #[test]
    fn the_published_two_of_three_table_is_reproduced() {
        use MissingDataPolicy::{Breaching, Missing, NotBreaching};

        for (pattern, breaching, not_breaching, missing) in [
            (
                "0 - X - X",
                AlarmState::Alarm,
                AlarmState::Alarm,
                AlarmState::Alarm,
            ),
            (
                "0 0 X 0 X",
                AlarmState::Alarm,
                AlarmState::Alarm,
                AlarmState::Alarm,
            ),
            (
                "0 - X - -",
                AlarmState::Alarm,
                AlarmState::Ok,
                AlarmState::Ok,
            ),
            (
                "- - - - 0",
                AlarmState::Alarm,
                AlarmState::Ok,
                AlarmState::Ok,
            ),
            // The M-out-of-N premature alarm case; same divergence as the table above.
            (
                "- - - X -",
                AlarmState::Alarm,
                AlarmState::Ok,
                AlarmState::Ok,
            ),
        ] {
            assert_eq!(
                state(pattern, 3, 2, Breaching),
                breaching,
                "{pattern} breaching"
            );
            assert_eq!(
                state(pattern, 3, 2, NotBreaching),
                not_breaching,
                "{pattern} notBreaching"
            );
            assert_eq!(state(pattern, 3, 2, Missing), missing, "{pattern} missing");
        }
    }

    /// The reason the range is wider than the window: a real reading from further back is used in
    /// preference to a fabricated one, and the policy never gets a say.
    #[test]
    fn real_readings_from_the_back_of_the_range_beat_the_missing_data_policy() {
        // Three real readings exist in the range, so the window is full and `breaching` — which
        // would otherwise turn the two gaps into breaches — changes nothing.
        for policy in [
            MissingDataPolicy::Breaching,
            MissingDataPolicy::NotBreaching,
            MissingDataPolicy::Missing,
        ] {
            assert_eq!(
                state("0 0 - 0 -", 3, 3, policy),
                AlarmState::Ok,
                "{policy:?}"
            );
        }
    }

    /// Breaching datapoints do not have to be consecutive, which is the whole point of M-out-of-N.
    #[test]
    fn breaching_readings_need_not_be_adjacent() {
        assert_eq!(
            state("X 0 X", 3, 2, MissingDataPolicy::NotBreaching),
            AlarmState::Alarm
        );
        assert_eq!(
            state("X 0 0", 3, 2, MissingDataPolicy::NotBreaching),
            AlarmState::Ok
        );
    }

    #[test]
    fn each_operator_breaches_on_its_own_side_of_the_threshold() {
        use ComparisonOperator::{
            GreaterThanOrEqualToThreshold, GreaterThanThreshold, LessThanOrEqualToThreshold,
            LessThanThreshold,
        };

        assert!(!GreaterThanThreshold.breaches(90.0, 90.0));
        assert!(GreaterThanThreshold.breaches(90.1, 90.0));
        assert!(GreaterThanOrEqualToThreshold.breaches(90.0, 90.0));
        assert!(!LessThanThreshold.breaches(120.0, 120.0));
        assert!(LessThanThreshold.breaches(119.0, 120.0));
        assert!(LessThanOrEqualToThreshold.breaches(120.0, 120.0));
    }

    /// A metric that aggregated to `NaN` must not page anyone.
    #[test]
    fn a_nan_reading_breaches_nothing() {
        for operator in [
            ComparisonOperator::GreaterThanThreshold,
            ComparisonOperator::GreaterThanOrEqualToThreshold,
            ComparisonOperator::LessThanThreshold,
            ComparisonOperator::LessThanOrEqualToThreshold,
        ] {
            assert!(!operator.breaches(f64::NAN, 1.0), "{operator:?}");
        }
    }

    /// The value an announcement quotes is the latest real one, not the latest slot.
    #[test]
    fn the_observed_value_skips_a_trailing_gap() {
        let evaluation =
            rule(3, 3, MissingDataPolicy::NotBreaching).evaluate(&[Some(1.0), Some(7.0), None]);

        assert_eq!(evaluation.observed, Some(7.0));
        assert_eq!(evaluation.real, 2);
        assert_eq!(evaluation.filled, 1);
    }

    #[test]
    fn an_evaluation_of_nothing_has_no_observed_value() {
        let evaluation = rule(1, 1, MissingDataPolicy::Breaching).evaluate(&[None, None]);

        assert_eq!(evaluation.state, AlarmState::Alarm);
        assert_eq!(evaluation.observed, None);
    }

    #[test]
    fn a_state_that_does_not_move_is_not_a_transition() {
        assert_eq!(Transition::between(AlarmState::Ok, AlarmState::Ok), None);
        assert_eq!(
            Transition::between(AlarmState::Ok, AlarmState::Alarm),
            Some(Transition {
                from: AlarmState::Ok,
                to: AlarmState::Alarm,
            })
        );
        // Recovery and insufficient data are transitions like any other, and are announced.
        assert!(Transition::between(AlarmState::Alarm, AlarmState::Ok).is_some());
        assert!(Transition::between(AlarmState::Ok, AlarmState::InsufficientData).is_some());
    }

    #[test]
    fn a_window_is_taken_from_the_end_of_the_grid() {
        let grid = [Some(1.0), Some(2.0), Some(3.0), Some(4.0)];

        assert_eq!(window(&grid, 2, 0), Some(&grid[2..4]));
        assert_eq!(window(&grid, 2, 1), Some(&grid[1..3]));
        assert_eq!(window(&grid, 4, 0), Some(&grid[..]));
    }

    /// A grid too short to hold the window is refused rather than padded, so a truncated fetch
    /// cannot be evaluated as a metric that stopped reporting.
    #[test]
    fn a_grid_too_short_for_the_window_yields_nothing() {
        let grid = [Some(1.0), Some(2.0)];

        assert_eq!(window(&grid, 3, 0), None);
        assert_eq!(window(&grid, 2, 1), None);
    }

    #[test]
    fn the_evaluation_range_is_the_window_plus_the_configured_lookback() {
        let rule = rule(3, 3, MissingDataPolicy::NotBreaching);

        assert_eq!(rule.evaluation_range(2), 5);
        assert_eq!(rule.evaluation_range(0), 3);
    }
}
