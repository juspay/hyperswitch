use router_env::logger;
use serde::{Deserialize, Serialize};
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
/// constants, so `dow`/`dom`/`hod` can never carry the wrong number of items.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
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

    /// Verify the per-document invariants: every slot has `k <= n`, and both the
    /// retries total (`Σn`) and the successes total (`Σk`) agree
    /// across the three slot families, since each family is an independent marginal of the
    /// same events. The merge path maintains these by construction; run this on any
    /// externally-sourced document (e.g. a migration CSV) that bypasses it.
    pub fn validate_invariants(&self) -> Result<(), String> {
        let retries = |family: &[SlotCounter]| family.iter().map(|slot| slot.n).sum::<u64>();
        let successes = |family: &[SlotCounter]| family.iter().map(|slot| slot.k).sum::<u64>();

        let every_slot_valid = self
            .dow
            .iter()
            .chain(self.dom.iter())
            .chain(self.hod.iter())
            .all(|slot| slot.k <= slot.n);

        let (dow_n, dom_n, hod_n) = (retries(&self.dow), retries(&self.dom), retries(&self.hod));
        let (dow_k, dom_k, hod_k) = (
            successes(&self.dow),
            successes(&self.dom),
            successes(&self.hod),
        );

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
}

impl EventSlots {
    /// Derive the day-of-week / day-of-month / hour-of-day bucket indices from an
    /// event's `created_at`.
    ///
    /// Hyperswitch persists `created_at` as a **UTC** `PrimitiveDateTime`, but that
    /// type carries no offset, so we make the timezone explicit via `assume_utc()`.
    pub fn from_utc(ts: PrimitiveDateTime) -> Self {
        let utc = ts.assume_utc();
        Self {
            dow: validate_slot(SlotFamily::Dow, utc.weekday().number_days_from_monday(), ts),
            dom: validate_slot(SlotFamily::Dom, utc.day().saturating_sub(1), ts),
            hod: validate_slot(SlotFamily::Hod, utc.hour(), ts),
        }
    }
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
    pub slot: u8,
    pub success: bool,
}

#[derive(Clone, Debug)]
pub struct StatsDelta {
    pub updates: Vec<(SlotFamily, SlotUpdate)>,
}

impl StatsDelta {
    pub fn for_event(slots: EventSlots, success: bool) -> Self {
        Self {
            updates: vec![
                (
                    SlotFamily::Dow,
                    SlotUpdate {
                        slot: slots.dow,
                        success,
                    },
                ),
                (
                    SlotFamily::Dom,
                    SlotUpdate {
                        slot: slots.dom,
                        success,
                    },
                ),
                (
                    SlotFamily::Hod,
                    SlotUpdate {
                        slot: slots.hod,
                        success,
                    },
                ),
            ],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SlotFamily {
    Dow,
    Dom,
    Hod,
}

impl SlotFamily {
    /// Day-of-week buckets (Monday..=Sunday).
    pub const DOW_SLOT_COUNT: usize = 7;
    /// Day-of-month buckets (day 1..=31 mapped to index 0..=30).
    pub const DOM_SLOT_COUNT: usize = 31;
    /// Hour-of-day buckets (0..=23).
    pub const HOD_SLOT_COUNT: usize = 24;

    /// The statically-known number of buckets for this family.
    pub const fn slot_count(self) -> usize {
        match self {
            Self::Dow => Self::DOW_SLOT_COUNT,
            Self::Dom => Self::DOM_SLOT_COUNT,
            Self::Hod => Self::HOD_SLOT_COUNT,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Dow => "dow",
            Self::Dom => "dom",
            Self::Hod => "hod",
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

    fn delta(slots: &[(SlotFamily, u8, bool)]) -> StatsDelta {
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
        let slots = EventSlots::from_utc(PrimitiveDateTime::new(date, clock));

        assert!(usize::from(slots.dow) < SlotFamily::DOW_SLOT_COUNT);
        assert!(usize::from(slots.dom) < SlotFamily::DOM_SLOT_COUNT);
        assert!(usize::from(slots.hod) < SlotFamily::HOD_SLOT_COUNT);
        // 2026-08-19 is a Wednesday (index 2); day 19 -> index 18; hour 13.
        assert_eq!(slots.dow, 2);
        assert_eq!(slots.dom, 18);
        assert_eq!(slots.hod, 13);
    }

    #[test]
    fn slot_arrays_have_statically_fixed_lengths() {
        let doc = StatsDocument::default();
        assert_eq!(doc.dow.len(), SlotFamily::DOW_SLOT_COUNT);
        assert_eq!(doc.dom.len(), SlotFamily::DOM_SLOT_COUNT);
        assert_eq!(doc.hod.len(), SlotFamily::HOD_SLOT_COUNT);
    }

    #[test]
    fn merge_into_empty_creates_entries() {
        let doc = StatsDocument::default().merge(&delta(&[(SlotFamily::Dow, 3, true)]));
        assert_eq!(doc.dow[3], SlotCounter { n: 1, k: 1 });
        assert!(is_empty(&doc.dom));
        assert!(is_empty(&doc.hod));
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
