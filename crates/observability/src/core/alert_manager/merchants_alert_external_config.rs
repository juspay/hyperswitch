//! Per-request logic for merchant alert delivery switches.

use api_models::observability::alert_manager::merchants_alert_external_config::{
    MerchantsAlertExternalConfigCreateRequest, MerchantsAlertExternalConfigKey,
    MerchantsAlertExternalConfigListRequest, MerchantsAlertExternalConfigListResponse,
    MerchantsAlertExternalConfigResponse, MerchantsAlertExternalConfigUpdateRequest,
};
use error_stack::{report, ResultExt};

use crate::{
    domain_models::alert_manager::merchants_alert_external_config::{
        MerchantsAlertExternalConfigListFilter, MerchantsAlertExternalConfigNew,
        MerchantsAlertExternalConfigUpdate,
    },
    errors::{ObservabilityApiResult, ObservabilityError, StorageErrorExt},
    state::AppState,
};

/// Turn merchant delivery on for a defined alert.
pub async fn create_merchant_alert_external_config(
    state: AppState,
    request: MerchantsAlertExternalConfigCreateRequest,
) -> ObservabilityApiResult<MerchantsAlertExternalConfigResponse> {
    let new = MerchantsAlertExternalConfigNew::try_from(request)?;

    ensure_alert_info_exists(&state, &new.name, &new.product).await?;

    let stored = state
        .store
        .insert_merchant_alert_external_config(new)
        .await
        .to_duplicate_response(ObservabilityError::DuplicateResource)
        .attach_printable("Failed to insert into merchants_alert_external_config")?;

    Ok(MerchantsAlertExternalConfigResponse::from(stored))
}

/// Rows matching the given filter.
pub async fn list_merchant_alert_external_configs(
    state: AppState,
    request: MerchantsAlertExternalConfigListRequest,
) -> ObservabilityApiResult<MerchantsAlertExternalConfigListResponse> {
    let filter = MerchantsAlertExternalConfigListFilter::from(request);

    let rows = state
        .store
        .list_merchant_alert_external_configs_by_filter(filter)
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to list merchants_alert_external_config")?;

    let data: Vec<MerchantsAlertExternalConfigResponse> = rows
        .into_iter()
        .map(MerchantsAlertExternalConfigResponse::from)
        .collect();

    Ok(MerchantsAlertExternalConfigListResponse {
        count: data.len(),
        data,
    })
}

/// One row by key.
pub async fn retrieve_merchant_alert_external_config(
    state: AppState,
    request: MerchantsAlertExternalConfigKey,
) -> ObservabilityApiResult<MerchantsAlertExternalConfigResponse> {
    let row = state
        .store
        .find_merchant_alert_external_config_by_name_product(&request.name, &request.product)
        .await
        .to_not_found_response(ObservabilityError::ResourceNotFound)?;

    Ok(MerchantsAlertExternalConfigResponse::from(row))
}

/// Change the given fields, merging `metadata` and stamping `last_updated_at`.
pub async fn update_merchant_alert_external_config(
    state: AppState,
    request: MerchantsAlertExternalConfigUpdateRequest,
) -> ObservabilityApiResult<MerchantsAlertExternalConfigResponse> {
    let (name, product) = (request.name.clone(), request.product.clone());
    let update = MerchantsAlertExternalConfigUpdate::try_from(request)?;

    ensure_alert_info_exists(&state, &name, &product).await?;

    let updated = state
        .store
        .update_merchant_alert_external_config_by_name_product(&name, &product, update)
        .await
        .to_not_found_response(ObservabilityError::ResourceNotFound)?;

    Ok(MerchantsAlertExternalConfigResponse::from(updated))
}

/// Remove the row.
pub async fn delete_merchant_alert_external_config(
    state: AppState,
    request: MerchantsAlertExternalConfigKey,
) -> ObservabilityApiResult<MerchantsAlertExternalConfigResponse> {
    let deleted = state
        .store
        .delete_merchant_alert_external_config_by_name_product(&request.name, &request.product)
        .await
        .to_not_found_response(ObservabilityError::ResourceNotFound)?;

    Ok(MerchantsAlertExternalConfigResponse::from(deleted))
}

/// `(name, product)` must already be defined in `alerts_info`, enabled or not — replaces r-apps'
/// `BEFORE INSERT OR UPDATE` trigger.
async fn ensure_alert_info_exists(
    state: &AppState,
    name: &str,
    product: &str,
) -> ObservabilityApiResult<()> {
    let alert = state
        .store
        .find_alert_info_by_name_product(name, product)
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to look up alerts_info")?;

    if alert.is_none() {
        Err(report!(ObservabilityError::InvalidRequest)).attach_printable(format!(
            "name and product combination ({name}, {product}) does not exist in alerts_info"
        ))?;
    }

    Ok(())
}
