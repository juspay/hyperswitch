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

pub fn is_enabled(
    connector: types::Connector,
    conf: &settings::ConnectorOnboarding,
) -> Option<bool> {
    match connector {
        enums::Connector::Paypal => Some(conf.paypal.enabled),
        _ => None,
    }
}

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

fn build_key(connector_id: &str, connector: enums::Connector) -> String {
    format!(
        "{}_{}_{}",
        consts::CONNECTOR_ONBOARDING_CONFIG_PREFIX,
        connector,
        connector_id,
    )
}
