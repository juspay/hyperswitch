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
    let key = format!("whconf_{merchant_id}_{connector_id}");
    let webhook_config = 
        db.find_config_by_key(&key)
            .await
            .ok()
            .map(|config|{
                if let Ok(h) = serde_json::from_str::<api::MerchantWebhookConfig>(&config.config) {
                    &h | &default_webhook_config()
                }else{
                    default_webhook_config()
                }
            });

    webhook_config.unwrap_or_else(|| default_webhook_config()).contains(event)
}
