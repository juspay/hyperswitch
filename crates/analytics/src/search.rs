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

#[cfg(all(test, feature = "deja"))]
mod tests {
    use api_models::analytics::search::SearchIndex;

    use crate::{
        enums::AuthInfo,
        opensearch::{OpenSearchQuery, OpenSearchQueryBuilder},
    };

    #[allow(
        clippy::expect_used,
        reason = "test helper: a free fn, so allow-expect-in-tests does not cover it; a fixture whose org id will not parse should fail the test loudly"
    )]
    fn org_scope(org_id: &str) -> AuthInfo {
        AuthInfo::OrgLevel {
            org_id: common_utils::id_type::OrganizationId::try_from(std::borrow::Cow::Owned(
                org_id.to_owned(),
            ))
            .expect("valid organization id"),
        }
    }

    fn builder(search_params: Vec<AuthInfo>) -> OpenSearchQueryBuilder {
        OpenSearchQueryBuilder::new(
            OpenSearchQuery::Search(SearchIndex::PaymentIntents),
            "example".to_string(),
            search_params,
            None,
        )
    }

    /// The missed-query body has to deserialize into BOTH shapes this seam's
    /// callers ask for, because the arm cannot tell which one asked: the real
    /// call moves the query builder, so the miss arm cannot read `query_type`.
    /// Asserted rather than argued — the arm rests on two serde properties in
    /// `api_models` that someone could remove without ever reading this file,
    /// and a body the caller cannot deserialize turns a survivable miss back
    /// into a failed request.
    #[test]
    fn the_missed_query_body_deserializes_for_both_callers() {
        let body = super::deja_empty_result_body();

        let single = serde_json::from_str::<api_models::analytics::search::OpensearchOutput>(&body)
            .expect("the single-index caller must be able to read the missed-query body");
        match single {
            api_models::analytics::search::OpensearchOutput::Success(success) => {
                assert_eq!(success.hits.total.value, 0);
                assert!(success.hits.hits.is_empty());
            }
            api_models::analytics::search::OpensearchOutput::Error(error) => {
                panic!("the untagged enum matched Error, not Success: {error:?}")
            }
        }

        let multi = serde_json::from_str::<api_models::analytics::search::OpenMsearchOutput>(&body)
            .expect("the multi-index caller must be able to read the missed-query body");
        assert!(
            multi.responses.is_empty(),
            "the multi-index caller zips responses against its index list"
        );
        assert!(multi.error.is_none());
    }

    #[test]
    fn query_args_include_stable_auth_scope_identity() {
        let first = builder(vec![org_scope("org_a"), org_scope("org_b")]);
        let same = builder(vec![org_scope("org_a"), org_scope("org_b")]);
        let reversed = builder(vec![org_scope("org_b"), org_scope("org_a")]);
        let different = builder(vec![org_scope("org_a"), org_scope("org_c")]);

        let make_merchant_scope =
            |merchant_ids: Vec<common_utils::id_type::MerchantId>| AuthInfo::MerchantLevel {
                org_id: common_utils::id_type::OrganizationId::try_from(
                    std::borrow::Cow::Borrowed("org_a"),
                )
                .expect("valid organization id"),
                merchant_ids,
                processor_merchant_ids: None,
            };
        let merchant_ids_reordered = builder(vec![make_merchant_scope(vec![
            common_utils::id_type::MerchantId::try_from(std::borrow::Cow::Borrowed("merchant_a"))
                .expect("valid merchant id"),
            common_utils::id_type::MerchantId::try_from(std::borrow::Cow::Borrowed("merchant_b"))
                .expect("valid merchant id"),
        ])]);
        let merchant_ids_sorted = builder(vec![make_merchant_scope(vec![
            common_utils::id_type::MerchantId::try_from(std::borrow::Cow::Borrowed("merchant_b"))
                .expect("valid merchant id"),
            common_utils::id_type::MerchantId::try_from(std::borrow::Cow::Borrowed("merchant_a"))
                .expect("valid merchant id"),
        ])]);

        assert_eq!(
            merchant_ids_reordered.deja_args(),
            merchant_ids_sorted.deja_args()
        );
        let expected = first.deja_args();
        assert_eq!(expected, same.deja_args());
        assert_eq!(expected, reversed.deja_args());
        assert_ne!(expected, different.deja_args());
    }

    /// The tests above call the helpers; this one reads the seam's own
    /// declaration, so naming a different `on_miss` body or `args` expression
    /// on `execute_search_to_text` fails here rather than passing unnoticed.
    #[test]
    fn the_seam_names_the_helpers_the_tests_exercise() {
        let source = include_str!("search.rs");
        // The last match: this test's own text repeats the anchor above the seam.
        let (_, after_operation) = source
            .rsplit_once("operation = \"execute_search\",")
            .expect("the seam must declare its operation");
        let (declaration, _) = after_operation
            // Not a bare `)]`: the comment inside the attribute quotes `#[serde(...)]`.
            .split_once("\n)]")
            .expect("the seam's attribute must be closed");
        for expected in [
            "on_miss = Ok(deja_empty_result_body())",
            "args = query_builder.deja_args()",
        ] {
            assert!(
                declaration.contains(expected),
                "the seam must name `{expected}`: these two expressions are what the \
                 attribute names, and a test that calls the helpers directly would not \
                 notice them being swapped: {declaration}"
            );
        }
    }
}

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

/// The body a missed OpenSearch query answers with.
///
/// A module-private function rather than a method on the miss, because the value
/// is not derived from the miss: it is a fixed document belonging to this
/// boundary's response type. Named so the arm and the test read the same bytes,
/// since a literal duplicated into a test can drift from the one that ships.
#[cfg(feature = "deja")]
fn deja_empty_result_body() -> String {
    serde_json::json!({
        "hits": {"total": {"value": 0}, "hits": []},
        "responses": [],
    })
    .to_string()
}

/// The OpenSearch query, resolved to the response text.
///
/// The seam sits here rather than on `OpenSearchClient::execute` because that
/// returns a streaming body, which cannot be captured or compared. The text IS
/// the third-party state that reaches the response.
///
/// `Http` rather than `Db`: the index is external state read over HTTP and
/// substituted from the tape, not part of the seeded store, so the seed planner
/// must not try to reconstruct it. A miss arm cannot issue a live query — the
/// reconstruct closure is sync and this is async — and running one would reach a
/// third-party index anyway, which is what this boundary exists to keep out.
///
/// So the miss arm answers with a well-formed empty result set, and that is a
/// declared fabrication: it asserts nothing matched, which the recording never
/// showed, making the search a false negative contained to a replay. Its
/// provenance is the ledger's `SubstituteOutcome::Synthesized`, not a marker
/// field in the body, which would be a one-sided change to one side of a
/// comparison and would destroy the evidence rather than carry it. A constant
/// is tolerable only because of where the value goes: into a response body,
/// never into a lookup key.
#[cfg_attr(
    feature = "deja",
    deja::boundary(
        boundary = "opensearch",
        component = "analytics::search",
        operation = "execute_search",
        op = Read,
        replay = Substitute,
        effect = Http,
        returns = Value,
        codec = deja::codec::ResultCodec::<String, OpenSearchError>,
        args = query_builder.deja_args(),
        // One body that both callers deserialize, and deliberately not a
        // function of `query_builder`: the real call MOVES it, so an arm that
        // read `query_type` to pick a shape would not compile, and reading the
        // variant back out of the recorded args image would be a string match
        // against a `Debug` rendering. This body cannot be wrong for either
        // caller — `OpensearchOutput` is `#[serde(untagged)]` with no
        // `deny_unknown_fields`, so it matches `Success` on `hits`, and
        // `OpenMsearchOutput::responses` is `#[serde(default)]`, so it matches on
        // an empty list. What that costs: `msearch_results` zips `responses`
        // against its index list, so it returns no index entries at all rather
        // than one empty entry per index.
        on_miss = Ok(deja_empty_result_body()),
    )
)]
async fn execute_search_to_text(
    client: &OpenSearchClient,
    query_builder: OpenSearchQueryBuilder,
) -> CustomResult<String, OpenSearchError> {
    client
        .execute(query_builder)
        .await
        .change_context(OpenSearchError::ConnectionError)?
        .text()
        .await
        .change_context(OpenSearchError::ResponseError)
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
        append_filter!(
            query_builder,
            filters,
            active_attempt_id,
            "active_attempt_id.keyword"
        );
        append_filter!(
            query_builder,
            filters,
            merchant_connector_id,
            "merchant_connector_id.keyword"
        );
        append_filter!(query_builder, filters, card_issuer, "card_issuer.keyword");
        append_filter!(
            query_builder,
            filters,
            routing_approach,
            "routing_approach.keyword"
        );
        append_filter!(
            query_builder,
            filters,
            refunds_status,
            "refunds_status.keyword"
        );
        append_filter!(
            query_builder,
            filters,
            dispute_status,
            "dispute_status.keyword"
        );
        append_filter!(
            query_builder,
            filters,
            client_source,
            "client_source.keyword"
        );
        append_filter!(
            query_builder,
            filters,
            client_version,
            "client_version.keyword"
        );
        append_filter!(query_builder, filters, first_attempt, "first_attempt");
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

    if let Some(time_range) = req.time_range {
        query_builder.set_time_range(time_range.into()).switch()?;
    };

    let response_text: OpenMsearchOutput = execute_search_to_text(client, query_builder)
        .await
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
        append_filter!(
            query_builder,
            filters,
            active_attempt_id,
            "active_attempt_id.keyword"
        );
        append_filter!(
            query_builder,
            filters,
            merchant_connector_id,
            "merchant_connector_id.keyword"
        );
        append_filter!(query_builder, filters, card_issuer, "card_issuer.keyword");
        append_filter!(
            query_builder,
            filters,
            routing_approach,
            "routing_approach.keyword"
        );
        append_filter!(
            query_builder,
            filters,
            refunds_status,
            "refunds_status.keyword"
        );
        append_filter!(
            query_builder,
            filters,
            dispute_status,
            "dispute_status.keyword"
        );
        append_filter!(
            query_builder,
            filters,
            client_source,
            "client_source.keyword"
        );
        append_filter!(
            query_builder,
            filters,
            client_version,
            "client_version.keyword"
        );
        append_filter!(query_builder, filters, first_attempt, "first_attempt");
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

    let response_text: OpensearchOutput = execute_search_to_text(client, query_builder)
        .await
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
