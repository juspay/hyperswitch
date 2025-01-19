
use api_models::webhooks::{self, WebhookResponseTracker};
use error_stack::ResultExt;
use error_stack::report;
use hyperswitch_interfaces::webhooks::IncomingWebhookRequestDetails;
use router_env::{instrument, tracing};

use crate::{
    core::errors::{self, CustomResult},
    routes::SessionState,
    types::{
        api::{self, IncomingWebhook},
        domain,
    },
    services::connector_integration_interface::ConnectorEnum,
};


#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
#[cfg(feature= "recovery")]
pub async fn recovery_incoming_webhook_flow(
    state: SessionState,
    _merchant_account: domain::MerchantAccount,
    _business_profile: domain::Profile,
    _key_store: domain::MerchantKeyStore,
    _webhook_details: api::IncomingWebhookDetails,
    source_verified: bool,
    connector: &ConnectorEnum,
    request_details: &IncomingWebhookRequestDetails<'_>,
    event_type: webhooks::IncomingWebhookEvent,
) -> CustomResult<WebhookResponseTracker, errors::ApiErrorResponse> {
    use error_stack::report;
    use hyperswitch_interfaces::recovery::{RecoveryAction, RecoveryActionTrait, RecoveryTrait};

    match source_verified {
        true => {
            let _db = &*state.store;
            let invoice_details = connector.get_recovery_details(request_details).change_context(errors::ApiErrorResponse::InternalServerError)?;
            // this should be fetched using merchant reference id api
            let _payment_intent = invoice_details.get_intent()?;
            let payment_attempt = invoice_details.get_attempt()?;

            // find optional running job associated with payment intent
            // let running_job = invoice_details.

            let passive_churn_recovery_data = payment_attempt.feature_metadata.and_then(|metadata|metadata.passive_churn_recovery);
            let triggered_by = passive_churn_recovery_data.map(|data|data.triggered_by);

            let action = RecoveryAction::find_action(event_type,triggered_by);

            match action{
                RecoveryAction::CancelInvoice => todo!(),
                RecoveryAction::FailPaymentExternal => todo!(),
                RecoveryAction::SuccessPaymentExternal => todo!(),
                RecoveryAction::PendingPayment => todo!(),
                RecoveryAction::NoAction => todo!(),
                RecoveryAction::InvalidAction => todo!(),
            }
        },
        false => Err(report!(
            errors::ApiErrorResponse::WebhookAuthenticationFailed
        ))
    }
}