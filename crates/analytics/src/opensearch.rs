use super::{health_check::HealthCheck, query::QueryResult, types::QueryExecutionError};
use api_models::analytics::search::SearchIndex;
use aws_config::{self, meta::region::RegionProviderChain, Region};
use common_utils::errors::CustomResult;
use data_models::errors::{StorageError, StorageResult};
use error_stack::{IntoReport, ResultExt};
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
use serde_json::{json, Value};
use storage_impl::errors::ApplicationError;
use strum::IntoEnumIterator;

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
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct OpenSearchConfig {
    host: String,
    auth: OpenSearchAuth,
    indexes: OpenSearchIndexes,
}

#[derive(Debug, thiserror::Error)]
pub enum OpenSearchError {
    #[error("Opensearch connection error")]
    ConnectionError,
    #[error("Opensearch NON-200 response content: '{0}'")]
    ResponseNotOK(String),
    #[error("Opensearch response error")]
    ResponseError,
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
            },
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
        }
    }

    pub async fn execute(
        &self,
        query_builder: OpenSearchQueryBuilder,
    ) -> CustomResult<Response, OpenSearchError> {
        match query_builder.query_type {
            OpenSearchQuery::Msearch => {
                let payload = query_builder
                    .construct_payload(SearchIndex::iter().collect())
                    .change_context(OpenSearchError::ResponseError)?;

                let mut payload_with_indexes: Vec<JsonBody<Value>> = vec![];
                for (index_hit, index) in payload.iter().to_owned().zip(SearchIndex::iter()) {
                    payload_with_indexes.push(
                        json!({"index": self.search_index_to_opensearch_index(index)}).into(),
                    );
                    payload_with_indexes.push(JsonBody::new(index_hit.clone()));
                }

                self.client
                    .msearch(MsearchParts::None)
                    .body(payload_with_indexes)
                    .send()
                    .await
                    .into_report()
                    .change_context(OpenSearchError::ResponseError)
            }
            OpenSearchQuery::Search(index) => {
                let payload = query_builder
                    .clone()
                    .construct_payload(vec![index.clone()])
                    .change_context(OpenSearchError::ResponseError)?;

                let final_payload = payload.get(0).unwrap_or(&Value::Null);

                self.client
                    .search(SearchParts::Index(&[
                        &self.search_index_to_opensearch_index(index)
                    ]))
                    .from(query_builder.offset.unwrap_or(0))
                    .size(query_builder.count.unwrap_or(10))
                    .body(final_payload)
                    .send()
                    .await
                    .into_report()
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
            .into_report()
            .change_context(QueryExecutionError::DatabaseError)?
            .json::<OpenSearchHealth>()
            .await
            .into_report()
            .change_context(QueryExecutionError::DatabaseError)?;

        if health.status != OpenSearchHealthStatus::Red {
            Ok(())
        } else {
            Err(QueryExecutionError::DatabaseError).into_report()
        }
    }
}

impl OpenSearchIndexes {
    pub fn validate(&self) -> Result<(), ApplicationError> {
        use common_utils::ext_traits::ConfigExt;

        common_utils::fp_utils::when(self.payment_attempts.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "Opensearch Payment Attempts index must not be empty".into(),
            ))
        })?;

        common_utils::fp_utils::when(self.payment_intents.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "Opensearch Payment Intents index must not be empty".into(),
            ))
        })?;

        common_utils::fp_utils::when(self.refunds.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "Opensearch Refunds index must not be empty".into(),
            ))
        })?;

        Ok(())
    }
}

impl OpenSearchAuth {
    pub fn validate(&self) -> Result<(), ApplicationError> {
        use common_utils::ext_traits::ConfigExt;

        match self {
            Self::Basic { username, password } => {
                common_utils::fp_utils::when(username.is_default_or_empty(), || {
                    Err(ApplicationError::InvalidConfigurationValueError(
                        "Opensearch Basic auth username must not be empty".into(),
                    ))
                })?;

                common_utils::fp_utils::when(password.is_default_or_empty(), || {
                    Err(ApplicationError::InvalidConfigurationValueError(
                        "Opensearch Basic auth password must not be empty".into(),
                    ))
                })?;
            }

            Self::Aws { region } => {
                common_utils::fp_utils::when(region.is_default_or_empty(), || {
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
        use common_utils::ext_traits::ConfigExt;

        common_utils::fp_utils::when(self.host.is_default_or_empty(), || {
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
    Msearch,
    Search(SearchIndex),
}

#[derive(Debug, Clone)]
pub struct OpenSearchQueryBuilder {
    pub query_type: OpenSearchQuery,
    pub query: String,
    pub offset: Option<i64>,
    pub count: Option<i64>,
    pub filters: Vec<(String, String)>,
}

impl OpenSearchQueryBuilder {
    pub fn new(query_type: OpenSearchQuery, query: String) -> Self {
        Self {
            query_type: query_type,
            query: query,
            offset: Default::default(),
            count: Default::default(),
            filters: Default::default(),
        }
    }

    pub fn set_offset_n_count(&mut self, offset: i64, count: i64) -> QueryResult<()> {
        self.offset = Some(offset);
        self.count = Some(count);
        Ok(())
    }

    pub fn add_filter_clause(&mut self, lhs: String, rhs: String) -> QueryResult<()> {
        self.filters.push((lhs, rhs));
        Ok(())
    }

    pub fn construct_payload(&self, indexes: Vec<SearchIndex>) -> QueryResult<Vec<Value>> {
        let filters = self
            .filters
            .iter()
            .map(|(k, v)| json!({"match_phrase" : {k : v}}))
            .collect::<Vec<Value>>();

        // TODO add index specific filters
        Ok(indexes.iter().map(|_| {
            json!({"query": {"bool": {"must": {"query_string": {"query": self.query}}, "filter": filters}}})
        }).collect::<Vec<Value>>())
    }
}
