use std::str::FromStr;

use api_models::enums;
use common_utils::errors::CustomResult;
use error_stack::ResultExt;
pub use hyperswitch_domain_models::{
    router_flow_types::authentication::{
        Authentication, PostAuthentication, PreAuthentication, PreAuthenticationVersionCall,
    },
    router_request_types::authentication::MessageCategory,
};

use crate::{
    connector, core::errors, services::connector_integration_interface::ConnectorEnum,
    types::storage,
};

#[derive(Clone, serde::Deserialize, Debug, serde::Serialize)]
pub struct AcquirerDetails {
    pub acquirer_bin: String,
    pub acquirer_merchant_mid: String,
    pub acquirer_country_code: Option<String>,
}

#[derive(Clone, serde::Deserialize, Debug, serde::Serialize)]
pub struct AuthenticationResponse {
    pub trans_status: common_enums::TransactionStatus,
    pub acs_url: Option<url::Url>,
    pub challenge_request: Option<String>,
    pub challenge_request_key: Option<String>,
    pub acs_reference_number: Option<String>,
    pub acs_trans_id: Option<String>,
    pub three_dsserver_trans_id: Option<String>,
    pub acs_signed_content: Option<String>,
}

impl TryFrom<storage::Authentication> for AuthenticationResponse {
    type Error = error_stack::Report<errors::ApiErrorResponse>;
    fn try_from(authentication: storage::Authentication) -> Result<Self, Self::Error> {
        let trans_status = authentication.trans_status.ok_or(errors::ApiErrorResponse::InternalServerError).attach_printable("trans_status must be populated in authentication table authentication call is successful")?;
        let acs_url = authentication
            .acs_url
            .map(|url| url::Url::from_str(&url))
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("not a valid URL")?;
        Ok(Self {
            trans_status,
            acs_url,
            challenge_request: authentication.challenge_request,
            acs_reference_number: authentication.acs_reference_number,
            acs_trans_id: authentication.acs_trans_id,
            three_dsserver_trans_id: authentication.threeds_server_transaction_id,
            acs_signed_content: authentication.acs_signed_content,
            challenge_request_key: authentication.challenge_request_key,
        })
    }
}

#[derive(Clone, serde::Deserialize, Debug, serde::Serialize)]
pub struct PostAuthenticationResponse {
    pub trans_status: String,
    pub authentication_value: Option<masking::Secret<String>>,
    pub eci: Option<String>,
}

#[derive(Clone)]
pub struct AuthenticationConnectorData {
    pub connector: ConnectorEnum,
    pub connector_name: enums::AuthenticationConnectors,
}

impl AuthenticationConnectorData {
    pub fn get_connector_by_name(name: &str) -> CustomResult<Self, errors::ApiErrorResponse> {
        let connector_name = enums::AuthenticationConnectors::from_str(name)
            .change_context(errors::ApiErrorResponse::IncorrectConnectorNameGiven)
            .attach_printable_lazy(|| format!("unable to parse connector: {name}"))?;
        let connector = Self::convert_connector(connector_name)?;
        Ok(Self {
            connector,
            connector_name,
        })
    }

    fn convert_connector(
        connector_name: enums::AuthenticationConnectors,
    ) -> CustomResult<ConnectorEnum, errors::ApiErrorResponse> {
        match connector_name {
            enums::AuthenticationConnectors::Threedsecureio => {
                Ok(ConnectorEnum::Old(Box::new(&connector::Threedsecureio)))
            }
            enums::AuthenticationConnectors::Netcetera => {
                Ok(ConnectorEnum::Old(Box::new(&connector::Netcetera)))
            }
            enums::AuthenticationConnectors::Gpayments => {
                Ok(ConnectorEnum::Old(Box::new(connector::Gpayments::new())))
            }
            enums::AuthenticationConnectors::CtpMastercard => {
                Ok(ConnectorEnum::Old(Box::new(&connector::CtpMastercard)))
            }
            enums::AuthenticationConnectors::CtpVisa => Ok(ConnectorEnum::Old(Box::new(
                connector::UnifiedAuthenticationService::new(),
            ))),
            enums::AuthenticationConnectors::UnifiedAuthenticationService => Ok(
                ConnectorEnum::Old(Box::new(connector::UnifiedAuthenticationService::new())),
            ),
            enums::AuthenticationConnectors::Juspaythreedsserver => Ok(ConnectorEnum::Old(
                Box::new(connector::Juspaythreedsserver::new()),
            )),
            enums::AuthenticationConnectors::Cardinal => Ok(ConnectorEnum::Old(Box::new(
                connector::UnifiedAuthenticationService::new(),
            ))),
        }
    }
}
