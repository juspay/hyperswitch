use diesel_models::observability::{
    alerts_info::{AlertsInfo, Snooze},
    merchants_alert_external_config::MerchantsAlertExternalConfig,
};
use error_stack::{report, ResultExt};
use time::PrimitiveDateTime;

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

const SNOOZE_ENTRY_PREFIXES: [&str; 2] = ["snooze_entry_", "custom_snooze_entry_"];

const SNOOZE_TIME_FORMAT: &[time::format_description::FormatItem<'static>] =
    time::macros::format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");

pub async fn create_definition(
    state: AppState,
    request: AlertDefinitionCreateRequest,
) -> ObservabilityApiResult<AlertDefinitionResponse> {
    request.snooze.as_ref().map(validate_snooze).transpose()?;

    let connection = state.database_connection().await?;
    let name = request.name.clone();
    let product = request.product.clone();

    request
        .into_insertable(common_utils::date_time::now())
        .insert(&connection)
        .await
        .to_duplicate_response(ObservabilityError::DuplicateDefinition { name, product })
        .map(AlertDefinitionResponse::from)
}

pub async fn read_definition(
    state: AppState,
    id: uuid::Uuid,
) -> ObservabilityApiResult<AlertDefinitionResponse> {
    let connection = state.database_connection().await?;

    AlertsInfo::find_by_id(&connection, id)
        .await
        .to_not_found_response(ObservabilityError::DefinitionNotFound { id: id.to_string() })
        .map(AlertDefinitionResponse::from)
}

pub async fn list_definitions(
    state: AppState,
) -> ObservabilityApiResult<AlertDefinitionListResponse> {
    let connection = state.database_connection().await?;

    AlertsInfo::list(&connection)
        .await
        .change_context(ObservabilityError::InternalServerError)
        .map(|definitions| definitions.into_iter().collect())
}

pub async fn update_definition(
    state: AppState,
    id: uuid::Uuid,
    request: AlertDefinitionUpdateRequest,
) -> ObservabilityApiResult<AlertDefinitionResponse> {
    request
        .snooze
        .as_ref()
        .and_then(Option::as_ref)
        .map(validate_snooze)
        .transpose()?;

    let connection = state.database_connection().await?;

    AlertsInfo::update_by_id(
        &connection,
        id,
        request.into_changeset(common_utils::date_time::now()),
    )
    .await
    .to_not_found_response(ObservabilityError::DefinitionNotFound { id: id.to_string() })
    .map(AlertDefinitionResponse::from)
}

pub async fn upsert_enablement(
    state: AppState,
    name: String,
    product: String,
    request: AlertEnablementUpsertRequest,
) -> ObservabilityApiResult<AlertEnablementResponse> {
    let connection = state.database_connection().await?;

    let definition = find_definition_for(&connection, &name, &product).await?;
    let definition_is_enabled = match definition {
        Some(definition) if definition.name != ALL_DEFINITIONS => {
            definition.is_enabled.unwrap_or(false)
        }
        _ => Err(report!(ObservabilityError::NotAnAlert {
            name: name.clone(),
            product: product.clone(),
        }))?,
    };

    request
        .into_upsertable(name, product, common_utils::date_time::now())
        .upsert(&connection)
        .await
        .change_context(ObservabilityError::InternalServerError)
        .map(|row| AlertEnablementResponse::new(row, Some(definition_is_enabled)))
}

pub async fn read_enablement(
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
        })?;

    let definition_is_enabled = find_definition_for(&connection, &name, &product)
        .await?
        .map(|definition| definition.is_enabled.unwrap_or(false));

    Ok(AlertEnablementResponse::new(row, definition_is_enabled))
}

pub async fn list_enablements(
    state: AppState,
) -> ObservabilityApiResult<AlertEnablementListResponse> {
    let connection = state.database_connection().await?;

    let rows = MerchantsAlertExternalConfig::list(&connection)
        .await
        .change_context(ObservabilityError::InternalServerError)?;

    let definitions = AlertsInfo::list(&connection)
        .await
        .change_context(ObservabilityError::InternalServerError)?
        .into_iter()
        .map(|definition| {
            (
                (definition.name.clone(), definition.product.clone()),
                definition.is_enabled.unwrap_or(false),
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

async fn find_definition_for(
    connection: &diesel_models::DatabaseConnectionWithContext<'_>,
    name: &str,
    product: &str,
) -> ObservabilityApiResult<Option<AlertsInfo>> {
    AlertsInfo::find_optional_by_name_and_product(connection, name, product)
        .await
        .change_context(ObservabilityError::InternalServerError)
}

fn validate_snooze(snooze: &Snooze) -> ObservabilityApiResult<()> {
    for (key, entry) in &snooze.0 {
        let keyed = SNOOZE_ENTRY_PREFIXES
            .iter()
            .any(|prefix| key.starts_with(prefix));
        let end_readable =
            PrimitiveDateTime::parse(&entry.snooze_end_time, SNOOZE_TIME_FORMAT).is_ok();
        let start_readable = entry
            .snooze_start_time
            .as_deref()
            .is_none_or(|start| PrimitiveDateTime::parse(start, SNOOZE_TIME_FORMAT).is_ok());

        if !(keyed && end_readable && start_readable) {
            Err(report!(ObservabilityError::InvalidSnooze {
                key: key.clone()
            }))?;
        }
    }

    Ok(())
}
