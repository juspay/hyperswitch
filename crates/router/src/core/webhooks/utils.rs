use error_stack::ResultExt;

use crate::{
    connection::RedisPool,
    core::errors::{self, CustomResult},
    types::api,
};

pub async fn lookup_webhook_event(
    connector_id: &str,
    merchant_id: &str,
    event: &str,
    conn: RedisPool,
) -> CustomResult<Option<api::WebhookFlow>, errors::WebhooksFlowError> {
    let redis_key = format!("whconf_{}_{}", merchant_id, connector_id);
    let mut webhook_config: api::MerchantWebhookConfig = conn
        .get_and_deserialize_key(&redis_key, "MerchantWebhookConfig")
        .await
        .change_context(errors::WebhooksFlowError::MerchantConfigNotFound)?;

    Ok(webhook_config.remove(event))
}
