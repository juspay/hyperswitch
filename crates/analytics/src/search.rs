use api_models::analytics::search::{
    GetGlobalSearchRequest, GetSearchRequestWithIndex, GetSearchResponse, OpenMsearchOutput,
    OpensearchOutput, SearchIndex,
};

use common_utils::errors::CustomResult;
// use common_utils::errors::ReportSwitchExt;
use serde_json::{Value};
use strum::IntoEnumIterator;
use error_stack::ResultExt;

use crate::{
    errors::AnalyticsError,
    opensearch::{OpenSearchQueryBuilder, OpenSearchQuery, OpenSearchClient},
};

pub async fn msearch_results(
    client: &OpenSearchClient,
    req: GetGlobalSearchRequest,
    merchant_id: &String,
) -> CustomResult<Vec<GetSearchResponse>, AnalyticsError> {

    let mut query_builder = OpenSearchQueryBuilder::new(OpenSearchQuery::Msearch, req.query);

    query_builder.add_filter_clause("merchant_id".to_string(), merchant_id.to_string()).change_context(AnalyticsError::UnknownError)?;

    let response_body = client.execute(query_builder)
        .await
        .change_context(AnalyticsError::UnknownError)?  
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

    let mut query_builder = OpenSearchQueryBuilder::new(OpenSearchQuery::Search(req.index), search_req.query);

    query_builder.add_filter_clause("merchant_id".to_string(), merchant_id.to_string()).change_context(AnalyticsError::UnknownError)?;

    query_builder.set_offset_n_count(search_req.offset, search_req.count).change_context(AnalyticsError::UnknownError)?;

    let response_body = client.execute(query_builder)
        .await
        .change_context(AnalyticsError::UnknownError)?
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
