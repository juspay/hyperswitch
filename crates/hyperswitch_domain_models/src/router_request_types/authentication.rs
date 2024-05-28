use cards::CardNumber;
use common_utils::{ext_traits::OptionExt, pii::Email};
use error_stack::{Report, ResultExt};
use serde::{Deserialize, Serialize};

use crate::{
    errors::api_error_response::ApiErrorResponse, payment_method_data::PaymentMethodData,
    router_request_types::BrowserInformation,
};

#[derive(Debug, Clone)]
pub enum AuthenticationResponseData {
    PreAuthNResponse {
        threeds_server_transaction_id: String,
        maximum_supported_3ds_version: common_utils::types::SemanticVersion,
        connector_authentication_id: String,
        three_ds_method_data: Option<String>,
        three_ds_method_url: Option<String>,
        message_version: common_utils::types::SemanticVersion,
        connector_metadata: Option<serde_json::Value>,
    },
    AuthNResponse {
        authn_flow_type: AuthNFlowType,
        authentication_value: Option<String>,
        trans_status: common_enums::TransactionStatus,
    },
    PostAuthNResponse {
        trans_status: common_enums::TransactionStatus,
        authentication_value: Option<String>,
        eci: Option<String>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChallengeParams {
    pub acs_url: Option<url::Url>,
    pub challenge_request: Option<String>,
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

#[derive(Clone, Default, Debug)]
pub struct PreAuthNRequestData {
    // card number
    #[allow(dead_code)]
    pub(crate) card_holder_account_number: CardNumber,
}

#[derive(Clone, Debug)]
pub struct ConnectorAuthenticationRequestData {
    pub payment_method_data: PaymentMethodData,
    pub billing_address: api_models::payments::Address,
    pub shipping_address: Option<api_models::payments::Address>,
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
}

#[derive(Clone, Debug, Deserialize)]
pub struct ExternalThreeDSConnectorMetadata {
    pub pull_mechanism_for_external_3ds_enabled: Option<bool>,
}
