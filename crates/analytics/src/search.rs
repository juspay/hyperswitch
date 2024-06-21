use api_models::analytics::search::{
    GetGlobalSearchRequest, GetSearchRequestWithIndex, GetSearchResponse, OpenMsearchOutput,
    OpensearchOutput, SearchIndex,
};
use common_utils::errors::{CustomResult, ReportSwitchExt};
use error_stack::ResultExt;
use router_env::tracing;
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

    let response_text: OpenMsearchOutput = client
        .execute(query_builder)
        .await
        .change_context(OpenSearchError::ConnectionError)?
        .text()
        .await
        .change_context(OpenSearchError::ResponseError)
        .and_then(|body: String| {
            serde_json::from_str::<OpenMsearchOutput>(&body)
                .change_context(OpenSearchError::DeserialisationError)
                .attach_printable(body.clone())
        })?;

    let response_body: OpenMsearchOutput = response_text;

    Ok(response_body
        .responses
        .into_iter()
        .zip(SearchIndex::iter())
        .map(|(index_hit, index)| match index_hit {
            OpensearchOutput::Success(success) => {
                if success.status == 200 {
                    GetSearchResponse {
                        count: success.hits.total.value,
                        index,
                        hits: success
                            .hits
                            .hits
                            .into_iter()
                            .map(|hit| hit.source)
                            .collect(),
                    }
                } else {
                    tracing::error!("Unexpected status code: {}", success.status,);
                    GetSearchResponse {
                        count: 0,
                        index,
                        hits: Vec::new(),
                    }
                }
            }
            OpensearchOutput::Error(error) => {
                tracing::error!(
                    index = ?index,
                    error_response = ?error,
                    "Search error"
                );
                GetSearchResponse {
                    count: 0,
                    index,
                    hits: Vec::new(),
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

    let response_text: OpensearchOutput = client
        .execute(query_builder)
        .await
        .change_context(OpenSearchError::ConnectionError)?
        .text()
        .await
        .change_context(OpenSearchError::ResponseError)
        .and_then(|body: String| {
            serde_json::from_str::<OpensearchOutput>(&body)
                .change_context(OpenSearchError::DeserialisationError)
                .attach_printable(body.clone())
        })?;

    let response_body: OpensearchOutput = response_text;

    match response_body {
        OpensearchOutput::Success(success) => {
            if success.status == 200 {
                Ok(GetSearchResponse {
                    count: success.hits.total.value,
                    index: req.index,
                    hits: success
                        .hits
                        .hits
                        .into_iter()
                        .map(|hit| hit.source)
                        .collect(),
                })
            } else {
                tracing::error!("Unexpected status code: {}", success.status);
                Ok(GetSearchResponse {
                    count: 0,
                    index: req.index,
                    hits: Vec::new(),
                })
            }
        }
        OpensearchOutput::Error(error) => {
            tracing::error!(
                index = ?req.index,
                error_response = ?error,
                "Search error"
            );
            Ok(GetSearchResponse {
                count: 0,
                index: req.index,
                hits: Vec::new(),
            })
        }
    }
}
