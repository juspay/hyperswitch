use time::PrimitiveDateTime;


#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RecoverySchedule {
    /// Static ladder positions consumed so far.
    ///
    /// Deliberately **not** `process_tracker.retry_count`, which counts every attempt with
    /// adaptive insertions included: the ladder has far fewer entries than the invoice has
    /// attempts, so feeding it `retry_count` would run off the end early.
    ///
    /// Defaults to `0` — including on rows written before this field existed — so the ladder
    /// always starts at position 1.
    #[serde(default)]
    pub static_rung: i32,
}

impl RecoverySchedule {
    /// Ladder position to query for this decision.
    pub fn next_rung(&self) -> i32 {
        self.static_rung + 1
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
    pub next_schedule: RecoverySchedule,
    /// Which algorithm won, for logging and analytics.
    pub source: ScheduleSource,
}

pub fn decide_next_retry(
    schedule: &RecoverySchedule,
    queried_rung: i32,
    static_time: PrimitiveDateTime,
    adaptive_time: Option<PrimitiveDateTime>,
) -> ScheduleDecision {
    let winning_adaptive_time =
        adaptive_time.filter(|adaptive_time| adaptive_time.date() < static_time.date());

    match winning_adaptive_time {
        Some(adaptive_time) => ScheduleDecision {
            schedule_time: adaptive_time,
            next_schedule: RecoverySchedule {
                // No static attempt was spent, so the ladder must not advance — the same
                // position is offered again on the next decision.
                static_rung: schedule.static_rung,
            },
            source: ScheduleSource::Adaptive,
        },
        None => ScheduleDecision {
            schedule_time: static_time,
            next_schedule: RecoverySchedule {
                static_rung: queried_rung,
            },
            source: ScheduleSource::Static,
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

    fn at_rung(static_rung: i32) -> RecoverySchedule {
        RecoverySchedule { static_rung }
    }

    // ---- rung sourcing ----------------------------------------------------

    #[test]
    fn a_fresh_schedule_starts_the_ladder_at_one() {
        // Nothing consumed yet, so the first position offered is 1. Never 0 — `get_delay`
        // returns `None` for a non-positive index, which would fail the very first decision.
        assert_eq!(RecoverySchedule::default().static_rung, 0);
        assert_eq!(RecoverySchedule::default().next_rung(), 1);
    }

    #[test]
    fn stored_rung_advances_by_one() {
        // Independent of `retry_count`, which runs ahead as adaptive retries are inserted.
        // Following it would exhaust a five-entry ladder long before rung 5.
        assert_eq!(at_rung(3).next_rung(), 4);
    }

    // ---- adaptive wins ----------------------------------------------------

    #[test]
    fn adaptive_earlier_day_wins_and_leaves_the_rung_unconsumed() {
        let schedule = at_rung(2);
        let decision = decide_next_retry(&schedule, 3, at(240), Some(at(72)));

        assert_eq!(decision.schedule_time, at(72));
        assert_eq!(decision.source, ScheduleSource::Adaptive);
        // Rung 3 was offered but not used, so it is offered again next time.
        assert_eq!(decision.next_schedule.static_rung, 2);
        assert_eq!(decision.next_schedule.next_rung(), 3);
    }

    #[test]
    fn adaptive_the_day_before_static_wins() {
        let static_time = at(240);
        let adaptive_time = at(240 - 1);
        assert!(adaptive_time.date() < static_time.date());

        let decision = decide_next_retry(
            &RecoverySchedule::default(),
            1,
            static_time,
            Some(adaptive_time),
        );

        assert_eq!(decision.schedule_time, adaptive_time);
        assert_eq!(decision.source, ScheduleSource::Adaptive);
    }

    // ---- static wins ------------------------------------------------------

    #[test]
    fn adaptive_later_day_loses_and_consumes_the_rung() {
        let decision = decide_next_retry(&at_rung(2), 3, at(240), Some(at(336)));

        assert_eq!(decision.schedule_time, at(240));
        assert_eq!(decision.source, ScheduleSource::Static);
        assert_eq!(decision.next_schedule.static_rung, 3);
    }

    #[test]
    fn no_adaptive_opinion_uses_static_and_consumes_the_rung() {
        let decision = decide_next_retry(&RecoverySchedule::default(), 1, at(240), None);

        assert_eq!(decision.schedule_time, at(240));
        assert_eq!(decision.source, ScheduleSource::Static);
        assert_eq!(decision.next_schedule.static_rung, 1);
    }

    // ---- ties go to static ------------------------------------------------

    #[test]
    fn adaptive_earlier_on_the_same_day_still_loses() {
        // Static on day 10 at 18:00, adaptive on day 10 at 09:00. Static already covers that
        // day, so the earlier instant does not earn a second attempt on it.
        let static_time = at(240 + 18);
        let adaptive_time = at(240 + 9);
        assert_eq!(static_time.date(), adaptive_time.date());

        let decision = decide_next_retry(
            &RecoverySchedule::default(),
            1,
            static_time,
            Some(adaptive_time),
        );

        assert_eq!(decision.schedule_time, static_time);
        assert_eq!(decision.source, ScheduleSource::Static);
        assert_eq!(decision.next_schedule.static_rung, 1);
    }

    #[test]
    fn adaptive_identical_to_static_loses() {
        let decision =
            decide_next_retry(&RecoverySchedule::default(), 1, at(240), Some(at(240)));

        assert_eq!(decision.source, ScheduleSource::Static);
    }

    // ---- successive decisions ---------------------------------------------

    #[test]
    fn a_rung_survives_repeated_adaptive_wins_and_is_spent_once() {
        // Driven through `next_rung` exactly as the workflow does, so rung sourcing is under
        // test rather than assumed.
        let schedule = RecoverySchedule::default();

        // Fresh invoice: rung 1 is offered, adaptive takes the slot.
        let queried_rung = schedule.next_rung();
        assert_eq!(queried_rung, 1);
        let first = decide_next_retry(&schedule, queried_rung, at(240), Some(at(72)));
        assert_eq!(first.source, ScheduleSource::Adaptive);

        // The process tracker's retry_count is now 2, but rung 1 was never used, so it is
        // offered again.
        let schedule = first.next_schedule;
        let queried_rung = schedule.next_rung();
        assert_eq!(queried_rung, 1);
        let second = decide_next_retry(&schedule, queried_rung, at(240), Some(at(120)));
        assert_eq!(second.source, ScheduleSource::Adaptive);

        // retry_count 3, and still rung 1.
        let schedule = second.next_schedule;
        let queried_rung = schedule.next_rung();
        assert_eq!(queried_rung, 1);
        let third = decide_next_retry(&schedule, queried_rung, at(240), None);
        assert_eq!(third.source, ScheduleSource::Static);
        assert_eq!(third.next_schedule.static_rung, 1);

        // Three attempts in, exactly one static position spent — the next query is rung 2,
        // not rung 4 as `retry_count` alone would have given.
        assert_eq!(third.next_schedule.next_rung(), 2);
    }

    // ---- persistence ------------------------------------------------------

    #[test]
    fn round_trips_through_json_and_absent_fields_default() {
        let schedule = at_rung(3);
        let encoded = serde_json::to_value(&schedule).expect("serialises");
        let decoded: RecoverySchedule = serde_json::from_value(encoded).expect("deserialises");
        assert_eq!(decoded, schedule);

        // Invoices already in flight have no `recovery_schedule` at all.
        let empty: RecoverySchedule = serde_json::from_value(serde_json::json!({}))
            .expect("absent fields fall back to defaults");
        assert_eq!(empty, RecoverySchedule::default());
        assert_eq!(empty.static_rung, 0);
        // …and therefore resume at whatever the ladder was already indexed by.
        assert_eq!(empty.next_rung(), 1);
    }
}
