use time::PrimitiveDateTime;

/// How far along the static ladder an invoice has walked. A rung is one position in the
/// `pt_mapping_adaptive_retries` gap list; past the end the ladder is spent.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StaticLadderProgress {
    /// Rungs already used; the next one to offer is `consumed_rungs + 1`.
    #[serde(default)]
    pub consumed_rungs: i32,
}

impl StaticLadderProgress {
    /// Opening position for an invoice entering recovery for the first time.
    pub fn seed_for_new_invoice(
        intent_retry_count: u16,
        max_hybrid_cascading_retry_count: u16,
    ) -> Self {
        Self {
            consumed_rungs: intent_retry_count
                .min(max_hybrid_cascading_retry_count)
                .into(),
        }
    }

    /// Opening position for an invoice already in recovery whose ladder state was never recorded
    /// — a row written before this field existed.
    pub fn seed_for_existing_invoice(
        intent_retry_count: u16,
        max_hybrid_cascading_retry_count: u16,
    ) -> Self {
        Self {
            consumed_rungs: intent_retry_count
                .min(max_hybrid_cascading_retry_count.saturating_sub(1))
                .into(),
        }
    }

    /// The rung to ask the ladder for on this decision — the one after those already used.
    pub fn next_rung(&self) -> i32 {
        self.consumed_rungs + 1
    }
}

/// What the static ladder is to the model's time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StaticLadderRole {
    /// The model's time stands as it is; the ladder is not consulted at all.
    Standby,
    /// The earlier calendar day of the two wins, ties to the ladder.
    Ceiling,
}

/// Which source produced the scheduled time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleSource {
    /// Whichever retry model the invoice was routed to.
    Model,
    /// The static ladder, under `StaticLadderRole::Ceiling`.
    Static,
    /// The MIT cascading ladder, the global fallback for whatever the model declines.
    Fallback,
}

/// Outcome of one scheduling decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleDecision {
    /// Time to schedule the next retry for.
    pub schedule_time: PrimitiveDateTime,
    /// State to persist back onto the tracking data.
    pub next_progress: StaticLadderProgress,
    /// Which source won, for logging and analytics.
    pub source: ScheduleSource,
}

/// A decision that leaves the ladder where it is, so the same rung is offered again.
fn keep_rung(
    schedule: &StaticLadderProgress,
    schedule_time: PrimitiveDateTime,
    source: ScheduleSource,
) -> ScheduleDecision {
    ScheduleDecision {
        schedule_time,
        next_progress: StaticLadderProgress {
            consumed_rungs: schedule.consumed_rungs,
        },
        source,
    }
}

/// A decision taken off the ladder, which uses up the rung it supplied. The only way the ladder
/// advances, and so the only way it is spent.
fn spend_rung(queried_rung: i32, schedule_time: PrimitiveDateTime) -> ScheduleDecision {
    ScheduleDecision {
        schedule_time,
        next_progress: StaticLadderProgress {
            consumed_rungs: queried_rung,
        },
        source: ScheduleSource::Static,
    }
}

/// Pick the next retry time under the role the invoice's variant assigns the static ladder.
/// `None` when every source in play has declined.
///
/// Under `Standby` the model's time stands and the ladder is not consulted, so callers pass
/// `None` for `static_time`. Under `Ceiling` the earlier calendar day wins and a tie goes to the
/// ladder, since it already covers that day. Only the static branch spends a rung.
pub fn decide_next_retry(
    schedule: &StaticLadderProgress,
    ladder: StaticLadderRole,
    queried_rung: i32,
    model_time: Option<PrimitiveDateTime>,
    static_time: Option<PrimitiveDateTime>,
    fallback_time: Option<PrimitiveDateTime>,
) -> Option<ScheduleDecision> {
    match ladder {
        StaticLadderRole::Standby => model_time
            .map(|schedule_time| keep_rung(schedule, schedule_time, ScheduleSource::Model))
            .or_else(|| {
                fallback_time.map(|schedule_time| {
                    keep_rung(schedule, schedule_time, ScheduleSource::Fallback)
                })
            }),

        StaticLadderRole::Ceiling => match (static_time, model_time) {
            (Some(static_time), Some(model_time)) if model_time.date() < static_time.date() => {
                Some(keep_rung(schedule, model_time, ScheduleSource::Model))
            }
            (Some(static_time), _) => Some(spend_rung(queried_rung, static_time)),
            (None, Some(model_time)) => {
                Some(keep_rung(schedule, model_time, ScheduleSource::Model))
            }
            (None, None) => fallback_time
                .map(|schedule_time| keep_rung(schedule, schedule_time, ScheduleSource::Fallback)),
        },
    }
}

#[cfg(test)]
mod tests {
    use time::Duration;

    use super::*;

    /// `hours` past a fixed epoch, so tests read as a timeline.
    fn at(hours: i64) -> PrimitiveDateTime {
        let epoch = PrimitiveDateTime::new(
            time::Date::from_calendar_date(2026, time::Month::January, 1)
                .expect("valid calendar date"),
            time::Time::MIDNIGHT,
        );
        epoch + Duration::hours(hours)
    }

    fn at_rung(consumed_rungs: i32) -> StaticLadderProgress {
        StaticLadderProgress { consumed_rungs }
    }

    // ---- seeding ----------------------------------------------------------
    //
    // Both constructors clamp the billing connector's own attempts against the cascading
    // allowance. They differ only in whether the ladder may open fully consumed: a new invoice
    // may, an invoice already in recovery keeps one position in hand.

    const HYBRID_CAP: u16 = 5;

    #[test]
    fn a_new_invoice_below_the_cap_consumes_what_the_connector_spent() {
        // Two billing-connector attempts, so the ladder resumes at position 3.
        let schedule = StaticLadderProgress::seed_for_new_invoice(2, HYBRID_CAP);

        assert_eq!(schedule.consumed_rungs, 2);
        assert_eq!(schedule.next_rung(), 3);
    }

    #[test]
    fn a_new_invoice_at_or_past_the_cap_opens_the_ladder_fully_consumed() {
        // The connector used the whole allowance before recovery ever saw the invoice, so there
        // is no cascading position left and the model carries it alone.
        assert_eq!(
            StaticLadderProgress::seed_for_new_invoice(HYBRID_CAP, HYBRID_CAP).consumed_rungs,
            5
        );
        // Beyond the cap clamps rather than running past it.
        assert_eq!(
            StaticLadderProgress::seed_for_new_invoice(9, HYBRID_CAP).consumed_rungs,
            5
        );
    }

    #[test]
    fn an_existing_invoice_below_the_cap_consumes_what_the_connector_spent() {
        let schedule = StaticLadderProgress::seed_for_existing_invoice(2, HYBRID_CAP);

        assert_eq!(schedule.consumed_rungs, 2);
        assert_eq!(schedule.next_rung(), 3);
    }

    #[test]
    fn an_existing_invoice_at_or_past_the_cap_keeps_one_position_in_hand() {
        // Unlike a new invoice, one position is held back — an invoice mid-recovery always has a
        // cascading retry left to offer.
        assert_eq!(
            StaticLadderProgress::seed_for_existing_invoice(HYBRID_CAP, HYBRID_CAP).consumed_rungs,
            4
        );
        assert_eq!(
            StaticLadderProgress::seed_for_existing_invoice(9, HYBRID_CAP).consumed_rungs,
            4
        );
        // And the position it offers is the last one on the ladder.
        assert_eq!(
            StaticLadderProgress::seed_for_existing_invoice(9, HYBRID_CAP).next_rung(),
            5
        );
    }

    #[test]
    fn an_unconfigured_cap_opens_the_ladder_at_the_top() {
        // A billing connector with no hybrid allowance configured reads as zero. Neither
        // constructor may underflow; both must leave the ladder unconsumed so the first decision
        // still has position 1 to offer.
        assert_eq!(
            StaticLadderProgress::seed_for_new_invoice(4, 0).consumed_rungs,
            0
        );
        assert_eq!(
            StaticLadderProgress::seed_for_existing_invoice(4, 0).consumed_rungs,
            0
        );
        assert_eq!(
            StaticLadderProgress::seed_for_existing_invoice(4, 0).next_rung(),
            1
        );
    }

    // ---- the decision rule ------------------------------------------------

    /// Every arm of both rules, with the roles interleaved: a model time later than the other
    /// candidate is overridden under `Ceiling` and survives under `Standby`.
    #[test]
    fn each_arm_picks_its_source_and_only_static_spends_a_position() {
        use ScheduleSource::{Fallback, Model, Static};
        use StaticLadderRole::{Ceiling, Standby};

        // 2 rungs used throughout, so `queried_rung` is 3: a spent rung leaves the count at 3,
        // a kept one at 2.
        #[rustfmt::skip]
        let cases = [
            // role,    model,          static,         fallback,       expected (source, rungs)
            (Ceiling,   Some(at(72)),   Some(at(240)),  None,           Some((Model, 2))),
            (Ceiling,   Some(at(336)),  Some(at(240)),  None,           Some((Static, 3))),
            (Ceiling,   Some(at(249)),  Some(at(258)),  None,           Some((Static, 3))),
            (Ceiling,   Some(at(336)),  None,           None,           Some((Model, 2))),
            (Ceiling,   None,           Some(at(240)),  None,           Some((Static, 3))),
            (Ceiling,   None,           None,           Some(at(240)),  Some((Fallback, 2))),
            (Ceiling,   None,           None,           None,           None),
            (Standby,   Some(at(336)),  None,           Some(at(240)),  Some((Model, 2))),
            (Standby,   None,           None,           Some(at(240)),  Some((Fallback, 2))),
            (Standby,   None,           None,           None,           None),
        ];

        for (ladder, model_time, static_time, fallback_time, expected) in cases {
            let schedule = at_rung(2);
            let row = format!(
                "{ladder:?} model={model_time:?} static={static_time:?} fallback={fallback_time:?}"
            );
            let decision = decide_next_retry(
                &schedule,
                ladder,
                schedule.next_rung(),
                model_time,
                static_time,
                fallback_time,
            );

            match expected {
                None => assert_eq!(
                    decision, None,
                    "{row} has nothing to offer, so must decline"
                ),
                Some((source, consumed_rungs)) => {
                    let decision = decision.expect(&row);
                    assert_eq!(decision.source, source, "{row} picked the wrong source");

                    let expected_time = match source {
                        Model => model_time,
                        Static => static_time,
                        Fallback => fallback_time,
                    }
                    .expect("the winning source must have been given a time");
                    assert_eq!(decision.schedule_time, expected_time, "{row} wrong time");

                    assert_eq!(
                        decision.next_progress.consumed_rungs, consumed_rungs,
                        "{row} spent the wrong number of ladder positions"
                    );
                }
            }
        }
    }

    // ---- persistence ------------------------------------------------------

    #[test]
    fn round_trips_through_json_and_absent_fields_default() {
        let schedule = at_rung(3);
        let encoded = serde_json::to_value(&schedule).expect("serialises");
        let decoded: StaticLadderProgress = serde_json::from_value(encoded).expect("deserialises");
        assert_eq!(decoded, schedule);

        // Invoices already in flight have no `static_ladder_progress` at all.
        let empty: StaticLadderProgress = serde_json::from_value(serde_json::json!({}))
            .expect("absent fields fall back to defaults");
        assert_eq!(empty, StaticLadderProgress::default());
        assert_eq!(empty.consumed_rungs, 0);
        // …and therefore resume at whatever the ladder was already indexed by.
        assert_eq!(empty.next_rung(), 1);
    }
}
