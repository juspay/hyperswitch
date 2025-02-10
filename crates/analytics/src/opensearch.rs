use std::collections::HashSet;

use api_models::{
    analytics::search::SearchIndex,
    errors::types::{ApiError, ApiErrorResponse},
};
use aws_config::{self, meta::region::RegionProviderChain, Region};
use common_utils::{
    errors::{CustomResult, ErrorSwitch},
    types::TimeRange,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::errors::{StorageError, StorageResult};
use opensearch::{
    auth::Credentials,
    cert::CertificateValidation,
    cluster::{Cluster, ClusterHealthParts},
    http::{
        request::JsonBody,
        response::Response,
        transport::{SingleNodeConnectionPool, Transport, TransportBuilder},
        Url,
    },
    MsearchParts, OpenSearch, SearchParts,
};
use serde_json::{json, Map, Value};
use storage_impl::errors::ApplicationError;
use time::PrimitiveDateTime;

use super::{health_check::HealthCheck, query::QueryResult, types::QueryExecutionError};
use crate::{enums::AuthInfo, query::QueryBuildingError};

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(tag = "auth")]
#[serde(rename_all = "lowercase")]
pub enum OpenSearchAuth {
    Basic { username: String, password: String },
    Aws { region: String },
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct OpenSearchIndexes {
    pub payment_attempts: String,
    pub payment_intents: String,
    pub refunds: String,
    pub disputes: String,
    pub sessionizer_payment_attempts: String,
    pub sessionizer_payment_intents: String,
    pub sessionizer_refunds: String,
    pub sessionizer_disputes: String,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
pub struct OpensearchTimeRange {
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub gte: PrimitiveDateTime,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub lte: Option<PrimitiveDateTime>,
}

impl From<TimeRange> for OpensearchTimeRange {
    fn from(time_range: TimeRange) -> Self {
        Self {
            gte: time_range.start_time,
            lte: time_range.end_time,
        }
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct OpenSearchConfig {
    host: String,
    auth: OpenSearchAuth,
    indexes: OpenSearchIndexes,
}

impl Default for OpenSearchConfig {
    fn default() -> Self {
        Self {
            host: "https://localhost:9200".to_string(),
            auth: OpenSearchAuth::Basic {
                username: "admin".to_string(),
                password: "admin".to_string(),
            },
            indexes: OpenSearchIndexes {
                payment_attempts: "hyperswitch-payment-attempt-events".to_string(),
                payment_intents: "hyperswitch-payment-intent-events".to_string(),
                refunds: "hyperswitch-refund-events".to_string(),
                disputes: "hyperswitch-dispute-events".to_string(),
                sessionizer_payment_attempts: "sessionizer-payment-attempt-events".to_string(),
                sessionizer_payment_intents: "sessionizer-payment-intent-events".to_string(),
                sessionizer_refunds: "sessionizer-refund-events".to_string(),
                sessionizer_disputes: "sessionizer-dispute-events".to_string(),
            },
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OpenSearchError {
    #[error("Opensearch connection error")]
    ConnectionError,
    #[error("Opensearch NON-200 response content: '{0}'")]
    ResponseNotOK(String),
    #[error("Opensearch bad request error")]
    BadRequestError(String),
    #[error("Opensearch response error")]
    ResponseError,
    #[error("Opensearch query building error")]
    QueryBuildingError,
    #[error("Opensearch deserialisation error")]
    DeserialisationError,
    #[error("Opensearch index access not present error: {0:?}")]
    IndexAccessNotPermittedError(SearchIndex),
    #[error("Opensearch unknown error")]
    UnknownError,
    #[error("Opensearch access forbidden error")]
    AccessForbiddenError,
}

impl ErrorSwitch<OpenSearchError> for QueryBuildingError {
    fn switch(&self) -> OpenSearchError {
        OpenSearchError::QueryBuildingError
    }
}

impl ErrorSwitch<ApiErrorResponse> for OpenSearchError {
    fn switch(&self) -> ApiErrorResponse {
        match self {
            Self::ConnectionError => ApiErrorResponse::InternalServerError(ApiError::new(
                "IR",
                0,
                "Connection error",
                None,
            )),
            Self::BadRequestError(response) => {
                ApiErrorResponse::BadRequest(ApiError::new("IR", 1, response.to_string(), None))
            }
            Self::ResponseNotOK(response) => ApiErrorResponse::InternalServerError(ApiError::new(
                "IR",
                1,
                format!("Something went wrong {}", response),
                None,
            )),
            Self::ResponseError => ApiErrorResponse::InternalServerError(ApiError::new(
                "IR",
                2,
                "Something went wrong",
                None,
            )),
            Self::QueryBuildingError => ApiErrorResponse::InternalServerError(ApiError::new(
                "IR",
                3,
                "Query building error",
                None,
            )),
            Self::DeserialisationError => ApiErrorResponse::InternalServerError(ApiError::new(
                "IR",
                4,
                "Deserialisation error",
                None,
            )),
            Self::IndexAccessNotPermittedError(index) => {
                ApiErrorResponse::ForbiddenCommonResource(ApiError::new(
                    "IR",
                    5,
                    format!("Index access not permitted: {index:?}"),
                    None,
                ))
            }
            Self::UnknownError => {
                ApiErrorResponse::InternalServerError(ApiError::new("IR", 6, "Unknown error", None))
            }
            Self::AccessForbiddenError => ApiErrorResponse::ForbiddenCommonResource(ApiError::new(
                "IR",
                7,
                "Access Forbidden error",
                None,
            )),
        }
    }
}

#[derive(Clone, Debug)]
pub struct OpenSearchClient {
    pub client: OpenSearch,
    pub transport: Transport,
    pub indexes: OpenSearchIndexes,
}

impl OpenSearchClient {
    pub async fn create(conf: &OpenSearchConfig) -> CustomResult<Self, OpenSearchError> {
        let url = Url::parse(&conf.host).map_err(|_| OpenSearchError::ConnectionError)?;
        let transport = match &conf.auth {
            OpenSearchAuth::Basic { username, password } => {
                let credentials = Credentials::Basic(username.clone(), password.clone());
                TransportBuilder::new(SingleNodeConnectionPool::new(url))
                    .cert_validation(CertificateValidation::None)
                    .auth(credentials)
                    .build()
                    .map_err(|_| OpenSearchError::ConnectionError)?
            }
            OpenSearchAuth::Aws { region } => {
                let region_provider = RegionProviderChain::first_try(Region::new(region.clone()));
                let sdk_config = aws_config::from_env().region(region_provider).load().await;
                let conn_pool = SingleNodeConnectionPool::new(url);
                TransportBuilder::new(conn_pool)
                    .auth(
                        sdk_config
                            .clone()
                            .try_into()
                            .map_err(|_| OpenSearchError::ConnectionError)?,
                    )
                    .service_name("es")
                    .build()
                    .map_err(|_| OpenSearchError::ConnectionError)?
            }
        };
        Ok(Self {
            transport: transport.clone(),
            client: OpenSearch::new(transport),
            indexes: conf.indexes.clone(),
        })
    }

    pub fn search_index_to_opensearch_index(&self, index: SearchIndex) -> String {
        match index {
            SearchIndex::PaymentAttempts => self.indexes.payment_attempts.clone(),
            SearchIndex::PaymentIntents => self.indexes.payment_intents.clone(),
            SearchIndex::Refunds => self.indexes.refunds.clone(),
            SearchIndex::Disputes => self.indexes.disputes.clone(),
            SearchIndex::SessionizerPaymentAttempts => {
                self.indexes.sessionizer_payment_attempts.clone()
            }
            SearchIndex::SessionizerPaymentIntents => {
                self.indexes.sessionizer_payment_intents.clone()
            }
            SearchIndex::SessionizerRefunds => self.indexes.sessionizer_refunds.clone(),
            SearchIndex::SessionizerDisputes => self.indexes.sessionizer_disputes.clone(),
        }
    }

    pub async fn execute(
        &self,
        query_builder: OpenSearchQueryBuilder,
    ) -> CustomResult<Response, OpenSearchError> {
        match query_builder.query_type {
            OpenSearchQuery::Msearch(ref indexes) => {
                let payload = query_builder
                    .construct_payload(indexes)
                    .change_context(OpenSearchError::QueryBuildingError)?;

                let payload_with_indexes = payload.into_iter().zip(indexes).fold(
                    Vec::new(),
                    |mut payload_with_indexes, (index_hit, index)| {
                        payload_with_indexes.push(
                            json!({"index": self.search_index_to_opensearch_index(*index)}).into(),
                        );
                        payload_with_indexes.push(JsonBody::new(index_hit.clone()));
                        payload_with_indexes
                    },
                );

                self.client
                    .msearch(MsearchParts::None)
                    .body(payload_with_indexes)
                    .send()
                    .await
                    .change_context(OpenSearchError::ResponseError)
            }
            OpenSearchQuery::Search(index) => {
                let payload = query_builder
                    .clone()
                    .construct_payload(&[index])
                    .change_context(OpenSearchError::QueryBuildingError)?;

                let final_payload = payload.first().unwrap_or(&Value::Null);

                self.client
                    .search(SearchParts::Index(&[
                        &self.search_index_to_opensearch_index(index)
                    ]))
                    .from(query_builder.offset.unwrap_or(0))
                    .size(query_builder.count.unwrap_or(10))
                    .body(final_payload)
                    .send()
                    .await
                    .change_context(OpenSearchError::ResponseError)
            }
        }
    }
}

#[async_trait::async_trait]
impl HealthCheck for OpenSearchClient {
    async fn deep_health_check(&self) -> CustomResult<(), QueryExecutionError> {
        let health = Cluster::new(&self.transport)
            .health(ClusterHealthParts::None)
            .send()
            .await
            .change_context(QueryExecutionError::DatabaseError)?
            .json::<OpenSearchHealth>()
            .await
            .change_context(QueryExecutionError::DatabaseError)?;

        if health.status != OpenSearchHealthStatus::Red {
            Ok(())
        } else {
            Err::<(), error_stack::Report<QueryExecutionError>>(
                QueryExecutionError::DatabaseError.into(),
            )
            .attach_printable_lazy(|| format!("Opensearch cluster health is red: {health:?}"))
        }
    }
}

impl OpenSearchIndexes {
    pub fn validate(&self) -> Result<(), ApplicationError> {
        use common_utils::{ext_traits::ConfigExt, fp_utils::when};

        when(self.payment_attempts.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "Opensearch Payment Attempts index must not be empty".into(),
            ))
        })?;

        when(self.payment_intents.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "Opensearch Payment Intents index must not be empty".into(),
            ))
        })?;

        when(self.refunds.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "Opensearch Refunds index must not be empty".into(),
            ))
        })?;

        when(self.disputes.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "Opensearch Disputes index must not be empty".into(),
            ))
        })?;

        when(
            self.sessionizer_payment_attempts.is_default_or_empty(),
            || {
                Err(ApplicationError::InvalidConfigurationValueError(
                    "Opensearch Sessionizer Payment Attempts index must not be empty".into(),
                ))
            },
        )?;

        when(
            self.sessionizer_payment_intents.is_default_or_empty(),
            || {
                Err(ApplicationError::InvalidConfigurationValueError(
                    "Opensearch Sessionizer Payment Intents index must not be empty".into(),
                ))
            },
        )?;

        when(self.sessionizer_refunds.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "Opensearch Sessionizer Refunds index must not be empty".into(),
            ))
        })?;

        when(self.sessionizer_disputes.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "Opensearch Sessionizer Disputes index must not be empty".into(),
            ))
        })?;

        Ok(())
    }
}

impl OpenSearchAuth {
    pub fn validate(&self) -> Result<(), ApplicationError> {
        use common_utils::{ext_traits::ConfigExt, fp_utils::when};

        match self {
            Self::Basic { username, password } => {
                when(username.is_default_or_empty(), || {
                    Err(ApplicationError::InvalidConfigurationValueError(
                        "Opensearch Basic auth username must not be empty".into(),
                    ))
                })?;

                when(password.is_default_or_empty(), || {
                    Err(ApplicationError::InvalidConfigurationValueError(
                        "Opensearch Basic auth password must not be empty".into(),
                    ))
                })?;
            }

            Self::Aws { region } => {
                when(region.is_default_or_empty(), || {
                    Err(ApplicationError::InvalidConfigurationValueError(
                        "Opensearch Aws auth region must not be empty".into(),
                    ))
                })?;
            }
        };

        Ok(())
    }
}

impl OpenSearchConfig {
    pub async fn get_opensearch_client(&self) -> StorageResult<OpenSearchClient> {
        Ok(OpenSearchClient::create(self)
            .await
            .map_err(|_| StorageError::InitializationError)?)
    }

    pub fn validate(&self) -> Result<(), ApplicationError> {
        use common_utils::{ext_traits::ConfigExt, fp_utils::when};

        when(self.host.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "Opensearch host must not be empty".into(),
            ))
        })?;

        self.indexes.validate()?;

        self.auth.validate()?;

        Ok(())
    }
}
#[derive(Debug, serde::Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OpenSearchHealthStatus {
    Red,
    Green,
    Yellow,
}

#[derive(Debug, serde::Deserialize)]
pub struct OpenSearchHealth {
    pub status: OpenSearchHealthStatus,
}

#[derive(Debug, Clone)]
pub enum OpenSearchQuery {
    Msearch(Vec<SearchIndex>),
    Search(SearchIndex),
}

#[derive(Debug, Clone)]
pub struct OpenSearchQueryBuilder {
    pub query_type: OpenSearchQuery,
    pub query: String,
    pub offset: Option<i64>,
    pub count: Option<i64>,
    pub filters: Vec<(String, Vec<Value>)>,
    pub time_range: Option<OpensearchTimeRange>,
    search_params: Vec<AuthInfo>,
    case_sensitive_fields: HashSet<&'static str>,
}

impl OpenSearchQueryBuilder {
    pub fn new(query_type: OpenSearchQuery, query: String, search_params: Vec<AuthInfo>) -> Self {
        Self {
            query_type,
            query,
            search_params,
            offset: Default::default(),
            count: Default::default(),
            filters: Default::default(),
            time_range: Default::default(),
            case_sensitive_fields: HashSet::from([
                "customer_email.keyword",
                "search_tags.keyword",
                "card_last_4.keyword",
                "payment_id.keyword",
                "amount",
                "customer_id.keyword",
            ]),
        }
    }

    pub fn set_offset_n_count(&mut self, offset: i64, count: i64) -> QueryResult<()> {
        self.offset = Some(offset);
        self.count = Some(count);
        Ok(())
    }

    pub fn set_time_range(&mut self, time_range: OpensearchTimeRange) -> QueryResult<()> {
        self.time_range = Some(time_range);
        Ok(())
    }

    pub fn add_filter_clause(&mut self, lhs: String, rhs: Vec<Value>) -> QueryResult<()> {
        self.filters.push((lhs, rhs));
        Ok(())
    }

    pub fn get_status_field(&self, index: SearchIndex) -> &str {
        match index {
            SearchIndex::Refunds | SearchIndex::SessionizerRefunds => "refund_status.keyword",
            SearchIndex::Disputes | SearchIndex::SessionizerDisputes => "dispute_status.keyword",
            _ => "status.keyword",
        }
    }

    pub fn get_amount_field(&self, index: SearchIndex) -> &str {
        match index {
            SearchIndex::Refunds | SearchIndex::SessionizerRefunds => "refund_amount",
            SearchIndex::Disputes | SearchIndex::SessionizerDisputes => "dispute_amount",
            _ => "amount",
        }
    }

    pub fn build_filter_array(
        &self,
        case_sensitive_filters: Vec<&(String, Vec<Value>)>,
        index: SearchIndex,
    ) -> Vec<Value> {
        let mut filter_array = Vec::new();
        if !self.query.is_empty() {
            filter_array.push(json!({
                "multi_match": {
                    "type": "phrase",
                    "query": self.query,
                    "lenient": true
                }
            }));
        }

        let case_sensitive_json_filters = case_sensitive_filters
            .into_iter()
            .map(|(k, v)| {
                let key = if *k == "amount" {
                    self.get_amount_field(index).to_string()
                } else {
                    k.clone()
                };
                json!({"terms": {key: v}})
            })
            .collect::<Vec<Value>>();

        filter_array.extend(case_sensitive_json_filters);

        if let Some(ref time_range) = self.time_range {
            let range = json!(time_range);
            filter_array.push(json!({
                "range": {
                    "@timestamp": range
                }
            }));
        }

        filter_array
    }

    pub fn build_case_insensitive_filters(
        &self,
        mut payload: Value,
        case_insensitive_filters: &[&(String, Vec<Value>)],
        auth_array: Vec<Value>,
        index: SearchIndex,
    ) -> Value {
        let mut must_array = case_insensitive_filters
            .iter()
            .map(|(k, v)| {
                let key = if *k == "status.keyword" {
                    self.get_status_field(index).to_string()
                } else {
                    k.clone()
                };
                json!({
                    "bool": {
                        "must": [
                            {
                                "bool": {
                                    "should": v.iter().map(|value| {
                                        json!({
                                            "term": {
                                                format!("{}", key): {
                                                    "value": value,
                                                    "case_insensitive": true
                                                }
                                            }
                                        })
                                    }).collect::<Vec<Value>>(),
                                    "minimum_should_match": 1
                                }
                            }
                        ]
                    }
                })
            })
            .collect::<Vec<Value>>();

        must_array.push(json!({ "bool": {
            "must": [
                {
                    "bool": {
                        "should": auth_array,
                        "minimum_should_match": 1
                    }
                }
            ]
        }}));

        if let Some(query) = payload.get_mut("query") {
            if let Some(bool_obj) = query.get_mut("bool") {
                if let Some(bool_map) = bool_obj.as_object_mut() {
                    bool_map.insert("must".to_string(), Value::Array(must_array));
                }
            }
        }

        payload
    }

    pub fn build_auth_array(&self) -> Vec<Value> {
        self.search_params
            .iter()
            .map(|user_level| match user_level {
                AuthInfo::OrgLevel { org_id } => {
                    let must_clauses = vec![json!({
                        "term": {
                            "organization_id.keyword": {
                                "value": org_id
                            }
                        }
                    })];

                    json!({
                        "bool": {
                            "must": must_clauses
                        }
                    })
                }
                AuthInfo::MerchantLevel {
                    org_id,
                    merchant_ids,
                } => {
                    let must_clauses = vec![
                        json!({
                            "term": {
                                "organization_id.keyword": {
                                    "value": org_id
                                }
                            }
                        }),
                        json!({
                            "terms": {
                                "merchant_id.keyword": merchant_ids
                            }
                        }),
                    ];

                    json!({
                        "bool": {
                            "must": must_clauses
                        }
                    })
                }
                AuthInfo::ProfileLevel {
                    org_id,
                    merchant_id,
                    profile_ids,
                } => {
                    let must_clauses = vec![
                        json!({
                            "term": {
                                "organization_id.keyword": {
                                    "value": org_id
                                }
                            }
                        }),
                        json!({
                            "term": {
                                "merchant_id.keyword": {
                                    "value": merchant_id
                                }
                            }
                        }),
                        json!({
                            "terms": {
                                "profile_id.keyword": profile_ids
                            }
                        }),
                    ];

                    json!({
                        "bool": {
                            "must": must_clauses
                        }
                    })
                }
            })
            .collect::<Vec<Value>>()
    }

    /// # Panics
    ///
    /// This function will panic if:
    ///
    /// * The structure of the JSON query is not as expected (e.g., missing keys or incorrect types).
    ///
    /// Ensure that the input data and the structure of the query are valid and correctly handled.
    pub fn construct_payload(&self, indexes: &[SearchIndex]) -> QueryResult<Vec<Value>> {
        let mut query_obj = Map::new();
        let bool_obj = Map::new();

        let (case_sensitive_filters, case_insensitive_filters): (Vec<_>, Vec<_>) = self
            .filters
            .iter()
            .partition(|(k, _)| self.case_sensitive_fields.contains(k.as_str()));

        let should_array = self.build_auth_array();

        query_obj.insert("bool".to_string(), Value::Object(bool_obj.clone()));

        let mut sort_obj = Map::new();
        sort_obj.insert(
            "@timestamp".to_string(),
            json!({
                "order": "desc"
            }),
        );

        Ok(indexes
            .iter()
            .map(|index| {
                let mut payload = json!({
                    "query": query_obj.clone(),
                    "sort": [
                        Value::Object(sort_obj.clone())
                    ]
                });
                let filter_array = self.build_filter_array(case_sensitive_filters.clone(), *index);
                if !filter_array.is_empty() {
                    payload
                        .get_mut("query")
                        .and_then(|query| query.get_mut("bool"))
                        .and_then(|bool_obj| bool_obj.as_object_mut())
                        .map(|bool_map| {
                            bool_map.insert("filter".to_string(), Value::Array(filter_array));
                        });
                }
                payload = self.build_case_insensitive_filters(
                    payload,
                    &case_insensitive_filters,
                    should_array.clone(),
                    *index,
                );
                payload
            })
            .collect::<Vec<Value>>())
    }
}
