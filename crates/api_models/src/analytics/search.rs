use serde_json::Value;

// how is the filter struct defined?

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetGlobalSearchRequest {
    pub query: String,
    #[serde(default)]
    pub filters: Option<SearchFilters>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSearchRequest {
    pub offset: i64,
    pub count: i64,
    pub query: String,
    #[serde(default)]
    pub filters: Option<SearchFilters>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSearchRequestWithIndex {
    pub index: SearchIndex,
    pub search_req: GetSearchRequest,
}

#[derive(Debug, strum::EnumIter, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchIndex {
    PaymentAttempts,
    PaymentIntents,
    Refunds,
    Disputes,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSearchResponse {
    pub count: u64,
    pub index: SearchIndex,
    pub hits: Vec<Value>,
}

#[derive(Debug, serde::Deserialize)]
pub struct OpenMsearchOutput<T> {
    pub responses: Vec<OpensearchOutput<T>>,
}

#[derive(Debug, serde::Deserialize)]
pub struct OpensearchOutput<T> {
    pub hits: OpensearchResults<T>,
}

#[derive(Debug, serde::Deserialize)]
pub struct OpensearchResults<T> {
    pub total: OpensearchResultsTotal,
    pub hits: Vec<OpensearchHits<T>>,
}

#[derive(Debug, serde::Deserialize)]
pub struct OpensearchResultsTotal {
    pub value: u64,
}

#[derive(Debug, serde::Deserialize)]
pub struct OpensearchHits<T> {
    pub _source: T,
}
