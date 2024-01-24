use crate::types::QueryExecutionError;
use common_utils::errors::CustomResult;

#[async_trait::async_trait]
pub trait HealthCheck {
    async fn deep_health_check(&self) -> CustomResult<(), QueryExecutionError>;
}
