use api_models::analytics::search::{
    GetGlobalSearchRequest, GetSearchRequestWithIndex, GetSearchResponse, OpenMsearchOutput,
    OpensearchOutput, SearchIndex, SearchStatus,
};
use common_utils::errors::{CustomResult, ReportSwitchExt};
use error_stack::ResultExt;
use router_env::tracing;
use serde_json::Value;

use crate::{
    enums::AuthInfo,
    opensearch::{OpenSearchClient, OpenSearchError, OpenSearchQuery, OpenSearchQueryBuilder},
};

pub fn convert_to_value<T: Into<Value>>(items: Vec<T>) -> Vec<Value> {
    items.into_iter().map(|item| item.into()).collect()
}

pub async fn msearch_results(
    client: &OpenSearchClient,
    req: GetGlobalSearchRequest,
    search_params: Vec<AuthInfo>,
    indexes: Vec<SearchIndex>,
) -> CustomResult<Vec<GetSearchResponse>, OpenSearchError> {
    if req.query.trim().is_empty()
        && req
            .filters
            .as_ref()
            .is_none_or(|filters| filters.is_all_none())
    {
        return Err(OpenSearchError::BadRequestError(
            "Both query and filters are empty".to_string(),
        )
        .into());
    }
    let mut query_builder = OpenSearchQueryBuilder::new(
        OpenSearchQuery::Msearch(indexes.clone()),
        req.query,
        search_params,
    );

    if let Some(filters) = req.filters {
        if let Some(currency) = filters.currency {
            if !currency.is_empty() {
                query_builder
                    .add_filter_clause("currency.keyword".to_string(), convert_to_value(currency))
                    .switch()?;
            }
        };
        if let Some(status) = filters.status {
            if !status.is_empty() {
                query_builder
                    .add_filter_clause("status.keyword".to_string(), convert_to_value(status))
                    .switch()?;
            }
        };
        if let Some(payment_method) = filters.payment_method {
            if !payment_method.is_empty() {
                query_builder
                    .add_filter_clause(
                        "payment_method.keyword".to_string(),
                        convert_to_value(payment_method),
                    )
                    .switch()?;
            }
        };
        if let Some(customer_email) = filters.customer_email {
            if !customer_email.is_empty() {
                query_builder
                    .add_filter_clause(
                        "customer_email.keyword".to_string(),
                        convert_to_value(
                            customer_email
                                .iter()
                                .filter_map(|email| {
                                    // TODO: Add trait based inputs instead of converting this to strings
                                    serde_json::to_value(email)
                                        .ok()
                                        .and_then(|a| a.as_str().map(|a| a.to_string()))
                                })
                                .collect(),
                        ),
                    )
                    .switch()?;
            }
        };
        if let Some(search_tags) = filters.search_tags {
            if !search_tags.is_empty() {
                query_builder
                    .add_filter_clause(
                        "feature_metadata.search_tags.keyword".to_string(),
                        convert_to_value(
                            search_tags
                                .iter()
                                .filter_map(|search_tag| {
                                    // TODO: Add trait based inputs instead of converting this to strings
                                    serde_json::to_value(search_tag)
                                        .ok()
                                        .and_then(|a| a.as_str().map(|a| a.to_string()))
                                })
                                .collect(),
                        ),
                    )
                    .switch()?;
            }
        };
        if let Some(connector) = filters.connector {
            if !connector.is_empty() {
                query_builder
                    .add_filter_clause("connector.keyword".to_string(), convert_to_value(connector))
                    .switch()?;
            }
        };
        if let Some(payment_method_type) = filters.payment_method_type {
            if !payment_method_type.is_empty() {
                query_builder
                    .add_filter_clause(
                        "payment_method_type.keyword".to_string(),
                        convert_to_value(payment_method_type),
                    )
                    .switch()?;
            }
        };
        if let Some(card_network) = filters.card_network {
            if !card_network.is_empty() {
                query_builder
                    .add_filter_clause(
                        "card_network.keyword".to_string(),
                        convert_to_value(card_network),
                    )
                    .switch()?;
            }
        };
        if let Some(card_last_4) = filters.card_last_4 {
            if !card_last_4.is_empty() {
                query_builder
                    .add_filter_clause(
                        "card_last_4.keyword".to_string(),
                        convert_to_value(card_last_4),
                    )
                    .switch()?;
            }
        };
        if let Some(payment_id) = filters.payment_id {
            if !payment_id.is_empty() {
                query_builder
                    .add_filter_clause(
                        "payment_id.keyword".to_string(),
                        convert_to_value(payment_id),
                    )
                    .switch()?;
            }
        };
        if let Some(amount) = filters.amount {
            if !amount.is_empty() {
                query_builder
                    .add_filter_clause("amount".to_string(), convert_to_value(amount))
                    .switch()?;
            }
        };
        if let Some(customer_id) = filters.customer_id {
            if !customer_id.is_empty() {
                query_builder
                    .add_filter_clause(
                        "customer_id.keyword".to_string(),
                        convert_to_value(customer_id),
                    )
                    .switch()?;
            }
        };
    };

    if let Some(time_range) = req.time_range {
        query_builder.set_time_range(time_range.into()).switch()?;
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
    search_params: Vec<AuthInfo>,
) -> CustomResult<GetSearchResponse, OpenSearchError> {
    let search_req = req.search_req;
    if search_req.query.trim().is_empty()
        && search_req
            .filters
            .as_ref()
            .is_none_or(|filters| filters.is_all_none())
    {
        return Err(OpenSearchError::BadRequestError(
            "Both query and filters are empty".to_string(),
        )
        .into());
    }
    let mut query_builder = OpenSearchQueryBuilder::new(
        OpenSearchQuery::Search(req.index),
        search_req.query,
        search_params,
    );

    if let Some(filters) = search_req.filters {
        if let Some(currency) = filters.currency {
            if !currency.is_empty() {
                query_builder
                    .add_filter_clause("currency.keyword".to_string(), convert_to_value(currency))
                    .switch()?;
            }
        };
        if let Some(status) = filters.status {
            if !status.is_empty() {
                query_builder
                    .add_filter_clause("status.keyword".to_string(), convert_to_value(status))
                    .switch()?;
            }
        };
        if let Some(payment_method) = filters.payment_method {
            if !payment_method.is_empty() {
                query_builder
                    .add_filter_clause(
                        "payment_method.keyword".to_string(),
                        convert_to_value(payment_method),
                    )
                    .switch()?;
            }
        };
        if let Some(customer_email) = filters.customer_email {
            if !customer_email.is_empty() {
                query_builder
                    .add_filter_clause(
                        "customer_email.keyword".to_string(),
                        convert_to_value(
                            customer_email
                                .iter()
                                .filter_map(|email| {
                                    // TODO: Add trait based inputs instead of converting this to strings
                                    serde_json::to_value(email)
                                        .ok()
                                        .and_then(|a| a.as_str().map(|a| a.to_string()))
                                })
                                .collect(),
                        ),
                    )
                    .switch()?;
            }
        };
        if let Some(search_tags) = filters.search_tags {
            if !search_tags.is_empty() {
                query_builder
                    .add_filter_clause(
                        "feature_metadata.search_tags.keyword".to_string(),
                        convert_to_value(
                            search_tags
                                .iter()
                                .filter_map(|search_tag| {
                                    // TODO: Add trait based inputs instead of converting this to strings
                                    serde_json::to_value(search_tag)
                                        .ok()
                                        .and_then(|a| a.as_str().map(|a| a.to_string()))
                                })
                                .collect(),
                        ),
                    )
                    .switch()?;
            }
        };
        if let Some(connector) = filters.connector {
            if !connector.is_empty() {
                query_builder
                    .add_filter_clause("connector.keyword".to_string(), convert_to_value(connector))
                    .switch()?;
            }
        };
        if let Some(payment_method_type) = filters.payment_method_type {
            if !payment_method_type.is_empty() {
                query_builder
                    .add_filter_clause(
                        "payment_method_type.keyword".to_string(),
                        convert_to_value(payment_method_type),
                    )
                    .switch()?;
            }
        };
        if let Some(card_network) = filters.card_network {
            if !card_network.is_empty() {
                query_builder
                    .add_filter_clause(
                        "card_network.keyword".to_string(),
                        convert_to_value(card_network),
                    )
                    .switch()?;
            }
        };
        if let Some(card_last_4) = filters.card_last_4 {
            if !card_last_4.is_empty() {
                query_builder
                    .add_filter_clause(
                        "card_last_4.keyword".to_string(),
                        convert_to_value(card_last_4),
                    )
                    .switch()?;
            }
        };
        if let Some(payment_id) = filters.payment_id {
            if !payment_id.is_empty() {
                query_builder
                    .add_filter_clause(
                        "payment_id.keyword".to_string(),
                        convert_to_value(payment_id),
                    )
                    .switch()?;
            }
        };
        if let Some(amount) = filters.amount {
            if !amount.is_empty() {
                query_builder
                    .add_filter_clause("amount".to_string(), convert_to_value(amount))
                    .switch()?;
            }
        };
        if let Some(customer_id) = filters.customer_id {
            if !customer_id.is_empty() {
                query_builder
                    .add_filter_clause(
                        "customer_id.keyword".to_string(),
                        convert_to_value(customer_id),
                    )
                    .switch()?;
            }
        };
    };

    if let Some(time_range) = search_req.time_range {
        query_builder.set_time_range(time_range.into()).switch()?;
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
