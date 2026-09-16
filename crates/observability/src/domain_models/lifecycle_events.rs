//! Validated alert lifecycle models and layer conversions.

use api_models::observability::alert_manager::lifecycle_events as api;
use diesel_models::observability::alert_manager::lifecycle_events as storage;
use error_stack::{report, ResultExt};
use time::{Duration, PrimitiveDateTime};

use crate::errors::{ObservabilityApiResult, ObservabilityError};

#[derive(Clone, Debug)]
pub struct LifecycleEvent {
    pub alert_key: String,
    pub detector: String,
    pub merchant_id: String,
    pub profile_id: String,
    pub state: String,
    pub first_seen: PrimitiveDateTime,
    pub last_seen: PrimitiveDateTime,
    pub recovered_at: PrimitiveDateTime,
    pub runs: i64,
    pub severity: String,
    pub sr: f64,
    pub failed: i64,
    pub total: i64,
    pub connector: String,
    pub notified_at: PrimitiveDateTime,
    pub ts_slack: String,
    pub sent: bool,
    pub last_updated_at: PrimitiveDateTime,
}

pub struct LifecycleEventsBatch {
    pub events: Vec<LifecycleEvent>,
    pub retention_cutoff: PrimitiveDateTime,
}

impl LifecycleEventsBatch {
    pub fn try_from_request(
        request: api::LifecycleEventsBatchRequest,
        now: PrimitiveDateTime,
    ) -> ObservabilityApiResult<Self> {
        if !matches!(request.version_bump_seconds, 0 | 1) {
            return Err(report!(ObservabilityError::InvalidRequest))
                .attach_printable("version_bump_seconds must be 0 or 1");
        }
        let last_updated_at = now + Duration::seconds(request.version_bump_seconds);
        let events = request
            .events
            .into_iter()
            .map(|event| LifecycleEvent::try_from_request(event, last_updated_at))
            .collect::<ObservabilityApiResult<Vec<_>>>()?;

        Ok(Self {
            events,
            retention_cutoff: now - Duration::days(90),
        })
    }
}

impl LifecycleEvent {
    fn try_from_request(
        event: api::LifecycleEventRequest,
        last_updated_at: PrimitiveDateTime,
    ) -> ObservabilityApiResult<Self> {
        if event.alert_key.chars().count() != 32 {
            return Err(report!(ObservabilityError::InvalidRequest))
                .attach_printable("alert_key must contain exactly 32 characters");
        }
        if !matches!(event.state.as_str(), "firing" | "recovered") {
            return Err(report!(ObservabilityError::InvalidRequest))
                .attach_printable("state must be firing or recovered");
        }
        if event.runs < 0 || event.failed < 0 || event.total < 0 {
            return Err(report!(ObservabilityError::InvalidRequest))
                .attach_printable("lifecycle counters must not be negative");
        }
        if !event.sr.is_finite() {
            return Err(report!(ObservabilityError::InvalidRequest))
                .attach_printable("sr must be finite");
        }

        Ok(Self {
            alert_key: event.alert_key,
            detector: event.detector,
            merchant_id: event.merchant_id,
            profile_id: event.profile_id,
            state: event.state,
            first_seen: event.first_seen,
            last_seen: event.last_seen,
            recovered_at: event.recovered_at,
            runs: event.runs,
            severity: event.severity,
            sr: event.sr,
            failed: event.failed,
            total: event.total,
            connector: event.connector,
            notified_at: event.notified_at,
            ts_slack: event.ts_slack,
            sent: event.sent,
            last_updated_at,
        })
    }
}

impl From<LifecycleEvent> for storage::LifecycleEventNew {
    fn from(event: LifecycleEvent) -> Self {
        Self {
            alert_key: event.alert_key,
            detector: event.detector,
            merchant_id: event.merchant_id,
            profile_id: event.profile_id,
            state: event.state,
            first_seen: event.first_seen,
            last_seen: event.last_seen,
            recovered_at: event.recovered_at,
            runs: event.runs,
            severity: event.severity,
            sr: event.sr,
            failed: event.failed,
            total: event.total,
            connector: event.connector,
            notified_at: event.notified_at,
            ts_slack: event.ts_slack,
            sent: event.sent,
            last_updated_at: event.last_updated_at,
        }
    }
}

impl From<storage::LifecycleEvent> for LifecycleEvent {
    fn from(event: storage::LifecycleEvent) -> Self {
        Self {
            alert_key: event.alert_key,
            detector: event.detector,
            merchant_id: event.merchant_id,
            profile_id: event.profile_id,
            state: event.state,
            first_seen: event.first_seen,
            last_seen: event.last_seen,
            recovered_at: event.recovered_at,
            runs: event.runs,
            severity: event.severity,
            sr: event.sr,
            failed: event.failed,
            total: event.total,
            connector: event.connector,
            notified_at: event.notified_at,
            ts_slack: event.ts_slack,
            sent: event.sent,
            last_updated_at: event.last_updated_at,
        }
    }
}

impl From<LifecycleEvent> for api::LifecycleEventResponse {
    fn from(event: LifecycleEvent) -> Self {
        Self {
            alert_key: event.alert_key,
            detector: event.detector,
            merchant_id: event.merchant_id,
            profile_id: event.profile_id,
            state: event.state,
            first_seen: event.first_seen,
            last_seen: event.last_seen,
            recovered_at: event.recovered_at,
            runs: event.runs,
            severity: event.severity,
            sr: event.sr,
            failed: event.failed,
            total: event.total,
            connector: event.connector,
            notified_at: event.notified_at,
            ts_slack: event.ts_slack,
            sent: event.sent,
            last_updated_at: event.last_updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use time::macros::datetime;

    use super::*;

    fn request() -> api::LifecycleEventRequest {
        api::LifecycleEventRequest {
            alert_key: "0123456789abcdef0123456789abcdef".into(),
            detector: "webhook_rejected".into(),
            merchant_id: "m1".into(),
            profile_id: String::new(),
            state: "firing".into(),
            first_seen: datetime!(2026-09-15 09:00),
            last_seen: datetime!(2026-09-15 09:45),
            recovered_at: datetime!(1970-01-01 00:00),
            runs: 4,
            severity: "critical".into(),
            sr: 0.0,
            failed: 10,
            total: 10,
            connector: String::new(),
            notified_at: datetime!(2026-09-15 09:45),
            ts_slack: String::new(),
            sent: false,
        }
    }

    #[test]
    fn validates_batch_and_applies_fixed_times() {
        let now = datetime!(2026-09-15 09:45);
        let batch = LifecycleEventsBatch::try_from_request(
            api::LifecycleEventsBatchRequest {
                events: vec![request()],
                version_bump_seconds: 1,
            },
            now,
        )
        .unwrap();
        assert_eq!(
            batch.events.first().map(|event| event.last_updated_at),
            Some(now + Duration::seconds(1))
        );
        assert_eq!(batch.retention_cutoff, datetime!(2026-06-17 09:45));
    }

    #[test]
    fn rejects_invalid_identity_state_counters_and_version() {
        for mutate in 0..3 {
            let mut event = request();
            match mutate {
                0 => event.alert_key = "short".into(),
                1 => event.state = "unknown".into(),
                _ => event.failed = -1,
            }
            assert!(LifecycleEventsBatch::try_from_request(
                api::LifecycleEventsBatchRequest {
                    events: vec![event],
                    version_bump_seconds: 0,
                },
                datetime!(2026-09-15 09:45),
            )
            .is_err());
        }
        assert!(LifecycleEventsBatch::try_from_request(
            api::LifecycleEventsBatchRequest {
                events: vec![request()],
                version_bump_seconds: 2,
            },
            datetime!(2026-09-15 09:45),
        )
        .is_err());
    }
}
