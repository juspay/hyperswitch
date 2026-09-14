use async_bb8_diesel::AsyncConnection;
use diesel_models::{
    observability::{
        alerts_intermediate::AlertStateRow, alerts_main::AnnouncementRow, raw_json::RawJson,
    },
    query::observability::alerts_intermediate::lock_lifecycle_state,
};
use error_stack::{report, ResultExt};
use time::PrimitiveDateTime;

use crate::{
    alert_manager::{
        core::{escalate, unrecognised},
        types::{
            lifecycle::{
                AlertStateEntry, AlertStateWrite, AnnouncementEntry, AnnouncementRequest,
                AnnouncementSaveResponse, Channel, LifecycleStateResponse,
                LifecycleStateSaveResponse, LifecycleStateWriteRequest,
            },
            ReadStatus, WriteStatus,
        },
    },
    errors::{ObservabilityApiResult, ObservabilityError},
    logger,
    state::AppState,
};

const NAME_MAX_BYTES: usize = 64;

const TS_SLACK_MAX_BYTES: usize = 255;

pub async fn read_state(
    state: AppState,
    channel: Channel,
) -> ObservabilityApiResult<LifecycleStateResponse> {
    let connection = state.database_connection().await?;

    let rows = AlertStateRow::list(&connection, channel.as_str())
        .await
        .map_err(|error| escalate(error, unrecognised))
        .attach_printable("Failed to read the lifecycle state")?;

    Ok(LifecycleStateResponse {
        status: if rows.is_empty() {
            ReadStatus::Absent
        } else {
            ReadStatus::Found
        },
        last_updated_at: rows.iter().filter_map(|row| row.last_updated_at).max(),
        alerts: rows.into_iter().map(AlertStateEntry::from).collect(),
    })
}

pub async fn write_state(
    state: AppState,
    channel: Channel,
    request: LifecycleStateWriteRequest,
) -> ObservabilityApiResult<LifecycleStateSaveResponse> {
    let limit = state.conf.lifecycle.max_alerts;
    if request.alerts.len() > limit {
        logger::warn!(
            alerts = request.alerts.len(),
            limit = limit,
            channel = channel.as_str(),
            "Lifecycle write rejected: over the configured alert cap"
        );
        Err(report!(ObservabilityError::StateTooLarge {
            alerts: request.alerts.len(),
            limit,
        }))?;
    }

    let now = stamp();
    let plan = WritePlan::build(request.alerts, channel, now)?;
    let expected = request.expected_last_updated_at;
    let connection = state.database_connection().await?;

    let borrowed = &connection;
    let applied = borrowed
        .raw_connection()
        .transaction_async(move |_| async move {
            lock_lifecycle_state(borrowed, channel.lock_key()).await?;

            let found = AlertStateRow::latest_last_updated_at(borrowed, channel.as_str()).await?;
            if found != expected {
                Err(WriteFailure::Stale { expected, found })?;
            }

            let existing =
                AnnouncementRow::existing_ids(borrowed, channel.as_str(), plan.referenced.clone())
                    .await?;
            if let Some(missing) = plan
                .referenced
                .iter()
                .find(|id| !existing.contains(id))
                .copied()
            {
                Err(WriteFailure::UnknownAnnouncement(missing))?;
            }

            let removed =
                AlertStateRow::delete_absent(borrowed, channel.as_str(), plan.keep).await?;
            let expected = plan.rows.len();
            let alerts = AlertStateRow::upsert_all(borrowed, plan.rows).await?;
            if alerts != expected {
                Err(WriteFailure::ForeignRows {
                    expected,
                    written: alerts,
                })?;
            }

            Ok::<_, WriteFailure>(Applied { alerts, removed })
        })
        .await
        .map_err(|failure| failure.into_report(channel))?;

    Ok(LifecycleStateSaveResponse {
        status: WriteStatus::Saved,
        last_updated_at: now,
        alerts: applied.alerts,
        removed: applied.removed,
    })
}

pub async fn record_announcement(
    state: AppState,
    channel: Channel,
    request: AnnouncementRequest,
) -> ObservabilityApiResult<AnnouncementSaveResponse> {
    fits(request.name.as_deref(), "name", NAME_MAX_BYTES)?;
    fits(request.product.as_deref(), "product", NAME_MAX_BYTES)?;
    fits(request.ts_slack.as_deref(), "ts_slack", TS_SLACK_MAX_BYTES)?;

    let now = stamp();
    let connection = state.database_connection().await?;

    let announcement = AnnouncementRow {
        id: uuid::Uuid::now_v7(),
        channel: Some(channel.as_str().to_owned()),
        name: request.name,
        product: request.product,
        dimensions: request.dimensions.map(RawJson::from),
        ts_slack: request.ts_slack,
        ts_alert: Some(now),
        duration: request.duration,
        sent: request.sent,
        critical: request.critical,
        rca_metadata: request.rca_metadata,
        metadata: request.metadata.map(RawJson::from),
        last_updated_at: Some(now),
    }
    .insert(&connection)
    .await
    .map_err(|error| escalate(error, unrecognised))
    .attach_printable("Failed to record an announcement")?;

    Ok(AnnouncementSaveResponse {
        status: WriteStatus::Saved,
        announcement: AnnouncementEntry::from(announcement),
    })
}

struct Applied {
    alerts: usize,
    removed: usize,
}

#[derive(Debug)]
struct WritePlan {
    rows: Vec<AlertStateRow>,
    keep: Vec<uuid::Uuid>,
    referenced: Vec<uuid::Uuid>,
}

impl WritePlan {
    fn build(
        alerts: Vec<AlertStateWrite>,
        channel: Channel,
        now: PrimitiveDateTime,
    ) -> ObservabilityApiResult<Self> {
        let mut rows = Vec::with_capacity(alerts.len());
        let mut keep = Vec::with_capacity(alerts.len());
        let mut referenced = Vec::new();

        for alert in alerts {
            fits(alert.name.as_deref(), "name", NAME_MAX_BYTES)?;
            fits(alert.product.as_deref(), "product", NAME_MAX_BYTES)?;
            fits(alert.group_id.as_deref(), "group_id", NAME_MAX_BYTES)?;
            fits(alert.priority.as_deref(), "priority", NAME_MAX_BYTES)?;
            fits(alert.ts_slack.as_deref(), "ts_slack", TS_SLACK_MAX_BYTES)?;

            let id_intermediate = alert.id_intermediate.unwrap_or_else(uuid::Uuid::now_v7);

            if keep.contains(&id_intermediate) {
                Err(
                    report!(ObservabilityError::InvalidRequest).attach_printable(format!(
                        "The lifecycle write carries id_intermediate {id_intermediate} twice"
                    )),
                )?;
            }

            keep.push(id_intermediate);
            if let Some(announcement) = alert.announcement_id {
                if !referenced.contains(&announcement) {
                    referenced.push(announcement);
                }
            }

            rows.push(AlertStateRow {
                id_intermediate,
                channel: Some(channel.as_str().to_owned()),
                id: alert.announcement_id,
                name: alert.name,
                product: alert.product,
                dimensions: alert.dimensions,
                ts_slack: alert.ts_slack,
                ts_alert: alert.ts_alert,
                latest_ts_alert: alert.latest_ts_alert,
                max_duration: alert.max_duration,
                other_metrics: alert.other_metrics,
                metadata: alert.metadata,
                metadata_alert_details: alert.metadata_alert_details,
                rca_metadata: alert.rca_metadata,
                group_id: alert.group_id,
                priority: alert.priority,
                last_updated_at: Some(now),
                recovered_ts: alert.recovered_ts,
            });
        }

        Ok(Self {
            rows,
            keep,
            referenced,
        })
    }
}

enum WriteFailure {
    Stale {
        expected: Option<PrimitiveDateTime>,
        found: Option<PrimitiveDateTime>,
    },
    UnknownAnnouncement(uuid::Uuid),
    ForeignRows {
        expected: usize,
        written: usize,
    },
    Storage(error_stack::Report<diesel_models::errors::DatabaseError>),
    Transaction(diesel::result::Error),
}

impl WriteFailure {
    fn into_report(self, channel: Channel) -> error_stack::Report<ObservabilityError> {
        match self {
            Self::Stale { expected, found } => {
                logger::warn!(
                    channel = channel.as_str(),
                    expected = ?expected,
                    found = ?found,
                    "Lifecycle write rejected: the state changed after it was read"
                );
                report!(ObservabilityError::StateChanged)
            }
            Self::UnknownAnnouncement(id) => {
                report!(ObservabilityError::UnknownAnnouncement { id: id.to_string() })
            }
            Self::ForeignRows { expected, written } => report!(ObservabilityError::InvalidRequest)
                .attach_printable(format!(
                    "{} of the {expected} alerts carry an id_intermediate owned by another channel",
                    expected - written
                )),
            Self::Storage(error) => escalate(error, unrecognised)
                .attach_printable("Failed to write the lifecycle state"),
            Self::Transaction(error) => report!(ObservabilityError::InternalServerError)
                .attach_printable(format!("The lifecycle write transaction failed: {error}")),
        }
    }
}

impl From<diesel::result::Error> for WriteFailure {
    fn from(error: diesel::result::Error) -> Self {
        Self::Transaction(error)
    }
}

impl From<error_stack::Report<diesel_models::errors::DatabaseError>> for WriteFailure {
    fn from(error: error_stack::Report<diesel_models::errors::DatabaseError>) -> Self {
        Self::Storage(error)
    }
}

fn stamp() -> PrimitiveDateTime {
    let now = common_utils::date_time::now();

    now.replace_millisecond(now.millisecond()).unwrap_or(now)
}

fn fits(value: Option<&str>, field: &'static str, max_bytes: usize) -> ObservabilityApiResult<()> {
    if let Some(value) = value {
        if value.len() > max_bytes {
            Err(
                report!(ObservabilityError::InvalidRequest).attach_printable(format!(
                    "The lifecycle {field} is {} bytes, over the {max_bytes} the column holds",
                    value.len()
                )),
            )?;
        }
    }

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn alert(id: Option<uuid::Uuid>, announcement: Option<uuid::Uuid>) -> AlertStateWrite {
        AlertStateWrite {
            id_intermediate: id,
            announcement_id: announcement,
            name: Some("sr_drop".to_owned()),
            product: Some("payments".to_owned()),
            dimensions: None,
            ts_slack: None,
            ts_alert: None,
            latest_ts_alert: None,
            max_duration: None,
            other_metrics: None,
            metadata: None,
            metadata_alert_details: None,
            rca_metadata: None,
            group_id: Some("sr_drop|merchant_1234".to_owned()),
            priority: Some("SEV2".to_owned()),
            recovered_ts: None,
        }
    }

    fn now() -> PrimitiveDateTime {
        common_utils::date_time::now()
    }

    #[test]
    fn a_plan_collects_the_announcements_its_rows_reference() {
        let first = uuid::Uuid::now_v7();
        let second = uuid::Uuid::now_v7();

        let plan = WritePlan::build(
            vec![
                alert(None, Some(first)),
                alert(None, Some(second)),
                alert(None, Some(first)),
                alert(None, None),
            ],
            Channel::Slack,
            now(),
        )
        .unwrap();

        assert_eq!(plan.referenced, vec![first, second]);
        assert_eq!(plan.rows.len(), 4);
        assert_eq!(plan.keep.len(), 4);
    }

    #[test]
    fn a_write_carrying_nothing_keeps_nothing() {
        let plan = WritePlan::build(Vec::new(), Channel::Slack, now()).unwrap();

        assert!(plan.keep.is_empty());
        assert!(plan.rows.is_empty());
        assert!(plan.referenced.is_empty());
    }

    #[test]
    fn every_row_is_stamped_with_the_servers_clock() {
        let at = now();
        let plan = WritePlan::build(
            vec![alert(None, None), alert(None, None)],
            Channel::Slack,
            at,
        )
        .unwrap();

        for row in &plan.rows {
            assert_eq!(row.last_updated_at, Some(at));
        }
    }

    #[test]
    fn a_row_without_an_id_is_given_one() {
        let plan = WritePlan::build(vec![alert(None, None)], Channel::Slack, now()).unwrap();

        assert_ne!(plan.rows[0].id_intermediate, uuid::Uuid::nil());
        assert_eq!(plan.keep[0], plan.rows[0].id_intermediate);
    }

    #[test]
    fn a_row_with_an_id_keeps_it() {
        let id = uuid::Uuid::now_v7();
        let plan = WritePlan::build(vec![alert(Some(id), None)], Channel::Slack, now()).unwrap();

        assert_eq!(plan.rows[0].id_intermediate, id);
        assert_eq!(plan.keep, vec![id]);
    }

    #[test]
    fn the_same_row_twice_in_one_write_is_rejected() {
        let id = uuid::Uuid::now_v7();
        let error = WritePlan::build(
            vec![alert(Some(id), None), alert(Some(id), None)],
            Channel::Slack,
            now(),
        )
        .unwrap_err();

        assert!(matches!(
            error.current_context(),
            ObservabilityError::InvalidRequest
        ));
    }

    #[test]
    fn a_value_wider_than_its_column_is_rejected_before_the_query_runs() {
        let mut wide = alert(None, None);
        wide.group_id = Some("g".repeat(NAME_MAX_BYTES + 1));

        assert!(WritePlan::build(vec![wide], Channel::Slack, now()).is_err());

        let mut long_thread = alert(None, None);
        long_thread.ts_slack = Some("t".repeat(TS_SLACK_MAX_BYTES + 1));

        assert!(WritePlan::build(vec![long_thread], Channel::Slack, now()).is_err());
    }

    #[test]
    fn a_stamp_survives_the_wire_format_unchanged() {
        #[derive(serde::Deserialize, serde::Serialize)]
        struct Carried(#[serde(with = "common_utils::custom_serde::iso8601")] PrimitiveDateTime);

        for _ in 0..1_000 {
            let stamped = stamp();
            let wire = serde_json::to_string(&Carried(stamped)).unwrap();
            let returned: Carried = serde_json::from_str(&wire).unwrap();

            assert_eq!(returned.0, stamped, "a stamp did not survive {wire}");
        }
    }

    #[test]
    fn an_absent_value_is_not_measured_against_its_column() {
        let mut bare = alert(None, None);
        bare.name = None;
        bare.group_id = None;

        assert!(WritePlan::build(vec![bare], Channel::Slack, now()).is_ok());
    }
}
