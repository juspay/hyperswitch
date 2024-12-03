use masking::Secret;

#[derive(Clone, serde::Deserialize, Debug, serde::Serialize)]
pub struct UasPreAuthenticationRequestData {
    pub service_details: Option<ServiceDetails>,
    pub transaction_details: Option<TransactionDetails>,
}

#[derive(Clone, serde::Deserialize, Debug, serde::Serialize)]
pub struct ServiceDetails {
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
    pub amount: common_utils::types::FloatMajorUnit,
    pub currency: common_enums::Currency,
}

#[derive(Clone, Debug)]
pub struct UasPostAuthenticationRequestData;

#[derive(Debug, Clone)]
pub enum UasAuthenticationResponseData {
    PreAuthentication {},
    PostAuthentication {
        authentication_details: PostAuthenticationDetails,
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
    pub dynamic_data_value: Option<String>,
    pub dynamic_data_type: String,
    pub ds_trans_id: Option<String>,
}
