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

/// Which algorithm produced the scheduled time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleSource {
    Static,
    Adaptive,
}

/// Outcome of one scheduling decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleDecision {
    /// Time to schedule the next retry for.
    pub schedule_time: PrimitiveDateTime,
    /// State to persist back onto the tracking data.
    pub next_progress: StaticLadderProgress,
    /// Which algorithm won, for logging and analytics.
    pub source: ScheduleSource,
}

/// The adaptive model decides whenever it has an opinion; the static ladder covers only the
/// decisions it declines. `None` when neither has anything left to offer.
///
/// The two candidates are NOT compared — an adaptive time is taken as it stands, however much later
/// than the ladder's it falls. The ladder is a standby, not a ceiling.
pub fn decide_next_retry(
    schedule: &StaticLadderProgress,
    queried_rung: i32,
    static_time: Option<PrimitiveDateTime>,
    adaptive_time: Option<PrimitiveDateTime>,
) -> Option<ScheduleDecision> {
    // Adaptive spends no ladder position, so the count stays put and the same position is offered
    // again on the next decision.
    let adaptive = |schedule_time| ScheduleDecision {
        schedule_time,
        next_progress: StaticLadderProgress {
            consumed_rungs: schedule.consumed_rungs,
        },
        source: ScheduleSource::Adaptive,
    };
    let static_ladder = |schedule_time| ScheduleDecision {
        schedule_time,
        next_progress: StaticLadderProgress {
            consumed_rungs: queried_rung,
        },
        source: ScheduleSource::Static,
    };

    match (adaptive_time, static_time) {
        (Some(adaptive_time), _) => Some(adaptive(adaptive_time)),
        // The model declined, so the ladder covers this decision and spends a position.
        (None, Some(static_time)) => Some(static_ladder(static_time)),
        (None, None) => None,
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

    /// A decision the caller would act on. Panics where the test's premise is that one exists.
    fn expect_decision(
        schedule: &StaticLadderProgress,
        queried_rung: i32,
        static_time: Option<PrimitiveDateTime>,
        adaptive_time: Option<PrimitiveDateTime>,
    ) -> ScheduleDecision {
        decide_next_retry(schedule, queried_rung, static_time, adaptive_time)
            .expect("a time was available")
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
        // is no cascading position left and the adaptive algorithm carries it alone.
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

    // ---- rung sourcing ----------------------------------------------------

    #[test]
    fn a_fresh_schedule_starts_the_ladder_at_one() {
        // Nothing consumed yet, so the first position offered is 1. Never 0 — `get_delay`
        // returns `None` for a non-positive index, which would fail the very first decision.
        assert_eq!(StaticLadderProgress::default().consumed_rungs, 0);
        assert_eq!(StaticLadderProgress::default().next_rung(), 1);
    }

    #[test]
    fn stored_rung_advances_by_one() {
        // Independent of `retry_count`, which runs ahead as adaptive retries are inserted.
        // Following it would exhaust a five-entry ladder long before rung 5.
        assert_eq!(at_rung(3).next_rung(), 4);
    }

    // ---- the model decides whenever it has an opinion ---------------------

    #[test]
    fn adaptive_wins_and_leaves_the_rung_unconsumed() {
        let decision = expect_decision(&at_rung(2), 3, Some(at(240)), Some(at(72)));

        assert_eq!(decision.schedule_time, at(72));
        assert_eq!(decision.source, ScheduleSource::Adaptive);
        // Rung 3 was offered but not used, so it is offered again next time.
        assert_eq!(decision.next_progress.consumed_rungs, 2);
        assert_eq!(decision.next_progress.next_rung(), 3);
    }

    #[test]
    fn a_later_adaptive_time_still_wins() {
        // The ladder is not a ceiling: an adaptive day after the static one is taken as it
        // stands. Nothing here compares the two.
        let static_time = at(240);
        let adaptive_time = at(336);
        assert!(adaptive_time.date() > static_time.date());

        let decision = expect_decision(&at_rung(2), 3, Some(static_time), Some(adaptive_time));

        assert_eq!(decision.schedule_time, adaptive_time);
        assert_eq!(decision.source, ScheduleSource::Adaptive);
        assert_eq!(decision.next_progress.consumed_rungs, 2);
    }

    #[test]
    fn an_adaptive_time_on_the_static_day_still_wins() {
        let static_time = at(240 + 18);
        let adaptive_time = at(240 + 9);
        assert_eq!(static_time.date(), adaptive_time.date());

        let decision = expect_decision(
            &StaticLadderProgress::default(),
            1,
            Some(static_time),
            Some(adaptive_time),
        );

        assert_eq!(decision.schedule_time, adaptive_time);
        assert_eq!(decision.source, ScheduleSource::Adaptive);
    }

    #[test]
    fn an_adaptive_time_identical_to_static_still_wins() {
        let decision = expect_decision(
            &StaticLadderProgress::default(),
            1,
            Some(at(240)),
            Some(at(240)),
        );

        assert_eq!(decision.source, ScheduleSource::Adaptive);
    }

    // ---- the ladder covers what the model declines ------------------------

    #[test]
    fn no_adaptive_opinion_uses_static_and_consumes_the_rung() {
        let decision = expect_decision(&StaticLadderProgress::default(), 1, Some(at(240)), None);

        assert_eq!(decision.schedule_time, at(240));
        assert_eq!(decision.source, ScheduleSource::Static);
        assert_eq!(decision.next_progress.consumed_rungs, 1);
    }

    #[test]
    fn an_exhausted_ladder_hands_the_slot_to_adaptive() {
        // Past the ladder's last entry there is no static candidate, but the invoice is not
        // finished — the adaptive algorithm carries it for the rest of the grace window.
        let decision = expect_decision(&at_rung(5), 6, None, Some(at(72)));

        assert_eq!(decision.schedule_time, at(72));
        assert_eq!(decision.source, ScheduleSource::Adaptive);
        // No position to spend, so the count stays where the ladder left it rather than
        // creeping past the end on every adaptive retry.
        assert_eq!(decision.next_progress.consumed_rungs, 5);
    }

    #[test]
    fn both_declining_yields_no_decision() {
        // Nothing left to schedule, and no third source behind these two.
        assert_eq!(decide_next_retry(&at_rung(5), 6, None, None), None);
    }

    // ---- successive decisions ---------------------------------------------

    #[test]
    fn a_rung_survives_repeated_adaptive_wins_and_is_spent_once() {
        // Driven through `next_rung` exactly as the workflow does, so rung sourcing is under
        // test rather than assumed.
        let schedule = StaticLadderProgress::default();

        // Fresh invoice: rung 1 is offered, adaptive takes the slot.
        let queried_rung = schedule.next_rung();
        assert_eq!(queried_rung, 1);
        let first = expect_decision(&schedule, queried_rung, Some(at(240)), Some(at(72)));
        assert_eq!(first.source, ScheduleSource::Adaptive);

        // The process tracker's retry_count is now 2, but rung 1 was never used, so it is
        // offered again.
        let schedule = first.next_progress;
        let queried_rung = schedule.next_rung();
        assert_eq!(queried_rung, 1);
        let second = expect_decision(&schedule, queried_rung, Some(at(240)), Some(at(120)));
        assert_eq!(second.source, ScheduleSource::Adaptive);

        // retry_count 3, and still rung 1. Only a decision the model declines spends it.
        let schedule = second.next_progress;
        let queried_rung = schedule.next_rung();
        assert_eq!(queried_rung, 1);
        let third = expect_decision(&schedule, queried_rung, Some(at(240)), None);
        assert_eq!(third.source, ScheduleSource::Static);
        assert_eq!(third.next_progress.consumed_rungs, 1);

        // Three attempts in, exactly one static position spent — the next query is rung 2,
        // not rung 4 as `retry_count` alone would have given.
        assert_eq!(third.next_progress.next_rung(), 2);
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
