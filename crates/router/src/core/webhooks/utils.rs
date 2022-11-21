use crate::{connection::RedisPool, types::api};

fn default_webhook_config() -> api::MerchantWebhookConfig {
    std::collections::HashSet::from([api::IncomingWebhookEvent::PaymentIntentSuccess])
}

pub async fn lookup_webhook_event(
    connector_id: &str,
    merchant_id: &str,
    event: &api::IncomingWebhookEvent,
    conn: RedisPool,
) -> bool {
    let redis_key = format!("whconf_{}_{}", merchant_id, connector_id);
    let webhook_config: api::MerchantWebhookConfig = conn
        .get_and_deserialize_key(&redis_key, "MerchantWebhookConfig")
        .await
        .map(|h| &h | &default_webhook_config())
        .unwrap_or_else(|_| default_webhook_config());

    webhook_config.contains(event)
}
