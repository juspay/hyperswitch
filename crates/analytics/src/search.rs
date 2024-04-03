use api_models::analytics::search::{
    GetGlobalSearchRequest, GetSearchRequestWithIndex, GetSearchResponse, OpenMsearchOutput,
    OpensearchOutput, SearchIndex,
};
use common_utils::errors::{CustomResult, ReportSwitchExt};
use error_stack::ResultExt;
use serde_json::Value;
use strum::IntoEnumIterator;

use crate::opensearch::{
    OpenSearchClient, OpenSearchError, OpenSearchQuery, OpenSearchQueryBuilder,
};

pub async fn msearch_results(
    client: &OpenSearchClient,
    req: GetGlobalSearchRequest,
    merchant_id: &String,
) -> CustomResult<Vec<GetSearchResponse>, OpenSearchError> {
    let mut query_builder = OpenSearchQueryBuilder::new(OpenSearchQuery::Msearch, req.query);

    query_builder
        .add_filter_clause("merchant_id".to_string(), merchant_id.to_string())
        .switch()?;

    let response_body = client
        .execute(query_builder)
        .await
        .change_context(OpenSearchError::ConnectionError)?
        .json::<OpenMsearchOutput<Value>>()
        .await
        .change_context(OpenSearchError::ResponseError)?;

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
) -> CustomResult<GetSearchResponse, OpenSearchError> {
    let search_req = req.search_req;

    let mut query_builder =
        OpenSearchQueryBuilder::new(OpenSearchQuery::Search(req.index), search_req.query);

    query_builder
        .add_filter_clause("merchant_id".to_string(), merchant_id.to_string())
        .switch()?;

    query_builder
        .set_offset_n_count(search_req.offset, search_req.count)
        .switch()?;

    let response_body = client
        .execute(query_builder)
        .await
        .change_context(OpenSearchError::ConnectionError)?
        .json::<OpensearchOutput<Value>>()
        .await
        .change_context(OpenSearchError::ResponseError)?;

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
