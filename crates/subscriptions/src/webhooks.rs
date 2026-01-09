use std::str::FromStr;

use api_models::webhooks::WebhookResponseTracker;
use common_enums::{connector_enums::Connector, InvoiceStatus};
use common_utils::{consts, errors::CustomResult, generate_id};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    business_profile, errors::api_error_response as errors, invoice, merchant_connector_account,
    platform,
};
use hyperswitch_interfaces::{
    api::ConnectorCommon, connector_integration_interface, errors::ConnectorError,
    webhooks::IncomingWebhook,
};
use router_env::{instrument, logger, tracing};

use crate::state::SubscriptionState as SessionState;
#[cfg(feature = "v1")]
use crate::subscription_handler::SubscriptionHandler;

#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub async fn incoming_webhook_flow(
    state: SessionState,
    platform: platform::Platform,
    business_profile: business_profile::Profile,
    _webhook_details: api_models::webhooks::IncomingWebhookDetails,
    source_verified: bool,
    connector_enum: &connector_integration_interface::ConnectorEnum,
    request_details: &hyperswitch_interfaces::webhooks::IncomingWebhookRequestDetails<'_>,
    event_type: api_models::webhooks::IncomingWebhookEvent,
    merchant_connector_account: merchant_connector_account::MerchantConnectorAccount,
) -> CustomResult<WebhookResponseTracker, errors::ApiErrorResponse> {
    let billing_connector_mca_id = merchant_connector_account.merchant_connector_id.clone();
    // Only process invoice_generated events for MIT payments
    if event_type != api_models::webhooks::IncomingWebhookEvent::InvoiceGenerated {
        return Ok(WebhookResponseTracker::NoEffect);
    }

    if !source_verified {
        logger::error!("Webhook source verification failed for subscription webhook flow");
        return Err(report!(
            errors::ApiErrorResponse::WebhookAuthenticationFailed
        ));
    }

    let connector_name = connector_enum.id().to_string();

    let connector = Connector::from_str(&connector_name)
        .change_context(ConnectorError::InvalidConnectorName)
        .change_context(errors::ApiErrorResponse::IncorrectConnectorNameGiven)
        .attach_printable_lazy(|| format!("unable to parse connector name {connector_name}"))?;

    let mit_payment_data = connector_enum
        .get_subscription_mit_payment_data(request_details)
        .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
        .attach_printable("Failed to extract MIT payment data from subscription webhook")?;

    let profile_id = business_profile.get_id().clone();

    let profile = SubscriptionHandler::find_business_profile(&state, &platform, &profile_id)
        .await
        .attach_printable("subscriptions: failed to find business profile in get_subscription")?;

    let handler = SubscriptionHandler::new(&state, &platform);

    let subscription_id = mit_payment_data.subscription_id.clone();

    let subscription_with_handler = handler
        .find_subscription(subscription_id.clone())
        .await
        .attach_printable("subscriptions: failed to get subscription entry in get_subscription")?;

    let invoice_handler = subscription_with_handler.get_invoice_handler(profile.clone());
    let invoice = invoice_handler
        .find_invoice_by_subscription_id_connector_invoice_id(
            &state,
            subscription_id,
            mit_payment_data.invoice_id.clone(),
        )
        .await
        .attach_printable(
            "subscriptions: failed to get invoice by subscription id and connector invoice id",
        )?;
    if let Some(invoice) = invoice {
        // During CIT payment we would have already created invoice entry with status as PaymentPending or Paid.
        // So we skip incoming webhook for the already processed invoice
        if invoice.status != InvoiceStatus::InvoiceCreated {
            logger::info!("Invoice is already being processed, skipping MIT payment creation");
            return Ok(WebhookResponseTracker::NoEffect);
        }
    }

    let payment_method_id = subscription_with_handler
        .subscription
        .payment_method_id
        .clone()
        .ok_or(errors::ApiErrorResponse::GenericNotFoundError {
            message: "No payment method found for subscription".to_string(),
        })
        .attach_printable("No payment method found for subscription")?;

    logger::info!("Payment method ID found: {}", payment_method_id);

    let payment_id = generate_id(consts::ID_LENGTH, "pay");
    let payment_id = common_utils::id_type::PaymentId::wrap(payment_id).change_context(
        errors::ApiErrorResponse::InvalidDataValue {
            field_name: "payment_id",
        },
    )?;

    // Multiple MIT payments for the same invoice_generated event is avoided by having the unique constraint on (subscription_id, connector_invoice_id) in the invoices table
    let invoice_entry = invoice_handler
        .create_invoice_entry(
            &state,
            billing_connector_mca_id.clone(),
            Some(payment_id),
            mit_payment_data.amount_due,
            mit_payment_data.currency_code,
            InvoiceStatus::PaymentPending,
            connector,
            None,
            Some(mit_payment_data.invoice_id.clone()),
        )
        .await?;

    // Create a sync job for the invoice with generated payment_id before initiating MIT payment creation.
    // This ensures that if payment creation call fails, the sync job can still retrieve the payment status
    invoice_handler
        .create_invoice_sync_job(
            &state,
            &invoice_entry,
            Some(mit_payment_data.invoice_id.clone()),
            connector,
        )
        .await?;

    let payment_response = invoice_handler
        .create_mit_payment(
            &state,
            mit_payment_data.amount_due,
            mit_payment_data.currency_code,
            &payment_method_id.clone(),
        )
        .await?;

    let update_request = invoice::InvoiceUpdateRequest::update_payment_and_status(
        payment_response.payment_method_id,
        Some(payment_response.payment_id.clone()),
        InvoiceStatus::from(payment_response.status),
        Some(mit_payment_data.invoice_id.clone()),
    );

    let _updated_invoice = invoice_handler
        .update_invoice(&state, invoice_entry.id.clone(), update_request)
        .await?;

    Ok(WebhookResponseTracker::NoEffect)
}
