use std::collections::HashSet;

use api_models::{
    analytics::search::{OpensearchRange, SearchIndex},
    errors::types::{ApiError, ApiErrorResponse},
    payments::{Order, SortBy, SortOn},
};
use aws_config::{self, meta::region::RegionProviderChain, Region};
use common_utils::{
    errors::{CustomResult, ErrorSwitch},
    types::TimeRange,
};
use error_stack::ResultExt;
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
use storage_impl::errors::{ApplicationError, StorageError, StorageResult};
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
    pub payouts: String,
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
    #[serde(default)]
    enabled: bool,
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
                payouts: "hyperswitch-payout-events".to_string(),
                sessionizer_payment_attempts: "sessionizer-payment-attempt-events".to_string(),
                sessionizer_payment_intents: "sessionizer-payment-intent-events".to_string(),
                sessionizer_refunds: "sessionizer-refund-events".to_string(),
                sessionizer_disputes: "sessionizer-dispute-events".to_string(),
            },
            enabled: false,
        }
    }
}

// Serialisable so the deja seam on the search query can record and replay a
// failed search as faithfully as a successful one. Every variant's payload
// already serialises.
#[derive(Debug, thiserror::Error, serde::Serialize, serde::Deserialize)]
pub enum OpenSearchError {
    #[error("Opensearch is not enabled")]
    NotEnabled,
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
                format!("Something went wrong {response}"),
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
            Self::NotEnabled => ApiErrorResponse::InternalServerError(ApiError::new(
                "IR",
                8,
                "Opensearch is not enabled",
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
            SearchIndex::Payouts => self.indexes.payouts.clone(),
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

    /// The OpenSearch round trip, recorded as its status and body. Only while
    /// recording does the codec read the body here, and the caller gets the
    /// response rebuilt from the original's own parts.
    ///
    /// `Http` rather than `Db`: the index is external state, not part of the seeded
    /// store. A miss answers with an empty result set, a fabrication the ledger
    /// records as `SubstituteOutcome::Synthesized`; it goes into a response body,
    /// never into a lookup key.
    #[cfg_attr(
        feature = "deja",
        deja::boundary(
            boundary = "opensearch",
            component = "analytics::opensearch",
            operation = "execute",
            op = Read,
            replay = Substitute,
            effect = Http,
            returns = Value,
            owned_codec = ResponseCodec,
            args = query_builder.deja_args(),
            on_miss = Ok(ResponseCodec::missed_query()),
        )
    )]
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

/// Records an OpenSearch response as its status and body. `read` is the only
/// step that runs outside the recorder's firewall, so it does nothing but read
/// the body and rebuild the response from the original's own parts.
#[cfg(feature = "deja")]
struct ResponseCodec;

#[cfg(feature = "deja")]
type ResponseEnvelope = deja::codec::ResultCodec<Value, OpenSearchError>;

#[cfg(feature = "deja")]
impl deja::codec::OwnedReplayCodec for ResponseCodec {
    type Value = CustomResult<Response, OpenSearchError>;
    /// The body as read, buffered so the tape and the rebuilt response share
    /// it, or why reading it failed.
    type Read = Result<opensearch_reqwest::Body, String>;

    async fn read(value: Self::Value) -> (Self::Value, Result<Self::Read, String>) {
        let response = match value {
            Ok(response) => response,
            Err(report) => return (Err(report), Ok(Ok(opensearch_reqwest::Body::from("")))),
        };
        let status = response.status_code();
        let headers = response.headers().clone();
        let url = response.url().clone();
        let method = response.method();
        // A body whose length the original knew rebuilds as one that knows it;
        // a decoded or chunked one, whose length it did not, as a stream.
        let sized = response.content_length().is_some();
        let (body, read) = match response.bytes().await {
            Ok(bytes) => {
                // Cloning `Bytes` shares the buffer; nothing is copied.
                let read = opensearch_reqwest::Body::from(bytes.clone());
                let body = if sized {
                    opensearch_reqwest::Body::from(bytes)
                } else {
                    opensearch_reqwest::Body::wrap_stream(futures::stream::once(
                        futures::future::ready(Ok::<_, std::convert::Infallible>(bytes)),
                    ))
                };
                (body, Ok(read))
            }
            Err(error) => {
                let reason = error.to_string();
                let body = opensearch_reqwest::Body::wrap_stream(futures::stream::once(
                    futures::future::ready(Err::<Vec<u8>, _>(error)),
                ));
                (body, Err(reason))
            }
        };
        (Ok(rebuilt(status, headers, url, method, body)), Ok(read))
    }

    /// A body that could not be read records its status and why, so a replay
    /// hands back a response whose body fails as the recorded one did.
    fn record(read: Self::Read, value: &Self::Value) -> (Value, bool) {
        match value {
            Ok(response) => {
                let status = response.status_code().as_u16();
                let recorded = match read {
                    Ok(body) => {
                        let body = String::from_utf8_lossy(body.as_bytes().unwrap_or_default());
                        json!({ "status": status, "body": body })
                    }
                    Err(reason) => json!({ "status": status, "read_error": reason }),
                };
                (ResponseEnvelope::ok_envelope(&recorded), false)
            }
            Err(report) => (ResponseEnvelope::err_envelope(report), true),
        }
    }

    fn reconstruct(recorded: Value) -> Option<Self::Value> {
        use deja::codec::ReplayCodec;
        match ResponseEnvelope::reconstruct(recorded)? {
            Ok(recorded) => {
                let status = opensearch_http::StatusCode::from_u16(
                    u16::try_from(recorded.get("status")?.as_u64()?).ok()?,
                )
                .ok()?;
                if let Some(reason) = recorded.get("read_error").and_then(Value::as_str) {
                    let failure = std::io::Error::other(reason.to_owned());
                    let body = opensearch_reqwest::Body::wrap_stream(futures::stream::once(
                        futures::future::ready(Err::<Vec<u8>, _>(failure)),
                    ));
                    return Some(Ok(Self::replayed(status, body)));
                }
                let body = recorded.get("body")?.as_str()?.to_owned();
                Some(Ok(Self::replayed(status, body.into())))
            }
            Err(report) => Some(Err(report)),
        }
    }
}

#[cfg(feature = "deja")]
impl ResponseCodec {
    /// A missed query's answer: an empty result set both callers read, as
    /// `OpensearchOutput::Success` and as an `OpenMsearchOutput` with no responses.
    fn missed_query() -> Response {
        Self::replayed(
            opensearch_http::StatusCode::OK,
            Self::missed_query_body().into(),
        )
    }

    fn missed_query_body() -> String {
        json!({
            "hits": {"total": {"value": 0}, "hits": []},
            "responses": [],
        })
        .to_string()
    }

    /// A response rebuilt from the tape, which holds its status and body.
    fn replayed(status: opensearch_http::StatusCode, body: opensearch_reqwest::Body) -> Response {
        let mut response = opensearch_http::Response::new(body);
        *response.status_mut() = status;
        Response::new(
            opensearch_reqwest::Response::from(response),
            opensearch::http::Method::Post,
        )
    }
}

/// The client's own `Response` from the original's parts and a body.
#[cfg(feature = "deja")]
fn rebuilt(
    status: opensearch_http::StatusCode,
    headers: opensearch_http::HeaderMap,
    url: Url,
    method: opensearch::http::Method,
    body: opensearch_reqwest::Body,
) -> Response {
    use opensearch_reqwest::ResponseBuilderExt;
    let mut response = match opensearch_http::Response::builder().url(url).body(()) {
        Ok(response) => response.map(|()| body),
        Err(_) => opensearch_http::Response::new(body),
    };
    *response.status_mut() = status;
    *response.headers_mut() = headers;
    Response::new(opensearch_reqwest::Response::from(response), method)
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

        when(self.payouts.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "Opensearch Payouts index must not be empty".into(),
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
    pub async fn get_opensearch_client(&self) -> StorageResult<Option<OpenSearchClient>> {
        if !self.enabled {
            return Ok(None);
        }
        Ok(Some(
            OpenSearchClient::create(self)
                .await
                .change_context(StorageError::InitializationError)?,
        ))
    }

    pub fn validate(&self) -> Result<(), ApplicationError> {
        use common_utils::{ext_traits::ConfigExt, fp_utils::when};

        if !self.enabled {
            return Ok(());
        }

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
    pub amount_range: Option<OpensearchRange>,
    search_params: Vec<AuthInfo>,
    case_sensitive_fields: HashSet<&'static str>,
    pub order: Option<Order>,
}

pub enum OpenSearchComparison {
    Equal,
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
}

impl OpenSearchComparison {
    fn range_operator(&self) -> Option<&'static str> {
        match self {
            Self::Equal => None,
            Self::GreaterThan => Some("gt"),
            Self::GreaterThanOrEqual => Some("gte"),
            Self::LessThan => Some("lt"),
            Self::LessThanOrEqual => Some("lte"),
        }
    }
}

const ACTIVE_ATTEMPT_FILTER_SCRIPT: &str = r#"
    if (!params.containsKey('_source')) return false;

    def source = params._source;
    if (source == null) return false;

    def activeAttemptId = source.active_attempt_id;
    if (activeAttemptId == null) return false;

    def attemptsList = source.attempts_list;
    if (attemptsList == null) return false;

    for (def attemptObject : attemptsList) {
        if (attemptObject == null) continue;
        if (!(attemptObject instanceof Map)) continue;

        def attemptId = attemptObject.get("attempt_id");
        if (attemptId == null) continue;

        if (attemptId == activeAttemptId) {
            def fieldValueForActiveAttempt = attemptObject.get(params.field);
            if (fieldValueForActiveAttempt == null) return false;

            return params.values.contains(fieldValueForActiveAttempt);
        }
    }

    return false;
"#;

impl OpenSearchQueryBuilder {
    pub fn new(
        query_type: OpenSearchQuery,
        query: String,
        search_params: Vec<AuthInfo>,
        order: Option<Order>,
    ) -> Self {
        Self {
            query_type,
            query,
            search_params,
            offset: Default::default(),
            count: Default::default(),
            filters: Default::default(),
            time_range: Default::default(),
            amount_range: Default::default(),
            case_sensitive_fields: HashSet::from([
                "customer_email.keyword",
                "search_tags.keyword",
                "card_last_4.keyword",
                "payment_id.keyword",
                "active_attempt_id.keyword",
                "merchant_connector_id.keyword",
                "amount",
                "first_attempt",
                "customer_id.keyword",
                "merchant_order_reference_id.keyword",
            ]),
            order,
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

    pub fn set_amount_range(&mut self, amount_range: OpensearchRange) -> QueryResult<()> {
        self.amount_range = Some(amount_range);
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

    fn parse_search_amounts(&self) -> Vec<Value> {
        let query = self.query.trim();
        if query.is_empty()
            || (query.len() > 1 && query.starts_with('0') && !query.starts_with("0."))
        {
            return Vec::new();
        }

        let amount_parts = query.split('.').collect::<Vec<_>>();
        let major_units = match amount_parts.as_slice() {
            [whole] if whole.chars().all(|char| char.is_ascii_digit()) => whole.parse::<u64>().ok(),
            [whole, fractional]
                if whole.chars().all(|char| char.is_ascii_digit())
                    && fractional.len() <= 2
                    && fractional.chars().all(|char| char.is_ascii_digit()) =>
            {
                let fractional_units = match fractional.len() {
                    0 => Some(0),
                    1 => fractional
                        .parse::<u64>()
                        .ok()
                        .and_then(|amount| amount.checked_mul(10)),
                    2 => fractional.parse::<u64>().ok(),
                    _ => None,
                };

                whole.parse::<u64>().ok().and_then(|whole_units| {
                    fractional_units.and_then(|fractional_units| {
                        whole_units
                            .checked_mul(100)
                            .and_then(|amount| amount.checked_add(fractional_units))
                    })
                })
            }
            _ => None,
        };

        let mut amounts = Vec::new();

        if amount_parts.len() == 1 {
            if let Ok(raw_amount) = query.parse::<u64>() {
                amounts.push(Value::from(raw_amount));
            }
        }

        if let Some(minor_amount) = major_units.and_then(|amount| {
            if amount_parts.len() == 1 {
                amount.checked_mul(100)
            } else {
                Some(amount)
            }
        }) {
            let minor_amount = Value::from(minor_amount);
            if !amounts.contains(&minor_amount) {
                amounts.push(minor_amount);
            }
        }

        amounts
    }

    fn make_query_filter(&self, index: SearchIndex) -> Option<Value> {
        if self.query.is_empty() {
            return None;
        }

        let text_query = json!({
            "multi_match": {
                "type": "phrase",
                "query": self.query,
                "lenient": true
            }
        });

        let amount_values = self.parse_search_amounts();
        if amount_values.is_empty() {
            return Some(text_query);
        }

        Some(json!({
            "bool": {
                "should": [
                    text_query,
                    {
                        "terms": {
                            self.get_amount_field(index): amount_values
                        }
                    }
                ],
                "minimum_should_match": 1
            }
        }))
    }

    fn make_active_attempt_script_filter(&self, field: &str, values: &Vec<Value>) -> Value {
        json!({
            "bool": {
                "should": [
                    {
                        "terms": {
                            format!("{}.keyword", field): values
                        }
                    },
                    {
                        "bool": {
                            "must": [
                                {
                                    "term": {
                                        "attempt_count": 1
                                    }
                                },
                                {
                                    "terms": {
                                        format!("attempts_list.{}.keyword", field): values
                                    }
                                }
                            ]
                        }
                    },
                    {
                        "bool": {
                            "must": [
                                {
                                    "range": {
                                        "attempt_count": {
                                            "gt": 1
                                        }
                                    }
                                },
                                {
                                    "bool": {
                                        "must_not": [
                                            {
                                                "exists": {
                                                    "field": field
                                                }
                                            }
                                        ]
                                    }
                                },
                                {
                                    "script": {
                                        "script": {
                                            "lang": "painless",
                                            "source": ACTIVE_ATTEMPT_FILTER_SCRIPT,
                                            "params": {
                                                "field": field,
                                                "values": values
                                            }
                                        }
                                    }
                                }
                            ]
                        }
                    }
                ],
                "minimum_should_match": 1
            }
        })
    }

    fn make_case_insensitive_multi_field_filter(&self, fields: &[&str], values: &[Value]) -> Value {
        json!({
            "bool": {
                "should": fields.iter().flat_map(|field| {
                    values.iter().map(|value| {
                        json!({
                            "term": {
                                *field: {
                                    "value": value,
                                    "case_insensitive": true
                                }
                            }
                        })
                    })
                }).collect::<Vec<Value>>(),
                "minimum_should_match": 1
            }
        })
    }

    pub fn build_filter_array(
        &self,
        case_sensitive_filters: Vec<&(String, Vec<Value>)>,
        index: SearchIndex,
    ) -> Vec<Value> {
        let mut filter_array = Vec::new();
        if let Some(query_filter) = self.make_query_filter(index) {
            filter_array.push(query_filter);
        }

        let case_sensitive_json_filters = case_sensitive_filters
            .into_iter()
            .map(|(k, v)| {
                if *k == "first_attempt" {
                    let mut should_clauses = Vec::new();

                    if v.iter().any(|value| value.as_bool() == Some(true)) {
                        should_clauses.push(json!({
                            "term": {
                                "attempt_count": 1
                            }
                        }));
                    }

                    if v.iter().any(|value| value.as_bool() == Some(false)) {
                        if let Some(operator) = OpenSearchComparison::GreaterThan.range_operator() {
                            should_clauses.push(json!({
                                "range": {
                                    "attempt_count": {
                                        (operator): 1
                                    }
                                }
                            }));
                        }
                    }

                    return json!({
                        "bool": {
                            "should": should_clauses,
                            "minimum_should_match": 1
                        }
                    });
                }

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

        if let Some(ref amount_range) = self.amount_range {
            let range = json!(amount_range);
            let amount_field = self.get_amount_field(index);
            filter_array.push(json!({
                "range": {
                    amount_field: range
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
                if *k == "card_discovery.keyword" {
                    return self.make_active_attempt_script_filter("card_discovery", v);
                }
                if *k == "refunds_status.keyword" {
                    return self.make_case_insensitive_multi_field_filter(
                        &[
                            "refunds_status.keyword",
                            "refunds_list.refund_status.keyword",
                            "refunds_list.status.keyword",
                        ],
                        v,
                    );
                }
                if *k == "dispute_status.keyword" {
                    return self.make_case_insensitive_multi_field_filter(
                        &[
                            "dispute_status.keyword",
                            "disputes_list.dispute_status.keyword",
                            "disputes_list.status.keyword",
                            "dispute_list.dispute_status.keyword",
                            "dispute_list.status.keyword",
                        ],
                        v,
                    );
                }
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
                                                key.to_string(): {
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
                    processor_merchant_ids,
                } => {
                    let mut must_clauses = vec![
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

                    if let Some(processor_mids) = processor_merchant_ids {
                        must_clauses.push(json!({
                            "terms": {
                                "processor_merchant_id.keyword": processor_mids
                            }
                        }));
                    }

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
                    processor_merchant_id,
                } => {
                    let mut must_clauses = vec![
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

                    if let Some(processor_mid) = processor_merchant_id {
                        must_clauses.push(json!({
                            "term": {
                                "processor_merchant_id.keyword": {
                                    "value": processor_mid
                                }
                            }
                        }));
                    }

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

        Ok(indexes
            .iter()
            .map(|index| {
                let mut sort_list = Vec::new();
                match &self.order {
                    Some(order) => {
                        let sort_on = match order.on {
                            SortOn::Amount => self.get_amount_field(*index).to_string(),
                            SortOn::AttemptCount => "attempt_count".to_string(),
                            SortOn::Created => "@timestamp".to_string(),
                            SortOn::Modified => "modified_at".to_string(),
                        };
                        let sort_by = match order.by {
                            SortBy::Asc => "asc",
                            SortBy::Desc => "desc",
                        };
                        sort_list.push(json!({
                            sort_on: {
                                "order": sort_by
                            }
                        }));
                    }
                    None => {
                        sort_list.push(json!({
                            "@timestamp": {
                                "order": "desc"
                            }
                        }));
                    }
                }
                let mut payload = json!({
                    "track_total_hits": true,
                    "query": query_obj.clone(),
                    "sort": sort_list.clone()
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
    /// Args for the OpenSearch seam: everything about the query that a replay
    /// must reproduce, and nothing that moves on its own. Built field by field
    /// rather than from `Debug` on the builder, whose `HashSet` renders in a
    /// per-process random order.
    #[cfg(feature = "deja")]
    pub fn deja_args(&self) -> Value {
        let mut auth_scope = self.build_auth_array();
        // Auth fragments are alternatives and the IDs inside `terms` are sets.
        // Canonicalize only the identity projection; leave the executable query alone.
        for fragment in &mut auth_scope {
            for field in [
                "merchant_id.keyword",
                "processor_merchant_id.keyword",
                "profile_id.keyword",
            ] {
                if let Some(values) = fragment
                    .pointer_mut("/bool/must")
                    .and_then(Value::as_array_mut)
                    .and_then(|clauses| {
                        clauses.iter_mut().find_map(|clause| {
                            clause
                                .get_mut("terms")
                                .and_then(|terms| terms.get_mut(field))
                                .and_then(Value::as_array_mut)
                        })
                    })
                {
                    values.sort_by_cached_key(Value::to_string);
                }
            }
        }
        auth_scope.sort_by_cached_key(Value::to_string);

        serde_json::json!({
            "query": self.query,
            "indexes": format!("{:?}", self.query_type),
            "offset": self.offset,
            "count": self.count,
            "filters": format!("{:?}", self.filters),
            "time_range": format!("{:?}", self.time_range),
            "amount_range": format!("{:?}", self.amount_range),
            "order": format!("{:?}", self.order),
            "auth_scope": auth_scope,
        })
    }
}

#[cfg(all(test, feature = "deja"))]
mod tests {
    use api_models::analytics::search::SearchIndex;

    use super::{OpenSearchQuery, OpenSearchQueryBuilder};
    use crate::enums::AuthInfo;

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
        let body = super::ResponseCodec::missed_query_body();

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
    /// declaration, so naming a different `on_miss` body or `args` expression on
    /// `execute` fails here rather than passing unnoticed.
    #[test]
    fn the_seam_names_the_helpers_the_tests_exercise() {
        let source = include_str!("opensearch.rs");
        // Built from parts so this test's own text is not a match.
        let start = source
            .find(&["deja::", "boundary("].concat())
            .expect("the seam attribute is present");
        let end = source[start..]
            .find(&["pub async fn ", "execute("].concat())
            .expect("the seam is declared on execute");
        let declaration = &source[start..start + end];
        for expected in [
            "owned_codec = ResponseCodec",
            "on_miss = Ok(ResponseCodec::missed_query())",
            "args = query_builder.deja_args()",
        ] {
            assert!(
                declaration.contains(expected),
                "the seam must name `{expected}`: {declaration}"
            );
        }
    }

    const GZIPPED: [u8; 52] = [
        31, 139, 8, 0, 0, 0, 0, 0, 2, 255, 171, 86, 202, 200, 44, 41, 86, 178, 170, 86, 42, 201,
        47, 73, 204, 1, 49, 202, 18, 115, 74, 83, 149, 172, 12, 107, 117, 160, 114, 209, 177, 181,
        181, 0, 3, 122, 183, 221, 40, 0, 0, 0,
    ];

    /// Serves `reply` to every connection, so one fixture can be fetched twice.
    fn serve(reply: Vec<u8>) -> String {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let url = format!(
            "http://{}/idx/_search",
            listener.local_addr().expect("addr")
        );
        std::thread::spawn(move || {
            for mut stream in listener.incoming().flatten() {
                let mut request = [0u8; 4096];
                let _ = stream.read(&mut request);
                let _ = stream.write_all(&reply);
            }
        });
        url
    }

    fn reply(head: &str, body: &[u8]) -> Vec<u8> {
        [head.as_bytes(), b"\r\n", body].concat()
    }

    async fn fetch(url: &str) -> opensearch::http::response::Response {
        let response = opensearch_reqwest::Client::new()
            .post(url)
            .send()
            .await
            .expect("the fixture server answers");
        opensearch::http::response::Response::new(response, opensearch::http::Method::Post)
    }

    /// What `execute`'s caller can observe of a response: its parts, its
    /// `content_length`, and its body, read last.
    async fn observed(
        response: opensearch::http::response::Response,
    ) -> (String, Option<u64>, Result<String, String>) {
        let parts = format!(
            "{:?} {:?} {:?} {} {:?} {:?}",
            response.status_code(),
            response.headers(),
            response.content_type(),
            response.url(),
            response.method(),
            response.warning_headers().collect::<Vec<_>>(),
        );
        let length = response.content_length();
        (
            parts,
            length,
            response.text().await.map_err(|error| error.to_string()),
        )
    }

    /// Recording must not change what `execute` returns: every accessor the
    /// client's `Response` offers reads the same through the codec as without
    /// it, for a sized body, a chunked one, a gzip one reqwest decodes, and one
    /// whose connection drops mid-body.
    ///
    /// One known difference, on the last alone: a body that fails can only be
    /// rebuilt as a stream (reqwest 0.12 has no public sized body that errors),
    /// so `content_length` reads `None` where the original read `Some`. Both
    /// callers go straight to `text()`, which fails either way.
    #[tokio::test]
    async fn the_caller_observes_the_same_response_through_the_codec() {
        use deja::codec::OwnedReplayCodec;
        let body = br#"{"hits":{"total":{"value":1},"hits":[]}}"#;
        let fixtures = [
            (
                "sized",
                reply(
                    &format!(
                        "HTTP/1.1 200 OK\r\ncontent-type: application/json; charset=UTF-8\r\n\
                         warning: 299 OpenSearch \"deprecated\"\r\ncontent-length: {}\r\n",
                        body.len()
                    ),
                    body,
                ),
                true,
            ),
            (
                "chunked",
                reply(
                    &format!(
                        "HTTP/1.1 404 Not Found\r\ncontent-type: application/json\r\n\
                         transfer-encoding: chunked\r\n\r\n{:x}",
                        body.len()
                    ),
                    &[&body[..], b"\r\n0\r\n\r\n"].concat(),
                ),
                true,
            ),
            (
                "gzip",
                reply(
                    &format!(
                        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\n\
                         content-encoding: gzip\r\ncontent-length: {}\r\n",
                        GZIPPED.len()
                    ),
                    &GZIPPED,
                ),
                true,
            ),
            (
                "truncated",
                reply(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 400\r\n",
                    body,
                ),
                false,
            ),
        ];
        for (name, fixture, reads) in fixtures {
            let url = serve(fixture);
            let (direct_parts, direct_length, direct_body) = observed(fetch(&url).await).await;
            let (rebuilt, read) = super::ResponseCodec::read(Ok(fetch(&url).await)).await;
            assert_eq!(matches!(read, Ok(Ok(_))), reads, "{name}: {read:?}");
            let (rebuilt_parts, rebuilt_length, rebuilt_body) =
                observed(rebuilt.expect("an Ok stays Ok")).await;
            assert_eq!(direct_parts, rebuilt_parts, "{name}: the parts differ");
            assert_eq!(direct_body.is_ok(), reads, "{name}: {direct_body:?}");
            if reads {
                assert_eq!(direct_length, rebuilt_length, "{name}: the length differs");
                assert_eq!(direct_body, rebuilt_body, "{name}: the body differs");
            } else {
                assert_eq!(
                    (direct_length, rebuilt_length),
                    (Some(400), None),
                    "{name}: the one known difference, a failed body's length"
                );
                assert!(rebuilt_body.is_err(), "{name}: the read failure survives");
            }
        }
    }

    /// What `read` saw goes on the tape and rebuilds on replay, error arm too.
    #[tokio::test]
    async fn a_recorded_response_replays_as_what_the_caller_read() {
        use deja::codec::OwnedReplayCodec;
        use error_stack::report;
        let url = serve(reply(
            "HTTP/1.1 404 Not Found\r\ncontent-length: 15\r\n",
            br#"{"found":false}"#,
        ));
        let (value, read) = super::ResponseCodec::read(Ok(fetch(&url).await)).await;
        let (recorded, is_error) =
            super::ResponseCodec::record(read.expect("the body reads"), &value);
        assert!(!is_error);
        let replayed = super::ResponseCodec::reconstruct(recorded)
            .expect("the envelope rebuilds")
            .expect("an Ok envelope");
        assert_eq!(replayed.status_code().as_u16(), 404);
        assert_eq!(
            replayed.text().await.expect("the replayed body reads"),
            r#"{"found":false}"#
        );

        // A body that could not be read replays as one that fails, at the
        // recorded status, as the caller saw it.
        let url = serve(reply(
            "HTTP/1.1 503 Service Unavailable\r\ncontent-length: 400\r\n",
            br#"{"partial":"#,
        ));
        let (value, read) = super::ResponseCodec::read(Ok(fetch(&url).await)).await;
        let (recorded, is_error) =
            super::ResponseCodec::record(read.expect("the read always reports"), &value);
        assert!(!is_error, "the call returned its response");
        assert!(
            recorded.pointer("/value/read_error").is_some(),
            "the tape names the failed read: {recorded}"
        );
        let replayed = super::ResponseCodec::reconstruct(recorded)
            .expect("the envelope rebuilds")
            .expect("an Ok envelope");
        assert_eq!(replayed.status_code().as_u16(), 503);
        assert!(replayed.text().await.is_err(), "the replayed body fails");

        let failed: common_utils::errors::CustomResult<
            opensearch::http::response::Response,
            super::OpenSearchError,
        > = Err(report!(super::OpenSearchError::ConnectionError));
        let (value, read) = super::ResponseCodec::read(failed).await;
        let (recorded, is_error) =
            super::ResponseCodec::record(read.expect("an error arm reads nothing"), &value);
        assert!(is_error);
        let replayed = super::ResponseCodec::reconstruct(recorded).expect("the envelope rebuilds");
        assert!(matches!(
            replayed.map(|_| ()).map_err(|report| report.current_context().to_string()),
            Err(message) if message == super::OpenSearchError::ConnectionError.to_string()
        ));
    }
}
