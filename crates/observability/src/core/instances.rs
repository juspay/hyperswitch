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

pub async fn retrieve_instances(
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

pub async fn save_instances(
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
    request.validate()?;

    let now = truncate_to_millisecond(common_utils::date_time::now());
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
                .map(|write| write.into_insertable(common_utils::generate_uuid_v7(), &parent, now))
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

pub async fn retrieve_dimensions(
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

pub async fn save_dimensions(
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
    request.validate()?;

    let now = truncate_to_millisecond(common_utils::date_time::now());
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
                .map(|write| write.into_insertable(common_utils::generate_uuid_v7(), &parent, now))
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

enum TransactionError {
    Observability(error_stack::Report<ObservabilityError>),
    Database(diesel::result::Error),
}

impl TransactionError {
    fn into_report(self, message: &'static str) -> error_stack::Report<ObservabilityError> {
        match self {
            Self::Observability(error) => error,
            Self::Database(error) => report!(error)
                .change_context(ObservabilityError::InternalServerError)
                .attach_printable(message),
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

fn truncate_to_millisecond(value: PrimitiveDateTime) -> PrimitiveDateTime {
    value
        .replace_millisecond(value.millisecond())
        .unwrap_or(value)
}
