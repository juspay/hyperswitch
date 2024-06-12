use api_models::analytics::search::{
    GetGlobalSearchRequest, GetSearchRequestWithIndex, GetSearchResponse, OpenMsearchOutput,
    OpensearchOutput, SearchIndex,
};
use common_utils::errors::{CustomResult, ReportSwitchExt};
use error_stack::ResultExt;
use log::error;
use crate::opensearch::{
    OpenSearchClient, OpenSearchError, OpenSearchQuery, OpenSearchQueryBuilder,
};
use strum::IntoEnumIterator;

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
        .json::<OpenMsearchOutput>()
        .await
        .change_context(OpenSearchError::ResponseError)?;

    Ok(response_body
        .responses
        .into_iter()
        .zip(SearchIndex::iter())
        .map(|(index_hit, index)| {
            match index_hit {
                OpensearchOutput::Success(success) => {
                    if success.status == 200 {
                        GetSearchResponse {
                            count: success.hits.total.value,
                            index,
                            hits: success.hits.hits.into_iter().map(|hit| hit._source).collect(),
                        }
                    } else {
                        error!("Unexpected status code: {}", success.status);
                        GetSearchResponse {
                            count: 0,
                            index,
                            hits: Vec::new(),
                        }
                    }
                },
                OpensearchOutput::Error(error) => {
                    error!(
                        "Search error for index {:?}: type = {}, reason = {}, status = {}",
                        index, error.error.error_type, error.error.reason, error.status
                    );
                    GetSearchResponse {
                        count: 0,
                        index,
                        hits: Vec::new(),
                    }
                }
            }
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
        .json::<OpensearchOutput>()
        .await
        .change_context(OpenSearchError::ResponseError)?;

    match response_body {
        OpensearchOutput::Success(success) => {
            if success.status == 200 {
                Ok(GetSearchResponse {
                    count: success.hits.total.value,
                    index: req.index,
                    hits: success.hits.hits.into_iter().map(|hit| hit._source).collect(),
                })
            } else {
                error!("Unexpected status code: {}", success.status);
                Ok(GetSearchResponse {
                    count: 0,
                    index: req.index,
                    hits: Vec::new(),
                })
            }
        },
        OpensearchOutput::Error(error) => {
            error!(
                "Search error for index {:?}: type = {}, reason = {}, status = {}",
                req.index, error.error.error_type, error.error.reason, error.status
            );
            Ok(GetSearchResponse {
                count: 0,
                index: req.index,
                hits: Vec::new(),
            })
        }
    }
}
