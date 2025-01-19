use api_models::payments::DeviceChannel;
use masking::Secret;

use crate::{address::Address, payment_method_data::PaymentMethodData};

#[derive(Clone, Debug)]
pub struct UasPreAuthenticationRequestData {
    pub service_details: Option<CtpServiceDetails>,
    pub transaction_details: Option<TransactionDetails>,
    pub pmt_details: Option<PaymentDetails>,
}

#[derive(Clone, Debug)]
pub struct UasAuthenticationRequestData {
    pub payment_method_data: PaymentMethodData,
    pub billing_address: Address,
    pub shipping_address: Option<Address>,
    pub browser_details: Option<super::BrowserInformation>,
    pub transaction_details: TransactionDetails,
    pub pre_authentication_data: super::authentication::PreAuthenticationData,
    pub return_url: Option<String>,
    pub sdk_information: Option<api_models::payments::SdkInformation>,
    pub email: Option<common_utils::pii::Email>,
    pub threeds_method_comp_ind: api_models::payments::ThreeDsCompletionIndicator,
    pub three_ds_requestor_url: String,
    pub webhook_url: String,
}

#[derive(Clone, Debug)]
pub struct CtpServiceDetails {
    pub service_session_ids: Option<ServiceSessionIds>,
    pub pmt_details: Option<PaymentDetails>,
}

#[derive(Debug, Clone)]
pub struct PaymentDetails {
    pub pan: cards::CardNumber,
    pub digital_card_id: Option<String>,
    pub payment_data_type: Option<String>,
    pub encrypted_src_card_details: Option<String>,
    pub card_expiry_date: Secret<String>,
    pub cardholder_name: Option<Secret<String>>,
    pub card_token_number: Secret<String>,
    pub account_type: Option<common_enums::CardNetwork>,
}

#[derive(Clone, serde::Deserialize, Debug, serde::Serialize)]
pub struct ServiceSessionIds {
    pub correlation_id: Option<String>,
    pub merchant_transaction_id: Option<String>,
    pub x_src_flow_id: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct TransactionDetails {
    pub amount: Option<common_utils::types::MinorUnit>,
    pub currency: Option<common_enums::Currency>,
    pub device_channel: Option<DeviceChannel>,
    pub message_category: Option<super::authentication::MessageCategory>,
}

#[derive(Clone, Debug)]
pub struct UasPostAuthenticationRequestData {
    pub threeds_server_transaction_id: Option<String>,
}

#[derive(Debug, Clone)]
pub enum UasAuthenticationResponseData {
    PreAuthentication {
        authentication_details: PreAuthenticationDetails,
    },
    Authentication {
        authentication_details: AuthenticationDetails,
    },
    PostAuthentication {
        authentication_details: PostAuthenticationDetails,
    },
}

#[derive(Debug, Clone)]
pub struct PreAuthenticationDetails {
    pub threeds_server_transaction_id: Option<String>,
    pub maximum_supported_3ds_version: Option<common_utils::types::SemanticVersion>,
    pub connector_authentication_id: Option<String>,
    pub three_ds_method_data: Option<String>,
    pub three_ds_method_url: Option<String>,
    pub message_version: Option<common_utils::types::SemanticVersion>,
    pub connector_metadata: Option<serde_json::Value>,
    pub directory_server_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AuthenticationDetails {
    pub authn_flow_type: super::authentication::AuthNFlowType,
    pub authentication_value: Option<String>,
    pub trans_status: common_enums::TransactionStatus,
    pub connector_metadata: Option<serde_json::Value>,
    pub ds_trans_id: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct PostAuthenticationDetails {
    pub eci: Option<String>,
    pub token_details: Option<TokenDetails>,
    pub dynamic_data_details: Option<DynamicData>,
    pub trans_status: Option<common_enums::TransactionStatus>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct TokenDetails {
    pub payment_token: cards::CardNumber,
    pub payment_account_reference: String,
    pub token_expiration_month: Secret<String>,
    pub token_expiration_year: Secret<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct DynamicData {
    pub dynamic_data_value: Option<Secret<String>>,
    pub dynamic_data_type: Option<String>,
    pub ds_trans_id: Option<String>,
}
