#[derive(Clone, Debug, serde::Deserialize)]
pub struct OpensearchConfig {
    host: String,
    auth: OpensearchAuth,
    indexes: OpensearchIndexes,
}

#[derive(Debug, thiserror::Error)]
pub enum OpensearchError {
    #[error("Opensearch connection error")]
    ConnectionError,
    #[error("Opensearch NON-200 response content: '{0}'")]
    ResponseNotOK(String),
    #[error("Opensearch response error")]
    ResponseError,
}

impl Default for OpensearchConfig {
    fn default() -> Self {
        Self {
            host: "https://localhost:9200".to_string(),
            auth: OpensearchAuth::Basic {
                username: "admin".to_string(),
                password: "admin".to_string(),
            },
            indexes: OpensearchIndexes {
                payment_attempts: "hyperswitch-payment-attempt-events".to_string(),
                payment_intents: "hyperswitch-payment-intent-events".to_string(),
                refunds: "hyperswitch-refund-events".to_string(),
            },
        }
    }
}

pub struct OpenSearchClient {
    client: OpenSearch,
    indexes: OpensearchIndexes,
}

#[allow(unused)]
impl OpenSearchClient {
    pub async fn create(conf: &OpensearchConfig) -> CustomResult<Self, OpensearchError> {
        Ok(Self {
            client: {
                let url = Url::parse(&conf.host).map_err(|_| OpensearchError::ConnectionError)?;
                let transport = match conf.auth {
                    OpensearchAuth::Basic { username, password } => {
                        let credentials = Credentials::Basic(username, password);
                        TransportBuilder::new(SingleNodeConnectionPool::new(url))
                            .cert_validation(CertificateValidation::None)
                            .auth(credentials)
                            .build()
                            .map_err(|_| OpensearchError::ConnectionError)?
                    }
                    OpensearchAuth::Aws { region } => {
                        let region_provider = RegionProviderChain::first_try(Region::new(region));
                        let sdk_config =
                            aws_config::from_env().region(region_provider).load().await;
                        let conn_pool = SingleNodeConnectionPool::new(url);
                        TransportBuilder::new(conn_pool)
                            .auth(
                                sdk_config
                                    .clone()
                                    .try_into()
                                    .map_err(|_| OpensearchError::ConnectionError)?,
                            )
                            .service_name("es")
                            .build()
                            .map_err(|_| OpensearchError::ConnectionError)?
                    }
                };
                OpenSearch::new(transport)
            },
            indexes: conf.indexes.clone(),
        })
    }
}

impl OpensearchIndexes {
    pub fn validate(&self) -> Result<(), ApplicationError> {
        use common_utils::ext_traits::ConfigExt;

        use crate::core::errors::ApplicationError;

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

impl OpensearchAuth {
    pub fn validate(&self) -> Result<(), ApplicationError> {
        use common_utils::ext_traits::ConfigExt;

        use crate::core::errors::ApplicationError;

        match self {
            Self::Basic { username, password } => {
                common_utils::fp_utils::when(self.username.is_default_or_empty(), || {
                    Err(ApplicationError::InvalidConfigurationValueError(
                        "Opensearch Basic auth username must not be empty".into(),
                    ))
                })?;

                common_utils::fp_utils::when(self.password.is_default_or_empty(), || {
                    Err(ApplicationError::InvalidConfigurationValueError(
                        "Opensearch Basic auth password must not be empty".into(),
                    ))
                })?;
            }

            Self::Aws { region } => {
                common_utils::fp_utils::when(self.password.is_default_or_empty(), || {
                    Err(ApplicationError::InvalidConfigurationValueError(
                        "Opensearch Aws auth region must not be empty".into(),
                    ))
                })?;
            }
        };

        Ok(())
    }
}

impl OpensearchConfig {
    pub async fn get_opensearch_client(&self) -> StorageResult<OpenSearchClient> {
        Ok(OpenSearchClient::create(self)
            .await
            .change_context(StorageError::InitializationError)?)
    }

    pub fn validate(&self) -> Result<(), ApplicationError> {
        use common_utils::ext_traits::ConfigExt;

        use crate::core::errors::ApplicationError;

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
