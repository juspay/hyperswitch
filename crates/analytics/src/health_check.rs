use common_utils::errors::CustomResult;

use crate::types::QueryExecutionError;

#[async_trait::async_trait]
pub trait HealthCheck {
    async fn deep_health_check(&self) -> CustomResult<(), QueryExecutionError>;
}
