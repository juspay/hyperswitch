use std::collections::HashMap;

use diesel_models::{
    errors::DatabaseError,
    observability::{
        alerts_info::AlertsInfo, merchant_thresholds::MerchantThreshold,
        merchants_alert_external_config::MerchantsAlertExternalConfig,
    },
};
use error_stack::{report, ResultExt};

use crate::{
    errors::{ObservabilityApiResult, ObservabilityError, StorageErrorExt},
    state::AppState,
    types::config::{
        AlertDefinitionCreateRequest, AlertDefinitionListResponse, AlertDefinitionResponse,
        AlertDefinitionUpdateRequest, AlertEnablementListResponse, AlertEnablementResponse,
        AlertEnablementUpsertRequest, MerchantThresholdDeleteResponse,
        MerchantThresholdListConstraints, MerchantThresholdListResponse, MerchantThresholdResponse,
        MerchantThresholdUpdateRequest, MerchantThresholdUpsertRequest,
    },
};

const ALL_DEFINITIONS: &str = "all";

pub async fn create_definition(
    state: AppState,
    request: AlertDefinitionCreateRequest,
) -> ObservabilityApiResult<AlertDefinitionResponse> {
    request.validate()?;

    let connection = state.database_connection().await?;
    let name = request.name.clone();
    let product = request.product.clone();

    request
        .into_insertable(
            common_utils::generate_uuid_v7(),
            common_utils::date_time::now(),
        )
        .insert(&connection)
        .await
        .to_duplicate_response(ObservabilityError::DuplicateDefinition { name, product })
        .attach_printable("Failed to insert the alert definition")
        .map(AlertDefinitionResponse::from)
}

pub async fn retrieve_definition(
    state: AppState,
    id: uuid::Uuid,
) -> ObservabilityApiResult<AlertDefinitionResponse> {
    let connection = state.database_connection().await?;

    AlertsInfo::find_by_id(&connection, id)
        .await
        .to_not_found_response(ObservabilityError::DefinitionNotFound { id: id.to_string() })
        .attach_printable("Failed to find the alert definition")
        .map(AlertDefinitionResponse::from)
}

pub async fn list_definitions(
    state: AppState,
) -> ObservabilityApiResult<AlertDefinitionListResponse> {
    let connection = state.database_connection().await?;

    let definitions = AlertsInfo::list(&connection)
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to list the alert definitions")?
        .into_iter()
        .map(AlertDefinitionResponse::from)
        .collect::<Vec<_>>();

    Ok(AlertDefinitionListResponse {
        count: definitions.len(),
        definitions,
    })
}

pub async fn update_definition(
    state: AppState,
    id: uuid::Uuid,
    request: AlertDefinitionUpdateRequest,
) -> ObservabilityApiResult<AlertDefinitionResponse> {
    request.validate()?;

    let connection = state.database_connection().await?;

    AlertsInfo::update_by_id(&connection, id, request.into())
        .await
        .to_not_found_response(ObservabilityError::DefinitionNotFound { id: id.to_string() })
        .attach_printable("Failed to update the alert definition")
        .map(AlertDefinitionResponse::from)
}

pub async fn upsert_enablement(
    state: AppState,
    name: String,
    product: String,
    request: AlertEnablementUpsertRequest,
) -> ObservabilityApiResult<AlertEnablementResponse> {
    request.validate()?;
    common_utils::fp_utils::when(name == ALL_DEFINITIONS, || {
        Err(report!(ObservabilityError::NotAnAlert {
            name: name.clone(),
            product: product.clone(),
        }))
    })?;

    let connection = state.database_connection().await?;

    let definition_is_enabled =
        AlertsInfo::find_optional_is_enabled_by_name_product(&connection, &name, &product)
            .await
            .change_context(ObservabilityError::InternalServerError)
            .attach_printable("Failed to find the alert definition for the enablement row")?
            .ok_or_else(|| {
                report!(ObservabilityError::NotAnAlert {
                    name: name.clone(),
                    product: product.clone(),
                })
            })?;

    request
        .to_insertable(name, product, common_utils::date_time::now())
        .upsert(&connection, request.into())
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to upsert the alert enablement row")
        .map(|row| AlertEnablementResponse::new(row, Some(definition_is_enabled)))
}

pub async fn retrieve_enablement(
    state: AppState,
    name: String,
    product: String,
) -> ObservabilityApiResult<AlertEnablementResponse> {
    let connection = state.database_connection().await?;

    let row = MerchantsAlertExternalConfig::find_by_name_product(&connection, &name, &product)
        .await
        .to_not_found_response(ObservabilityError::EnablementNotFound {
            name: name.clone(),
            product: product.clone(),
        })
        .attach_printable("Failed to find the alert enablement row")?;

    let definition_is_enabled =
        AlertsInfo::find_optional_is_enabled_by_name_product(&connection, &name, &product)
            .await
            .change_context(ObservabilityError::InternalServerError)
            .attach_printable("Failed to find the alert definition for the enablement row")?;

    Ok(AlertEnablementResponse::new(row, definition_is_enabled))
}

pub async fn list_enablements(
    state: AppState,
) -> ObservabilityApiResult<AlertEnablementListResponse> {
    let connection = state.database_connection().await?;

    let definitions_enabled = AlertsInfo::list_is_enabled(&connection)
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to list whether each alert definition is enabled")?
        .into_iter()
        .map(|(name, product, is_enabled)| ((name, product), is_enabled))
        .collect::<HashMap<_, _>>();

    let enablements = MerchantsAlertExternalConfig::list(&connection)
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to list the alert enablement rows")?
        .into_iter()
        .map(|row| {
            let definition_is_enabled = definitions_enabled
                .get(&(row.name.clone(), row.product.clone()))
                .copied();
            AlertEnablementResponse::new(row, definition_is_enabled)
        })
        .collect::<Vec<_>>();

    Ok(AlertEnablementListResponse {
        count: enablements.len(),
        enablements,
    })
}

pub async fn upsert_merchant_threshold(
    state: AppState,
    request: MerchantThresholdUpsertRequest,
) -> ObservabilityApiResult<MerchantThresholdResponse> {
    request.validate()?;

    let connection = state.database_connection().await?;

    request
        .to_insertable(
            common_utils::generate_uuid_v7(),
            common_utils::date_time::now(),
        )
        .upsert(&connection, request.into())
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to upsert the merchant threshold")
        .map(MerchantThresholdResponse::from)
}

pub async fn retrieve_merchant_threshold(
    state: AppState,
    id: uuid::Uuid,
) -> ObservabilityApiResult<MerchantThresholdResponse> {
    let connection = state.database_connection().await?;

    MerchantThreshold::find_by_id(&connection, id)
        .await
        .to_not_found_response(ObservabilityError::MerchantThresholdNotFound { id: id.to_string() })
        .attach_printable("Failed to find the merchant threshold")
        .map(MerchantThresholdResponse::from)
}

pub async fn list_merchant_thresholds(
    state: AppState,
    constraints: MerchantThresholdListConstraints,
) -> ObservabilityApiResult<MerchantThresholdListResponse> {
    let connection = state.database_connection().await?;

    let merchant_thresholds = MerchantThreshold::filter_by_constraints(
        &connection,
        constraints.name,
        constraints.product,
        constraints.merchant_id,
        constraints.is_enabled,
        constraints.author,
    )
    .await
    .change_context(ObservabilityError::InternalServerError)
    .attach_printable("Failed to list the merchant thresholds")?
    .into_iter()
    .map(MerchantThresholdResponse::from)
    .collect::<Vec<_>>();

    Ok(MerchantThresholdListResponse {
        count: merchant_thresholds.len(),
        merchant_thresholds,
    })
}

pub async fn update_merchant_threshold(
    state: AppState,
    id: uuid::Uuid,
    request: MerchantThresholdUpdateRequest,
) -> ObservabilityApiResult<MerchantThresholdResponse> {
    request.validate()?;

    let connection = state.database_connection().await?;

    MerchantThreshold::update_by_id(&connection, id, request.into())
        .await
        .map_err(|error| {
            let context = match error.current_context() {
                DatabaseError::NotFound => {
                    ObservabilityError::MerchantThresholdNotFound { id: id.to_string() }
                }
                DatabaseError::UniqueViolation => ObservabilityError::DuplicateMerchantThreshold,
                _ => ObservabilityError::InternalServerError,
            };
            error.change_context(context)
        })
        .attach_printable("Failed to update the merchant threshold")
        .map(MerchantThresholdResponse::from)
}

pub async fn delete_merchant_threshold(
    state: AppState,
    id: uuid::Uuid,
) -> ObservabilityApiResult<MerchantThresholdDeleteResponse> {
    let connection = state.database_connection().await?;

    MerchantThreshold::delete_by_id(&connection, id)
        .await
        .to_not_found_response(ObservabilityError::MerchantThresholdNotFound { id: id.to_string() })
        .attach_printable("Failed to delete the merchant threshold")
        .map(|deleted| MerchantThresholdDeleteResponse { id, deleted })
}
