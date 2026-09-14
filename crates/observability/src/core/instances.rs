use std::cmp::Ordering;

use async_bb8_diesel::AsyncConnection;
use diesel_models::{
    observability::{
        alerts_main::{slack as slack_main, xyne as xyne_main, AnnouncementRow},
        merchants_alert_external::{
            slack as slack_instance, xyne as xyne_instance, MerchantInstanceRow,
        },
        merchants_alert_external_dimension::DimensionInstance,
    },
    DatabaseConnectionWithContext, StorageResult,
};
use error_stack::{report, ResultExt};
use time::PrimitiveDateTime;

use crate::{
    errors::{ObservabilityApiResult, ObservabilityError},
    logger,
    state::AppState,
    types::{
        instances::{
            DimensionInstanceEntry, DimensionInstanceWrite, DimensionReadResponse,
            DimensionSaveResponse, DimensionWriteRequest, InstanceReadResponse,
            InstanceSaveResponse, InstanceWriteRequest, MerchantInstanceEntry,
            MerchantInstanceWrite, Truncation,
        },
        lifecycle::Channel,
        ReadStatus, WriteStatus,
    },
};

const NAME_MAX_BYTES: usize = 64;

const VALUE_MAX_BYTES: usize = 255;

pub const TRUNCATION_KEY: &str = "truncated_by_impact";

pub async fn read_instances(
    state: AppState,
    channel: Channel,
    announcement: uuid::Uuid,
) -> ObservabilityApiResult<InstanceReadResponse> {
    let connection = state.database_connection().await?;

    let rows = store::list(&connection, channel, announcement)
        .await
        .change_context(ObservabilityError::StorageUnavailable)
        .attach_printable("Failed to read the merchant alert instances")?;

    Ok(InstanceReadResponse {
        status: found_or_absent(rows.len()),
        merchants: rows.into_iter().map(MerchantInstanceEntry::from).collect(),
    })
}

pub async fn write_instances(
    state: AppState,
    channel: Channel,
    announcement: uuid::Uuid,
    request: InstanceWriteRequest,
) -> ObservabilityApiResult<InstanceSaveResponse> {
    let connection = state.database_connection().await?;
    let parent = store::announcement(&connection, channel, announcement).await?;

    let now = stamp();
    let plan = InstancePlan::build(
        request.merchants,
        Parent {
            announcement,
            thread: parent.ts_slack,
        },
        now,
        state.conf.instances.max_merchants,
    )?;

    let borrowed = &connection;
    let rows = plan.rows;
    let applied = borrowed
        .raw_connection()
        .transaction_async(move |_| async move {
            let removed = store::delete(borrowed, channel, announcement).await?;
            let stored = store::insert(borrowed, channel, rows).await?;

            Ok::<_, WriteFailure>(Applied { stored, removed })
        })
        .await
        .map_err(WriteFailure::into_report)?;

    Ok(InstanceSaveResponse {
        status: WriteStatus::Saved,
        ts_alert: now,
        merchants: applied.stored,
        removed: applied.removed,
        truncated: plan.truncated,
    })
}

pub async fn read_dimensions(
    state: AppState,
    announcement: uuid::Uuid,
) -> ObservabilityApiResult<DimensionReadResponse> {
    let connection = state.database_connection().await?;

    let rows = DimensionInstance::list_for_announcement(&connection, announcement)
        .await
        .change_context(ObservabilityError::StorageUnavailable)
        .attach_printable("Failed to read the alert dimension breakdown")?;

    Ok(DimensionReadResponse {
        status: found_or_absent(rows.len()),
        dimensions: rows.into_iter().map(DimensionInstanceEntry::from).collect(),
    })
}

pub async fn write_dimensions(
    state: AppState,
    announcement: uuid::Uuid,
    request: DimensionWriteRequest,
) -> ObservabilityApiResult<DimensionSaveResponse> {
    let connection = state.database_connection().await?;
    let parent = store::announcement(&connection, Channel::Slack, announcement).await?;

    let now = stamp();
    let plan = DimensionPlan::build(
        request.dimensions,
        Parent {
            announcement,
            thread: parent.ts_slack,
        },
        now,
        state.conf.instances.max_dimensions,
    )?;

    let borrowed = &connection;
    let rows = plan.rows;
    let applied = borrowed
        .raw_connection()
        .transaction_async(move |_| async move {
            let removed =
                DimensionInstance::delete_for_announcement(borrowed, announcement).await?;
            let stored = DimensionInstance::insert_all(borrowed, rows).await?;

            Ok::<_, WriteFailure>(Applied { stored, removed })
        })
        .await
        .map_err(WriteFailure::into_report)?;

    Ok(DimensionSaveResponse {
        status: WriteStatus::Saved,
        ts_alert: now,
        dimensions: applied.stored,
        removed: applied.removed,
        truncated: plan.truncated,
    })
}

struct Parent {
    announcement: uuid::Uuid,
    thread: Option<String>,
}

struct Applied {
    stored: usize,
    removed: usize,
}

fn found_or_absent(rows: usize) -> ReadStatus {
    if rows == 0 {
        ReadStatus::Absent
    } else {
        ReadStatus::Found
    }
}

struct InstancePlan {
    rows: Vec<MerchantInstanceRow>,
    truncated: Option<Truncation>,
}

impl InstancePlan {
    fn build(
        writes: Vec<MerchantInstanceWrite>,
        parent: Parent,
        now: PrimitiveDateTime,
        limit: usize,
    ) -> ObservabilityApiResult<Self> {
        for write in &writes {
            fits(write.name.as_deref(), "name", NAME_MAX_BYTES)?;
            fits(write.product.as_deref(), "product", NAME_MAX_BYTES)?;
            fits(write.merchant_id.as_deref(), "merchant_id", NAME_MAX_BYTES)?;
            fits(write.priority.as_deref(), "priority", NAME_MAX_BYTES)?;
            fits(write.tenant_id.as_deref(), "tenant_id", NAME_MAX_BYTES)?;
            fits(write.attribution.as_deref(), "attribution", VALUE_MAX_BYTES)?;
            fits(write.ts_slack.as_deref(), "ts_slack", VALUE_MAX_BYTES)?;
        }

        let (kept, truncated) = keep_the_worst(writes, limit, |write| {
            impact(write.current_metric, write.expected_metric)
        });
        let marker = truncated.as_ref().map(marker_for);

        let rows = kept
            .into_iter()
            .map(|write| MerchantInstanceRow {
                id: Some(parent.announcement),
                id_merchant_table: uuid::Uuid::now_v7(),
                id_intermediate: write.id_intermediate,
                name: write.name,
                product: write.product,
                merchant_id: write.merchant_id,
                dimensions: write.dimensions,
                auxiliary_dimensions: write.auxiliary_dimensions,
                current_metric: write.current_metric,
                expected_metric: write.expected_metric,
                attribution: write.attribution,
                max_duration: write.max_duration,
                start_time: write.start_time,
                is_visible: Some(write.is_visible.unwrap_or(true)),
                recovered_ts: write.recovered_ts,
                ts_slack: write.ts_slack.or_else(|| parent.thread.clone()),
                ts_alert: Some(now),
                latest_ts_alert: write.latest_ts_alert,
                last_updated_at: Some(now),
                slack_info: write.slack_info,
                communication_info: write.communication_info,
                metadata: write.metadata,
                metadata_alert_details: record_truncation(
                    write.metadata_alert_details,
                    marker.as_ref(),
                ),
                priority: write.priority,
                tenant_id: write.tenant_id,
            })
            .collect();

        Ok(Self { rows, truncated })
    }
}

struct DimensionPlan {
    rows: Vec<DimensionInstance>,
    truncated: Option<Truncation>,
}

impl DimensionPlan {
    fn build(
        writes: Vec<DimensionInstanceWrite>,
        parent: Parent,
        now: PrimitiveDateTime,
        limit: usize,
    ) -> ObservabilityApiResult<Self> {
        for write in &writes {
            fits(write.name.as_deref(), "name", NAME_MAX_BYTES)?;
            fits(write.product.as_deref(), "product", NAME_MAX_BYTES)?;
            fits(
                write.dimension_key.as_deref(),
                "dimension_key",
                NAME_MAX_BYTES,
            )?;
            fits(write.priority.as_deref(), "priority", NAME_MAX_BYTES)?;
            fits(write.tenant_id.as_deref(), "tenant_id", NAME_MAX_BYTES)?;
            fits(
                write.dimension_value.as_deref(),
                "dimension_value",
                VALUE_MAX_BYTES,
            )?;
            fits(write.attribution.as_deref(), "attribution", VALUE_MAX_BYTES)?;
            fits(write.ts_slack.as_deref(), "ts_slack", VALUE_MAX_BYTES)?;
        }

        let (kept, truncated) = keep_the_worst(writes, limit, |write| {
            impact(write.current_metric, write.expected_metric)
        });
        let marker = truncated.as_ref().map(marker_for);

        let rows = kept
            .into_iter()
            .map(|write| DimensionInstance {
                id: Some(parent.announcement),
                id_merchant_table: uuid::Uuid::now_v7(),
                id_intermediate: write.id_intermediate,
                name: write.name,
                product: write.product,
                dimension_key: write.dimension_key,
                dimension_value: write.dimension_value,
                dimensions: write.dimensions,
                auxiliary_dimensions: write.auxiliary_dimensions,
                current_metric: write.current_metric,
                expected_metric: write.expected_metric,
                attribution: write.attribution,
                max_duration: write.max_duration,
                is_visible: Some(write.is_visible.unwrap_or(true)),
                start_time: write.start_time,
                recovered_ts: write.recovered_ts,
                ts_slack: write.ts_slack.or_else(|| parent.thread.clone()),
                ts_alert: Some(now),
                latest_ts_alert: write.latest_ts_alert,
                last_updated_at: Some(now),
                slack_info: write.slack_info,
                communication_info: write.communication_info,
                metadata: write.metadata,
                metadata_alert_details: record_truncation(
                    write.metadata_alert_details,
                    marker.as_ref(),
                ),
                priority: write.priority,
                tenant_id: write.tenant_id,
            })
            .collect();

        Ok(Self { rows, truncated })
    }
}

#[derive(Debug)]
enum Impact {
    Absolute,
    Gap(f64),
    Unknown,
}

fn impact(current: Option<f64>, expected: Option<f64>) -> Impact {
    match (current, expected) {
        (Some(current), Some(expected)) => Impact::Gap((expected - current).abs()),
        (Some(_), None) => Impact::Absolute,
        (None, _) => Impact::Unknown,
    }
}

fn worst_first(left: &Impact, right: &Impact) -> Ordering {
    match (left, right) {
        (Impact::Absolute, Impact::Absolute) | (Impact::Unknown, Impact::Unknown) => {
            Ordering::Equal
        }
        (Impact::Absolute, _) | (Impact::Gap(_), Impact::Unknown) => Ordering::Less,
        (_, Impact::Absolute) | (Impact::Unknown, Impact::Gap(_)) => Ordering::Greater,
        (Impact::Gap(left), Impact::Gap(right)) => right.total_cmp(left),
    }
}

fn keep_the_worst<T>(
    mut rows: Vec<T>,
    limit: usize,
    impact_of: impl Fn(&T) -> Impact,
) -> (Vec<T>, Option<Truncation>) {
    let received = rows.len();
    if received <= limit {
        return (rows, None);
    }

    rows.sort_by(|left, right| worst_first(&impact_of(left), &impact_of(right)));
    rows.truncate(limit);

    logger::warn!(
        received = received,
        stored = limit,
        dropped = received - limit,
        "An alert instance write was truncated at the row cap"
    );

    (
        rows,
        Some(Truncation {
            received,
            stored: limit,
            dropped: received - limit,
        }),
    )
}

fn marker_for(truncation: &Truncation) -> serde_json::Value {
    serde_json::json!({
        "received": truncation.received,
        "stored": truncation.stored,
        "dropped": truncation.dropped,
    })
}

fn record_truncation(
    details: Option<serde_json::Value>,
    marker: Option<&serde_json::Value>,
) -> Option<serde_json::Value> {
    let Some(marker) = marker else {
        return details;
    };

    match details {
        Some(serde_json::Value::Object(mut fields)) => {
            fields.insert(TRUNCATION_KEY.to_owned(), marker.clone());
            Some(serde_json::Value::Object(fields))
        }
        Some(other) => Some(serde_json::json!({
            TRUNCATION_KEY: marker.clone(),
            "details": other,
        })),
        None => Some(serde_json::json!({ TRUNCATION_KEY: marker.clone() })),
    }
}

fn fits(value: Option<&str>, field: &'static str, max_bytes: usize) -> ObservabilityApiResult<()> {
    if let Some(value) = value {
        if value.len() > max_bytes {
            Err(
                report!(ObservabilityError::InvalidRequest).attach_printable(format!(
                    "The instance {field} is {} bytes, over the {max_bytes} the column holds",
                    value.len()
                )),
            )?;
        }
    }

    Ok(())
}

enum WriteFailure {
    Storage(error_stack::Report<diesel_models::errors::DatabaseError>),
    Transaction(diesel::result::Error),
}

impl WriteFailure {
    fn into_report(self) -> error_stack::Report<ObservabilityError> {
        match self {
            Self::Storage(error) => error
                .change_context(ObservabilityError::StorageUnavailable)
                .attach_printable("Failed to write the alert instances"),
            Self::Transaction(error) => report!(ObservabilityError::StorageUnavailable)
                .attach_printable(format!(
                    "The alert instance write transaction failed: {error}"
                )),
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

mod store {
    use super::{
        report, slack_instance, slack_main, xyne_instance, xyne_main, AnnouncementRow, Channel,
        DatabaseConnectionWithContext, MerchantInstanceRow, ObservabilityApiResult,
        ObservabilityError, ResultExt, StorageResult,
    };

    pub(super) async fn announcement(
        conn: &DatabaseConnectionWithContext<'_>,
        channel: Channel,
        id: uuid::Uuid,
    ) -> ObservabilityApiResult<AnnouncementRow> {
        match channel {
            Channel::Slack => slack_main::Announcement::find_by_id(conn, id).await,
            Channel::Xyne => xyne_main::Announcement::find_by_id(conn, id).await,
        }
        .change_context(ObservabilityError::StorageUnavailable)
        .attach_printable("Failed to look up the announcement an instance write references")?
        .ok_or_else(|| report!(ObservabilityError::UnknownAnnouncement { id: id.to_string() }))
    }

    pub(super) async fn list(
        conn: &DatabaseConnectionWithContext<'_>,
        channel: Channel,
        announcement: uuid::Uuid,
    ) -> StorageResult<Vec<MerchantInstanceRow>> {
        match channel {
            Channel::Slack => {
                slack_instance::MerchantInstance::list_for_announcement(conn, announcement).await
            }
            Channel::Xyne => {
                xyne_instance::MerchantInstance::list_for_announcement(conn, announcement).await
            }
        }
    }

    pub(super) async fn delete(
        conn: &DatabaseConnectionWithContext<'_>,
        channel: Channel,
        announcement: uuid::Uuid,
    ) -> StorageResult<usize> {
        match channel {
            Channel::Slack => {
                slack_instance::MerchantInstance::delete_for_announcement(conn, announcement).await
            }
            Channel::Xyne => {
                xyne_instance::MerchantInstance::delete_for_announcement(conn, announcement).await
            }
        }
    }

    pub(super) async fn insert(
        conn: &DatabaseConnectionWithContext<'_>,
        channel: Channel,
        rows: Vec<MerchantInstanceRow>,
    ) -> StorageResult<usize> {
        match channel {
            Channel::Slack => slack_instance::MerchantInstance::insert_all(conn, rows).await,
            Channel::Xyne => xyne_instance::MerchantInstance::insert_all(conn, rows).await,
        }
    }
}
