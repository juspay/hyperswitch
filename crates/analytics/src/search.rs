use api_models::analytics::{GetGlobalSearchRequest, GetSearchRequest, SearchIndex};
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

async fn get_opensearch_client(url: String) -> Result<OpenSearch, OpensearchError> {
    let transport = Transport::single_node(&url).map_err(|_| OpensearchError::ConnectionError)?;
    Ok(OpenSearch::new(transport))
}

async fn msearch_results(
    req: GetGlobalSearchRequest,
    merchant_id: &String,
    url: String,
) -> CustomResult<(), AnalyticsError> {
    let client = get_opensearch_client(url)
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

    let mut response_body = response
        .json::<Value>()
        .await
        .map_err(|_| AnalyticsError::UnknownError)?;

    Ok(())
}

async fn search_results(
    req: GetSearchRequest,
    merchant_id: &String,
    index: SearchIndex,
) -> CustomResult<(), AnalyticsError> {
    let client = get_opensearch_client(url)
        .await
        .map_err(|_| AnalyticsError::UnknownError)?;

    let mut response = client
        .search(SearchParts::Index(&[&index.to_string()]))
        .body(json!({"query": {"bool": {"must": {"query_string": {"query": req.query}}, "filter": {"match_phrase": {"merchant_id": merchant_id}}}}}))
        .send()
        .await
        .map_err(|_| AnalyticsError::UnknownError)?;

    let mut response_body = response
        .json::<Value>()
        .await
        .map_err(|_| AnalyticsError::UnknownError)?;

    Ok(())
}
