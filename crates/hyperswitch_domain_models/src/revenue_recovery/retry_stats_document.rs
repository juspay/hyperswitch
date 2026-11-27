use router_env::logger;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use time::PrimitiveDateTime;

/// Success counter for a single slot. Invariant: `k <= n` — every successful retry
/// is also counted as a retry, so the success count can never exceed the total.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotCounter {
    /// `n` = total number of retries recorded in this slot.
    #[serde(default)]
    pub n: u64,
    /// `k` = number of those retries that succeeded (`k <= n`).
    #[serde(default)]
    pub k: u64,
}

impl SlotCounter {
    pub fn record(&mut self, success: bool) {
        self.n = self.n.saturating_add(1);
        if success {
            self.k = self.k.saturating_add(1);
        }
    }
}

/// Stats document persisted per key. Each slot family holds a fixed number
/// of buckets whose count is statically fixed by the [`SlotFamily`] associated
/// constants, so `dow`/`dom`/`hod`/`delta_hours` can never carry the wrong number of items.
#[serde_as]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatsDocument {
    /// `dow` = day of week: one bucket per weekday (Monday..=Sunday).
    #[serde(default)]
    pub dow: [SlotCounter; SlotFamily::DOW_SLOT_COUNT],
    /// `dom` = day of month: one bucket per calendar day (1..=31).
    #[serde(default)]
    pub dom: [SlotCounter; SlotFamily::DOM_SLOT_COUNT],
    /// `hod` = hour of day: one bucket per hour (0..=23).
    #[serde(default)]
    pub hod: [SlotCounter; SlotFamily::HOD_SLOT_COUNT],
    /// `delta_hours` = whole hours from the previous failure to the retry: one bucket per
    /// hour (0..=719). Recorded only when the previous failure's time is known and under 720
    /// hours old, so its totals can be lower than the other families'.
    #[serde(default = "empty_delta_hours")]
    #[serde_as(as = "[_; SlotFamily::DELTA_HOURS_SLOT_COUNT]")]
    pub delta_hours: [SlotCounter; SlotFamily::DELTA_HOURS_SLOT_COUNT],
}

fn empty_delta_hours() -> [SlotCounter; SlotFamily::DELTA_HOURS_SLOT_COUNT] {
    [SlotCounter::default(); SlotFamily::DELTA_HOURS_SLOT_COUNT]
}

impl Default for StatsDocument {
    fn default() -> Self {
        Self {
            dow: Default::default(),
            dom: Default::default(),
            hod: Default::default(),
            delta_hours: empty_delta_hours(),
        }
    }
}

impl StatsDocument {
    pub fn from_json(value: &serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(value.clone()).map_err(|error| {
            logger::error!(
                ?error,
                "revenue_recovery_retry_stats: failed to deserialize stats document"
            );
            error
        })
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_else(|error| {
            logger::error!(
                ?error,
                "revenue_recovery_retry_stats: failed to serialize stats document"
            );
            serde_json::json!({})
        })
    }

    /// Verify the per-document invariants: every slot has `k <= n`, both the
    /// retries total (`Σn`) and the successes total (`Σk`) agree
    /// across `dow`/`dom`/`hod`, since each is an independent marginal of the
    /// same events, and `delta_hours` totals do not exceed them. The merge path maintains these
    /// by construction; run this on any externally-sourced document (e.g. a migration CSV) that
    /// bypasses it.
    pub fn validate_invariants(&self) -> Result<(), String> {
        let retries = |family: &[SlotCounter]| family.iter().map(|slot| slot.n).sum::<u64>();
        let successes = |family: &[SlotCounter]| family.iter().map(|slot| slot.k).sum::<u64>();

        let every_slot_valid = self
            .dow
            .iter()
            .chain(self.dom.iter())
            .chain(self.hod.iter())
            .chain(self.delta_hours.iter())
            .all(|slot| slot.k <= slot.n);

        let (dow_n, dom_n, hod_n) = (retries(&self.dow), retries(&self.dom), retries(&self.hod));
        let (dow_k, dom_k, hod_k) = (
            successes(&self.dow),
            successes(&self.dom),
            successes(&self.hod),
        );
        let (delta_n, delta_k) = (retries(&self.delta_hours), successes(&self.delta_hours));

        if !every_slot_valid {
            Err("slot counter has k>n".to_string())
        } else if dow_n != dom_n || dom_n != hod_n {
            Err(format!(
                "n totals differ across slot families (dow={dow_n}, dom={dom_n}, hod={hod_n})"
            ))
        } else if dow_k != dom_k || dom_k != hod_k {
            Err(format!(
                "k totals differ across slot families (dow={dow_k}, dom={dom_k}, hod={hod_k})"
            ))
        } else if delta_n > dow_n || delta_k > dow_k {
            Err(format!(
                "delta_hours totals exceed the other families' (delta_hours n={delta_n} k={delta_k}, \
                 dow n={dow_n} k={dow_k})"
            ))
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventSlots {
    /// `dow` = day of week: bucket index 0..=6 (Monday..=Sunday).
    pub dow: u8,
    /// `dom` = day of month: bucket index 0..=30 (calendar day 1..=31 minus 1).
    pub dom: u8,
    /// `hod` = hour of day: bucket index 0..=23.
    pub hod: u8,
    /// `delta_hours` = bucket index 0..=719, or `None` when the previous failure's time is
    /// unknown, later than the retry, or 720 or more hours earlier.
    pub delta_hours: Option<u16>,
}

impl EventSlots {
    /// Derive the day-of-week / day-of-month / hour-of-day bucket indices from an
    /// event's `created_at`, and the hours-since-previous-failure bucket from
    /// `previous_failure_at`.
    ///
    /// Hyperswitch persists `created_at` as a **UTC** `PrimitiveDateTime`, but that
    /// type carries no offset, so we make the timezone explicit via `assume_utc()`.
    pub fn from_utc(ts: PrimitiveDateTime, previous_failure_at: Option<PrimitiveDateTime>) -> Self {
        let utc = ts.assume_utc();
        Self {
            dow: validate_slot(SlotFamily::Dow, utc.weekday().number_days_from_monday(), ts),
            dom: validate_slot(SlotFamily::Dom, utc.day().saturating_sub(1), ts),
            hod: validate_slot(SlotFamily::Hod, utc.hour(), ts),
            delta_hours: previous_failure_at
                .and_then(|previous_failure_at| delta_hours_slot(previous_failure_at, ts)),
        }
    }
}

/// Whole hours from `previous_failure_at` to `retry_at`, when that is 0..=719.
fn delta_hours_slot(
    previous_failure_at: PrimitiveDateTime,
    retry_at: PrimitiveDateTime,
) -> Option<u16> {
    let since_failure = retry_at - previous_failure_at;
    if since_failure.is_negative() {
        logger::warn!(
            %previous_failure_at,
            %retry_at,
            "revenue_recovery_retry_stats: previous failure is later than the retry; \
             skipping the delta_hours update"
        );
        return None;
    }
    u16::try_from(since_failure.whole_hours())
        .ok()
        .filter(|slot| usize::from(*slot) < SlotFamily::DELTA_HOURS_SLOT_COUNT)
}

/// Guard a derived slot index against its family's static bucket count. Returns the
/// index unchanged, but logs an error for any out-of-range value so the anomaly is
/// visible
fn validate_slot(family: SlotFamily, slot: u8, ts: PrimitiveDateTime) -> u8 {
    if usize::from(slot) >= family.slot_count() {
        logger::error!(
            slot_family = family.name(),
            slot,
            bucket_count = family.slot_count(),
            timestamp = %ts,
            "revenue_recovery_retry_stats: derived slot index is out of range for its family; \
             dropping this family's update (possible timezone/timestamp anomaly)"
        );
    }
    slot
}

#[derive(Clone, Copy, Debug)]
pub struct SlotUpdate {
    pub slot: u16,
    pub success: bool,
}

#[derive(Clone, Debug)]
pub struct StatsDelta {
    pub updates: Vec<(SlotFamily, SlotUpdate)>,
}

impl StatsDelta {
    pub fn for_event(slots: EventSlots, success: bool) -> Self {
        let calendar = [
            (SlotFamily::Dow, slots.dow),
            (SlotFamily::Dom, slots.dom),
            (SlotFamily::Hod, slots.hod),
        ]
        .into_iter()
        .map(|(family, slot)| (family, u16::from(slot)));
        let delta_hours = slots.delta_hours.map(|slot| (SlotFamily::DeltaHours, slot));

        Self {
            updates: calendar
                .chain(delta_hours)
                .map(|(family, slot)| (family, SlotUpdate { slot, success }))
                .collect(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SlotFamily {
    Dow,
    Dom,
    Hod,
    DeltaHours,
}

impl SlotFamily {
    /// Day-of-week buckets (Monday..=Sunday).
    pub const DOW_SLOT_COUNT: usize = 7;
    /// Day-of-month buckets (day 1..=31 mapped to index 0..=30).
    pub const DOM_SLOT_COUNT: usize = 31;
    /// Hour-of-day buckets (0..=23).
    pub const HOD_SLOT_COUNT: usize = 24;
    /// Hours-since-previous-failure buckets (0..=719, i.e. 30 days).
    pub const DELTA_HOURS_SLOT_COUNT: usize = 720;

    /// The statically-known number of buckets for this family.
    pub const fn slot_count(self) -> usize {
        match self {
            Self::Dow => Self::DOW_SLOT_COUNT,
            Self::Dom => Self::DOM_SLOT_COUNT,
            Self::Hod => Self::HOD_SLOT_COUNT,
            Self::DeltaHours => Self::DELTA_HOURS_SLOT_COUNT,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Dow => "dow",
            Self::Dom => "dom",
            Self::Hod => "hod",
            Self::DeltaHours => "delta_hours",
        }
    }
}

impl StatsDocument {
    /// Fold a [`StatsDelta`] into this document, returning the updated document. Takes
    /// `self` by value so calls chain fluently (`doc.merge(&d1).merge(&d2)`); start
    /// from `StatsDocument::default()` when there is no existing document.
    pub fn merge(mut self, delta: &StatsDelta) -> Self {
        for (family, update) in &delta.updates {
            let slots: &mut [SlotCounter] = match family {
                SlotFamily::Dow => &mut self.dow,
                SlotFamily::Dom => &mut self.dom,
                SlotFamily::Hod => &mut self.hod,
                SlotFamily::DeltaHours => &mut self.delta_hours,
            };
            // Out-of-range slots are dropped rather than panicking; valid time-derived
            // slots always fall within the statically-sized bucket range.
            if let Some(counter) = slots.get_mut(usize::from(update.slot)) {
                counter.record(update.success);
            }
        }
        self
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn delta(slots: &[(SlotFamily, u16, bool)]) -> StatsDelta {
        StatsDelta {
            updates: slots
                .iter()
                .map(|(f, s, ok)| {
                    (
                        *f,
                        SlotUpdate {
                            slot: *s,
                            success: *ok,
                        },
                    )
                })
                .collect(),
        }
    }

    fn is_empty(slots: &[SlotCounter]) -> bool {
        slots.iter().all(|c| *c == SlotCounter::default())
    }

    #[test]
    fn from_utc_produces_in_range_slots() {
        let date = time::Date::from_calendar_date(2026, time::Month::August, 19)
            .expect("valid calendar date");
        let clock = time::Time::from_hms(13, 30, 0).expect("valid time");
        let slots = EventSlots::from_utc(PrimitiveDateTime::new(date, clock), None);

        assert!(usize::from(slots.dow) < SlotFamily::DOW_SLOT_COUNT);
        assert!(usize::from(slots.dom) < SlotFamily::DOM_SLOT_COUNT);
        assert!(usize::from(slots.hod) < SlotFamily::HOD_SLOT_COUNT);
        // 2026-08-19 is a Wednesday (index 2); day 19 -> index 18; hour 13.
        assert_eq!(slots.dow, 2);
        assert_eq!(slots.dom, 18);
        assert_eq!(slots.hod, 13);
        assert_eq!(slots.delta_hours, None);
    }

    fn at(day: u8, hour: u8, minute: u8) -> PrimitiveDateTime {
        let date = time::Date::from_calendar_date(2026, time::Month::November, day)
            .expect("valid calendar date");
        PrimitiveDateTime::new(
            date,
            time::Time::from_hms(hour, minute, 0).expect("valid time"),
        )
    }

    #[test]
    fn delta_hours_slot_is_whole_hours_since_the_previous_failure() {
        let failed_at = at(1, 10, 0);
        let slot = |retry_at| EventSlots::from_utc(retry_at, Some(failed_at)).delta_hours;

        assert_eq!(slot(at(1, 10, 0)), Some(0));
        assert_eq!(slot(at(1, 10, 59)), Some(0));
        assert_eq!(slot(at(3, 10, 0)), Some(48));
        // 719 h 59 min is still the last bucket; 720 h is past it.
        assert_eq!(
            slot(at(1, 10, 0) + time::Duration::minutes(719 * 60 + 59)),
            Some(719)
        );
        assert_eq!(slot(at(1, 10, 0) + time::Duration::hours(720)), None);
        // A previous failure later than the retry has no bucket, even under an hour later.
        assert_eq!(slot(at(1, 9, 0)), None);
        assert_eq!(slot(at(1, 9, 30)), None);
        assert_eq!(EventSlots::from_utc(at(3, 10, 0), None).delta_hours, None);
    }

    #[test]
    fn delta_hours_is_updated_only_when_its_slot_is_known() {
        let families = |previous_failure_at| {
            StatsDelta::for_event(
                EventSlots::from_utc(at(3, 10, 0), previous_failure_at),
                true,
            )
            .updates
            .iter()
            .map(|(family, update)| (*family, update.slot))
            .collect::<Vec<_>>()
        };

        assert!(families(Some(at(1, 10, 0))).contains(&(SlotFamily::DeltaHours, 48)));
        assert!(!families(None)
            .iter()
            .any(|(family, _)| *family == SlotFamily::DeltaHours));
        assert_eq!(families(None).len(), 3);
    }

    #[test]
    fn slot_arrays_have_statically_fixed_lengths() {
        let doc = StatsDocument::default();
        assert_eq!(doc.dow.len(), SlotFamily::DOW_SLOT_COUNT);
        assert_eq!(doc.dom.len(), SlotFamily::DOM_SLOT_COUNT);
        assert_eq!(doc.hod.len(), SlotFamily::HOD_SLOT_COUNT);
        assert_eq!(doc.delta_hours.len(), SlotFamily::DELTA_HOURS_SLOT_COUNT);
    }

    #[test]
    fn documents_stored_before_delta_hours_still_read() {
        let stored = serde_json::json!({
            "dow": [{"n": 2, "k": 1}, {}, {}, {}, {}, {}, {}],
            "dom": std::iter::once(serde_json::json!({"n": 2, "k": 1}))
                .chain(std::iter::repeat_n(serde_json::json!({}), 30))
                .collect::<Vec<_>>(),
            "hod": std::iter::once(serde_json::json!({"n": 2, "k": 1}))
                .chain(std::iter::repeat_n(serde_json::json!({}), 23))
                .collect::<Vec<_>>(),
        });

        let doc = StatsDocument::from_json(&stored).expect("old document reads");

        assert_eq!(doc.dow[0], SlotCounter { n: 2, k: 1 });
        assert!(is_empty(&doc.delta_hours));
        assert_eq!(doc.validate_invariants(), Ok(()));
    }

    #[test]
    fn delta_hours_round_trips_through_json() {
        let doc = StatsDocument::default().merge(&StatsDelta::for_event(
            EventSlots::from_utc(at(3, 10, 0), Some(at(1, 10, 0))),
            true,
        ));

        let read_back = StatsDocument::from_json(&doc.to_json()).expect("document reads back");

        assert_eq!(read_back, doc);
        assert_eq!(read_back.delta_hours[48], SlotCounter { n: 1, k: 1 });
    }

    #[test]
    fn delta_hours_totals_may_not_exceed_the_other_families() {
        let calendar_only = StatsDelta::for_event(EventSlots::from_utc(at(3, 10, 0), None), false);
        let with_delta =
            StatsDelta::for_event(EventSlots::from_utc(at(3, 10, 0), Some(at(1, 10, 0))), true);
        let doc = StatsDocument::default()
            .merge(&calendar_only)
            .merge(&with_delta);
        assert_eq!(doc.validate_invariants(), Ok(()));

        let mut too_many = doc.clone();
        too_many.delta_hours[5] = SlotCounter { n: 2, k: 0 };
        assert!(too_many.validate_invariants().is_err());

        let mut k_above_n = doc;
        k_above_n.delta_hours[48] = SlotCounter { n: 1, k: 2 };
        assert!(k_above_n.validate_invariants().is_err());
    }

    #[test]
    fn merge_into_empty_creates_entries() {
        let doc = StatsDocument::default().merge(&delta(&[(SlotFamily::Dow, 3, true)]));
        assert_eq!(doc.dow[3], SlotCounter { n: 1, k: 1 });
        assert!(is_empty(&doc.dom));
        assert!(is_empty(&doc.hod));
        assert!(is_empty(&doc.delta_hours));
    }

    #[test]
    fn merge_accumulates_across_calls() {
        let d1 = delta(&[(SlotFamily::Hod, 9, true)]);
        let d2 = delta(&[(SlotFamily::Hod, 9, false)]);
        let d3 = delta(&[(SlotFamily::Hod, 10, true)]);
        let doc = StatsDocument::default().merge(&d1).merge(&d2).merge(&d3);
        assert_eq!(doc.hod[9], SlotCounter { n: 2, k: 1 });
        assert_eq!(doc.hod[10], SlotCounter { n: 1, k: 1 });
    }

    #[test]
    fn merge_is_associative_over_deltas() {
        let d1 = delta(&[(SlotFamily::Dom, 9, true)]);
        let d2 = delta(&[(SlotFamily::Dom, 9, false)]);
        let d3 = delta(&[(SlotFamily::Dom, 9, true)]);
        let left = StatsDocument::default().merge(&d1).merge(&d2).merge(&d3);
        let mut combined = StatsDelta { updates: vec![] };
        combined.updates.extend(d2.updates.clone());
        combined.updates.extend(d3.updates.clone());
        let right = StatsDocument::default().merge(&d1).merge(&combined);
        assert_eq!(left.dom[9], right.dom[9]);
    }

    #[test]
    fn out_of_range_slot_is_ignored() {
        // day-of-month index 31 is out of range (valid 0..=30) and must not panic.
        let doc = StatsDocument::default().merge(&delta(&[(SlotFamily::Dom, 31, true)]));
        assert!(is_empty(&doc.dom));
        let doc = StatsDocument::default().merge(&delta(&[(SlotFamily::DeltaHours, 720, true)]));
        assert!(is_empty(&doc.delta_hours));
    }

    #[test]
    fn marginal_consistency_holds() {
        let d = delta(&[
            (SlotFamily::Dow, 1, true),
            (SlotFamily::Dom, 5, true),
            (SlotFamily::Hod, 9, true),
        ]);
        let doc = StatsDocument::default().merge(&d);
        let sum = |m: &[SlotCounter]| m.iter().map(|c| c.n).sum::<u64>();
        assert_eq!(sum(&doc.dow), sum(&doc.dom));
        assert_eq!(sum(&doc.dom), sum(&doc.hod));
    }
}
