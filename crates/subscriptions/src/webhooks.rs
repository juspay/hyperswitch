use std::str::FromStr;

use api_models::webhooks::WebhookResponseTracker;
use common_enums::{connector_enums::Connector, InvoiceStatus, SubscriptionStatus};
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
use sha2::{Digest, Sha256};

use crate::state::SubscriptionState as SessionState;
#[cfg(feature = "v1")]
use crate::subscription_handler::SubscriptionHandler;

fn should_apply_stripe_invoice_status(existing: &InvoiceStatus, incoming: &InvoiceStatus) -> bool {
    if existing == incoming || *existing == InvoiceStatus::InvoicePaid {
        return false;
    }
    if *incoming == InvoiceStatus::InvoicePaid {
        return true;
    }
    match existing {
        InvoiceStatus::InvoiceCreated => true,
        InvoiceStatus::PaymentPending => !matches!(incoming, InvoiceStatus::InvoiceCreated),
        _ => false,
    }
}

fn should_apply_subscription_status(existing: &str, incoming: &SubscriptionStatus) -> bool {
    !matches!(
        SubscriptionStatus::from_str(existing),
        Ok(SubscriptionStatus::Cancelled)
    ) || matches!(incoming, SubscriptionStatus::Cancelled)
}

fn should_apply_unversioned_subscription_webhook(
    existing: &str,
    incoming: &SubscriptionStatus,
) -> bool {
    matches!(incoming, SubscriptionStatus::Cancelled)
        && should_apply_subscription_status(existing, incoming)
}

fn subscription_status_from_renewal_invoice(
    existing_subscription_status: &str,
    invoice_status: &InvoiceStatus,
) -> Option<SubscriptionStatus> {
    match (
        SubscriptionStatus::from_str(existing_subscription_status),
        invoice_status,
    ) {
        (Ok(SubscriptionStatus::Cancelled), _) => None,
        (_, InvoiceStatus::PaymentFailed) => Some(SubscriptionStatus::Unpaid),
        (Ok(SubscriptionStatus::Unpaid), InvoiceStatus::InvoicePaid) => {
            Some(SubscriptionStatus::Active)
        }
        _ => None,
    }
}

fn subscription_binding_matches(
    stored_client_secret: Option<&str>,
    webhook_binding: Option<&str>,
) -> bool {
    let (Some(stored_client_secret), Some(webhook_binding)) =
        (stored_client_secret, webhook_binding)
    else {
        return false;
    };
    let expected = hex::encode(Sha256::digest(stored_client_secret.as_bytes()));
    expected == webhook_binding
}

async fn converge_stripe_invoice(
    invoice_handler: &crate::core::invoice_handler::InvoiceHandler,
    state: &SessionState,
    mut current: invoice::Invoice,
    connector_invoice_id: common_utils::id_type::InvoiceId,
    incoming_status: InvoiceStatus,
    billing_period_end: time::PrimitiveDateTime,
) -> Result<invoice::Invoice, error_stack::Report<errors::ApiErrorResponse>> {
    for _ in 0..3 {
        let target_status = if should_apply_stripe_invoice_status(&current.status, &incoming_status)
        {
            incoming_status.clone()
        } else {
            current.status.clone()
        };
        let needs_update = current.status != target_status
            || current.connector_invoice_id.as_ref() != Some(&connector_invoice_id)
            || current.billing_period_end != Some(billing_period_end);
        if !needs_update {
            return Ok(current);
        }

        if let Some(updated) = invoice_handler
            .update_invoice_if_status(
                state,
                current.id.clone(),
                current.status.clone(),
                invoice::InvoiceUpdateRequest::update_connector_status_and_period(
                    connector_invoice_id.clone(),
                    target_status,
                    Some(billing_period_end),
                ),
            )
            .await?
        {
            return Ok(updated);
        }
        current = invoice_handler
            .get_invoice_by_id(state, current.id.clone())
            .await?;
    }
    Ok(current)
}

async fn converge_subscription_from_stripe_invoice(
    subscription: &mut crate::subscription_handler::SubscriptionWithHandler<'_>,
    invoice_handler: &crate::core::invoice_handler::InvoiceHandler,
    state: &SessionState,
    invoice_id: common_utils::id_type::InvoiceId,
    billing_period_end: time::PrimitiveDateTime,
    first_invoice: bool,
) -> Result<(), error_stack::Report<errors::ApiErrorResponse>> {
    for _ in 0..3 {
        let invoice = invoice_handler
            .get_invoice_by_id(state, invoice_id.clone())
            .await?;
        let target_status = if first_invoice {
            match invoice.status {
                InvoiceStatus::InvoicePaid => Some(SubscriptionStatus::Active),
                InvoiceStatus::PaymentFailed => Some(SubscriptionStatus::Unpaid),
                _ => Some(SubscriptionStatus::Pending),
            }
        } else {
            subscription_status_from_renewal_invoice(
                &subscription.subscription.status,
                &invoice.status,
            )
        };
        if target_status.as_ref().is_some_and(|target_status| {
            !should_apply_subscription_status(&subscription.subscription.status, target_status)
        }) {
            return Ok(());
        }
        let mut subscription_update =
            hyperswitch_domain_models::subscription::SubscriptionUpdate::new(
                None,
                None,
                target_status.map(|status| status.to_string()),
                None,
                None,
            );
        subscription_update.last_applied_billing_period_end = Some(billing_period_end);
        if subscription
            .update_subscription_if_status_and_invoice_status(
                subscription.subscription.status.clone(),
                invoice.id,
                invoice.status,
                billing_period_end,
                subscription_update,
            )
            .await?
        {
            return Ok(());
        }
    }
    Ok(())
}

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
    // Invoice events carry renewal outcomes; subscription events synchronize billing status.
    if !matches!(
        event_type,
        api_models::webhooks::IncomingWebhookEvent::InvoiceGenerated
            | api_models::webhooks::IncomingWebhookEvent::SubscriptionUpdated
            | api_models::webhooks::IncomingWebhookEvent::SubscriptionDeleted
    ) {
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

    let handler = SubscriptionHandler::new(&state, &platform);
    if matches!(
        event_type,
        api_models::webhooks::IncomingWebhookEvent::SubscriptionUpdated
            | api_models::webhooks::IncomingWebhookEvent::SubscriptionDeleted
    ) {
        let subscription_data = connector_enum
            .get_subscription_webhook_data(request_details)
            .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
            .attach_printable("Failed to extract subscription status from webhook")?;
        let mut subscription = if let Some(hyperswitch_subscription_id) =
            subscription_data.hyperswitch_subscription_id
        {
            handler
                .find_subscription(hyperswitch_subscription_id)
                .await?
        } else {
            handler
                .find_subscription_by_connector_id(
                    &billing_connector_mca_id,
                    subscription_data
                        .connector_subscription_id
                        .get_string_repr()
                        .to_string(),
                )
                .await?
        };
        if subscription.subscription.merchant_connector_id.as_ref()
            != Some(&billing_connector_mca_id)
            || (connector == Connector::Stripebilling
                && !subscription_binding_matches(
                    subscription.subscription.client_secret.as_deref(),
                    subscription_data
                        .hyperswitch_subscription_binding
                        .as_deref(),
                ))
            || subscription
                .subscription
                .connector_subscription_id
                .as_ref()
                .is_some_and(|existing_id| {
                    existing_id
                        != subscription_data
                            .connector_subscription_id
                            .get_string_repr()
                })
        {
            return Err(report!(
                errors::ApiErrorResponse::WebhookAuthenticationFailed
            ));
        }
        let connector_subscription_id = subscription_data
            .connector_subscription_id
            .get_string_repr()
            .to_string();
        subscription
            .bind_connector_subscription_id(connector_subscription_id.clone())
            .await?;
        if subscription
            .subscription
            .connector_subscription_id
            .as_deref()
            != Some(connector_subscription_id.as_str())
        {
            return Err(report!(
                errors::ApiErrorResponse::WebhookAuthenticationFailed
            ));
        }
        // Stripe does not guarantee webhook ordering. Until event versions are persisted, external
        // events may only advance to terminal cancellation. Synchronous APIs own reversible states
        // such as active and paused so delayed webhooks cannot overwrite them.
        if should_apply_unversioned_subscription_webhook(
            &subscription.subscription.status,
            &subscription_data.status,
        ) {
            for _ in 0..3 {
                if !should_apply_unversioned_subscription_webhook(
                    &subscription.subscription.status,
                    &subscription_data.status,
                ) {
                    break;
                }
                let expected_status = subscription.subscription.status.clone();
                let updated = subscription
                    .update_subscription_if_status(
                        expected_status,
                        hyperswitch_domain_models::subscription::SubscriptionUpdate::new(
                            None,
                            None,
                            Some(subscription_data.status.to_string()),
                            None,
                            None,
                        ),
                    )
                    .await?;
                if updated {
                    break;
                }
            }
        }
        return Ok(WebhookResponseTracker::NoEffect);
    }

    let mit_payment_data = connector_enum
        .get_subscription_mit_payment_data(request_details)
        .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
        .attach_printable("Failed to extract MIT payment data from subscription webhook")?;
    let stripe_billing_period_end = if connector == Connector::Stripebilling {
        Some(mit_payment_data.billing_period_end.ok_or_else(|| {
            report!(errors::ApiErrorResponse::WebhookProcessingFailure)
                .attach_printable("Stripe invoice webhook is missing its billing period")
        })?)
    } else {
        None
    };

    if mit_payment_data.first_invoice {
        if let Some(hyperswitch_subscription_id) = mit_payment_data.hyperswitch_subscription_id {
            let mut subscription = handler
                .find_subscription(hyperswitch_subscription_id)
                .await?;
            if subscription.subscription.merchant_connector_id.as_ref()
                != Some(&billing_connector_mca_id)
                || (connector == Connector::Stripebilling
                    && !subscription_binding_matches(
                        subscription.subscription.client_secret.as_deref(),
                        mit_payment_data.hyperswitch_subscription_binding.as_deref(),
                    ))
                || subscription
                    .subscription
                    .connector_subscription_id
                    .as_ref()
                    .is_some_and(|existing_id| {
                        existing_id != mit_payment_data.subscription_id.get_string_repr()
                    })
            {
                return Err(report!(
                    errors::ApiErrorResponse::WebhookAuthenticationFailed
                ));
            }
            let invoice_handler = subscription.get_invoice_handler(business_profile.clone());
            let connector_invoice_id = mit_payment_data.invoice_id.clone();
            let invoice = match invoice_handler
                .find_invoice_by_subscription_id_connector_invoice_id(
                    &state,
                    subscription.subscription.id.clone(),
                    connector_invoice_id.clone(),
                )
                .await?
            {
                Some(invoice) => invoice,
                None => {
                    let invoice = invoice_handler.get_latest_invoice(&state).await?;
                    if invoice
                        .connector_invoice_id
                        .as_ref()
                        .is_some_and(|existing_id| existing_id != &connector_invoice_id)
                    {
                        logger::warn!(
                            subscription_id = %subscription.subscription.id.get_string_repr(),
                            connector_invoice_id = %connector_invoice_id.get_string_repr(),
                            "Ignoring replayed first-invoice webhook because the local placeholder has already been bound"
                        );
                        return Ok(WebhookResponseTracker::NoEffect);
                    }
                    invoice
                }
            };
            let invoice_status = mit_payment_data
                .status
                .unwrap_or(InvoiceStatus::InvoiceCreated);
            let invoice_entry = converge_stripe_invoice(
                &invoice_handler,
                &state,
                invoice,
                connector_invoice_id,
                invoice_status,
                stripe_billing_period_end
                    .ok_or_else(|| report!(errors::ApiErrorResponse::WebhookProcessingFailure))?,
            )
            .await?;
            invoice_handler
                .update_invoice(
                    &state,
                    invoice_entry.id.clone(),
                    invoice::InvoiceUpdateRequest::update_amount_and_currency(
                        mit_payment_data.amount_due,
                        mit_payment_data.currency_code.to_string(),
                    ),
                )
                .await?;
            let connector_subscription_id = mit_payment_data
                .subscription_id
                .get_string_repr()
                .to_string();
            subscription
                .bind_connector_subscription_id(connector_subscription_id.clone())
                .await?;
            if subscription
                .subscription
                .connector_subscription_id
                .as_deref()
                != Some(connector_subscription_id.as_str())
            {
                return Err(report!(
                    errors::ApiErrorResponse::WebhookAuthenticationFailed
                ));
            }
            converge_subscription_from_stripe_invoice(
                &mut subscription,
                &invoice_handler,
                &state,
                invoice_entry.id.clone(),
                stripe_billing_period_end
                    .ok_or_else(|| report!(errors::ApiErrorResponse::WebhookProcessingFailure))?,
                true,
            )
            .await?;
            logger::info!(
                invoice_id = %invoice_entry.id.get_string_repr(),
                "First subscription invoice converged through webhook"
            );
        } else {
            logger::info!(
                "Skipping first subscription invoice webhook without Hyperswitch metadata"
            );
        }
        return Ok(WebhookResponseTracker::NoEffect);
    }

    let profile_id = business_profile.get_id().clone();

    let profile = SubscriptionHandler::find_business_profile(&state, &platform, &profile_id)
        .await
        .attach_printable("subscriptions: failed to find business profile in get_subscription")?;

    let mut subscription_with_handler = handler
        .find_subscription_by_connector_id(
            &billing_connector_mca_id,
            mit_payment_data
                .subscription_id
                .get_string_repr()
                .to_string(),
        )
        .await
        .attach_printable("subscriptions: failed to get subscription entry in get_subscription")?;

    let subscription_id = subscription_with_handler.subscription.id.clone();

    if connector == Connector::Stripebilling
        && !subscription_binding_matches(
            subscription_with_handler
                .subscription
                .client_secret
                .as_deref(),
            mit_payment_data.hyperswitch_subscription_binding.as_deref(),
        )
    {
        return Err(report!(
            errors::ApiErrorResponse::WebhookAuthenticationFailed
        ));
    }

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
    if connector == Connector::Stripebilling {
        let invoice_status = mit_payment_data
            .status
            .unwrap_or(InvoiceStatus::InvoiceCreated);

        if let Some(existing_invoice) = invoice.as_ref() {
            let converged_invoice = converge_stripe_invoice(
                &invoice_handler,
                &state,
                existing_invoice.clone(),
                mit_payment_data.invoice_id.clone(),
                invoice_status,
                stripe_billing_period_end
                    .ok_or_else(|| report!(errors::ApiErrorResponse::WebhookProcessingFailure))?,
            )
            .await?;
            converge_subscription_from_stripe_invoice(
                &mut subscription_with_handler,
                &invoice_handler,
                &state,
                converged_invoice.id,
                stripe_billing_period_end
                    .ok_or_else(|| report!(errors::ApiErrorResponse::WebhookProcessingFailure))?,
                false,
            )
            .await?;
            return Ok(WebhookResponseTracker::NoEffect);
        }

        if invoice_status == InvoiceStatus::InvoiceCreated {
            // Native Stripe Billing subscriptions collect renewals themselves. `invoice.created`
            // only means an invoice exists, so initiating another MIT here would double-charge it.
            return Ok(WebhookResponseTracker::NoEffect);
        }

        let connector_invoice_id = mit_payment_data.invoice_id;
        let create_result = invoice_handler
            .create_invoice_entry(
                &state,
                billing_connector_mca_id,
                None,
                mit_payment_data.amount_due,
                mit_payment_data.currency_code,
                invoice_status.clone(),
                connector,
                None,
                Some(connector_invoice_id.clone()),
                mit_payment_data.billing_period_end,
            )
            .await;

        if let Err(create_error) = create_result {
            // Stripe can send `invoice.paid` and the compatibility event
            // `invoice.payment_succeeded` concurrently. Re-query after a unique-key conflict and
            // treat it as idempotent only after confirming that the record already exists.
            let existing_invoice = invoice_handler
                .find_invoice_by_subscription_id_connector_invoice_id(
                    &state,
                    invoice_handler.subscription.id.clone(),
                    connector_invoice_id.clone(),
                )
                .await?;

            if let Some(existing_invoice) = existing_invoice {
                let converged_invoice = converge_stripe_invoice(
                    &invoice_handler,
                    &state,
                    existing_invoice,
                    connector_invoice_id.clone(),
                    invoice_status,
                    stripe_billing_period_end.ok_or_else(|| {
                        report!(errors::ApiErrorResponse::WebhookProcessingFailure)
                    })?,
                )
                .await?;
                converge_subscription_from_stripe_invoice(
                    &mut subscription_with_handler,
                    &invoice_handler,
                    &state,
                    converged_invoice.id,
                    stripe_billing_period_end.ok_or_else(|| {
                        report!(errors::ApiErrorResponse::WebhookProcessingFailure)
                    })?,
                    false,
                )
                .await?;
                logger::info!(
                    "Invoice was created by a concurrent webhook, treating the duplicate as success"
                );
                return Ok(WebhookResponseTracker::NoEffect);
            }

            return Err(create_error);
        }

        let created_invoice = invoice_handler
            .find_invoice_by_subscription_id_connector_invoice_id(
                &state,
                invoice_handler.subscription.id.clone(),
                connector_invoice_id,
            )
            .await?
            .ok_or_else(|| report!(errors::ApiErrorResponse::InternalServerError))?;
        converge_subscription_from_stripe_invoice(
            &mut subscription_with_handler,
            &invoice_handler,
            &state,
            created_invoice.id,
            stripe_billing_period_end
                .ok_or_else(|| report!(errors::ApiErrorResponse::WebhookProcessingFailure))?,
            false,
        )
        .await?;

        return Ok(WebhookResponseTracker::NoEffect);
    }

    if invoice.is_some() {
        // Hyperswitch initiates MIT payments for non-native billing connectors and must avoid a
        // duplicate charge when the invoice already exists.
        logger::info!("Invoice is already being processed, skipping MIT payment creation");
        return Ok(WebhookResponseTracker::NoEffect);
    }

    let payment_method_id = subscription_with_handler
        .subscription
        .payment_method_id
        .clone()
        .ok_or(errors::ApiErrorResponse::GenericNotFoundError {
            message: "No reusable payment reference found for subscription".to_string(),
        })
        .attach_printable("No reusable payment reference found for subscription")?;
    let recurring_details =
        api_models::mandates::RecurringDetails::PaymentMethodId(payment_method_id);

    let payment_id = generate_id(consts::ID_LENGTH, "pay");
    let payment_id = common_utils::id_type::PaymentId::wrap(payment_id).change_context(
        errors::ApiErrorResponse::InvalidDataValue {
            field_name: "payment_id".into(),
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
            mit_payment_data.billing_period_end,
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
            recurring_details,
        )
        .await?;

    let update_request = invoice::InvoiceUpdateRequest::update_payment_and_status(
        payment_response.reusable_payment_method_id(),
        Some(payment_response.payment_id.clone()),
        InvoiceStatus::from(payment_response.status),
        Some(mit_payment_data.invoice_id.clone()),
    );

    let _updated_invoice = invoice_handler
        .update_invoice(&state, invoice_entry.id.clone(), update_request)
        .await?;

    Ok(WebhookResponseTracker::NoEffect)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stripe_failed_invoice_can_transition_to_paid() {
        assert!(should_apply_stripe_invoice_status(
            &InvoiceStatus::PaymentFailed,
            &InvoiceStatus::InvoicePaid,
        ));
    }

    #[test]
    fn stripe_paid_invoice_cannot_be_downgraded() {
        assert!(!should_apply_stripe_invoice_status(
            &InvoiceStatus::InvoicePaid,
            &InvoiceStatus::PaymentFailed,
        ));
    }

    #[test]
    fn duplicate_stripe_invoice_status_is_idempotent() {
        assert!(!should_apply_stripe_invoice_status(
            &InvoiceStatus::InvoicePaid,
            &InvoiceStatus::InvoicePaid,
        ));
    }

    #[test]
    fn stripe_pending_invoice_cannot_return_to_created() {
        assert!(!should_apply_stripe_invoice_status(
            &InvoiceStatus::PaymentPending,
            &InvoiceStatus::InvoiceCreated,
        ));
    }

    #[test]
    fn cancelled_subscription_cannot_be_reactivated_by_a_late_webhook() {
        assert!(!should_apply_subscription_status(
            &SubscriptionStatus::Cancelled.to_string(),
            &SubscriptionStatus::Active,
        ));
        assert!(should_apply_subscription_status(
            &SubscriptionStatus::Active.to_string(),
            &SubscriptionStatus::Cancelled,
        ));
    }

    #[test]
    fn reversible_subscription_status_is_not_applied_from_unversioned_webhook() {
        assert!(!should_apply_unversioned_subscription_webhook(
            &SubscriptionStatus::Paused.to_string(),
            &SubscriptionStatus::Active,
        ));
        assert!(!should_apply_unversioned_subscription_webhook(
            &SubscriptionStatus::Active.to_string(),
            &SubscriptionStatus::Paused,
        ));
    }

    #[test]
    fn renewal_failure_marks_subscription_unpaid_and_same_invoice_retry_restores_active() {
        assert!(matches!(
            subscription_status_from_renewal_invoice(
                &SubscriptionStatus::Active.to_string(),
                &InvoiceStatus::PaymentFailed,
            ),
            Some(SubscriptionStatus::Unpaid)
        ));
        assert!(matches!(
            subscription_status_from_renewal_invoice(
                &SubscriptionStatus::Unpaid.to_string(),
                &InvoiceStatus::InvoicePaid,
            ),
            Some(SubscriptionStatus::Active)
        ));
        assert!(matches!(
            subscription_status_from_renewal_invoice(
                &SubscriptionStatus::Active.to_string(),
                &InvoiceStatus::InvoicePaid,
            ),
            None
        ));
        assert!(matches!(
            subscription_status_from_renewal_invoice(
                &SubscriptionStatus::Cancelled.to_string(),
                &InvoiceStatus::PaymentFailed,
            ),
            None
        ));
    }

    #[test]
    fn stripe_subscription_binding_requires_matching_client_secret_fingerprint() {
        let client_secret = "sub_hyperswitch_secret_high_entropy";
        let binding = hex::encode(Sha256::digest(client_secret.as_bytes()));
        assert!(subscription_binding_matches(
            Some(client_secret),
            Some(&binding)
        ));
        assert!(!subscription_binding_matches(
            Some(client_secret),
            Some("wrong-binding")
        ));
        assert!(!subscription_binding_matches(Some(client_secret), None));
    }
}
