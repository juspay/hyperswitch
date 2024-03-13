use api_models::analytics::search::{
    GetGlobalSearchRequest, GetSearchRequestWithIndex, GetSearchResponse, OpenMsearchOutput,
    OpensearchOutput, SearchIndex,
};

use common_utils::errors::CustomResult;
use opensearch::{http::request::JsonBody, MsearchParts, SearchParts};
use serde_json::{json, Value};
use strum::IntoEnumIterator;

use crate::{
    errors::AnalyticsError,
    opensearch::{OpenSearchClient, OpenSearchIndexes},
};

#[derive(Debug, thiserror::Error)]
pub enum OpensearchError {
    #[error("Opensearch connection error")]
    ConnectionError,
    #[error("Opensearch NON-200 response content: '{0}'")]
    ResponseNotOK(String),
    #[error("Opensearch response error")]
    ResponseError,
}

pub fn search_index_to_opensearch_index(index: SearchIndex, config: &OpenSearchIndexes) -> String {
    match index {
        SearchIndex::PaymentAttempts => config.payment_attempts.clone(),
        SearchIndex::PaymentIntents => config.payment_intents.clone(),
        SearchIndex::Refunds => config.refunds.clone(),
    }
}

pub async fn msearch_results(
    client: &OpenSearchClient,
    req: GetGlobalSearchRequest,
    merchant_id: &String,
) -> CustomResult<Vec<GetSearchResponse>, AnalyticsError> {
    // let client = get_opensearch_client(config.clone())
    //     .await
    //     .map_err(|_| AnalyticsError::UnknownError)?;

    let mut msearch_vector: Vec<JsonBody<Value>> = vec![];
    for index in SearchIndex::iter() {
        msearch_vector
            .push(json!({"index": search_index_to_opensearch_index(index,&client.indexes)}).into());
        msearch_vector.push(json!({"query": {"bool": {"must": {"query_string": {"query": req.query}}, "filter": {"match_phrase": {"merchant_id": merchant_id}}}}}).into());
    }

    let response = client
        .client
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
            index,
            hits: index_hit
                .hits
                .hits
                .into_iter()
                .map(|hit| hit._source)
                .collect(),
        })
        .collect())
}

pub async fn search_results(
    client: &OpenSearchClient,
    req: GetSearchRequestWithIndex,
    merchant_id: &String,
) -> CustomResult<GetSearchResponse, AnalyticsError> {
    let search_req = req.search_req;

    let response = client.client
        .search(SearchParts::Index(&[&search_index_to_opensearch_index(req.index.clone(),&client.indexes)]))
        .from(search_req.offset)
        .size(search_req.count)
        .body(json!({"query": {"bool": {"must": {"query_string": {"query": search_req.query}}, "filter": {"match_phrase": {"merchant_id": merchant_id}}}}}))
        .send()
        .await
        .map_err(|_| AnalyticsError::UnknownError)?;

    let response_body = response
        .json::<OpensearchOutput<Value>>()
        .await
        .map_err(|_| AnalyticsError::UnknownError)?;

    Ok(GetSearchResponse {
        count: response_body.hits.total.value,
        index: req.index,
        hits: response_body
            .hits
            .hits
            .into_iter()
            .map(|hit| hit._source)
            .collect(),
    })
}
