use api_models::payments;
use cards::CardNumber;
use common_utils::pii::Email;
use serde::{Deserialize, Serialize};

use super::{
    api::{self, authentication},
    storage, BrowserInformation, RouterData,
};
use crate::services;

#[derive(Debug, Clone)]
pub enum AuthenticationResponseData {
    PreAuthNResponse {
        threeds_server_transaction_id: String,
        maximum_supported_3ds_version: (i64, i64, i64),
        connector_authentication_id: String,
        three_ds_method_data: String,
        three_ds_method_url: Option<String>,
        message_version: String,
        connector_metadata: Option<serde_json::Value>,
    },
    AuthNResponse {
        authn_flow_type: AuthNFlowType,
        authentication_value: Option<String>,
        trans_status: api_models::payments::TransactionStatus,
    },
    PostAuthNResponse {
        trans_status: api_models::payments::TransactionStatus,
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

#[derive(Clone, Default, Debug)]
pub struct PreAuthNRequestData {
    // card number
    pub card_holder_account_number: CardNumber,
}

#[derive(Clone, Debug)]
pub struct ConnectorAuthenticationRequestData {
    pub payment_method_data: payments::PaymentMethodData,
    pub billing_address: api_models::payments::Address,
    pub shipping_address: Option<api_models::payments::Address>,
    pub browser_details: Option<BrowserInformation>,
    pub amount: Option<i64>,
    pub currency: Option<common_enums::Currency>,
    pub message_category: authentication::MessageCategory,
    pub device_channel: api_models::payments::DeviceChannel,
    pub authentication_data: (
        crate::core::authentication::types::AuthenticationData,
        storage::Authentication,
    ),
    pub return_url: Option<String>,
    pub sdk_information: Option<api_models::payments::SdkInformation>,
    pub email: Option<Email>,
    pub threeds_method_comp_ind: api_models::payments::ThreeDsCompletionIndicator,
    pub three_ds_requestor_url: String,
}

#[derive(Clone, Debug)]
pub struct ConnectorPostAuthenticationRequestData {
    pub authentication_data: crate::core::authentication::types::AuthenticationData,
}

pub type PreAuthNRouterData =
    RouterData<api::PreAuthentication, PreAuthNRequestData, AuthenticationResponseData>;

pub type ConnectorAuthenticationRouterData =
    RouterData<api::Authentication, ConnectorAuthenticationRequestData, AuthenticationResponseData>;

pub type ConnectorPostAuthenticationRouterData = RouterData<
    api::PostAuthentication,
    ConnectorPostAuthenticationRequestData,
    AuthenticationResponseData,
>;

pub type ConnectorAuthenticationType = dyn services::ConnectorIntegration<
    api::Authentication,
    ConnectorAuthenticationRequestData,
    AuthenticationResponseData,
>;

pub type ConnectorPostAuthenticationType = dyn services::ConnectorIntegration<
    api::PostAuthentication,
    ConnectorPostAuthenticationRequestData,
    AuthenticationResponseData,
>;

pub type ConnectorPreAuthenticationType = dyn services::ConnectorIntegration<
    api::PreAuthentication,
    PreAuthNRequestData,
    AuthenticationResponseData,
>;
