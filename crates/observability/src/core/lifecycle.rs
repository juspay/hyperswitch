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
    errors::{ObservabilityApiResult, ObservabilityError},
    state::AppState,
    types::{
        lifecycle::{
            AlertStateEntry, AlertStateWrite, AnnouncementEntry, AnnouncementListRequest,
            AnnouncementListResponse, AnnouncementRequest, AnnouncementSaveResponse,
            AnnouncementUpdateRequest, Channel, LifecycleStateResponse, LifecycleStateSaveResponse,
            LifecycleStateWriteRequest,
        },
        ReadStatus, WriteStatus,
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
            AlertsIntermediate::lock_channel(borrowed, channel).await?;

            let found =
                AlertsIntermediate::find_latest_last_updated_at_by_channel(borrowed, channel)
                    .await?
                    .map(utils::truncate_to_millisecond);
            if found != expected {
                Err(WriteFailure::Stale { expected, found })?;
            }

            let existing = AlertsMain::list_ids_by_channel_and_ids(
                borrowed,
                channel,
                plan.referenced.iter().copied().collect(),
            )
            .await?
            .into_iter()
            .collect::<HashSet<_>>();
            if let Some(missing) = plan
                .referenced
                .iter()
                .find(|id| !existing.contains(id))
                .copied()
            {
                Err(WriteFailure::UnknownAnnouncement(missing))?;
            }

            let removed =
                AlertsIntermediate::delete_by_channel_excluding_ids(borrowed, channel, plan.keep)
                    .await?;

            let mut alerts = 0;
            for batch in plan.rows.chunks(ALERTS_PER_STATEMENT) {
                alerts +=
                    AlertsIntermediateNew::bulk_upsert_within_channel(borrowed, batch.to_vec())
                        .await?;
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
        .map_err(WriteFailure::into_report)?;

    Ok(LifecycleStateSaveResponse {
        status: WriteStatus::Saved,
        last_updated_at: (applied.alerts > 0).then_some(now),
        alerts: applied.alerts,
        removed: applied.removed,
        id_intermediates,
    })
}

pub async fn record_announcement(
    state: AppState,
    channel: Channel,
    request: AnnouncementRequest,
) -> ObservabilityApiResult<AnnouncementSaveResponse> {
    let channel = <&'static str>::from(channel);

    utils::optional_within_width(request.name.as_deref(), "name", NAME_MAX_CHARS)?;
    utils::optional_within_width(request.product.as_deref(), "product", NAME_MAX_CHARS)?;
    utils::optional_within_width(request.ts_slack.as_deref(), "ts_slack", VALUE_MAX_CHARS)?;

    let now = utils::truncate_to_millisecond(common_utils::date_time::now());
    let connection = state.database_connection().await?;

    let announcement = AlertsMainNew {
        id: uuid::Uuid::now_v7(),
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

    Ok(AnnouncementSaveResponse {
        status: WriteStatus::Saved,
        announcement: AnnouncementEntry::from(announcement),
    })
}

pub async fn list_announcements(
    state: AppState,
    channel: Channel,
    request: AnnouncementListRequest,
) -> ObservabilityApiResult<AnnouncementListResponse> {
    let channel = <&'static str>::from(channel);

    let end = request
        .end
        .unwrap_or_else(|| utils::truncate_to_millisecond(common_utils::date_time::now()));
    let start = request
        .start
        .unwrap_or(end - time::Duration::days(DEFAULT_ANNOUNCEMENT_WINDOW_DAYS));
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
) -> ObservabilityApiResult<AnnouncementSaveResponse> {
    let channel = <&'static str>::from(channel);

    let patch =
        serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(request.metadata.get())
            .change_context(ObservabilityError::InvalidRequest)
            .attach_printable("The announcement metadata is not a JSON object")?;
    let metadata = RawJson::from(request.metadata);

    let now = utils::truncate_to_millisecond(common_utils::date_time::now());
    let connection = state.database_connection().await?;

    let borrowed = &connection;
    let announcement = borrowed
        .raw_connection()
        .transaction_async(move |_| async move {
            let announcement = AlertsMain::update_by_channel_and_id(
                borrowed,
                channel,
                id,
                AlertsMainUpdate::Metadata {
                    metadata,
                    last_updated_at: now,
                },
            )
            .await?;

            for row in
                AlertsIntermediate::list_by_channel_and_announcement(borrowed, channel, id).await?
            {
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
                .await?;
            }

            Ok::<_, WriteFailure>(announcement)
        })
        .await
        .map_err(|failure| match failure {
            WriteFailure::Storage(error)
                if matches!(
                    error.current_context(),
                    diesel_models::errors::DatabaseError::NotFound
                ) =>
            {
                error.change_context(ObservabilityError::UnknownAnnouncement { id: id.to_string() })
            }
            failure => failure.into_report(),
        })?;

    Ok(AnnouncementSaveResponse {
        status: WriteStatus::Saved,
        announcement: AnnouncementEntry::from(announcement),
    })
}

struct Applied {
    alerts: usize,
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
            utils::optional_within_width(alert.name.as_deref(), "name", NAME_MAX_CHARS)?;
            utils::optional_within_width(alert.product.as_deref(), "product", NAME_MAX_CHARS)?;
            utils::optional_within_width(alert.group_id.as_deref(), "group_id", NAME_MAX_CHARS)?;
            utils::optional_within_width(alert.priority.as_deref(), "priority", NAME_MAX_CHARS)?;
            utils::optional_within_width(alert.ts_slack.as_deref(), "ts_slack", VALUE_MAX_CHARS)?;

            let id_intermediate = alert.id_intermediate.unwrap_or_else(uuid::Uuid::now_v7);

            if !seen.insert(id_intermediate) {
                Err(
                    report!(ObservabilityError::InvalidRequest).attach_printable(format!(
                        "The lifecycle write carries id_intermediate {id_intermediate} twice"
                    )),
                )?;
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
    fn into_report(self) -> error_stack::Report<ObservabilityError> {
        match self {
            Self::Stale { expected, found } => report!(ObservabilityError::StateChanged)
                .attach_printable(format!(
                    "Expected the lifecycle state at {expected:?}, found it at {found:?}"
                )),
            Self::UnknownAnnouncement(id) => {
                report!(ObservabilityError::UnknownAnnouncement { id: id.to_string() })
            }
            Self::ForeignRows { expected, written } => {
                report!(ObservabilityError::ForeignAlertState {
                    alerts: expected - written,
                })
            }
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
