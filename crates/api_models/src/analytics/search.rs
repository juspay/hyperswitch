use common_utils::{hashing::HashedString, types::TimeRange};
use masking::WithType;
use serde_json::Value;

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct SearchFilters {
    pub payment_method: Option<Vec<String>>,
    pub currency: Option<Vec<String>>,
    pub status: Option<Vec<String>>,
    pub customer_email: Option<Vec<HashedString<common_utils::pii::EmailStrategy>>>,
    pub search_tags: Option<Vec<HashedString<WithType>>>,
    pub connector: Option<Vec<String>>,
    pub payment_method_type: Option<Vec<String>>,
    pub card_network: Option<Vec<String>>,
    pub card_last_4: Option<Vec<String>>,
    pub payment_id: Option<Vec<String>>,
    pub amount: Option<Vec<u64>>,
    pub customer_id: Option<Vec<String>>,
}
impl SearchFilters {
    pub fn is_all_none(&self) -> bool {
        self.payment_method.is_none()
            && self.currency.is_none()
            && self.status.is_none()
            && self.customer_email.is_none()
            && self.search_tags.is_none()
            && self.connector.is_none()
            && self.payment_method_type.is_none()
            && self.card_network.is_none()
            && self.card_last_4.is_none()
            && self.payment_id.is_none()
            && self.amount.is_none()
            && self.customer_id.is_none()
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
