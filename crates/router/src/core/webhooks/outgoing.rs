use api_models::webhooks;
use diesel_models::{dispute, enums, payment_intent::PaymentIntent, refund};
use error_stack::{IntoReport, ResultExt};
use futures::TryFutureExt;

use super::create_event_and_trigger_outgoing_webhook;
use crate::{
    core::{
        errors, payments,
        webhooks::types::{OutgoingWebhookTrigger, OutgoingWebhookType},
    },
    routes::AppState,
    services,
    types::{
        api, domain,
        transformers::{ForeignInto, ForeignTryInto},
    },
};

#[async_trait::async_trait]
impl OutgoingWebhookTrigger for PaymentIntent {
    async fn construct_outgoing_webhook_content(
        &self,
        state: &AppState,
        merchant_account: domain::MerchantAccount,
        merchant_key_store: domain::MerchantKeyStore,
    ) -> errors::CustomResult<webhooks::OutgoingWebhookContent, errors::ApiErrorResponse> {
        payments::payments_core::<api::PSync, api::PaymentsResponse, _, _, _>(
            state,
            merchant_account,
            merchant_key_store,
            payments::operations::PaymentStatus,
            api::PaymentsRetrieveRequest {
                resource_id: api_models::payments::PaymentIdType::PaymentIntentId(
                    self.payment_id.clone(),
                ),
                merchant_id: Some(self.merchant_id.clone()),
                force_sync: false,
                ..Default::default()
            },
            services::AuthFlow::Merchant,
            payments::CallConnectorAction::Avoid,
        )
        .await
        .and_then(|application_response| match application_response {
            services::ApplicationResponse::Json(payments_response) => Ok(
                webhooks::OutgoingWebhookContent::PaymentDetails(payments_response),
            ),
            // This state isn't possible
            _ => Err(errors::ApiErrorResponse::GenericNotFoundError {
                message: "Failed while getting payment response".to_string(),
            })
            .into_report(),
        })
    }

    async fn trigger_outgoing_webhook<W: OutgoingWebhookType>(
        &self,
        state: &AppState,
    ) -> errors::CustomResult<(), errors::ApiErrorResponse> {
        let (merchant_account, merchant_key_store) = state
            .store
            .get_merchant_key_store_by_merchant_id(
                &self.merchant_id,
                &state.store.get_master_key().to_vec().into(),
            )
            .and_then(|key_store| async {
                Ok((
                    state
                        .store
                        .find_merchant_account_by_merchant_id(&self.merchant_id, &key_store)
                        .await?,
                    key_store,
                ))
            })
            .await
            .change_context(errors::ApiErrorResponse::MerchantAccountNotFound)?;

        let webhook_content = self
            .construct_outgoing_webhook_content(state, merchant_account.clone(), merchant_key_store)
            .await?;

        create_event_and_trigger_outgoing_webhook::<W>(
            state.clone(),
            merchant_account,
            self.status
                .foreign_try_into()
                .into_report()
                .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)?,
            enums::EventClass::Payments,
            None,
            self.payment_id.clone(),
            enums::EventObjectType::PaymentDetails,
            webhook_content,
        )
        .await
    }
}

#[async_trait::async_trait]
impl OutgoingWebhookTrigger for refund::Refund {
    async fn construct_outgoing_webhook_content(
        &self,
        _state: &AppState,
        _merchant_account: domain::MerchantAccount,
        _merchant_key_store: domain::MerchantKeyStore,
    ) -> errors::CustomResult<webhooks::OutgoingWebhookContent, errors::ApiErrorResponse> {
        Ok(webhooks::OutgoingWebhookContent::RefundDetails(
            self.clone().foreign_into(),
        ))
    }

    async fn trigger_outgoing_webhook<W: OutgoingWebhookType>(
        &self,
        state: &AppState,
    ) -> errors::CustomResult<(), errors::ApiErrorResponse> {
        let (merchant_account, merchant_key_store) = state
            .store
            .get_merchant_key_store_by_merchant_id(
                &self.merchant_id,
                &state.store.get_master_key().to_vec().into(),
            )
            .and_then(|key_store| async {
                Ok((
                    state
                        .store
                        .find_merchant_account_by_merchant_id(&self.merchant_id, &key_store)
                        .await?,
                    key_store,
                ))
            })
            .await
            .change_context(errors::ApiErrorResponse::MerchantAccountNotFound)?;

        let webhook_content = self
            .construct_outgoing_webhook_content(state, merchant_account.clone(), merchant_key_store)
            .await?;

        create_event_and_trigger_outgoing_webhook::<W>(
            state.clone(),
            merchant_account,
            self.refund_status
                .foreign_try_into()
                .into_report()
                .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)?,
            enums::EventClass::Refunds,
            None,
            self.refund_id.clone(),
            enums::EventObjectType::RefundDetails,
            webhook_content,
        )
        .await
    }
}

#[async_trait::async_trait]
impl OutgoingWebhookTrigger for dispute::Dispute {
    async fn construct_outgoing_webhook_content(
        &self,
        _state: &AppState,
        _merchant_account: domain::MerchantAccount,
        _merchant_key_store: domain::MerchantKeyStore,
    ) -> errors::CustomResult<webhooks::OutgoingWebhookContent, errors::ApiErrorResponse> {
        Ok(webhooks::OutgoingWebhookContent::DisputeDetails(Box::new(
            self.clone().foreign_into(),
        )))
    }

    async fn trigger_outgoing_webhook<W: OutgoingWebhookType>(
        &self,
        state: &AppState,
    ) -> errors::CustomResult<(), errors::ApiErrorResponse> {
        let (merchant_account, merchant_key_store) = state
            .store
            .get_merchant_key_store_by_merchant_id(
                &self.merchant_id,
                &state.store.get_master_key().to_vec().into(),
            )
            .and_then(|key_store| async {
                Ok((
                    state
                        .store
                        .find_merchant_account_by_merchant_id(&self.merchant_id, &key_store)
                        .await?,
                    key_store,
                ))
            })
            .await
            .change_context(errors::ApiErrorResponse::MerchantAccountNotFound)?;

        let webhook_content = self
            .construct_outgoing_webhook_content(state, merchant_account.clone(), merchant_key_store)
            .await?;

        create_event_and_trigger_outgoing_webhook::<W>(
            state.clone(),
            merchant_account,
            self.dispute_status
                .foreign_try_into()
                .into_report()
                .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)?,
            enums::EventClass::Disputes,
            None,
            self.dispute_id.clone(),
            enums::EventObjectType::DisputeDetails,
            webhook_content,
        )
        .await
    }
}
