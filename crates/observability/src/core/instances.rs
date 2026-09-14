use async_bb8_diesel::AsyncConnection;
use diesel_models::observability::{
    alerts_main::AlertsMain,
    merchants_alert_external::{MerchantsAlertExternal, MerchantsAlertExternalNew},
    merchants_alert_external_dimension::{
        MerchantsAlertExternalDimension, MerchantsAlertExternalDimensionNew,
    },
};
use error_stack::{report, ResultExt};

use crate::{
    core::utils::{self, TransactionError, NAME_MAX_CHARS, VALUE_MAX_CHARS},
    errors::{ObservabilityApiResult, ObservabilityError, StorageErrorExt},
    state::AppState,
    types::{
        instances::{
            DimensionInstanceEntry, DimensionListResponse, DimensionWriteRequest,
            InstanceListResponse, InstanceSaveResponse, InstanceWriteRequest,
            MerchantInstanceEntry,
        },
        lifecycle::Channel,
    },
};

const MAX_MERCHANTS: usize = 500;

const MAX_DIMENSIONS: usize = 500;

pub async fn read_instances(
    state: AppState,
    channel: Channel,
    announcement: uuid::Uuid,
) -> ObservabilityApiResult<InstanceListResponse> {
    let channel = <&'static str>::from(channel);
    let connection = state.database_connection().await?;

    AlertsMain::find_by_channel_and_id(&connection, channel, announcement)
        .await
        .to_not_found_response(ObservabilityError::UnknownAnnouncement {
            id: announcement.to_string(),
        })
        .attach_printable("Failed to find the announcement of the merchant alert instances")?;

    let merchants = MerchantsAlertExternal::list_by_channel_and_announcement(
        &connection,
        channel,
        announcement,
    )
    .await
    .change_context(ObservabilityError::InternalServerError)
    .attach_printable("Failed to list the merchant alert instances")?
    .into_iter()
    .map(MerchantInstanceEntry::from)
    .collect::<Vec<_>>();

    Ok(InstanceListResponse {
        count: merchants.len(),
        merchants,
    })
}

pub async fn write_instances(
    state: AppState,
    channel: Channel,
    announcement: uuid::Uuid,
    request: InstanceWriteRequest,
) -> ObservabilityApiResult<InstanceSaveResponse> {
    let channel = <&'static str>::from(channel);

    if request.merchants.len() > MAX_MERCHANTS {
        Err(report!(ObservabilityError::InstancesTooLarge {
            merchants: request.merchants.len(),
            limit: MAX_MERCHANTS,
        }))?;
    }

    for write in &request.merchants {
        utils::optional_within_width(write.name.as_deref(), "name", NAME_MAX_CHARS)?;
        utils::optional_within_width(write.product.as_deref(), "product", NAME_MAX_CHARS)?;
        utils::optional_within_width(write.merchant_id.as_deref(), "merchant_id", NAME_MAX_CHARS)?;
        utils::optional_within_width(write.priority.as_deref(), "priority", NAME_MAX_CHARS)?;
        utils::optional_within_width(write.tenant_id.as_deref(), "tenant_id", NAME_MAX_CHARS)?;
        utils::optional_within_width(write.attribution.as_deref(), "attribution", VALUE_MAX_CHARS)?;
        utils::optional_within_width(write.ts_slack.as_deref(), "ts_slack", VALUE_MAX_CHARS)?;
    }

    let now = utils::truncate_to_millisecond(common_utils::date_time::now());
    let connection = state.database_connection().await?;

    let borrowed = &connection;
    let (stored, removed) = borrowed
        .raw_connection()
        .transaction_async(move |_| async move {
            MerchantsAlertExternal::lock_announcement(borrowed, announcement)
                .await
                .change_context(ObservabilityError::InternalServerError)
                .attach_printable(
                    "Failed to lock the announcement for a merchant alert instance write",
                )?;

            let parent = AlertsMain::find_by_channel_and_id(borrowed, channel, announcement)
                .await
                .to_not_found_response(ObservabilityError::UnknownAnnouncement {
                    id: announcement.to_string(),
                })
                .attach_printable(
                    "Failed to find the announcement of the merchant alert instances",
                )?;

            let removed = MerchantsAlertExternal::delete_by_channel_and_announcement(
                borrowed,
                channel,
                announcement,
            )
            .await
            .change_context(ObservabilityError::InternalServerError)
            .attach_printable("Failed to delete the merchant alert instances")?;

            let rows = request
                .merchants
                .into_iter()
                .map(|write| MerchantsAlertExternalNew {
                    id: announcement,
                    channel: channel.to_owned(),
                    id_merchant_table: common_utils::generate_uuid_v7(),
                    id_intermediate: write.id_intermediate,
                    name: write.name.unwrap_or_default(),
                    product: write.product.unwrap_or_default(),
                    merchant_id: write.merchant_id.unwrap_or_default(),
                    dimensions: write.dimensions.unwrap_or_else(utils::empty_json_text),
                    auxiliary_dimensions: write
                        .auxiliary_dimensions
                        .unwrap_or_else(utils::empty_json_text),
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
                    slack_info: write.slack_info.unwrap_or_else(utils::empty_object),
                    communication_info: write
                        .communication_info
                        .unwrap_or_else(utils::empty_object),
                    metadata: write.metadata.unwrap_or_else(utils::empty_json_text),
                    metadata_alert_details: write
                        .metadata_alert_details
                        .unwrap_or_else(utils::empty_object),
                    priority: write.priority.unwrap_or_default(),
                    tenant_id: write.tenant_id.unwrap_or_default(),
                })
                .collect();

            let stored = MerchantsAlertExternalNew::bulk_insert(borrowed, rows)
                .await
                .change_context(ObservabilityError::InternalServerError)
                .attach_printable("Failed to insert the merchant alert instances")?;

            Ok::<_, TransactionError>((stored, removed))
        })
        .await
        .map_err(|error| {
            error.into_report("Failed to run the merchant alert instance write transaction")
        })?;

    Ok(InstanceSaveResponse {
        stored,
        removed,
        ts_alert: (stored > 0).then_some(now),
    })
}

pub async fn read_dimensions(
    state: AppState,
    channel: Channel,
    announcement: uuid::Uuid,
) -> ObservabilityApiResult<DimensionListResponse> {
    let channel = <&'static str>::from(channel);
    let connection = state.database_connection().await?;

    AlertsMain::find_by_channel_and_id(&connection, channel, announcement)
        .await
        .to_not_found_response(ObservabilityError::UnknownAnnouncement {
            id: announcement.to_string(),
        })
        .attach_printable("Failed to find the announcement of the alert dimensions")?;

    let dimensions = MerchantsAlertExternalDimension::list_by_channel_and_announcement(
        &connection,
        channel,
        announcement,
    )
    .await
    .change_context(ObservabilityError::InternalServerError)
    .attach_printable("Failed to list the alert dimensions")?
    .into_iter()
    .map(DimensionInstanceEntry::from)
    .collect::<Vec<_>>();

    Ok(DimensionListResponse {
        count: dimensions.len(),
        dimensions,
    })
}

pub async fn write_dimensions(
    state: AppState,
    channel: Channel,
    announcement: uuid::Uuid,
    request: DimensionWriteRequest,
) -> ObservabilityApiResult<InstanceSaveResponse> {
    let channel = <&'static str>::from(channel);

    if request.dimensions.len() > MAX_DIMENSIONS {
        Err(report!(ObservabilityError::DimensionsTooLarge {
            dimensions: request.dimensions.len(),
            limit: MAX_DIMENSIONS,
        }))?;
    }

    for write in &request.dimensions {
        utils::optional_within_width(write.name.as_deref(), "name", NAME_MAX_CHARS)?;
        utils::optional_within_width(write.product.as_deref(), "product", NAME_MAX_CHARS)?;
        utils::optional_within_width(
            write.dimension_key.as_deref(),
            "dimension_key",
            NAME_MAX_CHARS,
        )?;
        utils::optional_within_width(write.priority.as_deref(), "priority", NAME_MAX_CHARS)?;
        utils::optional_within_width(write.tenant_id.as_deref(), "tenant_id", NAME_MAX_CHARS)?;
        utils::optional_within_width(
            write.dimension_value.as_deref(),
            "dimension_value",
            VALUE_MAX_CHARS,
        )?;
        utils::optional_within_width(write.attribution.as_deref(), "attribution", VALUE_MAX_CHARS)?;
        utils::optional_within_width(write.ts_slack.as_deref(), "ts_slack", VALUE_MAX_CHARS)?;
    }

    let now = utils::truncate_to_millisecond(common_utils::date_time::now());
    let connection = state.database_connection().await?;

    let borrowed = &connection;
    let (stored, removed) = borrowed
        .raw_connection()
        .transaction_async(move |_| async move {
            MerchantsAlertExternalDimension::lock_announcement(borrowed, announcement)
                .await
                .change_context(ObservabilityError::InternalServerError)
                .attach_printable("Failed to lock the announcement for an alert dimension write")?;

            let parent = AlertsMain::find_by_channel_and_id(borrowed, channel, announcement)
                .await
                .to_not_found_response(ObservabilityError::UnknownAnnouncement {
                    id: announcement.to_string(),
                })
                .attach_printable("Failed to find the announcement of the alert dimensions")?;

            let removed = MerchantsAlertExternalDimension::delete_by_channel_and_announcement(
                borrowed,
                channel,
                announcement,
            )
            .await
            .change_context(ObservabilityError::InternalServerError)
            .attach_printable("Failed to delete the alert dimensions")?;

            let rows = request
                .dimensions
                .into_iter()
                .map(|write| MerchantsAlertExternalDimensionNew {
                    id: announcement,
                    channel: channel.to_owned(),
                    id_merchant_table: common_utils::generate_uuid_v7(),
                    id_intermediate: write.id_intermediate,
                    name: write.name.unwrap_or_default(),
                    product: write.product.unwrap_or_default(),
                    dimension_key: write.dimension_key.unwrap_or_default(),
                    dimension_value: write.dimension_value.unwrap_or_default(),
                    dimensions: write.dimensions.unwrap_or_else(utils::empty_json_text),
                    auxiliary_dimensions: write
                        .auxiliary_dimensions
                        .unwrap_or_else(utils::empty_json_text),
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
                    slack_info: write.slack_info.unwrap_or_else(utils::empty_object),
                    communication_info: write
                        .communication_info
                        .unwrap_or_else(utils::empty_object),
                    metadata: write.metadata.unwrap_or_else(utils::empty_json_text),
                    metadata_alert_details: write
                        .metadata_alert_details
                        .unwrap_or_else(utils::empty_object),
                    priority: write.priority.unwrap_or_default(),
                    tenant_id: write.tenant_id.unwrap_or_default(),
                })
                .collect();

            let stored = MerchantsAlertExternalDimensionNew::bulk_insert(borrowed, rows)
                .await
                .change_context(ObservabilityError::InternalServerError)
                .attach_printable("Failed to insert the alert dimensions")?;

            Ok::<_, TransactionError>((stored, removed))
        })
        .await
        .map_err(|error| {
            error.into_report("Failed to run the alert dimension write transaction")
        })?;

    Ok(InstanceSaveResponse {
        stored,
        removed,
        ts_alert: (stored > 0).then_some(now),
    })
}
