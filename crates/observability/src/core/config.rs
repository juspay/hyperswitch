use std::collections::HashMap;

use diesel_models::observability::{
    alerts_info::AlertsInfo, merchants_alert_external_config::MerchantsAlertExternalConfig,
};
use error_stack::{report, ResultExt};

use crate::{
    errors::{ObservabilityApiResult, ObservabilityError, StorageErrorExt},
    state::AppState,
    types::config::{
        AlertDefinitionCreateRequest, AlertDefinitionListResponse, AlertDefinitionResponse,
        AlertDefinitionUpdateRequest, AlertEnablementListResponse, AlertEnablementResponse,
        AlertEnablementUpsertRequest,
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
        .into_insertable(common_utils::date_time::now())
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
    common_utils::fp_utils::when(name == ALL_DEFINITIONS, || {
        Err(report!(ObservabilityError::NotAnAlert {
            name: name.clone(),
            product: product.clone(),
        }))
    })?;

    let connection = state.database_connection().await?;

    let definition_is_enabled =
        AlertsInfo::find_optional_is_enabled_by_name_and_product(&connection, &name, &product)
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
        .map(|row| AlertEnablementResponse::new(row, definition_is_enabled))
}

pub async fn retrieve_enablement(
    state: AppState,
    name: String,
    product: String,
) -> ObservabilityApiResult<AlertEnablementResponse> {
    let connection = state.database_connection().await?;

    let row = MerchantsAlertExternalConfig::find_by_name_and_product(&connection, &name, &product)
        .await
        .to_not_found_response(ObservabilityError::EnablementNotFound {
            name: name.clone(),
            product: product.clone(),
        })
        .attach_printable("Failed to find the alert enablement row")?;

    let definition_is_enabled =
        AlertsInfo::find_optional_is_enabled_by_name_and_product(&connection, &name, &product)
            .await
            .change_context(ObservabilityError::InternalServerError)
            .attach_printable("Failed to find the alert definition for the enablement row")?
            .flatten();

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
                .copied()
                .flatten();
            AlertEnablementResponse::new(row, definition_is_enabled)
        })
        .collect::<Vec<_>>();

    Ok(AlertEnablementListResponse {
        count: enablements.len(),
        enablements,
    })
}
