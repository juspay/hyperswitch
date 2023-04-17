use crate::{
    db::{get_and_deserialize_key, StorageInterface},
    types::api,
};

fn default_webhook_config() -> api::MerchantWebhookConfig {
    std::collections::HashSet::from([
        api::IncomingWebhookEvent::PaymentIntentSuccess,
        api::IncomingWebhookEvent::PaymentIntentFailure,
        api::IncomingWebhookEvent::PaymentIntentProcessing,
        api::IncomingWebhookEvent::PaymentActionRequired,
        api::IncomingWebhookEvent::RefundSuccess,
    ])
}

pub async fn lookup_webhook_event(
    db: &dyn StorageInterface,
    connector_id: &str,
    merchant_id: &str,
    event: &api::IncomingWebhookEvent,
) -> bool {
    let redis_key = format!("whconf_{merchant_id}_{connector_id}");
    let webhook_config: api::MerchantWebhookConfig =
        get_and_deserialize_key(db, &redis_key, "MerchantWebhookConfig")
            .await
            .map(|h| &h | &default_webhook_config())
            .unwrap_or_else(|_| default_webhook_config());

    webhook_config.contains(event)
}
