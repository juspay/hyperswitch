use api_models::analytics::{
    GetGlobalSearchRequest, GetSearchRequest, GetSearchResponse, SearchIndex,
};
use common_utils::errors::CustomResult;
use opensearch::http::transport::Transport;
use opensearch::{http::request::JsonBody, MsearchParts, OpenSearch, SearchParts};
use serde_json::{json, Value};
use strum::IntoEnumIterator;

use crate::errors::AnalyticsError;

#[derive(Debug, thiserror::Error)]
pub enum OpensearchError {
    #[error("Opensearch connection error")]
    ConnectionError,
    #[error("Opensearch NON-200 response content: '{0}'")]
    ResponseNotOK(String),
    #[error("Opensearch response error")]
    ResponseError,
}

#[derive(Debug, serde::Deserialize)]
struct OpenMsearchOutput<T> {
    responses: Vec<OpensearchOutput<T>>,
}

#[derive(Debug, serde::Deserialize)]
struct OpensearchOutput<T> {
    hits: OpensearchResults<T>,
}

#[derive(Debug, serde::Deserialize)]
struct OpensearchResults<T> {
    total: OpensearchResultsTotal,
    hits: Vec<OpensearchHits<T>>,
}

#[derive(Debug, serde::Deserialize)]
struct OpensearchResultsTotal {
    value: u64,
}

#[derive(Debug, serde::Deserialize)]
struct OpensearchHits<T> {
    _search: T,
}

async fn get_opensearch_client(url: String) -> Result<OpenSearch, OpensearchError> {
    let transport = Transport::single_node(&url).map_err(|_| OpensearchError::ConnectionError)?;
    Ok(OpenSearch::new(transport))
}

pub async fn msearch_results(
    req: GetGlobalSearchRequest,
    merchant_id: &String,
    url: &String,
) -> CustomResult<Vec<GetSearchResponse>, AnalyticsError> {
    let client = get_opensearch_client(url.to_owned())
        .await
        .map_err(|_| AnalyticsError::UnknownError)?;

    let mut msearch_vector: Vec<JsonBody<Value>> = vec![];
    for index in SearchIndex::iter() {
        msearch_vector.push(json!({"index": index.to_string()}).into());
        msearch_vector.push(json!({"query": {"bool": {"must": {"query_string": {"query": req.query}}, "filter": {"match_phrase": {"merchant_id": merchant_id}}}}}).into());
    }

    let response = client
        .msearch(MsearchParts::None)
        .body(msearch_vector)
        .send()
        .await
        .map_err(|_| AnalyticsError::UnknownError)?;

    let response_body = response
        .json::<OpenMsearchOutput<Value>>()
        .await
        .map_err(|_| AnalyticsError::UnknownError)?;

    Ok(response_body
        .responses
        .into_iter()
        .zip(SearchIndex::iter())
        .map(|(index_hit, index)| GetSearchResponse {
            count: index_hit.hits.total.value,
            index: index,
            hits: index_hit
                .hits
                .hits
                .into_iter()
                .map(|hit| hit._search)
                .collect(),
        })
        .collect())
}

pub async fn search_results(
    req: GetSearchRequest,
    merchant_id: &String,
    url: &String,
    index: SearchIndex,
) -> CustomResult<GetSearchResponse, AnalyticsError> {
    let client = get_opensearch_client(url.to_owned())
        .await
        .map_err(|_| AnalyticsError::UnknownError)?;

    let response = client
        .search(SearchParts::Index(&[&index.to_string()]))
        .body(json!({"query": {"bool": {"must": {"query_string": {"query": req.query}}, "filter": {"match_phrase": {"merchant_id": merchant_id}}}}}))
        .send()
        .await
        .map_err(|_| AnalyticsError::UnknownError)?;

    let response_body = response
        .json::<OpensearchOutput<Value>>()
        .await
        .map_err(|_| AnalyticsError::UnknownError)?;

    Ok(GetSearchResponse {
        count: response_body.hits.total.value,
        index: index,
        hits: response_body
            .hits
            .hits
            .into_iter()
            .map(|hit| hit._search)
            .collect(),
    })
}
