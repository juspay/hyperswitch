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

/// Check whether the merchant has configured to process the webhook `event`
/// First check for the key "whconf_{merchant_id}_{connector_id}" in redis,
/// if not found, fetch from configs table in database, if not found use default
pub async fn lookup_webhook_event(
    db: &dyn StorageInterface,
    connector_id: &str,
    merchant_id: &str,
    event: &api::IncomingWebhookEvent,
) -> bool {
    let redis_key = format!("whconf_{merchant_id}_{connector_id}");
    let merchant_webhook_config_result =
        get_and_deserialize_key(db, &redis_key, "MerchantWebhookConfig")
            .await
            .map(|h| &h | &default_webhook_config());

    match merchant_webhook_config_result {
        Ok(merchant_webhook_config) => merchant_webhook_config.contains(event),
        Err(..) => {
            //if failed to fetch from redis. fetch from db and populate redis
            db.find_config_by_key(&redis_key)
                .await
                .map(|config| {
                    if let Ok(set) =
                        serde_json::from_str::<api::MerchantWebhookConfig>(&config.config)
                    {
                        &set | &default_webhook_config()
                    } else {
                        default_webhook_config()
                    }
                })
                .unwrap_or_else(|_| default_webhook_config())
                .contains(event)
        }
    }
}
