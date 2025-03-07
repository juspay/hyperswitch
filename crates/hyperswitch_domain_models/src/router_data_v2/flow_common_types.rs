use common_utils::{pii, types::MinorUnit};

use crate::{
    payment_address::PaymentAddress,
    payment_method_data::ApplePayFlow,
    router_data::{
        AccessToken, ConnectorResponseData, PaymentMethodBalance, PaymentMethodToken,
        RecurringMandatePaymentData,
    },
};

#[derive(Debug, Clone)]
pub struct PaymentFlowData {
    pub merchant_id: common_utils::id_type::MerchantId,
    pub customer_id: Option<common_utils::id_type::CustomerId>,
    pub connector_customer: Option<String>,
    pub payment_id: String,
    pub attempt_id: String,
    pub status: common_enums::AttemptStatus,
    pub payment_method: common_enums::PaymentMethod,
    pub description: Option<String>,
    pub address: PaymentAddress,
    pub auth_type: common_enums::AuthenticationType,
    pub connector_meta_data: Option<pii::SecretSerdeValue>,
    pub amount_captured: Option<i64>,
    // minor amount for amount framework
    pub minor_amount_captured: Option<MinorUnit>,
    pub access_token: Option<AccessToken>,
    pub session_token: Option<String>,
    pub reference_id: Option<String>,
    pub payment_method_token: Option<PaymentMethodToken>,
    pub recurring_mandate_payment_data: Option<RecurringMandatePaymentData>,
    pub preprocessing_id: Option<String>,
    /// This is the balance amount for gift cards or voucher
    pub payment_method_balance: Option<PaymentMethodBalance>,

    ///for switching between two different versions of the same connector
    pub connector_api_version: Option<String>,
    /// Contains a reference ID that should be sent in the connector request
    pub connector_request_reference_id: String,

    pub test_mode: Option<bool>,
    pub connector_http_status_code: Option<u16>,
    pub external_latency: Option<u128>,
    /// Contains apple pay flow type simplified or manual
    pub apple_pay_flow: Option<ApplePayFlow>,

    /// This field is used to store various data regarding the response from connector
    pub connector_response: Option<ConnectorResponseData>,
    pub payment_method_status: Option<common_enums::PaymentMethodStatus>,
}

#[derive(Debug, Clone)]
pub struct RefundFlowData {
    pub merchant_id: common_utils::id_type::MerchantId,
    pub customer_id: Option<common_utils::id_type::CustomerId>,
    pub payment_id: String,
    pub attempt_id: String,
    pub status: common_enums::AttemptStatus,
    pub payment_method: common_enums::PaymentMethod,
    pub connector_meta_data: Option<pii::SecretSerdeValue>,
    pub amount_captured: Option<i64>,
    // minor amount for amount framework
    pub minor_amount_captured: Option<MinorUnit>,
    /// Contains a reference ID that should be sent in the connector request
    pub connector_request_reference_id: String,
    pub refund_id: String,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone)]
pub struct PayoutFlowData {
    pub merchant_id: common_utils::id_type::MerchantId,
    pub customer_id: Option<common_utils::id_type::CustomerId>,
    pub connector_customer: Option<String>,
    pub address: PaymentAddress,
    pub connector_meta_data: Option<pii::SecretSerdeValue>,
    pub connector_wallets_details: Option<pii::SecretSerdeValue>,
    /// Contains a reference ID that should be sent in the connector request
    pub connector_request_reference_id: String,
    pub payout_method_data: Option<api_models::payouts::PayoutMethodData>,
    pub quote_id: Option<String>,
}

#[cfg(feature = "frm")]
#[derive(Debug, Clone)]
pub struct FrmFlowData {
    pub merchant_id: common_utils::id_type::MerchantId,
    pub payment_id: String,
    pub attempt_id: String,
    pub payment_method: common_enums::enums::PaymentMethod,
    pub connector_request_reference_id: String,
    pub auth_type: common_enums::enums::AuthenticationType,
    pub connector_wallets_details: Option<pii::SecretSerdeValue>,
    pub connector_meta_data: Option<pii::SecretSerdeValue>,
    pub amount_captured: Option<i64>,
    // minor amount for amount framework
    pub minor_amount_captured: Option<MinorUnit>,
}

#[derive(Debug, Clone)]
pub struct ExternalAuthenticationFlowData {
    pub merchant_id: common_utils::id_type::MerchantId,
    pub connector_meta_data: Option<pii::SecretSerdeValue>,
    pub address: PaymentAddress,
}

#[derive(Debug, Clone)]
pub struct DisputesFlowData {
    pub merchant_id: common_utils::id_type::MerchantId,
    pub payment_id: String,
    pub attempt_id: String,
    pub payment_method: common_enums::enums::PaymentMethod,
    pub connector_meta_data: Option<pii::SecretSerdeValue>,
    pub amount_captured: Option<i64>,
    // minor amount for amount framework
    pub minor_amount_captured: Option<MinorUnit>,
    /// Contains a reference ID that should be sent in the connector request
    pub connector_request_reference_id: String,
    pub dispute_id: String,
}

#[derive(Debug, Clone)]
pub struct MandateRevokeFlowData {
    pub merchant_id: common_utils::id_type::MerchantId,
    pub customer_id: common_utils::id_type::CustomerId,
    pub payment_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WebhookSourceVerifyData {
    pub merchant_id: common_utils::id_type::MerchantId,
}

#[derive(Debug, Clone)]
pub struct AccessTokenFlowData {}

#[derive(Debug, Clone)]
pub struct FilesFlowData {
    pub merchant_id: common_utils::id_type::MerchantId,
    pub payment_id: String,
    pub attempt_id: String,
    pub connector_meta_data: Option<pii::SecretSerdeValue>,
    pub connector_request_reference_id: String,
}

#[derive(Debug, Clone)]
pub struct RevenueRecoveryRecordBackData;

#[derive(Debug, Clone)]
pub struct UasFlowData {
    pub authenticate_by: String,
    pub source_authentication_id: String,
}

#[derive(Debug, Clone)]
pub struct GetAdditionalRevenueRecoveryFlowCommonData;
