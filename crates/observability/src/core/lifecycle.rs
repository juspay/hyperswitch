use async_bb8_diesel::AsyncConnection;
use diesel_models::observability::{
    alerts_intermediate::AlertStateRow, alerts_main::AnnouncementRow, raw_json::RawJson,
};
use error_stack::{report, ResultExt};
use time::PrimitiveDateTime;

use crate::{
    errors::{ObservabilityApiResult, ObservabilityError},
    logger,
    state::AppState,
    types::{
        lifecycle::{
            AlertStateEntry, AlertStateWrite, AnnouncementEntry, AnnouncementRequest,
            AnnouncementSaveResponse, Channel, LifecycleStateResponse, LifecycleStateSaveResponse,
            LifecycleStateWriteRequest,
        },
        ReadStatus, WriteStatus,
    },
};

const NAME_MAX_BYTES: usize = 64;

const TS_SLACK_MAX_BYTES: usize = 255;

const ALERTS_PER_STATEMENT: usize = 1_000;

const MAX_ALERTS: usize = 5_000;

pub async fn read_state(
    state: AppState,
    channel: Channel,
) -> ObservabilityApiResult<LifecycleStateResponse> {
    let connection = state.database_connection().await?;

    let rows = AlertStateRow::list_by_channel(&connection, channel.as_str())
        .await
        .change_context(ObservabilityError::InternalServerError)
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
    let limit = MAX_ALERTS;
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
            AlertStateRow::lock_channel(borrowed, channel.lock_key()).await?;

            let found =
                AlertStateRow::find_latest_last_updated_at_by_channel(borrowed, channel.as_str())
                    .await?;
            if found != expected {
                Err(WriteFailure::Stale { expected, found })?;
            }

            let existing = AnnouncementRow::list_ids_by_channel_and_ids(
                borrowed,
                channel.as_str(),
                plan.referenced.clone(),
            )
            .await?;
            if let Some(missing) = plan
                .referenced
                .iter()
                .find(|id| !existing.contains(id))
                .copied()
            {
                Err(WriteFailure::UnknownAnnouncement(missing))?;
            }

            let removed = AlertStateRow::delete_by_channel_excluding_ids(
                borrowed,
                channel.as_str(),
                plan.keep,
            )
            .await?;

            let mut alerts = 0;
            for batch in plan.rows.chunks(ALERTS_PER_STATEMENT) {
                alerts +=
                    AlertStateRow::bulk_upsert_within_channel(borrowed, batch.to_vec()).await?;
            }
            if alerts != plan.rows.len() {
                Err(WriteFailure::ForeignRows {
                    expected: plan.rows.len(),
                    written: alerts,
                })?;
            }

            Ok::<_, WriteFailure>(Applied { alerts, removed })
        })
        .await
        .map_err(|failure| failure.into_report(channel))?;

    Ok(LifecycleStateSaveResponse {
        status: WriteStatus::Saved,
        last_updated_at: (applied.alerts > 0).then_some(now),
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
    .change_context(ObservabilityError::InternalServerError)
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
            Self::Storage(error) => error
                .change_context(ObservabilityError::InternalServerError)
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
