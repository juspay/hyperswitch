use std::cmp::Ordering;

use async_bb8_diesel::AsyncConnection;
use diesel_models::observability::{
    alerts_main::AlertsMain,
    merchants_alert_external::{MerchantsAlertExternal, MerchantsAlertExternalNew},
    merchants_alert_external_dimension::{
        MerchantsAlertExternalDimension, MerchantsAlertExternalDimensionNew,
    },
};
use error_stack::{report, ResultExt};
use time::PrimitiveDateTime;

use crate::{
    core::utils,
    errors::{ObservabilityApiResult, ObservabilityError, StorageErrorExt},
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

const NAME_MAX_CHARS: usize = 64;

const VALUE_MAX_CHARS: usize = 255;

const MAX_MERCHANTS: usize = 500;

const MAX_DIMENSIONS: usize = 500;

const EMPTY_JSON_TEXT: &str = "{}";

const TRUNCATION_KEY: &str = "truncated_by_impact";

pub async fn read_instances(
    state: AppState,
    channel: Channel,
    announcement: uuid::Uuid,
) -> ObservabilityApiResult<InstanceReadResponse> {
    let channel = <&'static str>::from(channel);
    let connection = state.database_connection().await?;

    AlertsMain::find_by_channel_and_id(&connection, channel, announcement)
        .await
        .to_not_found_response(ObservabilityError::UnknownAnnouncement {
            id: announcement.to_string(),
        })?;

    let rows = MerchantsAlertExternal::list_by_channel_and_announcement(
        &connection,
        channel,
        announcement,
    )
    .await
    .change_context(ObservabilityError::InternalServerError)
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
    let channel = <&'static str>::from(channel);

    for write in &request.merchants {
        within_width(write.name.as_deref(), "name", NAME_MAX_CHARS)?;
        within_width(write.product.as_deref(), "product", NAME_MAX_CHARS)?;
        within_width(write.merchant_id.as_deref(), "merchant_id", NAME_MAX_CHARS)?;
        within_width(write.priority.as_deref(), "priority", NAME_MAX_CHARS)?;
        within_width(write.tenant_id.as_deref(), "tenant_id", NAME_MAX_CHARS)?;
        within_width(write.attribution.as_deref(), "attribution", VALUE_MAX_CHARS)?;
        within_width(write.ts_slack.as_deref(), "ts_slack", VALUE_MAX_CHARS)?;
    }

    let now = utils::truncate_to_millisecond(common_utils::date_time::now());
    let (writes, truncated) = keep_the_worst(request.merchants, MAX_MERCHANTS, |write| {
        impact(write.current_metric, write.expected_metric)
    });
    let marker = truncated.as_ref().map(marker_for);
    let connection = state.database_connection().await?;

    let borrowed = &connection;
    let applied = borrowed
        .raw_connection()
        .transaction_async(move |_| async move {
            AlertsMain::lock_instances_by_id(borrowed, announcement).await?;

            let parent =
                AlertsMain::find_by_channel_and_id(borrowed, channel, announcement).await?;

            let removed = MerchantsAlertExternal::delete_by_channel_and_announcement(
                borrowed,
                channel,
                announcement,
            )
            .await?;

            let rows = writes
                .into_iter()
                .map(|write| MerchantsAlertExternalNew {
                    id: announcement,
                    channel: channel.to_owned(),
                    id_merchant_table: uuid::Uuid::now_v7(),
                    id_intermediate: write.id_intermediate,
                    name: write.name.unwrap_or_default(),
                    product: write.product.unwrap_or_default(),
                    merchant_id: write.merchant_id.unwrap_or_default(),
                    dimensions: write.dimensions.unwrap_or_else(empty_json_text),
                    auxiliary_dimensions: write
                        .auxiliary_dimensions
                        .unwrap_or_else(empty_json_text),
                    current_metric: write.current_metric,
                    expected_metric: write.expected_metric,
                    attribution: write.attribution.unwrap_or_default(),
                    max_duration: write.max_duration,
                    start_time: write.start_time,
                    is_visible: write.is_visible.unwrap_or(true),
                    recovered_ts: write.recovered_ts,
                    ts_slack: write
                        .ts_slack
                        .or_else(|| parent.ts_slack.clone())
                        .unwrap_or_default(),
                    ts_alert: now,
                    latest_ts_alert: write.latest_ts_alert,
                    last_updated_at: now,
                    slack_info: write.slack_info.unwrap_or_else(empty_object),
                    communication_info: write.communication_info.unwrap_or_else(empty_object),
                    metadata: write.metadata.unwrap_or_else(empty_json_text),
                    metadata_alert_details: record_truncation(
                        write.metadata_alert_details.unwrap_or_else(empty_object),
                        marker.as_ref(),
                    ),
                    priority: write.priority.unwrap_or_default(),
                    tenant_id: write.tenant_id.unwrap_or_default(),
                })
                .collect();

            let stored = MerchantsAlertExternalNew::bulk_insert(borrowed, rows).await?;

            Ok::<_, WriteFailure>(Applied { stored, removed })
        })
        .await
        .map_err(|failure| failure.into_report(announcement))?;

    Ok(InstanceSaveResponse {
        status: WriteStatus::Saved,
        ts_alert: (applied.stored > 0).then_some(now),
        merchants: applied.stored,
        removed: applied.removed,
        truncated,
    })
}

pub async fn read_dimensions(
    state: AppState,
    channel: Channel,
    announcement: uuid::Uuid,
) -> ObservabilityApiResult<DimensionReadResponse> {
    let channel = <&'static str>::from(channel);
    let connection = state.database_connection().await?;

    AlertsMain::find_by_channel_and_id(&connection, channel, announcement)
        .await
        .to_not_found_response(ObservabilityError::UnknownAnnouncement {
            id: announcement.to_string(),
        })?;

    let rows = MerchantsAlertExternalDimension::list_by_channel_and_announcement(
        &connection,
        channel,
        announcement,
    )
    .await
    .change_context(ObservabilityError::InternalServerError)
    .attach_printable("Failed to read the alert dimension breakdown")?;

    Ok(DimensionReadResponse {
        status: found_or_absent(rows.len()),
        dimensions: rows.into_iter().map(DimensionInstanceEntry::from).collect(),
    })
}

pub async fn write_dimensions(
    state: AppState,
    channel: Channel,
    announcement: uuid::Uuid,
    request: DimensionWriteRequest,
) -> ObservabilityApiResult<DimensionSaveResponse> {
    let channel = <&'static str>::from(channel);

    for write in &request.dimensions {
        within_width(write.name.as_deref(), "name", NAME_MAX_CHARS)?;
        within_width(write.product.as_deref(), "product", NAME_MAX_CHARS)?;
        within_width(
            write.dimension_key.as_deref(),
            "dimension_key",
            NAME_MAX_CHARS,
        )?;
        within_width(write.priority.as_deref(), "priority", NAME_MAX_CHARS)?;
        within_width(write.tenant_id.as_deref(), "tenant_id", NAME_MAX_CHARS)?;
        within_width(
            write.dimension_value.as_deref(),
            "dimension_value",
            VALUE_MAX_CHARS,
        )?;
        within_width(write.attribution.as_deref(), "attribution", VALUE_MAX_CHARS)?;
        within_width(write.ts_slack.as_deref(), "ts_slack", VALUE_MAX_CHARS)?;
    }

    let now = utils::truncate_to_millisecond(common_utils::date_time::now());
    let (writes, truncated) = keep_the_worst(request.dimensions, MAX_DIMENSIONS, |write| {
        impact(write.current_metric, write.expected_metric)
    });
    let marker = truncated.as_ref().map(marker_for);
    let connection = state.database_connection().await?;

    let borrowed = &connection;
    let applied = borrowed
        .raw_connection()
        .transaction_async(move |_| async move {
            AlertsMain::lock_instances_by_id(borrowed, announcement).await?;

            let parent =
                AlertsMain::find_by_channel_and_id(borrowed, channel, announcement).await?;

            let removed = MerchantsAlertExternalDimension::delete_by_channel_and_announcement(
                borrowed,
                channel,
                announcement,
            )
            .await?;

            let rows = writes
                .into_iter()
                .map(|write| MerchantsAlertExternalDimensionNew {
                    id: announcement,
                    channel: channel.to_owned(),
                    id_merchant_table: uuid::Uuid::now_v7(),
                    id_intermediate: write.id_intermediate,
                    name: write.name.unwrap_or_default(),
                    product: write.product.unwrap_or_default(),
                    dimension_key: write.dimension_key.unwrap_or_default(),
                    dimension_value: write.dimension_value.unwrap_or_default(),
                    dimensions: write.dimensions.unwrap_or_else(empty_json_text),
                    auxiliary_dimensions: write
                        .auxiliary_dimensions
                        .unwrap_or_else(empty_json_text),
                    current_metric: write.current_metric,
                    expected_metric: write.expected_metric,
                    attribution: write.attribution.unwrap_or_default(),
                    max_duration: write.max_duration,
                    is_visible: write.is_visible.unwrap_or(true),
                    start_time: write.start_time,
                    recovered_ts: write.recovered_ts,
                    ts_slack: write
                        .ts_slack
                        .or_else(|| parent.ts_slack.clone())
                        .unwrap_or_default(),
                    ts_alert: now,
                    latest_ts_alert: write.latest_ts_alert,
                    last_updated_at: now,
                    slack_info: write.slack_info.unwrap_or_else(empty_object),
                    communication_info: write.communication_info.unwrap_or_else(empty_object),
                    metadata: write.metadata.unwrap_or_else(empty_json_text),
                    metadata_alert_details: record_truncation(
                        write.metadata_alert_details.unwrap_or_else(empty_object),
                        marker.as_ref(),
                    ),
                    priority: write.priority.unwrap_or_default(),
                    tenant_id: write.tenant_id.unwrap_or_default(),
                })
                .collect();

            let stored = MerchantsAlertExternalDimensionNew::bulk_insert(borrowed, rows).await?;

            Ok::<_, WriteFailure>(Applied { stored, removed })
        })
        .await
        .map_err(|failure| failure.into_report(announcement))?;

    Ok(DimensionSaveResponse {
        status: WriteStatus::Saved,
        ts_alert: (applied.stored > 0).then_some(now),
        dimensions: applied.stored,
        removed: applied.removed,
        truncated,
    })
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

fn empty_object() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

fn empty_json_text() -> serde_json::Value {
    serde_json::Value::String(EMPTY_JSON_TEXT.to_owned())
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
    details: serde_json::Value,
    marker: Option<&serde_json::Value>,
) -> serde_json::Value {
    let Some(marker) = marker else {
        return details;
    };

    match details {
        serde_json::Value::Object(mut fields) => {
            fields.insert(TRUNCATION_KEY.to_owned(), marker.clone());
            serde_json::Value::Object(fields)
        }
        other => serde_json::json!({
            TRUNCATION_KEY: marker.clone(),
            "details": other,
        }),
    }
}

enum WriteFailure {
    Storage(error_stack::Report<diesel_models::errors::DatabaseError>),
    Transaction(diesel::result::Error),
}

impl WriteFailure {
    fn into_report(self, announcement: uuid::Uuid) -> error_stack::Report<ObservabilityError> {
        match self {
            Self::Storage(error)
                if matches!(
                    error.current_context(),
                    diesel_models::errors::DatabaseError::NotFound
                ) =>
            {
                error.change_context(ObservabilityError::UnknownAnnouncement {
                    id: announcement.to_string(),
                })
            }
            Self::Storage(error) => error
                .change_context(ObservabilityError::InternalServerError)
                .attach_printable("Failed to write the alert instances"),
            Self::Transaction(error) => report!(ObservabilityError::InternalServerError)
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

fn within_width(value: Option<&str>, field: &str, max_chars: usize) -> ObservabilityApiResult<()> {
    value.map_or(Ok(()), |value| utils::within_width(value, field, max_chars))
}
