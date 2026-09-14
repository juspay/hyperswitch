use std::collections::HashSet;

use async_bb8_diesel::AsyncConnection;
use diesel_models::observability::{
    alerts_intermediate::{AlertsIntermediate, AlertsIntermediateNew, AlertsIntermediateUpdate},
    alerts_main::{AlertsMain, AlertsMainNew, AlertsMainUpdate},
    raw_json::RawJson,
};
use error_stack::{report, ResultExt};
use time::PrimitiveDateTime;

use crate::{
    core::utils::{self, NAME_MAX_CHARS, VALUE_MAX_CHARS},
    errors::{ObservabilityApiResult, ObservabilityError, StorageErrorExt},
    state::AppState,
    types::lifecycle::{
        AlertStateEntry, AlertStateWrite, AnnouncementEntry, AnnouncementListRequest,
        AnnouncementListResponse, AnnouncementRequest, AnnouncementUpdateRequest, Channel,
        LifecycleStateResponse, LifecycleStateSaveResponse, LifecycleStateWriteRequest,
    },
};

const ALERTS_PER_STATEMENT: usize = 1_000;

const MAX_ALERTS: usize = 5_000;

const DEFAULT_ANNOUNCEMENT_WINDOW_DAYS: i64 = 7;

const MAX_ANNOUNCEMENT_WINDOW_DAYS: i64 = 30;

const MERCHANT_VISIBILITY_KEY: &str = "is_visible_to_merchant";

pub async fn read_state(
    state: AppState,
    channel: Channel,
) -> ObservabilityApiResult<LifecycleStateResponse> {
    let channel = <&'static str>::from(channel);
    let connection = state.database_connection().await?;

    let rows = AlertsIntermediate::list_by_channel(&connection, channel)
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to read the lifecycle state")?;

    Ok(LifecycleStateResponse {
        count: rows.len(),
        last_updated_at: rows.iter().map(|row| row.last_updated_at).max(),
        alerts: rows.into_iter().map(AlertStateEntry::from).collect(),
    })
}

pub async fn write_state(
    state: AppState,
    channel: Channel,
    request: LifecycleStateWriteRequest,
) -> ObservabilityApiResult<LifecycleStateSaveResponse> {
    let channel = <&'static str>::from(channel);

    if request.alerts.len() > MAX_ALERTS {
        Err(report!(ObservabilityError::StateTooLarge {
            alerts: request.alerts.len(),
            limit: MAX_ALERTS,
        }))?;
    }

    let now = utils::truncate_to_millisecond(common_utils::date_time::now());
    let plan = WritePlan::build(request.alerts, channel, now)?;
    let id_intermediates = plan.keep.clone();
    let expected = request
        .expected_last_updated_at
        .map(utils::truncate_to_millisecond);
    let connection = state.database_connection().await?;

    let borrowed = &connection;
    let applied = borrowed
        .raw_connection()
        .transaction_async(move |_| async move {
            AlertsIntermediate::lock_channel(borrowed, channel)
                .await
                .change_context(ObservabilityError::InternalServerError)
                .attach_printable("Failed to lock the lifecycle state of the channel")?;

            let found =
                AlertsIntermediate::find_latest_last_updated_at_by_channel(borrowed, channel)
                    .await
                    .change_context(ObservabilityError::InternalServerError)
                    .attach_printable("Failed to read the lifecycle state watermark")?
                    .map(utils::truncate_to_millisecond);
            if found != expected {
                Err(
                    report!(ObservabilityError::StateChanged).attach_printable(format!(
                        "Expected the lifecycle state at {expected:?}, found it at {found:?}"
                    )),
                )?;
            }

            let existing = AlertsMain::list_ids_by_channel_and_ids(
                borrowed,
                channel,
                plan.referenced.iter().copied().collect(),
            )
            .await
            .change_context(ObservabilityError::InternalServerError)
            .attach_printable("Failed to list the announcements the lifecycle state references")?
            .into_iter()
            .collect::<HashSet<_>>();
            if let Some(missing) = plan.referenced.iter().find(|id| !existing.contains(id)) {
                Err(report!(ObservabilityError::UnknownAnnouncement {
                    id: missing.to_string(),
                }))?;
            }

            let removed =
                AlertsIntermediate::delete_by_channel_excluding_ids(borrowed, channel, plan.keep)
                    .await
                    .change_context(ObservabilityError::InternalServerError)
                    .attach_printable(
                        "Failed to delete the lifecycle state rows the write does not carry",
                    )?;

            let expected_alerts = plan.rows.len();
            let mut rows = plan.rows.into_iter();
            let mut stored = 0;
            loop {
                let batch = rows.by_ref().take(ALERTS_PER_STATEMENT).collect::<Vec<_>>();
                if batch.is_empty() {
                    break;
                }
                stored += AlertsIntermediateNew::bulk_upsert_within_channel(borrowed, batch)
                    .await
                    .change_context(ObservabilityError::InternalServerError)
                    .attach_printable("Failed to write the lifecycle state rows")?;
            }
            if stored != expected_alerts {
                Err(report!(ObservabilityError::ForeignAlertState {
                    alerts: expected_alerts - stored,
                }))?;
            }

            Ok::<_, TransactionError>(Applied { stored, removed })
        })
        .await
        .map_err(TransactionError::into_report)?;

    Ok(LifecycleStateSaveResponse {
        last_updated_at: (applied.stored > 0).then_some(now),
        stored: applied.stored,
        removed: applied.removed,
        id_intermediates,
    })
}

pub async fn record_announcement(
    state: AppState,
    channel: Channel,
    request: AnnouncementRequest,
) -> ObservabilityApiResult<AnnouncementEntry> {
    let channel = <&'static str>::from(channel);

    utils::within_width(&request.name, "name", NAME_MAX_CHARS)?;
    utils::within_width(&request.product, "product", NAME_MAX_CHARS)?;
    utils::optional_within_width(request.ts_slack.as_deref(), "ts_slack", VALUE_MAX_CHARS)?;

    let now = utils::truncate_to_millisecond(common_utils::date_time::now());
    let connection = state.database_connection().await?;

    let announcement = AlertsMainNew {
        id: common_utils::generate_uuid_v7(),
        channel: channel.to_owned(),
        name: request.name,
        product: request.product,
        dimensions: utils::or_empty_list(request.dimensions.map(RawJson::from))?,
        ts_slack: request.ts_slack,
        ts_alert: now,
        duration: request.duration.unwrap_or_default(),
        sent: request.sent.unwrap_or_default(),
        critical: request.critical.unwrap_or_default(),
        rca_metadata: request.rca_metadata.unwrap_or_else(utils::empty_object),
        metadata: request.metadata.map(RawJson::from),
        last_updated_at: now,
    }
    .insert(&connection)
    .await
    .change_context(ObservabilityError::InternalServerError)
    .attach_printable("Failed to record an announcement")?;

    Ok(AnnouncementEntry::from(announcement))
}

pub async fn list_announcements(
    state: AppState,
    channel: Channel,
    request: AnnouncementListRequest,
) -> ObservabilityApiResult<AnnouncementListResponse> {
    let channel = <&'static str>::from(channel);

    not_before_unix_epoch(request.start, "start")?;
    not_before_unix_epoch(request.end, "end")?;

    let end = request
        .end
        .unwrap_or_else(|| utils::truncate_to_millisecond(common_utils::date_time::now()));
    let start = request.start.unwrap_or_else(|| {
        end.saturating_sub(time::Duration::days(DEFAULT_ANNOUNCEMENT_WINDOW_DAYS))
    });
    if end < start || end - start > time::Duration::days(MAX_ANNOUNCEMENT_WINDOW_DAYS) {
        Err(report!(ObservabilityError::InvalidAnnouncementWindow {
            max_days: MAX_ANNOUNCEMENT_WINDOW_DAYS,
        }))?;
    }

    let connection = state.database_connection().await?;

    let announcements =
        AlertsMain::list_by_channel_and_ts_alert_window(&connection, channel, start, end)
            .await
            .change_context(ObservabilityError::InternalServerError)
            .attach_printable("Failed to list announcements")?
            .into_iter()
            .map(AnnouncementEntry::from)
            .collect::<Vec<_>>();

    Ok(AnnouncementListResponse {
        count: announcements.len(),
        announcements,
    })
}

pub async fn update_announcement(
    state: AppState,
    channel: Channel,
    id: uuid::Uuid,
    request: AnnouncementUpdateRequest,
) -> ObservabilityApiResult<AnnouncementEntry> {
    let channel = <&'static str>::from(channel);

    let patch =
        serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(request.metadata.get())
            .change_context(ObservabilityError::InvalidDataValue {
                field_name: "metadata",
            })
            .attach_printable("The announcement metadata is not a JSON object")?;
    let metadata = RawJson::from(request.metadata);

    let now = utils::truncate_to_millisecond(common_utils::date_time::now());
    let connection = state.database_connection().await?;

    let borrowed = &connection;
    let announcement = borrowed
        .raw_connection()
        .transaction_async(move |_| async move {
            AlertsIntermediate::lock_channel(borrowed, channel)
                .await
                .change_context(ObservabilityError::InternalServerError)
                .attach_printable("Failed to lock the lifecycle state of the channel")?;

            let announcement = AlertsMain::update_by_channel_and_id(
                borrowed,
                channel,
                id,
                AlertsMainUpdate::Metadata {
                    metadata,
                    last_updated_at: now,
                },
            )
            .await
            .to_not_found_response(ObservabilityError::UnknownAnnouncement { id: id.to_string() })
            .attach_printable("Failed to update the announcement metadata")?;

            let rows = AlertsIntermediate::list_by_channel_and_announcement(borrowed, channel, id)
                .await
                .change_context(ObservabilityError::InternalServerError)
                .attach_printable("Failed to list the lifecycle state rows of the announcement")?;
            for row in rows {
                let mut merged = match row.metadata {
                    Some(serde_json::Value::Object(existing)) => existing,
                    _ => serde_json::Map::new(),
                };
                merged.extend(
                    patch
                        .iter()
                        .filter(|(key, _)| key.as_str() != MERCHANT_VISIBILITY_KEY)
                        .map(|(key, value)| (key.clone(), value.clone())),
                );

                AlertsIntermediate::update_by_id_intermediate(
                    borrowed,
                    row.id_intermediate,
                    AlertsIntermediateUpdate::Metadata {
                        metadata: serde_json::Value::Object(merged),
                    },
                )
                .await
                .change_context(ObservabilityError::InternalServerError)
                .attach_printable("Failed to merge the announcement metadata into its state row")?;
            }

            Ok::<_, TransactionError>(announcement)
        })
        .await
        .map_err(TransactionError::into_report)?;

    Ok(AnnouncementEntry::from(announcement))
}

struct Applied {
    stored: usize,
    removed: usize,
}

struct WritePlan {
    rows: Vec<AlertsIntermediateNew>,
    keep: Vec<uuid::Uuid>,
    referenced: HashSet<uuid::Uuid>,
}

impl WritePlan {
    fn build(
        alerts: Vec<AlertStateWrite>,
        channel: &str,
        now: PrimitiveDateTime,
    ) -> ObservabilityApiResult<Self> {
        let mut rows = Vec::with_capacity(alerts.len());
        let mut keep = Vec::with_capacity(alerts.len());
        let mut seen = HashSet::with_capacity(alerts.len());
        let mut referenced = HashSet::new();

        for alert in alerts {
            utils::within_width(&alert.name, "name", NAME_MAX_CHARS)?;
            utils::within_width(&alert.product, "product", NAME_MAX_CHARS)?;
            utils::optional_within_width(alert.group_id.as_deref(), "group_id", NAME_MAX_CHARS)?;
            utils::optional_within_width(alert.priority.as_deref(), "priority", NAME_MAX_CHARS)?;
            utils::optional_within_width(alert.ts_slack.as_deref(), "ts_slack", VALUE_MAX_CHARS)?;

            not_before_unix_epoch(alert.ts_alert, "ts_alert")?;
            not_before_unix_epoch(alert.latest_ts_alert, "latest_ts_alert")?;
            not_before_unix_epoch(alert.recovered_ts, "recovered_ts")?;

            let id_intermediate = alert
                .id_intermediate
                .unwrap_or_else(common_utils::generate_uuid_v7);

            if !seen.insert(id_intermediate) {
                Err(report!(ObservabilityError::InvalidRequestData {
                    message: "The lifecycle write carries an id_intermediate more than once"
                        .to_owned(),
                })
                .attach_printable(format!(
                    "The lifecycle write carries id_intermediate {id_intermediate} more than once"
                )))?;
            }

            keep.push(id_intermediate);
            referenced.extend(alert.announcement_id);

            rows.push(AlertsIntermediateNew {
                id_intermediate,
                channel: channel.to_owned(),
                id: alert.announcement_id,
                name: alert.name,
                product: alert.product,
                dimensions: alert
                    .dimensions
                    .unwrap_or_else(|| serde_json::Value::Array(Vec::new())),
                ts_slack: alert.ts_slack,
                ts_alert: alert.ts_alert.unwrap_or(now),
                latest_ts_alert: alert.latest_ts_alert.unwrap_or(now),
                max_duration: alert.max_duration.unwrap_or_default(),
                other_metrics: alert.other_metrics,
                metadata: alert.metadata,
                metadata_alert_details: alert.metadata_alert_details,
                rca_metadata: alert.rca_metadata.unwrap_or_else(utils::empty_object),
                group_id: alert.group_id.unwrap_or_default(),
                priority: alert.priority.unwrap_or_default(),
                last_updated_at: now,
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

enum TransactionError {
    Observability(error_stack::Report<ObservabilityError>),
    Database(diesel::result::Error),
}

impl TransactionError {
    fn into_report(self) -> error_stack::Report<ObservabilityError> {
        match self {
            Self::Observability(error) => error,
            Self::Database(error) => report!(error)
                .change_context(ObservabilityError::InternalServerError)
                .attach_printable("Failed to run the lifecycle transaction"),
        }
    }
}

impl From<diesel::result::Error> for TransactionError {
    fn from(error: diesel::result::Error) -> Self {
        Self::Database(error)
    }
}

impl From<error_stack::Report<ObservabilityError>> for TransactionError {
    fn from(error: error_stack::Report<ObservabilityError>) -> Self {
        Self::Observability(error)
    }
}

fn not_before_unix_epoch(
    value: Option<PrimitiveDateTime>,
    field_name: &'static str,
) -> ObservabilityApiResult<()> {
    if value.is_some_and(|value| value.assume_utc() < time::OffsetDateTime::UNIX_EPOCH) {
        Err(report!(ObservabilityError::InvalidDataValue { field_name }))?;
    }

    Ok(())
}
