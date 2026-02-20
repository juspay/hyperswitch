use common_utils::{hashing::HashedString, id_type, types::TimeRange};
use masking::WithType;
use serde_json::Value;

#[cfg(feature = "v1")]
use crate::payments::PaymentListFilterConstraints;

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

#[cfg(feature = "v1")]
impl TryFrom<(PaymentListFilterConstraints, String, SearchFilters)> for GetSearchRequestWithIndex {
    type Error = String;

    fn try_from(
        (constraints, query, filters): (PaymentListFilterConstraints, String, SearchFilters),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            index: SearchIndex::SessionizerPaymentIntents,
            search_req: GetSearchRequest {
                offset: i64::from(constraints.offset.unwrap_or(0)),
                count: i64::from(constraints.limit),
                query,
                filters: Some(filters),
                time_range: constraints.time_range,
                order: Some(constraints.order),
            },
        })
    }
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

#[cfg(all(feature = "v1", feature = "olap"))]
#[derive(Debug, Clone, serde::Deserialize)]
pub struct OpenSearchPaymentIntentSource {
    pub payment_id: String,
    pub merchant_id: String,
    pub status: String,
    pub amount: i64,
    pub currency: String,
    pub created_at: i64,
    pub modified_at: i64,
    pub attempt_count: i64,
    pub active_attempt_id: Option<String>,
    #[serde(default)]
    pub attempts_list: Vec<OpenSearchPaymentAttemptSource>,
    pub processor_merchant_id: Option<String>,
    pub customer_id: Option<String>,
    pub description: Option<String>,
    pub return_url: Option<String>,
    pub client_secret: Option<String>,
    pub business_label: Option<String>,
    pub business_country: Option<String>,
    pub business_sub_label: Option<String>,
    pub profile_id: Option<String>,
    pub organization_id: Option<String>,
    pub merchant_order_reference_id: Option<String>,
    pub shipping_cost: Option<i64>,
    pub metadata: Option<String>,
    pub browser_info: Option<String>,
    pub payment_method: Option<String>,
    pub payment_method_type: Option<String>,
    pub authentication_type: Option<String>,
    pub setup_future_usage: Option<String>,
    pub mit_category: Option<String>,
    pub connector: Option<String>,
    pub merchant_connector_id: Option<String>,
    pub payment_method_id: Option<String>,
    pub payment_method_data: Option<String>,
    pub statement_descriptor_name: Option<String>,
    pub statement_descriptor_suffix: Option<String>,
    pub mandate_id: Option<String>,
}

#[cfg(all(feature = "v1", feature = "olap"))]
#[derive(Debug, Clone, serde::Deserialize)]
pub struct OpenSearchPaymentAttemptSource {
    pub attempt_id: String,
    pub created_at: i64,
    pub modified_at: i64,
    pub status: Option<String>,
    pub connector_transaction_id: Option<String>,
    pub capture_method: Option<String>,
    pub net_amount: Option<i64>,
    pub amount_capturable: Option<i64>,
    pub amount_received: Option<i64>,
    pub capture_on: Option<i64>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub browser_info: Option<String>,
}

#[cfg(all(feature = "v1", feature = "olap"))]
impl OpenSearchPaymentIntentSource {
    pub fn get_active_attempt(&self) -> Option<&OpenSearchPaymentAttemptSource> {
        let active_id = self.active_attempt_id.as_ref()?;
        self.attempts_list
            .iter()
            .find(|attempt| &attempt.attempt_id == active_id)
    }
}
