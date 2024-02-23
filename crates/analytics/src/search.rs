use actix_web::web::Json;
use api_models::analytics::{GetGlobalSearchRequest, GetSearchRequest, SearchIndex};
use common_utils::errors::CustomResult;
use opensearch::{http::request::JsonBody, MsearchParts, OpenSearch, SearchParts};
use serde_json::{json, Value};
use strum::IntoEnumIterator;

use crate::errors::AnalyticsError;
use crate::AnalyticsProvider;

async fn msearch_results(
    pool: &AnalyticsProvider,
    req: GetGlobalSearchRequest,
    merchant_id: &String,
) -> CustomResult<(), AnalyticsError> {
    let client = OpenSearch::default();

    let mut msearch_vector: Vec<JsonBody<Value>> = vec![];
    SearchIndex::iter().map(|index| {
        msearch_vector.push(json!({"index": index.to_string()}).into());
        msearch_vector.push(json!({"query": {"bool": {"must": {"query_string": {"query": req.query}}, "filter": {"match_phrase": {"merchant_id": merchant_id}}}}}).into());
    });

    let mut response = client
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
    pool: &AnalyticsProvider,
    req: GetSearchRequest,
    merchant_id: &String,
    index: SearchIndex,
) -> CustomResult<(), AnalyticsError> {
    let client = OpenSearch::default();

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
