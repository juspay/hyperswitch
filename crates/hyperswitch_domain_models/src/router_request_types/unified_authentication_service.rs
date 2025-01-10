use common_utils::types::MinorUnit;
use masking::Secret;
use time::PrimitiveDateTime;

#[derive(Clone, serde::Deserialize, Debug, serde::Serialize)]
pub struct UasPreAuthenticationRequestData {
    pub service_details: Option<CtpServiceDetails>,
    pub transaction_details: Option<TransactionDetails>,
}

#[derive(Clone, serde::Deserialize, Debug, serde::Serialize)]
pub struct CtpServiceDetails {
    pub service_session_ids: Option<ServiceSessionIds>,
}

#[derive(Clone, serde::Deserialize, Debug, serde::Serialize)]
pub struct ServiceSessionIds {
    pub correlation_id: Option<String>,
    pub merchant_transaction_id: Option<String>,
    pub x_src_flow_id: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct TransactionDetails {
    pub amount: MinorUnit,
    pub currency: common_enums::Currency,
}

#[derive(Clone, Debug)]
pub struct UasPostAuthenticationRequestData {}

#[derive(Debug, Clone)]
pub enum UasAuthenticationResponseData {
    PreAuthentication {},
    PostAuthentication {
        authentication_details: PostAuthenticationDetails,
    },
    Confirmation {},
    Webhook {
        trans_status: common_enums::TransactionStatus,
        authentication_value: Option<String>,
        eci: Option<String>,
    },
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct PostAuthenticationDetails {
    pub eci: Option<String>,
    pub token_details: TokenDetails,
    pub dynamic_data_details: Option<DynamicData>,
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
    pub dynamic_data_type: String,
    pub ds_trans_id: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct UasConfirmationRequestData {
    pub x_src_flow_id: Option<String>,
    pub transaction_amount: MinorUnit,
    pub transaction_currency: common_enums::Currency,
    pub checkout_event_type: Option<String>,
    pub checkout_event_status: Option<String>,
    pub confirmation_status: Option<String>,
    pub confirmation_reason: Option<String>,
    pub confirmation_timestamp: Option<PrimitiveDateTime>,
    pub network_authorization_code: Option<String>,
    pub network_transaction_identifier: Option<String>,
    pub correlation_id: Option<String>,
    pub merchant_transaction_id: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct UasWebhookRequestData {
    pub body: Vec<u8>,
}
