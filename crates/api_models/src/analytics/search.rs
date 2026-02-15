use common_utils::{hashing::HashedString, id_type, types::TimeRange};
use masking::WithType;
use serde_json::Value;

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct SearchFilters {
    pub payment_method: Option<Vec<common_enums::PaymentMethod>>,
    pub currency: Option<Vec<common_enums::Currency>>,
    pub status: Option<Vec<common_enums::IntentStatus>>,
    pub customer_email: Option<Vec<HashedString<common_utils::pii::EmailStrategy>>>,
    pub search_tags: Option<Vec<HashedString<WithType>>>,
    pub connector: Option<Vec<common_enums::connector_enums::Connector>>,
    pub payment_method_type: Option<Vec<common_enums::PaymentMethodType>>,
    pub card_network: Option<Vec<common_enums::CardNetwork>>,
    pub card_last_4: Option<Vec<String>>,
    pub payment_id: Option<Vec<id_type::PaymentId>>,
    pub amount: Option<Vec<u64>>,
    pub amount_filter: Option<super::super::payments::AmountFilter>,
    pub customer_id: Option<Vec<id_type::CustomerId>>,
    pub authentication_type: Option<Vec<common_enums::AuthenticationType>>,
    pub card_discovery: Option<Vec<common_enums::CardDiscovery>>,
    pub merchant_order_reference_id: Option<Vec<String>>,
}
impl SearchFilters {
    pub fn is_all_none(&self) -> bool {
        matches!(
            self,
            Self {
                payment_method: None,
                currency: None,
                status: None,
                customer_email: None,
                search_tags: None,
                connector: None,
                payment_method_type: None,
                card_network: None,
                card_last_4: None,
                payment_id: None,
                amount: None,
                amount_filter: None,
                customer_id: None,
                authentication_type: None,
                card_discovery: None,
                merchant_order_reference_id: None,
            }
        )
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetGlobalSearchRequest {
    pub query: String,
    #[serde(default)]
    pub filters: Option<SearchFilters>,
    #[serde(default)]
    pub time_range: Option<TimeRange>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSearchRequest {
    pub offset: i64,
    pub count: i64,
    pub query: String,
    #[serde(default)]
    pub filters: Option<SearchFilters>,
    #[serde(default)]
    pub time_range: Option<TimeRange>,
    #[serde(default)]
    pub order: Option<crate::payments::Order>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSearchRequestWithIndex {
    pub index: SearchIndex,
    pub search_req: GetSearchRequest,
}

#[derive(
    Debug, strum::EnumIter, Clone, serde::Deserialize, serde::Serialize, Copy, Eq, PartialEq,
)]
#[serde(rename_all = "snake_case")]
pub enum SearchIndex {
    PaymentAttempts,
    PaymentIntents,
    Refunds,
    Disputes,
    Payouts,
    SessionizerPaymentAttempts,
    SessionizerPaymentIntents,
    SessionizerRefunds,
    SessionizerDisputes,
}

#[derive(Debug, strum::EnumIter, Clone, serde::Deserialize, serde::Serialize, Copy)]
pub enum SearchStatus {
    Success,
    Failure,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSearchResponse {
    pub count: u64,
    pub index: SearchIndex,
    pub hits: Vec<Value>,
    pub status: SearchStatus,
}

#[derive(Debug, serde::Deserialize)]
pub struct OpenMsearchOutput {
    #[serde(default)]
    pub responses: Vec<OpensearchOutput>,
    pub error: Option<OpensearchErrorDetails>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum OpensearchOutput {
    Success(OpensearchSuccess),
    Error(OpensearchError),
}

#[derive(Debug, serde::Deserialize)]
pub struct OpensearchError {
    pub error: OpensearchErrorDetails,
    pub status: u16,
}

#[derive(Debug, serde::Deserialize)]
pub struct OpensearchErrorDetails {
    #[serde(rename = "type")]
    pub error_type: String,
    pub reason: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct OpensearchSuccess {
    pub hits: OpensearchHits,
}

#[derive(Debug, serde::Deserialize)]
pub struct OpensearchHits {
    pub total: OpensearchResultsTotal,
    pub hits: Vec<OpensearchHit>,
}

#[derive(Debug, serde::Deserialize)]
pub struct OpensearchResultsTotal {
    pub value: u64,
}

#[derive(Debug, serde::Deserialize)]
pub struct OpensearchHit {
    #[serde(rename = "_source")]
    pub source: Value,
}
