use api_models::revenue_recovery_reports::{UploadStatus, UploadStatusData};
use error_stack::ResultExt;
use redis_interface::RedisKey;
use router_env::{instrument, logger, tracing};
use serde_json::json;

use crate::{
    core::errors::{self, RouterResult},
    routes::SessionState,
};

pub struct RevenueRecoveryUploadStatusManager;

impl RevenueRecoveryUploadStatusManager {
    fn get_upload_status_key(file_id: &str) -> RedisKey {
        RedisKey::from(format!("revenue_recovery_upload:{}", file_id))
    }

    #[instrument(skip_all)]
    pub async fn set_upload_status(
        state: &SessionState,
        file_id: &str,
        status_data: UploadStatusData,
        ttl_seconds: i64,
    ) -> RouterResult<()> {
        let redis_conn = state
            .store
            .get_redis_conn()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to get redis connection")?;

        let key = Self::get_upload_status_key(file_id);
        let serialized_data = serde_json::to_string(&status_data)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to serialize upload status data")?;

        redis_conn
            .set_key_with_expiry(&key, serialized_data, ttl_seconds)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to set upload status in Redis")?;

        logger::debug!("Upload status set in Redis for file_id: {}", file_id);
        Ok(())
    }

    #[instrument(skip_all)]
    pub async fn get_upload_status(
        state: &SessionState,
        file_id: &str,
    ) -> RouterResult<Option<UploadStatusData>> {
        let redis_conn = state
            .store
            .get_redis_conn()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to get redis connection")?;

        let key = Self::get_upload_status_key(file_id);
        let serialized_data: Option<String> = redis_conn
            .get_key(&key)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to get upload status from Redis")?;

        match serialized_data {
            Some(data) => {
                let status_data: UploadStatusData = serde_json::from_str(&data)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to deserialize upload status data from Redis")?;
                Ok(Some(status_data))
            }
            None => Ok(None),
        }
    }
}
