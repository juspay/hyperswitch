use error_stack::ResultExt;

use crate::{
    core::errors,
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
    let merchant_webhook_config_result =
        get_and_deserialize_key(db, &redis_key, "MerchantWebhookConfig")
            .await
            .map(|h| &h | &default_webhook_config());

    match merchant_webhook_config_result {
        Ok(merchant_webhook_config) => merchant_webhook_config.contains(event),
        Err(..) => {
            //if failed to fetch from redis. fetch from db and populate redis
            db.find_config_by_key_cached(&redis_key)
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

pub trait WebhookApiErrorSwitch<T> {
    fn switch(self) -> errors::RouterResult<T>;
}

impl<T> WebhookApiErrorSwitch<T> for errors::CustomResult<T, errors::ConnectorError> {
    fn switch(self) -> errors::RouterResult<T> {
        match self {
            Ok(res) => Ok(res),
            Err(e) => match e.current_context() {
                errors::ConnectorError::WebhookSourceVerificationFailed => {
                    Err(e).change_context(errors::ApiErrorResponse::WebhookAuthenticationFailed)
                }

                errors::ConnectorError::WebhookSignatureNotFound
                | errors::ConnectorError::WebhookReferenceIdNotFound
                | errors::ConnectorError::WebhookResourceObjectNotFound
                | errors::ConnectorError::WebhookBodyDecodingFailed
                | errors::ConnectorError::WebhooksNotImplemented => {
                    Err(e).change_context(errors::ApiErrorResponse::WebhookBadRequest)
                }

                errors::ConnectorError::WebhookEventTypeNotFound => {
                    Err(e).change_context(errors::ApiErrorResponse::WebhookUnprocessableEntity)
                }

                _ => Err(e).change_context(errors::ApiErrorResponse::InternalServerError),
            },
        }
    }
}
