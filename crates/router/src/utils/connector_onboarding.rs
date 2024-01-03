use error_stack::ResultExt;

use super::errors::StorageErrorExt;
use crate::{
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
        .change_context(ApiErrorResponse::InternalServerError)?;

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
