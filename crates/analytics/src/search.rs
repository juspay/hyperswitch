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
use masking::ExposeInterface;

pub fn convert_to_value<T: Into<Value>>(items: Vec<T>) -> Vec<Value> {
    items.into_iter().map(|item| item.into()).collect()
}

pub fn get_search_filters(
    constraints: &api_models::payments::PaymentListFilterConstraints,
) -> api_models::analytics::search::SearchFilters {
    api_models::analytics::search::SearchFilters {
        payment_method: constraints.payment_method.clone(),
        currency: constraints.currency.clone(),
        status: constraints.status.clone(),
        payment_method_type: constraints.payment_method_type.clone(),
        authentication_type: constraints.authentication_type.clone(),
        card_network: constraints.card_network.clone(),
        connector: constraints.connector.clone(),
        card_discovery: constraints.card_discovery.clone(),
        customer_id: constraints
            .customer_id
            .as_ref()
            .map(|customer_id| vec![customer_id.clone()]),
        payment_id: constraints
            .payment_id
            .as_ref()
            .map(|payment_id| vec![payment_id.clone()]),
        merchant_order_reference_id: constraints
            .merchant_order_reference_id
            .as_ref()
            .map(|merchant_order_reference_id| vec![merchant_order_reference_id.clone()]),
        customer_email: constraints.customer_email.as_ref().map(|customer_email| {
            vec![common_utils::hashing::HashedString::from(
                customer_email.clone().expose(),
            )]
        }),
        search_tags: None,
        card_last_4: None,
        amount: None,
        amount_filter: constraints.amount_filter.clone(),
    }
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
        if let Some(currency) = filters.currency {
            if !currency.is_empty() {
                let currency_strings: Vec<String> = currency
                    .iter()
                    .map(|currency| currency.to_string())
                    .collect();
                query_builder
                    .add_filter_clause(
                        "currency.keyword".to_string(),
                        convert_to_value(currency_strings),
                    )
                    .switch()?;
            }
        };
        if let Some(status) = filters.status {
            if !status.is_empty() {
                let status_strings: Vec<String> =
                    status.iter().map(|status| status.to_string()).collect();
                query_builder
                    .add_filter_clause(
                        "status.keyword".to_string(),
                        convert_to_value(status_strings),
                    )
                    .switch()?;
            }
        };
        if let Some(payment_method) = filters.payment_method {
            if !payment_method.is_empty() {
                let payment_method_strings: Vec<String> = payment_method
                    .iter()
                    .map(|payment_method| payment_method.to_string())
                    .collect();
                query_builder
                    .add_filter_clause(
                        "payment_method.keyword".to_string(),
                        convert_to_value(payment_method_strings),
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
                let connector_strings: Vec<String> =
                    connector.iter().map(|connector| connector.to_string()).collect();
                query_builder
                    .add_filter_clause(
                        "connector.keyword".to_string(),
                        convert_to_value(connector_strings),
                    )
                    .switch()?;
            }
        };
        if let Some(payment_method_type) = filters.payment_method_type {
            if !payment_method_type.is_empty() {
                let payment_method_type_strings: Vec<String> = payment_method_type
                    .iter()
                    .map(|payment_method_type| payment_method_type.to_string())
                    .collect();
                query_builder
                    .add_filter_clause(
                        "payment_method_type.keyword".to_string(),
                        convert_to_value(payment_method_type_strings),
                    )
                    .switch()?;
            }
        };
        if let Some(card_network) = filters.card_network {
            if !card_network.is_empty() {
                let card_network_strings: Vec<String> = card_network
                    .iter()
                    .map(|card_network| card_network.to_string())
                    .collect();
                query_builder
                    .add_filter_clause(
                        "card_network.keyword".to_string(),
                        convert_to_value(card_network_strings),
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
                let payment_id_strings: Vec<String> = payment_id
                    .iter()
                    .map(|payment_id| payment_id.get_string_repr().to_string())
                    .collect();
                query_builder
                    .add_filter_clause(
                        "payment_id.keyword".to_string(),
                        convert_to_value(payment_id_strings),
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
                let customer_id_strings: Vec<String> = customer_id
                    .iter()
                    .map(|customer_id| customer_id.get_string_repr().to_string())
                    .collect();
                query_builder
                    .add_filter_clause(
                        "customer_id.keyword".to_string(),
                        convert_to_value(customer_id_strings),
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
        if let Some(currency) = filters.currency {
            if !currency.is_empty() {
                let currency_strings: Vec<String> =
                    currency.iter().map(|currency| currency.to_string()).collect();
                query_builder
                    .add_filter_clause(
                        "currency.keyword".to_string(),
                        convert_to_value(currency_strings),
                    )
                    .switch()?;
            }
        };
        if let Some(status) = filters.status {
            if !status.is_empty() {
                let status_strings: Vec<String> = status.iter().map(|status| status.to_string()).collect();
                query_builder
                    .add_filter_clause(
                        "status.keyword".to_string(),
                        convert_to_value(status_strings),
                    )
                    .switch()?;
            }
        };
        if let Some(payment_method) = filters.payment_method {
            if !payment_method.is_empty() {
                let payment_method_strings: Vec<String> =
                    payment_method.iter().map(|payment_method| payment_method.to_string()).collect();
                query_builder
                    .add_filter_clause(
                        "payment_method.keyword".to_string(),
                        convert_to_value(payment_method_strings),
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
        if let Some(authentication_type) = filters.authentication_type {
            if !authentication_type.is_empty() {
                let authentication_type_strings: Vec<String> = authentication_type
                    .iter()
                    .map(|at| at.to_string())
                    .collect();
                query_builder
                    .add_filter_clause(
                        "authentication_type.keyword".to_string(),
                        convert_to_value(authentication_type_strings),
                    )
                    .switch()?;
            }
        };
        if let Some(card_discovery) = filters.card_discovery {
            if !card_discovery.is_empty() {
                let card_discovery_strings: Vec<String> = card_discovery
                    .iter()
                    .map(|card_discovery| card_discovery.to_string())
                    .collect();
                query_builder
                    .add_filter_clause(
                        "card_discovery.keyword".to_string(),
                        convert_to_value(card_discovery_strings),
                    )
                    .switch()?;
            }
        };
        if let Some(merchant_order_reference_id) = filters.merchant_order_reference_id {
            if !merchant_order_reference_id.is_empty() {
                query_builder
                    .add_filter_clause(
                        "merchant_order_reference_id.keyword".to_string(),
                        convert_to_value(merchant_order_reference_id),
                    )
                    .switch()?;
            }
        };
        if let Some(amount_filter) = filters.amount_filter.as_ref() {
            if amount_filter.start_amount.is_some() || amount_filter.end_amount.is_some() {
                let amount_range = crate::opensearch::OpensearchRange {
                    gte: amount_filter.start_amount,
                    lte: amount_filter.end_amount,
                };
                query_builder.set_amount_range(amount_range).switch()?;
            }
        };
        if let Some(connector) = filters.connector {
            if !connector.is_empty() {
                let connector_strings: Vec<String> =
                    connector.iter().map(|c| c.to_string()).collect();
                query_builder
                    .add_filter_clause(
                        "connector.keyword".to_string(),
                        convert_to_value(connector_strings),
                    )
                    .switch()?;
            }
        };
        if let Some(payment_method_type) = filters.payment_method_type {
            if !payment_method_type.is_empty() {
                let payment_method_type_strings: Vec<String> = payment_method_type
                    .iter()
                    .map(|pmt| pmt.to_string())
                    .collect();
                query_builder
                    .add_filter_clause(
                        "payment_method_type.keyword".to_string(),
                        convert_to_value(payment_method_type_strings),
                    )
                    .switch()?;
            }
        };
        if let Some(card_network) = filters.card_network {
            if !card_network.is_empty() {
                let card_network_strings: Vec<String> =
                    card_network.iter().map(|cn| cn.to_string()).collect();
                query_builder
                    .add_filter_clause(
                        "card_network.keyword".to_string(),
                        convert_to_value(card_network_strings),
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
                let payment_id_strings: Vec<String> = payment_id
                    .iter()
                    .map(|payment_id| payment_id.get_string_repr().to_string())
                    .collect();
                query_builder
                    .add_filter_clause(
                        "payment_id.keyword".to_string(),
                        convert_to_value(payment_id_strings),
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
                let customer_id_strings: Vec<String> = customer_id
                    .iter()
                    .map(|customer_id| customer_id.get_string_repr().to_string())
                    .collect();
                query_builder
                    .add_filter_clause(
                        "customer_id.keyword".to_string(),
                        convert_to_value(customer_id_strings),
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
