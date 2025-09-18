use common_utils::{ext_traits::OptionExt, pii::Email};
use error_stack::{Report, ResultExt};
use serde::{Deserialize, Serialize};

use crate::{
    address,
    errors::api_error_response::ApiErrorResponse,
    payment_method_data::{Card, PaymentMethodData},
    router_request_types::BrowserInformation,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChallengeParams {
    pub acs_url: Option<url::Url>,
    pub challenge_request: Option<String>,
    pub challenge_request_key: Option<String>,
    pub acs_reference_number: Option<String>,
    pub acs_trans_id: Option<String>,
    pub three_dsserver_trans_id: Option<String>,
    pub acs_signed_content: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AuthNFlowType {
    Challenge(Box<ChallengeParams>),
    Frictionless,
}

impl AuthNFlowType {
    pub fn get_acs_url(&self) -> Option<String> {
        if let Self::Challenge(challenge_params) = self {
            challenge_params.acs_url.as_ref().map(ToString::to_string)
        } else {
            None
        }
    }
    pub fn get_challenge_request(&self) -> Option<String> {
        if let Self::Challenge(challenge_params) = self {
            challenge_params.challenge_request.clone()
        } else {
            None
        }
    }
    pub fn get_challenge_request_key(&self) -> Option<String> {
        if let Self::Challenge(challenge_params) = self {
            challenge_params.challenge_request_key.clone()
        } else {
            None
        }
    }
    pub fn get_acs_reference_number(&self) -> Option<String> {
        if let Self::Challenge(challenge_params) = self {
            challenge_params.acs_reference_number.clone()
        } else {
            None
        }
    }
    pub fn get_acs_trans_id(&self) -> Option<String> {
        if let Self::Challenge(challenge_params) = self {
            challenge_params.acs_trans_id.clone()
        } else {
            None
        }
    }
    pub fn get_acs_signed_content(&self) -> Option<String> {
        if let Self::Challenge(challenge_params) = self {
            challenge_params.acs_signed_content.clone()
        } else {
            None
        }
    }
    pub fn get_decoupled_authentication_type(&self) -> common_enums::DecoupledAuthenticationType {
        match self {
            Self::Challenge(_) => common_enums::DecoupledAuthenticationType::Challenge,
            Self::Frictionless => common_enums::DecoupledAuthenticationType::Frictionless,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MessageExtensionAttribute {
    pub id: String,
    pub name: String,
    pub criticality_indicator: bool,
    pub data: serde_json::Value,
}

#[derive(Clone, Default, Debug)]
pub struct PreAuthNRequestData {
    // card data
    pub card: Card,
}

#[derive(Clone, Debug)]
pub struct ConnectorAuthenticationRequestData {
    pub payment_method_data: PaymentMethodData,
    pub billing_address: address::Address,
    pub shipping_address: Option<address::Address>,
    pub browser_details: Option<BrowserInformation>,
    pub amount: Option<i64>,
    pub currency: Option<common_enums::Currency>,
    pub message_category: MessageCategory,
    pub device_channel: api_models::payments::DeviceChannel,
    pub pre_authentication_data: PreAuthenticationData,
    pub return_url: Option<String>,
    pub sdk_information: Option<api_models::payments::SdkInformation>,
    pub email: Option<Email>,
    pub threeds_method_comp_ind: api_models::payments::ThreeDsCompletionIndicator,
    pub three_ds_requestor_url: String,
    pub webhook_url: String,
    pub force_3ds_challenge: bool,
}

#[derive(Clone, serde::Deserialize, Debug, serde::Serialize, PartialEq, Eq)]
pub enum MessageCategory {
    Payment,
    NonPayment,
}

#[derive(Clone, Debug)]
pub struct ConnectorPostAuthenticationRequestData {
    pub threeds_server_transaction_id: String,
}

#[derive(Clone, Debug)]
pub struct PreAuthenticationData {
    pub threeds_server_transaction_id: String,
    pub message_version: common_utils::types::SemanticVersion,
    pub acquirer_bin: Option<String>,
    pub acquirer_merchant_id: Option<String>,
    pub acquirer_country_code: Option<String>,
    pub connector_metadata: Option<serde_json::Value>,
}

impl TryFrom<&diesel_models::authentication::Authentication> for PreAuthenticationData {
    type Error = Report<ApiErrorResponse>;

    fn try_from(
        authentication: &diesel_models::authentication::Authentication,
    ) -> Result<Self, Self::Error> {
        let error_message = ApiErrorResponse::UnprocessableEntity { message: "Pre Authentication must be completed successfully before Authentication can be performed".to_string() };
        let threeds_server_transaction_id = authentication
            .threeds_server_transaction_id
            .clone()
            .get_required_value("threeds_server_transaction_id")
            .change_context(error_message.clone())?;
        let message_version = authentication
            .message_version
            .clone()
            .get_required_value("message_version")
            .change_context(error_message)?;
        Ok(Self {
            threeds_server_transaction_id,
            message_version,
            acquirer_bin: authentication.acquirer_bin.clone(),
            acquirer_merchant_id: authentication.acquirer_merchant_id.clone(),
            connector_metadata: authentication.connector_metadata.clone(),
            acquirer_country_code: authentication.acquirer_country_code.clone(),
        })
    }
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct ThreeDsMethodData {
    pub three_ds_method_data_submission: bool,
    pub three_ds_method_data: String,
    pub three_ds_method_url: Option<String>,
}
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct AcquirerDetails {
    pub acquirer_bin: String,
    pub acquirer_merchant_id: String,
    pub acquirer_country_code: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ExternalThreeDSConnectorMetadata {
    pub pull_mechanism_for_external_3ds_enabled: Option<bool>,
}

#[derive(Clone, Debug)]
pub struct AuthenticationStore {
    pub cavv: Option<masking::Secret<String>>,
    pub authentication: diesel_models::authentication::Authentication,
}
