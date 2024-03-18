use super::{health_check::HealthCheck, types::QueryExecutionError};
use api_models::analytics::search::SearchIndex;
use aws_config::{self, meta::region::RegionProviderChain, Region};
use common_utils::errors::CustomResult;
use data_models::errors::{StorageError, StorageResult};
use opensearch::{
    auth::Credentials,
    cert::CertificateValidation,
    cluster::{Cluster, ClusterHealthParts},
    http::{
        request::JsonBody,
        transport::{SingleNodeConnectionPool, Transport, TransportBuilder},
        Url,
    },
    MsearchParts, OpenSearch, SearchParts,
};
use storage_impl::errors::ApplicationError;

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

#[allow(unused)]
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

    // pub async fn execute(&self) -> CustomResult<Self, OpenSearchError> {}
}
use error_stack::IntoReport;
use error_stack::ResultExt;

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

pub enum OpenSearchQuery {
    Msearch,
    Search(SearchIndex),
    HealthCheck,
}

#[derive(Debug)]
pub struct OpenSearchQueryBuilder {
    pub query: Option<String>,
    pub offset: i64,
    pub count: i64,
    query_type: OpenSearchQuery,
    distinct: bool,
    merchant_id: TableEngine,
}
