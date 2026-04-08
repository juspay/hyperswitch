use common_utils::{hashing::HashedString, types::TimeRange};
use hyperswitch_masking::{ExposeInterface, WithType};
use serde_json::Value;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct OpensearchRange {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gte: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lte: Option<i64>,
}

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
    pub amount_filter: Option<OpensearchRange>,
    pub customer_id: Option<Vec<String>>,
    pub authentication_type: Option<Vec<String>>,
    pub card_discovery: Option<Vec<String>>,
    pub merchant_order_reference_id: Option<Vec<String>>,
}

#[cfg(feature = "v1")]
pub fn convert_to_strings<T: ToString>(items: &[T]) -> Vec<String> {
    items.iter().map(|item| item.to_string()).collect()
}

#[cfg(feature = "v1")]
impl From<&crate::payments::PaymentListFilterConstraints> for SearchFilters {
    fn from(constraints: &crate::payments::PaymentListFilterConstraints) -> Self {
        Self {
            payment_method: constraints
                .payment_method
                .as_deref()
                .map(convert_to_strings),
            currency: constraints.currency.as_deref().map(convert_to_strings),
            status: constraints.status.as_deref().map(convert_to_strings),
            payment_method_type: constraints
                .payment_method_type
                .as_deref()
                .map(convert_to_strings),
            authentication_type: constraints
                .authentication_type
                .as_deref()
                .map(convert_to_strings),
            card_network: constraints.card_network.as_deref().map(convert_to_strings),
            connector: constraints.connector.as_deref().map(convert_to_strings),
            card_discovery: constraints
                .card_discovery
                .as_deref()
                .map(convert_to_strings),
            customer_id: constraints
                .customer_id
                .as_ref()
                .map(|customer_id| vec![customer_id.get_string_repr().to_string()]),
            payment_id: constraints
                .payment_id
                .as_ref()
                .map(|payment_id| vec![payment_id.get_string_repr().to_string()]),
            merchant_order_reference_id: constraints
                .merchant_order_reference_id
                .as_ref()
                .map(|merchant_order_reference_id| vec![merchant_order_reference_id.clone()]),
            customer_email: constraints
                .customer_email
                .as_ref()
                .map(|customer_email| vec![HashedString::from(customer_email.clone().expose())]),
            search_tags: None,
            card_last_4: None,
            amount: None,
            amount_filter: constraints
                .amount_filter
                .as_ref()
                .map(|amount| OpensearchRange {
                    gte: amount.start_amount,
                    lte: amount.end_amount,
                }),
        }
    }
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
