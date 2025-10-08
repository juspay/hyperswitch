use api_models::subscription::{
    self as subscription_types, SubscriptionResponse, SubscriptionStatus,
};
use common_enums::connector_enums;
use common_utils::id_type::GenerateId;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    api::ApplicationResponse, merchant_context::MerchantContext,
    router_response_types::subscriptions as subscription_response_types,
};

use super::errors::{self, RouterResponse};
use crate::{
    core::subscription::{
        billing_processor_handler::BillingHandler, invoice_handler::InvoiceHandler,
        subscription_handler::SubscriptionHandler,
    },
    routes::SessionState,
};

pub mod billing_processor_handler;
pub mod invoice_handler;
pub mod payments_api_client;
pub mod subscription_handler;

pub const SUBSCRIPTION_CONNECTOR_ID: &str = "DefaultSubscriptionConnectorId";
pub const SUBSCRIPTION_PAYMENT_ID: &str = "DefaultSubscriptionPaymentId";

pub async fn create_subscription(
    state: SessionState,
    merchant_context: MerchantContext,
    profile_id: common_utils::id_type::ProfileId,
    request: subscription_types::CreateSubscriptionRequest,
) -> RouterResponse<subscription_types::ConfirmSubscriptionResponse> {
    let subscription_id = common_utils::id_type::SubscriptionId::generate();

    let profile =
        SubscriptionHandler::find_business_profile(&state, &merchant_context, &profile_id)
            .await
            .attach_printable("subscriptions: failed to find business profile")?;
    let _customer =
        SubscriptionHandler::find_customer(&state, &merchant_context, &request.customer_id)
            .await
            .attach_printable("subscriptions: failed to find customer")?;
    let billing_handler = BillingHandler::create(
        &state,
        merchant_context.get_merchant_account(),
        merchant_context.get_merchant_key_store(),
        profile.clone(),
    )
    .await?;

    let subscription_handler = SubscriptionHandler::new(&state, &merchant_context);
    let mut subscription = subscription_handler
        .create_subscription_entry(
            subscription_id,
            &request.customer_id,
            billing_handler.connector_data.connector_name,
            billing_handler.merchant_connector_id.clone(),
            request.merchant_reference_id.clone(),
            &profile.clone(),
        )
        .await
        .attach_printable("subscriptions: failed to create subscription entry")?;
    let invoice_handler = subscription.get_invoice_handler(profile.clone());
    let payment = invoice_handler
        .create_payment_with_confirm_false(subscription.handler.state, &request)
        .await
        .attach_printable("subscriptions: failed to create payment")?;
    let invoice_entry = invoice_handler
        .create_invoice_entry(
            &state,
            billing_handler.merchant_connector_id,
            Some(payment.payment_id.clone()),
            request.amount,
            request.currency,
            connector_enums::InvoiceStatus::InvoiceCreated,
            billing_handler.connector_data.connector_name,
            None,
            None,
        )
        .await
        .attach_printable("subscriptions: failed to create invoice")?;

    subscription
        .update_subscription(
            hyperswitch_domain_models::subscription::SubscriptionUpdate::new(
                payment.payment_method_id.clone(),
                None,
                None,
            ),
        )
        .await
        .attach_printable("subscriptions: failed to update subscription")?;

    let response = subscription.generate_response(
        &invoice_entry,
        &payment,
        subscription_response_types::SubscriptionStatus::Created,
    )?;

    Ok(ApplicationResponse::Json(response))
}

pub async fn get_subscription_plans(
    state: SessionState,
    merchant_context: MerchantContext,
    profile_id: common_utils::id_type::ProfileId,
    query: subscription_types::GetPlansQuery,
) -> RouterResponse<Vec<subscription_types::GetPlansResponse>> {
    let profile =
        SubscriptionHandler::find_business_profile(&state, &merchant_context, &profile_id)
            .await
            .attach_printable("subscriptions: failed to find business profile")?;

    let subscription_handler = SubscriptionHandler::new(&state, &merchant_context);

    if let Some(client_secret) = query.client_secret {
        subscription_handler
            .find_and_validate_subscription(&client_secret.into())
            .await?
    };

    let billing_handler = BillingHandler::create(
        &state,
        merchant_context.get_merchant_account(),
        merchant_context.get_merchant_key_store(),
        profile.clone(),
    )
    .await?;

    let get_plans_response = billing_handler
        .get_subscription_plans(&state, query.limit, query.offset)
        .await?;

    let mut response = Vec::new();

    for plan in &get_plans_response.list {
        let plan_price_response = billing_handler
            .get_subscription_plan_prices(&state, plan.subscription_provider_plan_id.clone())
            .await?;

        response.push(subscription_types::GetPlansResponse {
            plan_id: plan.subscription_provider_plan_id.clone(),
            name: plan.name.clone(),
            description: plan.description.clone(),
            price_id: plan_price_response
                .list
                .into_iter()
                .map(subscription_types::SubscriptionPlanPrices::from)
                .collect::<Vec<_>>(),
        })
    }

    Ok(ApplicationResponse::Json(response))
}
/// Creates and confirms a subscription in one operation.
/// This method combines the creation and confirmation flow to reduce API calls
pub async fn create_and_confirm_subscription(
    state: SessionState,
    merchant_context: MerchantContext,
    profile_id: common_utils::id_type::ProfileId,
    request: subscription_types::CreateAndConfirmSubscriptionRequest,
) -> RouterResponse<subscription_types::ConfirmSubscriptionResponse> {
    let subscription_id = common_utils::id_type::SubscriptionId::generate();

    let profile =
        SubscriptionHandler::find_business_profile(&state, &merchant_context, &profile_id)
            .await
            .attach_printable("subscriptions: failed to find business profile")?;
    let customer =
        SubscriptionHandler::find_customer(&state, &merchant_context, &request.customer_id)
            .await
            .attach_printable("subscriptions: failed to find customer")?;

    let billing_handler = BillingHandler::create(
        &state,
        merchant_context.get_merchant_account(),
        merchant_context.get_merchant_key_store(),
        profile.clone(),
    )
    .await?;
    let subscription_handler = SubscriptionHandler::new(&state, &merchant_context);
    let mut subs_handler = subscription_handler
        .create_subscription_entry(
            subscription_id.clone(),
            &request.customer_id,
            billing_handler.connector_data.connector_name,
            billing_handler.merchant_connector_id.clone(),
            request.merchant_reference_id.clone(),
            &profile.clone(),
        )
        .await
        .attach_printable("subscriptions: failed to create subscription entry")?;
    let invoice_handler = subs_handler.get_invoice_handler(profile.clone());

    let customer_create_response = billing_handler
        .create_customer_on_connector(
            &state,
            customer.clone(),
            request.customer_id.clone(),
            request.billing.clone(),
            request
                .payment_details
                .payment_method_data
                .clone()
                .and_then(|data| data.payment_method_data),
        )
        .await?;
    let _customer_updated_response = SubscriptionHandler::update_connector_customer_id_in_customer(
        &state,
        &merchant_context,
        &billing_handler.merchant_connector_id,
        &customer,
        customer_create_response,
    )
    .await
    .attach_printable("Failed to update customer with connector customer ID")?;

    let subscription_create_response = billing_handler
        .create_subscription_on_connector(
            &state,
            subs_handler.subscription.clone(),
            request.item_price_id.clone(),
            request.billing.clone(),
        )
        .await?;

    let invoice_details = subscription_create_response.invoice_details;
    let (amount, currency) = InvoiceHandler::get_amount_and_currency(
        (request.amount, request.currency),
        invoice_details.clone(),
    );

    let payment_response = invoice_handler
        .create_and_confirm_payment(&state, &request, amount, currency)
        .await?;

    let invoice_entry = invoice_handler
        .create_invoice_entry(
            &state,
            profile.get_billing_processor_id()?,
            Some(payment_response.payment_id.clone()),
            amount,
            currency,
            invoice_details
                .clone()
                .and_then(|invoice| invoice.status)
                .unwrap_or(connector_enums::InvoiceStatus::InvoiceCreated),
            billing_handler.connector_data.connector_name,
            None,
            invoice_details.clone().map(|invoice| invoice.id),
        )
        .await?;

    invoice_handler
        .create_invoice_sync_job(
            &state,
            &invoice_entry,
            invoice_details.clone().map(|details| details.id),
            billing_handler.connector_data.connector_name,
        )
        .await?;

    subs_handler
        .update_subscription(
            hyperswitch_domain_models::subscription::SubscriptionUpdate::new(
                payment_response.payment_method_id.clone(),
                Some(SubscriptionStatus::from(subscription_create_response.status).to_string()),
                Some(
                    subscription_create_response
                        .subscription_id
                        .get_string_repr()
                        .to_string(),
                ),
            ),
        )
        .await?;

    let response = subs_handler.generate_response(
        &invoice_entry,
        &payment_response,
        subscription_create_response.status,
    )?;

    Ok(ApplicationResponse::Json(response))
}

pub async fn confirm_subscription(
    state: SessionState,
    merchant_context: MerchantContext,
    profile_id: common_utils::id_type::ProfileId,
    request: subscription_types::ConfirmSubscriptionRequest,
    subscription_id: common_utils::id_type::SubscriptionId,
) -> RouterResponse<subscription_types::ConfirmSubscriptionResponse> {
    // Find the subscription from database
    let profile =
        SubscriptionHandler::find_business_profile(&state, &merchant_context, &profile_id)
            .await
            .attach_printable("subscriptions: failed to find business profile")?;
    let customer =
        SubscriptionHandler::find_customer(&state, &merchant_context, &request.customer_id)
            .await
            .attach_printable("subscriptions: failed to find customer")?;

    let handler = SubscriptionHandler::new(&state, &merchant_context);
    let mut subscription_entry = handler.find_subscription(subscription_id).await?;
    let invoice_handler = subscription_entry.get_invoice_handler(profile.clone());
    let invoice = invoice_handler
        .get_latest_invoice(&state)
        .await
        .attach_printable("subscriptions: failed to get latest invoice")?;
    let payment_response = invoice_handler
        .confirm_payment(
            &state,
            invoice
                .payment_intent_id
                .ok_or(errors::ApiErrorResponse::MissingRequiredField {
                    field_name: "payment_intent_id",
                })?,
            &request,
        )
        .await?;

    let billing_handler = BillingHandler::create(
        &state,
        merchant_context.get_merchant_account(),
        merchant_context.get_merchant_key_store(),
        profile.clone(),
    )
    .await?;
    let invoice_handler = subscription_entry.get_invoice_handler(profile);
    let subscription = subscription_entry.subscription.clone();

    let customer_create_response = billing_handler
        .create_customer_on_connector(
            &state,
            customer.clone(),
            subscription.customer_id.clone(),
            request.billing.clone(),
            request
                .payment_details
                .payment_method_data
                .payment_method_data,
        )
        .await?;
    let _customer_updated_response = SubscriptionHandler::update_connector_customer_id_in_customer(
        &state,
        &merchant_context,
        &billing_handler.merchant_connector_id,
        &customer,
        customer_create_response,
    )
    .await
    .attach_printable("Failed to update customer with connector customer ID")?;

    let subscription_create_response = billing_handler
        .create_subscription_on_connector(
            &state,
            subscription,
            request.item_price_id,
            request.billing,
        )
        .await?;

    let invoice_details = subscription_create_response.invoice_details;
    let invoice_entry = invoice_handler
        .update_invoice(
            &state,
            invoice.id,
            payment_response.payment_method_id.clone(),
            Some(payment_response.payment_id.clone()),
            invoice_details
                .clone()
                .and_then(|invoice| invoice.status)
                .unwrap_or(connector_enums::InvoiceStatus::InvoiceCreated),
            invoice_details.clone().map(|invoice| invoice.id),
        )
        .await?;

    invoice_handler
        .create_invoice_sync_job(
            &state,
            &invoice_entry,
            invoice_details.map(|invoice| invoice.id),
            billing_handler.connector_data.connector_name,
        )
        .await?;

    subscription_entry
        .update_subscription(
            hyperswitch_domain_models::subscription::SubscriptionUpdate::new(
                payment_response.payment_method_id.clone(),
                Some(SubscriptionStatus::from(subscription_create_response.status).to_string()),
                Some(
                    subscription_create_response
                        .subscription_id
                        .get_string_repr()
                        .to_string(),
                ),
            ),
        )
        .await?;

    let response = subscription_entry.generate_response(
        &invoice_entry,
        &payment_response,
        subscription_create_response.status,
    )?;

    Ok(ApplicationResponse::Json(response))
}

pub async fn get_subscription(
    state: SessionState,
    merchant_context: MerchantContext,
    profile_id: common_utils::id_type::ProfileId,
    subscription_id: common_utils::id_type::SubscriptionId,
) -> RouterResponse<SubscriptionResponse> {
    let _profile =
        SubscriptionHandler::find_business_profile(&state, &merchant_context, &profile_id)
            .await
            .attach_printable(
                "subscriptions: failed to find business profile in get_subscription",
            )?;
    let handler = SubscriptionHandler::new(&state, &merchant_context);
    let subscription = handler
        .find_subscription(subscription_id)
        .await
        .attach_printable("subscriptions: failed to get subscription entry in get_subscription")?;

    Ok(ApplicationResponse::Json(
        subscription.to_subscription_response(),
    ))
}

pub async fn get_estimate(
    state: SessionState,
    merchant_context: MerchantContext,
    profile_id: common_utils::id_type::ProfileId,
    query: subscription_types::EstimateSubscriptionQuery,
) -> RouterResponse<subscription_types::EstimateSubscriptionResponse> {
    let profile =
        SubscriptionHandler::find_business_profile(&state, &merchant_context, &profile_id)
            .await
            .attach_printable("subscriptions: failed to find business profile in get_estimate")?;
    let billing_handler = BillingHandler::create(
        &state,
        merchant_context.get_merchant_account(),
        merchant_context.get_merchant_key_store(),
        profile,
    )
    .await?;
    let estimate = billing_handler
        .get_subscription_estimate(&state, query)
        .await?;
    Ok(ApplicationResponse::Json(estimate.into()))
}
