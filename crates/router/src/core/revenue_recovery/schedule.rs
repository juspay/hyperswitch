use time::PrimitiveDateTime;

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StaticLadderProgress {
    /// Static ladder positions consumed so far.
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

    /// Ladder position to query for this decision.
    pub fn next_rung(&self) -> i32 {
        self.consumed_rungs + 1
    }
}

/// Which source produced the scheduled time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleSource {
    /// Whichever retry model the invoice was routed to.
    Model,
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

/// The retry model decides whenever it has an opinion; the MIT cascading ladder is the global
/// fallback for everything it declines. `None` when neither has anything to offer.
///
/// The two are NOT compared — the model's time is taken as it stands, however much later than the
/// fallback's it falls. The fallback is a standby, not a ceiling.
///
/// Neither source spends a ladder position, so `next_progress` echoes what it was handed. The
/// counter is inert here and kept only so the persisted field keeps round-tripping; removing it
/// reaches merchant-facing config and belongs in its own change.
pub fn decide_next_retry(
    schedule: &StaticLadderProgress,
    model_time: Option<PrimitiveDateTime>,
    fallback_time: Option<PrimitiveDateTime>,
) -> Option<ScheduleDecision> {
    let decision = |schedule_time, source| ScheduleDecision {
        schedule_time,
        next_progress: StaticLadderProgress {
            consumed_rungs: schedule.consumed_rungs,
        },
        source,
    };

    model_time
        .map(|schedule_time| decision(schedule_time, ScheduleSource::Model))
        // The model declined, so the global fallback gets the last word. `None` here means there is
        // genuinely nothing left to schedule for this invoice.
        .or_else(|| {
            fallback_time.map(|schedule_time| decision(schedule_time, ScheduleSource::Fallback))
        })
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

    /// A decision the caller would act on. Panics where the test's premise is that one exists.
    fn expect_decision(
        schedule: &StaticLadderProgress,
        model_time: Option<PrimitiveDateTime>,
        fallback_time: Option<PrimitiveDateTime>,
    ) -> ScheduleDecision {
        decide_next_retry(schedule, model_time, fallback_time).expect("a time was available")
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

    // ---- the model decides, the global fallback covers what it declines ---

    #[test]
    fn a_later_model_time_still_wins() {
        // The fallback is not a ceiling: a model day after the fallback's is taken as it stands.
        // Nothing here compares the two.
        let fallback_time = at(240);
        let model_time = at(336);
        assert!(model_time.date() > fallback_time.date());

        let decision = expect_decision(&at_rung(2), Some(model_time), Some(fallback_time));

        assert_eq!(decision.schedule_time, model_time);
        assert_eq!(decision.source, ScheduleSource::Model);
    }

    #[test]
    fn no_model_opinion_uses_the_global_fallback() {
        let decision = expect_decision(&StaticLadderProgress::default(), None, Some(at(240)));

        assert_eq!(decision.schedule_time, at(240));
        assert_eq!(decision.source, ScheduleSource::Fallback);
    }

    #[test]
    fn both_declining_yields_no_decision() {
        assert_eq!(decide_next_retry(&at_rung(5), None, None), None);
    }

    // ---- the rung counter is inert ----------------------------------------

    #[test]
    fn no_source_spends_a_ladder_position() {
        // With the static ladder gone, neither source advances the count: whatever a decision is
        // handed, it hands back. Pinned because the field is still persisted and still seeded, so
        // a future edit that starts moving it would otherwise change stored state unnoticed.
        for (model_time, fallback_time, expected_source) in [
            (Some(at(72)), None, ScheduleSource::Model),
            (Some(at(72)), Some(at(240)), ScheduleSource::Model),
            (None, Some(at(240)), ScheduleSource::Fallback),
        ] {
            for consumed_rungs in [0, 1, 5] {
                let schedule = at_rung(consumed_rungs);
                let decision = expect_decision(&schedule, model_time, fallback_time);

                assert_eq!(decision.source, expected_source);
                assert_eq!(decision.next_progress, schedule);
                assert_eq!(decision.next_progress.consumed_rungs, consumed_rungs);
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
