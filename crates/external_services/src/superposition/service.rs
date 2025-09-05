use std::sync::Arc;

use common_utils::errors::CustomResult;
use error_stack::ResultExt;

use super::{
    interface::{ConfigContext, SuperpositionError, SuperpositionInterface},
    superposition::{SuperpositionClient, SuperpositionClientConfig},
};

/// Configuration for the superposition service
pub type SuperpositionConfig = SuperpositionClientConfig;

/// Main superposition service implementation  
#[derive(Debug)]
pub struct SuperpositionService {
    superposition_client: Option<Arc<SuperpositionClient>>,
}

impl SuperpositionService {
    /// Create a new configuration service
    pub async fn new(config: SuperpositionConfig) -> CustomResult<Self, SuperpositionError> {
        let superposition_client = if config.enabled {
            Some(Arc::new(
                SuperpositionClient::new(config).await.change_context(
                    SuperpositionError::ClientError(
                        "Failed to initialize Superposition client".to_string(),
                    ),
                )?,
            ))
        } else {
            None
        };

        Ok(Self {
            superposition_client,
        })
    }
}

#[async_trait::async_trait]
impl SuperpositionInterface for SuperpositionService {
    async fn get_config_string(
        &self,
        key: &str,
        context: Option<ConfigContext>,
        default_value: String,
    ) -> CustomResult<String, SuperpositionError> {
        if let Some(superposition_client) = &self.superposition_client {
            match superposition_client
                .get_string_value(key, context.as_ref())
                .await
            {
                Ok(value) => return Ok(value),
                Err(_) => {} // Continue to default
            }
        }
        Ok(default_value)
    }

    async fn get_config_bool(
        &self,
        key: &str,
        context: Option<ConfigContext>,
        default_value: bool,
    ) -> CustomResult<bool, SuperpositionError> {
        if let Some(superposition_client) = &self.superposition_client {
            match superposition_client
                .get_bool_value(key, context.as_ref())
                .await
            {
                Ok(value) => return Ok(value),
                Err(_) => {} // Continue to default
            }
        }
        Ok(default_value)
    }

    async fn get_config_int(
        &self,
        key: &str,
        context: Option<ConfigContext>,
        default_value: i64,
    ) -> CustomResult<i64, SuperpositionError> {
        if let Some(superposition_client) = &self.superposition_client {
            match superposition_client
                .get_int_value(key, context.as_ref())
                .await
            {
                Ok(value) => return Ok(value),
                Err(_) => {} // Continue to default
            }
        }
        Ok(default_value)
    }
}
