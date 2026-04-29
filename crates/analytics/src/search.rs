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

macro_rules! append_filter {
    ($builder:ident, $filters:ident, $field:ident, $es_key:expr) => {
        if let Some(val) = &$filters.$field {
            if !val.is_empty() {
                $builder
                    .add_filter_clause($es_key.to_string(), convert_to_value(val.clone()))
                    .switch()?;
            }
        }
    };
    ($builder:ident, $filters:ident, $field:ident, $es_key:expr, $transform:expr) => {
        if let Some(val) = &$filters.$field {
            if !val.is_empty() {
                $builder
                    .add_filter_clause($es_key.to_string(), convert_to_value($transform(val)))
                    .switch()?;
            }
        }
    };
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
        None,
    );

    if let Some(filters) = req.filters {
        append_filter!(query_builder, filters, currency, "currency.keyword");
        append_filter!(query_builder, filters, status, "status.keyword");
        append_filter!(
            query_builder,
            filters,
            payment_method,
            "payment_method.keyword"
        );
        append_filter!(
            query_builder,
            filters,
            customer_email,
            "customer_email.keyword",
            |emails: &Vec<_>| {
                emails
                    .iter()
                    .filter_map(|email| {
                        serde_json::to_value(email)
                            .ok()
                            .and_then(|a| a.as_str().map(|a| a.to_string()))
                    })
                    .collect::<Vec<String>>()
            }
        );
        append_filter!(
            query_builder,
            filters,
            search_tags,
            "feature_metadata.search_tags.keyword",
            |tags: &Vec<_>| {
                tags.iter()
                    .filter_map(|tag| {
                        serde_json::to_value(tag)
                            .ok()
                            .and_then(|a| a.as_str().map(|a| a.to_string()))
                    })
                    .collect::<Vec<String>>()
            }
        );

        append_filter!(query_builder, filters, connector, "connector.keyword");
        append_filter!(
            query_builder,
            filters,
            payment_method_type,
            "payment_method_type.keyword"
        );
        append_filter!(query_builder, filters, card_network, "card_network.keyword");
        append_filter!(query_builder, filters, card_last_4, "card_last_4.keyword");
        append_filter!(query_builder, filters, payment_id, "payment_id.keyword");
        append_filter!(query_builder, filters, amount, "amount");
        append_filter!(query_builder, filters, customer_id, "customer_id.keyword");
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
        && search_params.is_empty()
    {
        return Err(OpenSearchError::BadRequestError(
            "Query, filters and search_params are all empty".to_string(),
        )
        .into());
    }
    let mut query_builder = OpenSearchQueryBuilder::new(
        OpenSearchQuery::Search(req.index),
        search_req.query,
        search_params,
        search_req.order,
    );

    if let Some(filters) = search_req.filters {
        append_filter!(query_builder, filters, currency, "currency.keyword");
        append_filter!(query_builder, filters, status, "status.keyword");
        append_filter!(
            query_builder,
            filters,
            payment_method,
            "payment_method.keyword"
        );
        append_filter!(
            query_builder,
            filters,
            customer_email,
            "customer_email.keyword",
            |emails: &Vec<_>| {
                emails
                    .iter()
                    .filter_map(|email| {
                        serde_json::to_value(email)
                            .ok()
                            .and_then(|a| a.as_str().map(|a| a.to_string()))
                    })
                    .collect::<Vec<String>>()
            }
        );
        append_filter!(
            query_builder,
            filters,
            search_tags,
            "feature_metadata.search_tags.keyword",
            |tags: &Vec<_>| {
                tags.iter()
                    .filter_map(|tag| {
                        serde_json::to_value(tag)
                            .ok()
                            .and_then(|a| a.as_str().map(|a| a.to_string()))
                    })
                    .collect::<Vec<String>>()
            }
        );

        if let Some(amount_filter) = filters.amount_filter {
            query_builder.set_amount_range(amount_filter).switch()?;
        };
        append_filter!(query_builder, filters, connector, "connector.keyword");
        append_filter!(
            query_builder,
            filters,
            payment_method_type,
            "payment_method_type.keyword"
        );
        append_filter!(query_builder, filters, card_network, "card_network.keyword");
        append_filter!(query_builder, filters, card_last_4, "card_last_4.keyword");
        append_filter!(query_builder, filters, payment_id, "payment_id.keyword");
        append_filter!(query_builder, filters, amount, "amount");
        append_filter!(query_builder, filters, customer_id, "customer_id.keyword");
        append_filter!(
            query_builder,
            filters,
            authentication_type,
            "authentication_type.keyword"
        );
        append_filter!(
            query_builder,
            filters,
            card_discovery,
            "card_discovery.keyword"
        );
        append_filter!(
            query_builder,
            filters,
            merchant_order_reference_id,
            "merchant_order_reference_id.keyword"
        );
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
