use crate::{
    core::errors::{api_error_response::NotImplementedMessage, ApiErrorResponse, RouterResult},
    routes::app::settings,
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
