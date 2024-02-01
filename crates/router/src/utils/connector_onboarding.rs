use diesel_models::{ConfigNew, ConfigUpdate};
use error_stack::ResultExt;

use super::errors::StorageErrorExt;
use crate::{
    consts,
    core::errors::{api_error_response::NotImplementedMessage, ApiErrorResponse, RouterResult},
    routes::{app::settings, AppState},
    types::{self, api::enums},
};

pub mod paypal;

/// Returns the authentication type for a given connector based on the provided connector data. If the connector is PayPal, it returns a `BodyKey` containing the client secret and client id from the connector data. For any other connector, it returns a `NotImplemented` error with a message indicating that onboarding is not implemented for the given connector.
pub fn get_connector_auth(
    connector: enums::Connector,
    connector_data: &settings::ConnectorOnboarding,
) -> RouterResult<types::ConnectorAuthType> {
    match connector {
        enums::Connector::Paypal => Ok(types::ConnectorAuthType::BodyKey {
            api_key: connector_data.paypal.client_secret.clone(),
            key1: connector_data.paypal.client_id.clone(),
        }),
        _ => Err(ApiErrorResponse::NotImplemented {
            message: NotImplementedMessage::Reason(format!(
                "Onboarding is not implemented for {}",
                connector
            )),
        }
        .into()),
    }
}

/// Checks if a given connector is enabled based on the settings provided for onboarding.
/// 
/// # Arguments
/// * `connector` - The type of connector to check for enabled status.
/// * `conf` - The settings for connector onboarding.
/// 
/// # Returns
/// * `Some(true)` if the connector is enabled, `None` for any other connector type.
pub fn is_enabled(
    connector: types::Connector,
    conf: &settings::ConnectorOnboarding,
) -> Option<bool> {
    match connector {
        enums::Connector::Paypal => Some(conf.paypal.enabled),
        _ => None,
    }
}

/// Checks if a connector exists for a given merchant by querying the database using the provided `state`, `connector_id`, and `merchant_id`. 
///
/// # Arguments
///
/// * `state` - The application state containing the database connection and other necessary information.
/// * `connector_id` - The unique identifier of the connector to be checked.
/// * `merchant_id` - The unique identifier of the merchant for whom the connector is being checked.
///
/// # Returns
///
/// This method returns a `RouterResult` indicating whether the connector exists or not. If the connector exists, it returns `Ok(())`, otherwise it returns an error response.
pub async fn check_if_connector_exists(
    state: &AppState,
    connector_id: &str,
    merchant_id: &str,
) -> RouterResult<()> {
    let key_store = state
        .store
        .get_merchant_key_store_by_merchant_id(
            merchant_id,
            &state.store.get_master_key().to_vec().into(),
        )
        .await
        .to_not_found_response(ApiErrorResponse::MerchantAccountNotFound)?;

    let _connector = state
        .store
        .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
            merchant_id,
            connector_id,
            &key_store,
        )
        .await
        .to_not_found_response(ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: connector_id.to_string(),
        })?;

    Ok(())
}

/// Sets the tracking id in the configs table for the given connector and connector id in the application state.
pub async fn set_tracking_id_in_configs(
    state: &AppState,
    connector_id: &str,
    connector: enums::Connector,
) -> RouterResult<()> {
    let timestamp = common_utils::date_time::now_unix_timestamp().to_string();
    let find_config = state
        .store
        .find_config_by_key(&build_key(connector_id, connector))
        .await;

    if find_config.is_ok() {
        state
            .store
            .update_config_by_key(
                &build_key(connector_id, connector),
                ConfigUpdate::Update {
                    config: Some(timestamp),
                },
            )
            .await
            .change_context(ApiErrorResponse::InternalServerError)
            .attach_printable("Error updating data in configs table")?;
    } else if find_config
        .as_ref()
        .map_err(|e| e.current_context().is_db_not_found())
        .err()
        .unwrap_or(false)
    {
        state
            .store
            .insert_config(ConfigNew {
                key: build_key(connector_id, connector),
                config: timestamp,
            })
            .await
            .change_context(ApiErrorResponse::InternalServerError)
            .attach_printable("Error inserting data in configs table")?;
    } else {
        find_config.change_context(ApiErrorResponse::InternalServerError)?;
    }

    Ok(())
}

/// Retrieves the tracking ID from the specified connector configurations in the given state. 
/// It uses the connector ID and enum to construct a key and retrieve the corresponding timestamp 
/// from the store. It then formats and returns the tracking ID as a string in the format: 
/// "{connector_id}_{timestamp}".
pub async fn get_tracking_id_from_configs(
    state: &AppState,
    connector_id: &str,
    connector: enums::Connector,
) -> RouterResult<String> {
    let timestamp = state
        .store
        .find_config_by_key_unwrap_or(
            &build_key(connector_id, connector),
            Some(common_utils::date_time::now_unix_timestamp().to_string()),
        )
        .await
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Error getting data from configs table")?
        .config;

    Ok(format!("{}_{}", connector_id, timestamp))
}

/// This method takes a connector ID and a connector enum and builds a key using the `CONNECTOR_ONBOARDING_CONFIG_PREFIX` constant as a prefix. The key is formatted as `CONNECTOR_ONBOARDING_CONFIG_PREFIX_connector_connector_id`.
fn build_key(connector_id: &str, connector: enums::Connector) -> String {
    format!(
        "{}_{}_{}",
        consts::CONNECTOR_ONBOARDING_CONFIG_PREFIX,
        connector,
        connector_id,
    )
}
