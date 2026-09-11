//! Per-request logic for the alert configuration resources:

use diesel_models::{
    errors::DatabaseError,
    observability::{
        alerts_info::AlertsInfo, merchants_alert_external_config::MerchantsAlertExternalConfig,
    },
};
use error_stack::report;

use crate::{
    alert_manager::types::config::{
        AlertDefinitionCreateRequest, AlertDefinitionListResponse, AlertDefinitionResponse,
        AlertDefinitionUpdateRequest, AlertEnablementListResponse, AlertEnablementResponse,
        AlertEnablementUpsertRequest,
    },
    errors::{ObservabilityApiResult, ObservabilityError},
    state::AppState,
};

/// Escalate a storage failure, keeping the report and everything attached to it.
fn escalate(
    error: error_stack::Report<DatabaseError>,
    recognise: impl FnOnce(DatabaseError) -> Option<ObservabilityError>,
) -> error_stack::Report<ObservabilityError> {
    let context =
        recognise(*error.current_context()).unwrap_or(ObservabilityError::StorageUnavailable);

    error.change_context(context)
}

/// No condition this route can describe better than "the database did not answer".
fn unrecognised(_: DatabaseError) -> Option<ObservabilityError> {
    None
}

/// Create a definition.
pub async fn create_definition(
    state: AppState,
    request: AlertDefinitionCreateRequest,
) -> ObservabilityApiResult<AlertDefinitionResponse> {
    let connection = state.database_connection().await?;
    let name = request.name.clone();
    let product = request.product.clone();

    request
        .into_insertable(common_utils::date_time::now())
        .insert(&connection)
        .await
        .map_err(|error| {
            escalate(error, |context| {
                matches!(context, DatabaseError::UniqueViolation)
                    .then_some(ObservabilityError::DuplicateDefinition { name, product })
            })
        })
        .map(AlertDefinitionResponse::from)
}

/// Read one definition by id.
pub async fn read_definition(
    state: AppState,
    id: uuid::Uuid,
) -> ObservabilityApiResult<AlertDefinitionResponse> {
    let connection = state.database_connection().await?;

    AlertsInfo::find_by_id(&connection, id)
        .await
        .map_err(|error| escalate(error, definition_not_found(id)))
        .map(AlertDefinitionResponse::from)
}

/// Every definition, including the reserved `all` row.
pub async fn list_definitions(
    state: AppState,
) -> ObservabilityApiResult<AlertDefinitionListResponse> {
    let connection = state.database_connection().await?;

    AlertsInfo::list(&connection)
        .await
        .map_err(|error| escalate(error, unrecognised))
        .map(|definitions| definitions.into_iter().collect())
}

/// Apply a partial change to a definition.
pub async fn update_definition(
    state: AppState,
    id: uuid::Uuid,
    request: AlertDefinitionUpdateRequest,
) -> ObservabilityApiResult<AlertDefinitionResponse> {
    let connection = state.database_connection().await?;

    AlertsInfo::update_by_id(
        &connection,
        id,
        request.into_changeset(common_utils::date_time::now()),
    )
    .await
    .map_err(|error| escalate(error, definition_not_found(id)))
    .map(AlertDefinitionResponse::from)
}

/// Recognise "no definition with this id", which only a route that knows the id can name.
fn definition_not_found(
    id: uuid::Uuid,
) -> impl FnOnce(DatabaseError) -> Option<ObservabilityError> {
    move |context| {
        matches!(context, DatabaseError::NotFound)
            .then(|| ObservabilityError::DefinitionNotFound { id: id.to_string() })
    }
}

/// Write the enablement row for a name and product.
pub async fn upsert_enablement(
    state: AppState,
    name: String,
    product: String,
    request: AlertEnablementUpsertRequest,
) -> ObservabilityApiResult<AlertEnablementResponse> {
    let connection = state.database_connection().await?;

    let definition = find_definition_for(&connection, &name, &product).await?;
    let definition_is_enabled = match definition {
        Some(definition) if !definition.is_all_definitions() => definition.is_enabled(),
        _ => Err(report!(ObservabilityError::NotAnAlert {
            name: name.clone(),
            product: product.clone(),
        }))?,
    };

    request
        .into_upsertable(name, product, common_utils::date_time::now())
        .upsert(&connection)
        .await
        .map_err(|error| escalate(error, unrecognised))
        .map(|row| AlertEnablementResponse::new(row, Some(definition_is_enabled)))
}

/// Read the enablement row for a name and product.
pub async fn read_enablement(
    state: AppState,
    name: String,
    product: String,
) -> ObservabilityApiResult<AlertEnablementResponse> {
    let connection = state.database_connection().await?;

    let row = MerchantsAlertExternalConfig::find_by_name_and_product(
        &connection,
        name.clone(),
        product.clone(),
    )
    .await
    .map_err(|error| {
        let (name, product) = (name.clone(), product.clone());
        escalate(error, |context| {
            matches!(context, DatabaseError::NotFound)
                .then_some(ObservabilityError::EnablementNotFound { name, product })
        })
    })?;

    let definition_is_enabled = find_definition_for(&connection, &name, &product)
        .await?
        .map(|definition| definition.is_enabled());

    Ok(AlertEnablementResponse::new(row, definition_is_enabled))
}

/// Every enablement row, each resolved against its definition.
pub async fn list_enablements(
    state: AppState,
) -> ObservabilityApiResult<AlertEnablementListResponse> {
    let connection = state.database_connection().await?;

    let rows = MerchantsAlertExternalConfig::list(&connection)
        .await
        .map_err(|error| escalate(error, unrecognised))?;

    let definitions = AlertsInfo::list(&connection)
        .await
        .map_err(|error| escalate(error, unrecognised))?
        .into_iter()
        .map(|definition| {
            (
                (definition.name.clone(), definition.product.clone()),
                definition.is_enabled(),
            )
        })
        .collect::<std::collections::HashMap<_, _>>();

    let enablements = rows
        .into_iter()
        .map(|row| {
            let definition_is_enabled = definitions
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

/// The definition for a name and product, if there is one.
async fn find_definition_for(
    connection: &diesel_models::DatabaseConnectionWithContext<'_>,
    name: &str,
    product: &str,
) -> ObservabilityApiResult<Option<AlertsInfo>> {
    AlertsInfo::find_optional_by_name_and_product(connection, name.to_owned(), product.to_owned())
        .await
        .map_err(|error| escalate(error, unrecognised))
}
