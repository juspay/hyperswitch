use api_models::payments::DeviceChannel;
use common_enums::MerchantCategoryCode;
use common_types::payments::MerchantCountryCode;
use common_utils::types::MinorUnit;
use masking::Secret;

use crate::address::Address;

#[derive(Clone, Debug)]
pub struct UasPreAuthenticationRequestData {
    pub service_details: Option<CtpServiceDetails>,
    pub transaction_details: Option<TransactionDetails>,
    pub payment_details: Option<PaymentDetails>,
    pub authentication_info: Option<AuthenticationInfo>,
    pub merchant_details: Option<MerchantDetails>,
    pub billing_address: Option<Address>,
    pub acquirer_bin: Option<String>,
    pub acquirer_merchant_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MerchantDetails {
    pub merchant_id: Option<String>,
    pub merchant_name: Option<String>,
    pub merchant_category_code: Option<MerchantCategoryCode>,
    pub merchant_country_code: Option<MerchantCountryCode>,
    pub endpoint_prefix: Option<String>,
    pub three_ds_requestor_url: Option<String>,
    pub three_ds_requestor_id: Option<String>,
    pub three_ds_requestor_name: Option<String>,
    pub notification_url: Option<url::Url>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct AuthenticationInfo {
    pub authentication_type: Option<String>,
    pub authentication_reasons: Option<Vec<String>>,
    pub consent_received: bool,
    pub is_authenticated: bool,
    pub locale: Option<String>,
    pub supported_card_brands: Option<String>,
    pub encrypted_payload: Option<Secret<String>>,
}
#[derive(Clone, Debug)]
pub struct UasAuthenticationRequestData {
    pub browser_details: Option<super::BrowserInformation>,
    pub transaction_details: TransactionDetails,
    pub pre_authentication_data: super::authentication::PreAuthenticationData,
    pub return_url: Option<String>,
    pub sdk_information: Option<api_models::payments::SdkInformation>,
    pub email: Option<common_utils::pii::Email>,
    pub threeds_method_comp_ind: api_models::payments::ThreeDsCompletionIndicator,
    pub webhook_url: String,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct CtpServiceDetails {
    pub service_session_ids: Option<ServiceSessionIds>,
    pub payment_details: Option<PaymentDetails>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PaymentDetails {
    pub pan: cards::CardNumber,
    pub digital_card_id: Option<String>,
    pub payment_data_type: Option<common_enums::PaymentMethodType>,
    pub encrypted_src_card_details: Option<String>,
    pub card_expiry_month: Secret<String>,
    pub card_expiry_year: Secret<String>,
    pub cardholder_name: Option<Secret<String>>,
    pub card_token_number: Option<Secret<String>>,
    pub account_type: Option<common_enums::PaymentMethodType>,
    pub card_cvc: Option<Secret<String>>,
}

#[derive(Clone, serde::Deserialize, Debug, serde::Serialize)]
pub struct ServiceSessionIds {
    pub correlation_id: Option<String>,
    pub merchant_transaction_id: Option<String>,
    pub x_src_flow_id: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct TransactionDetails {
    pub amount: Option<MinorUnit>,
    pub currency: Option<common_enums::Currency>,
    pub device_channel: Option<DeviceChannel>,
    pub message_category: Option<super::authentication::MessageCategory>,
    pub force_3ds_challenge: Option<bool>,
    pub psd2_sca_exemption_type: Option<common_enums::ScaExemptionType>,
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
    Confirmation {},
}

#[derive(Debug, Clone, serde::Serialize)]
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
    pub authentication_value: Option<Secret<String>>,
    pub trans_status: common_enums::TransactionStatus,
    pub connector_metadata: Option<serde_json::Value>,
    pub ds_trans_id: Option<String>,
    pub eci: Option<String>,
    pub challenge_code: Option<String>,
    pub challenge_cancel: Option<String>,
    pub challenge_code_reason: Option<String>,
    pub message_extension: Option<common_utils::pii::SecretSerdeValue>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct PostAuthenticationDetails {
    pub eci: Option<String>,
    pub token_details: Option<TokenDetails>,
    pub dynamic_data_details: Option<DynamicData>,
    pub trans_status: Option<common_enums::TransactionStatus>,
    pub challenge_cancel: Option<String>,
    pub challenge_code_reason: Option<String>,
    pub raw_card_details: Option<RawCardDetails>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct RawCardDetails {
    pub pan: cards::CardNumber,
    pub expiration_month: Secret<String>,
    pub expiration_year: Secret<String>,
    pub card_security_code: Option<Secret<String>>,
    pub payment_account_reference: Option<String>,
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

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct UasConfirmationRequestData {
    pub x_src_flow_id: Option<String>,
    pub transaction_amount: MinorUnit,
    pub transaction_currency: common_enums::Currency,
    // Type of event associated with the checkout. Valid values are: - 01 - Authorise - 02 - Capture - 03 - Refund - 04 - Cancel - 05 - Fraud - 06 - Chargeback - 07 - Other
    pub checkout_event_type: Option<String>,
    pub checkout_event_status: Option<String>,
    pub confirmation_status: Option<String>,
    pub confirmation_reason: Option<String>,
    pub confirmation_timestamp: Option<String>,
    // Authorisation code associated with an approved transaction.
    pub network_authorization_code: Option<String>,
    // The unique authorisation related tracing value assigned by a Payment Network and provided in an authorisation response. Required only when checkoutEventType=01. If checkoutEventType=01 and the value of networkTransactionIdentifier is unknown, please pass UNAVLB
    pub network_transaction_identifier: Option<String>,
    pub correlation_id: Option<String>,
    pub merchant_transaction_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ThreeDsMetaData {
    pub merchant_category_code: Option<MerchantCategoryCode>,
    pub merchant_country_code: Option<MerchantCountryCode>,
    pub merchant_name: Option<String>,
    pub endpoint_prefix: Option<String>,
    pub three_ds_requestor_name: Option<String>,
    pub three_ds_requestor_id: Option<String>,
    pub merchant_configuration_id: Option<String>,
}

#[cfg(feature = "v1")]
impl From<PostAuthenticationDetails>
    for Option<api_models::authentication::AuthenticationPaymentMethodDataResponse>
{
    fn from(item: PostAuthenticationDetails) -> Self {
        match (item.raw_card_details, item.token_details) {
            (Some(card_data), _) => Some(
                api_models::authentication::AuthenticationPaymentMethodDataResponse::CardData {
                    card_expiry_year: Some(card_data.expiration_year),
                    card_expiry_month: Some(card_data.expiration_month),
                },
            ),
            (None, Some(network_token_data)) => {
                Some(
                    api_models::authentication::AuthenticationPaymentMethodDataResponse::NetworkTokenData {
                        network_token_expiry_year: Some(network_token_data.token_expiration_year),
                        network_token_expiry_month: Some(network_token_data.token_expiration_month),
                    },
                )
            }
            (None, None) => None,
        }
    }
}
