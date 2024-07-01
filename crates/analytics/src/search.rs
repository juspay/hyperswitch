use api_models::analytics::search::{
    GetGlobalSearchRequest, GetSearchRequestWithIndex, GetSearchResponse, OpenMsearchOutput,
    OpensearchOutput, SearchIndex, SearchStatus,
};
use common_utils::errors::{CustomResult, ReportSwitchExt};
use error_stack::ResultExt;
use router_env::tracing;

use crate::opensearch::{
    OpenSearchClient, OpenSearchError, OpenSearchQuery, OpenSearchQueryBuilder,
};

pub async fn msearch_results(
    client: &OpenSearchClient,
    req: GetGlobalSearchRequest,
    merchant_id: &String,
    indexes: Vec<SearchIndex>,
) -> CustomResult<Vec<GetSearchResponse>, OpenSearchError> {
    let mut query_builder =
        OpenSearchQueryBuilder::new(OpenSearchQuery::Msearch(indexes.clone()), req.query);

    query_builder
        .add_filter_clause(
            "merchant_id.keyword".to_string(),
            vec![merchant_id.to_string()],
        )
        .switch()?;

    if let Some(filters) = req.filters {
        if let Some(currency) = filters.currency {
            if !currency.is_empty() {
                query_builder
                    .add_filter_clause("currency.keyword".to_string(), currency.clone())
                    .switch()?;
            }
        };
        if let Some(status) = filters.status {
            if !status.is_empty() {
                query_builder
                    .add_filter_clause("status.keyword".to_string(), status.clone())
                    .switch()?;
            }
        };
        if let Some(payment_method) = filters.payment_method {
            if !payment_method.is_empty() {
                query_builder
                    .add_filter_clause("payment_method.keyword".to_string(), payment_method.clone())
                    .switch()?;
            }
        };
        if let Some(customer_email) = filters.customer_email {
            if !customer_email.is_empty() {
                query_builder
                    .add_filter_clause("customer_email.keyword".to_string(), customer_email.clone())
                    .switch()?;
            }
        };
    };

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
        .zip(indexes)
        .map(|(index_hit, index)| match index_hit {
            OpensearchOutput::Success(success) => GetSearchResponse {
                count: success.hits.total.value,
                index,
                hits: success
                    .hits
                    .hits
                    .into_iter()
                    .map(|hit| hit.source)
                    .collect(),
                status: SearchStatus::Success,
            },
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
                    status: SearchStatus::Failure,
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
        .add_filter_clause(
            "merchant_id.keyword".to_string(),
            vec![merchant_id.to_string()],
        )
        .switch()?;

    if let Some(filters) = search_req.filters {
        if let Some(currency) = filters.currency {
            if !currency.is_empty() {
                query_builder
                    .add_filter_clause("currency.keyword".to_string(), currency.clone())
                    .switch()?;
            }
        };
        if let Some(status) = filters.status {
            if !status.is_empty() {
                query_builder
                    .add_filter_clause("status.keyword".to_string(), status.clone())
                    .switch()?;
            }
        };
        if let Some(payment_method) = filters.payment_method {
            if !payment_method.is_empty() {
                query_builder
                    .add_filter_clause("payment_method.keyword".to_string(), payment_method.clone())
                    .switch()?;
            }
        };
        if let Some(customer_email) = filters.customer_email {
            if !customer_email.is_empty() {
                query_builder
                    .add_filter_clause("customer_email.keyword".to_string(), customer_email.clone())
                    .switch()?;
            }
        };
    };
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
        OpensearchOutput::Success(success) => Ok(GetSearchResponse {
            count: success.hits.total.value,
            index: req.index,
            hits: success
                .hits
                .hits
                .into_iter()
                .map(|hit| hit.source)
                .collect(),
            status: SearchStatus::Success,
        }),
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
                status: SearchStatus::Failure,
            })
        }
    }
}
