use api_models::subscription::{self as subscription_types, SubscriptionResponse};
use common_enums::connector_enums;
use common_utils::id_type::GenerateId;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    api::ApplicationResponse, invoice::InvoiceUpdateRequest, platform::Platform,
    subscription::SubscriptionUpdate,
};

pub type RouterResponse<T> =
    Result<ApplicationResponse<T>, error_stack::Report<errors::ApiErrorResponse>>;
use api_models::enums::SubscriptionStatus;

use crate::{
    core::{
        billing_processor_handler::BillingHandler, invoice_handler::InvoiceHandler,
        subscription_handler::SubscriptionHandler,
    },
    state::SubscriptionState as SessionState,
};

pub mod billing_processor_handler;
pub mod errors;
pub mod invoice_handler;
pub mod payments_api_client;
pub mod subscription_handler;

pub const SUBSCRIPTION_CONNECTOR_ID: &str = "DefaultSubscriptionConnectorId";
pub const SUBSCRIPTION_PAYMENT_ID: &str = "DefaultSubscriptionPaymentId";

pub async fn create_subscription(
    state: SessionState,
    platform: Platform,
    profile_id: common_utils::id_type::ProfileId,
    request: subscription_types::CreateSubscriptionRequest,
) -> RouterResponse<SubscriptionResponse> {
    let subscription_id = common_utils::id_type::SubscriptionId::generate();

    let profile = SubscriptionHandler::find_business_profile(&state, &platform, &profile_id)
        .await
        .attach_printable("subscriptions: failed to find business profile")?;
    let _customer = SubscriptionHandler::find_customer(&state, &platform, &request.customer_id)
        .await
        .attach_printable("subscriptions: failed to find customer")?;
    let billing_handler = BillingHandler::create(
        &state,
        platform.get_processor().get_account(),
        platform.get_processor().get_key_store(),
        profile.clone(),
    )
    .await?;

    let subscription_handler = SubscriptionHandler::new(&state, &platform);
    let mut subscription = subscription_handler
        .create_subscription_entry(
            subscription_id,
            &request.customer_id,
            billing_handler.connector_name,
            billing_handler.merchant_connector_id.clone(),
            request.merchant_reference_id.clone(),
            &profile.clone(),
            request.plan_id.clone(),
            Some(request.item_price_id.clone()),
        )
        .await
        .attach_printable("subscriptions: failed to create subscription entry")?;

    let estimate_request = subscription_types::EstimateSubscriptionQuery {
        plan_id: request.plan_id.clone(),
        item_price_id: request.item_price_id.clone(),
        coupon_code: None,
    };

    let estimate = billing_handler
        .get_subscription_estimate(&state, estimate_request)
        .await?;

    let invoice_handler = subscription.get_invoice_handler(profile.clone());
    let payment = invoice_handler
        .create_payment_with_confirm_false(
            subscription.handler.state,
            &request,
            estimate.total,
            estimate.currency,
        )
        .await
        .attach_printable("subscriptions: failed to create payment")?;

    let invoice = invoice_handler
        .create_invoice_entry(
            &state,
            billing_handler.merchant_connector_id,
            Some(payment.payment_id.clone()),
            estimate.total,
            estimate.currency,
            connector_enums::InvoiceStatus::InvoiceCreated,
            billing_handler.connector_name,
            None,
            None,
        )
        .await
        .attach_printable("subscriptions: failed to create invoice")?;

    subscription
        .update_subscription(SubscriptionUpdate::new(
            None,
            payment.payment_method_id.clone(),
            None,
            request.plan_id,
            Some(request.item_price_id),
        ))
        .await
        .attach_printable("subscriptions: failed to update subscription")?;

    let response = subscription.to_subscription_response(Some(payment), Some(&invoice))?;

    Ok(ApplicationResponse::Json(response))
}

pub async fn get_subscription_plans(
    state: SessionState,
    platform: Platform,
    profile_id: common_utils::id_type::ProfileId,
    query: subscription_types::GetPlansQuery,
) -> RouterResponse<Vec<subscription_types::GetPlansResponse>> {
    let profile = SubscriptionHandler::find_business_profile(&state, &platform, &profile_id)
        .await
        .attach_printable("subscriptions: failed to find business profile")?;

    let subscription_handler = SubscriptionHandler::new(&state, &platform);

    if let Some(client_secret) = query.client_secret {
        subscription_handler
            .find_and_validate_subscription(&client_secret.into())
            .await?
    };

    let billing_handler = BillingHandler::create(
        &state,
        platform.get_processor().get_account(),
        platform.get_processor().get_key_store(),
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
        });
    }
    Ok(ApplicationResponse::Json(response))
}
/// Creates and confirms a subscription in one operation.
pub async fn create_and_confirm_subscription(
    state: SessionState,
    platform: Platform,
    profile_id: common_utils::id_type::ProfileId,
    request: subscription_types::CreateAndConfirmSubscriptionRequest,
) -> RouterResponse<subscription_types::ConfirmSubscriptionResponse> {
    request
        .validate()
        .map_err(|message| errors::ApiErrorResponse::InvalidRequestData {
            message: message.to_string(),
        })?;

    let subscription_id = common_utils::id_type::SubscriptionId::generate();
    let profile = SubscriptionHandler::find_business_profile(&state, &platform, &profile_id)
        .await
        .attach_printable("subscriptions: failed to find business profile")?;
    let customer = SubscriptionHandler::find_customer(&state, &platform, &request.customer_id)
        .await
        .attach_printable("subscriptions: failed to find customer")?;

    let billing_handler = BillingHandler::create(
        &state,
        platform.get_processor().get_account(),
        platform.get_processor().get_key_store(),
        profile.clone(),
    )
    .await?;
    let subscription_handler = SubscriptionHandler::new(&state, &platform);
    let mut subs_handler = subscription_handler
        .create_subscription_entry(
            subscription_id.clone(),
            &request.customer_id,
            billing_handler.connector_name,
            billing_handler.merchant_connector_id.clone(),
            request.merchant_reference_id.clone(),
            &profile.clone(),
            request.plan_id.clone(),
            Some(request.item_price_id.clone()),
        )
        .await
        .attach_printable("subscriptions: failed to create subscription entry")?;
    let invoice_handler = subs_handler.get_invoice_handler(profile.clone());

    let customer_create_response = billing_handler
        .create_customer_on_connector(
            &state,
            customer.clone(),
            request.customer_id.clone(),
            request.get_billing_address(),
            request
                .payment_details
                .payment_method_data
                .clone()
                .and_then(|data| data.payment_method_data),
        )
        .await?;
    let _customer_updated_response = SubscriptionHandler::update_connector_customer_id_in_customer(
        &state,
        &platform,
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
            Some(request.item_price_id.clone()),
            request.get_billing_address(),
        )
        .await?;

    let invoice_details = subscription_create_response.invoice_details;
    let (amount, currency) =
        InvoiceHandler::get_amount_and_currency((None, None), invoice_details.clone());

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
            billing_handler.connector_name,
            None,
            invoice_details.clone().map(|invoice| invoice.id),
        )
        .await?;

    invoice_handler
        .create_invoice_sync_job(
            &state,
            &invoice_entry,
            invoice_details.clone().map(|details| details.id),
            billing_handler.connector_name,
        )
        .await?;

    subs_handler
        .update_subscription(SubscriptionUpdate::new(
            Some(
                subscription_create_response
                    .subscription_id
                    .get_string_repr()
                    .to_string(),
            ),
            payment_response.payment_method_id.clone(),
            Some(SubscriptionStatus::from(subscription_create_response.status).to_string()),
            request.plan_id,
            Some(request.item_price_id),
        ))
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
    platform: Platform,
    profile_id: common_utils::id_type::ProfileId,
    request: subscription_types::ConfirmSubscriptionRequest,
    subscription_id: common_utils::id_type::SubscriptionId,
) -> RouterResponse<subscription_types::ConfirmSubscriptionResponse> {
    // Validate request
    request
        .validate()
        .map_err(|message| errors::ApiErrorResponse::InvalidRequestData {
            message: message.to_string(),
        })?;
    // Find the subscription from database
    let profile = SubscriptionHandler::find_business_profile(&state, &platform, &profile_id)
        .await
        .attach_printable("subscriptions: failed to find business profile")?;

    let handler = SubscriptionHandler::new(&state, &platform);
    if let Some(client_secret) = request.client_secret.clone() {
        handler
            .find_and_validate_subscription(&client_secret.into())
            .await?
    };

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
        platform.get_processor().get_account(),
        platform.get_processor().get_key_store(),
        profile.clone(),
    )
    .await?;
    let customer = SubscriptionHandler::find_customer(
        &state,
        &platform,
        &subscription_entry.subscription.customer_id,
    )
    .await
    .attach_printable("subscriptions: failed to find customer")?;
    let invoice_handler = subscription_entry.get_invoice_handler(profile);
    let subscription = subscription_entry.subscription.clone();

    let customer_create_response = billing_handler
        .create_customer_on_connector(
            &state,
            customer.clone(),
            subscription.customer_id.clone(),
            payment_response.get_billing_address(),
            request
                .payment_details
                .payment_method_data
                .as_ref()
                .and_then(|data| data.payment_method_data.clone()),
        )
        .await?;
    let _customer_updated_response = SubscriptionHandler::update_connector_customer_id_in_customer(
        &state,
        &platform,
        &billing_handler.merchant_connector_id,
        &customer,
        customer_create_response,
    )
    .await
    .attach_printable("Failed to update customer with connector customer ID")?;

    let subscription_create_response = billing_handler
        .create_subscription_on_connector(
            &state,
            subscription.clone(),
            subscription.item_price_id.clone(),
            payment_response.get_billing_address(),
        )
        .await?;

    let invoice_details = subscription_create_response.invoice_details;
    let update_request = InvoiceUpdateRequest::update_payment_and_status(
        payment_response.payment_method_id.clone(),
        Some(payment_response.payment_id.clone()),
        invoice_details
            .clone()
            .and_then(|invoice| invoice.status)
            .unwrap_or(connector_enums::InvoiceStatus::InvoiceCreated),
        invoice_details.clone().map(|invoice| invoice.id),
    );
    let invoice_entry = invoice_handler
        .update_invoice(&state, invoice.id, update_request)
        .await?;

    invoice_handler
        .create_invoice_sync_job(
            &state,
            &invoice_entry,
            invoice_details.map(|invoice| invoice.id),
            billing_handler.connector_name,
        )
        .await?;

    subscription_entry
        .update_subscription(SubscriptionUpdate::new(
            Some(
                subscription_create_response
                    .subscription_id
                    .get_string_repr()
                    .to_string(),
            ),
            payment_response.payment_method_id.clone(),
            Some(SubscriptionStatus::from(subscription_create_response.status).to_string()),
            subscription.plan_id.clone(),
            subscription.item_price_id.clone(),
        ))
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
    platform: Platform,
    profile_id: common_utils::id_type::ProfileId,
    subscription_id: common_utils::id_type::SubscriptionId,
) -> RouterResponse<SubscriptionResponse> {
    let _profile = SubscriptionHandler::find_business_profile(&state, &platform, &profile_id)
        .await
        .attach_printable("subscriptions: failed to find business profile in get_subscription")?;
    let handler = SubscriptionHandler::new(&state, &platform);
    let subscription = handler
        .find_subscription(subscription_id)
        .await
        .attach_printable("subscriptions: failed to get subscription entry in get_subscription")?;

    let response = subscription.to_subscription_response(None, None)?;

    Ok(ApplicationResponse::Json(response))
}

pub async fn get_estimate(
    state: SessionState,
    platform: Platform,
    profile_id: common_utils::id_type::ProfileId,
    query: subscription_types::EstimateSubscriptionQuery,
) -> RouterResponse<subscription_types::EstimateSubscriptionResponse> {
    let profile = SubscriptionHandler::find_business_profile(&state, &platform, &profile_id)
        .await
        .attach_printable("subscriptions: failed to find business profile in get_estimate")?;
    let billing_handler = BillingHandler::create(
        &state,
        platform.get_processor().get_account(),
        platform.get_processor().get_key_store(),
        profile,
    )
    .await?;
    let estimate = billing_handler
        .get_subscription_estimate(&state, query)
        .await?;
    Ok(ApplicationResponse::Json(estimate.into()))
}

pub async fn pause_subscription(
    state: SessionState,
    platform: Platform,
    profile_id: common_utils::id_type::ProfileId,
    subscription_id: common_utils::id_type::SubscriptionId,
    request: subscription_types::PauseSubscriptionRequest,
) -> RouterResponse<subscription_types::PauseSubscriptionResponse> {
    let _profile = SubscriptionHandler::find_business_profile(&state, &platform, &profile_id)
        .await
        .attach_printable("subscriptions: failed to find business profile in pause_subscription")?;

    let handler = SubscriptionHandler::new(&state, &platform);
    let mut subscription_entry = handler.find_subscription(subscription_id).await?;

    let billing_handler = BillingHandler::create(
        &state,
        platform.get_processor().get_account(),
        platform.get_processor().get_key_store(),
        _profile.clone(),
    )
    .await?;

    // Call the billing processor to pause the subscription
    let pause_response = billing_handler
        .pause_subscription_on_connector(&state, &subscription_entry.subscription, &request)
        .await?;
    let status = SubscriptionStatus::from(pause_response.status);
    // Update the subscription status in our database
    subscription_entry
        .update_subscription(SubscriptionUpdate::update_status(status.to_string()))
        .await?;

    let response = subscription_types::PauseSubscriptionResponse {
        id: subscription_entry.subscription.id.clone(),
        status,
        merchant_reference_id: subscription_entry
            .subscription
            .merchant_reference_id
            .clone(),
        profile_id: subscription_entry.subscription.profile_id.clone(),
        merchant_id: subscription_entry.subscription.merchant_id.clone(),
        customer_id: subscription_entry.subscription.customer_id.clone(),
        paused_at: pause_response.paused_at,
    };

    Ok(ApplicationResponse::Json(response))
}

pub async fn resume_subscription(
    state: SessionState,
    platform: Platform,
    profile_id: common_utils::id_type::ProfileId,
    subscription_id: common_utils::id_type::SubscriptionId,
    request: subscription_types::ResumeSubscriptionRequest,
) -> RouterResponse<subscription_types::ResumeSubscriptionResponse> {
    let _profile = SubscriptionHandler::find_business_profile(&state, &platform, &profile_id)
        .await
        .attach_printable(
            "subscriptions: failed to find business profile in resume_subscription",
        )?;

    let handler = SubscriptionHandler::new(&state, &platform);
    let mut subscription_entry = handler.find_subscription(subscription_id).await?;

    let billing_handler = BillingHandler::create(
        &state,
        platform.get_processor().get_account(),
        platform.get_processor().get_key_store(),
        _profile.clone(),
    )
    .await?;

    // Call the billing processor to resume the subscription
    let resume_response = billing_handler
        .resume_subscription_on_connector(&state, &subscription_entry.subscription, &request)
        .await?;

    let status = SubscriptionStatus::from(resume_response.status);
    // Update the subscription status in our database
    subscription_entry
        .update_subscription(SubscriptionUpdate::update_status(status.to_string()))
        .await?;

    let response = subscription_types::ResumeSubscriptionResponse {
        id: subscription_entry.subscription.id.clone(),
        status,
        merchant_reference_id: subscription_entry
            .subscription
            .merchant_reference_id
            .clone(),
        profile_id: subscription_entry.subscription.profile_id.clone(),
        merchant_id: subscription_entry.subscription.merchant_id.clone(),
        customer_id: subscription_entry.subscription.customer_id.clone(),
        next_billing_at: resume_response.next_billing_at,
    };

    Ok(ApplicationResponse::Json(response))
}

pub async fn cancel_subscription(
    state: SessionState,
    platform: Platform,
    profile_id: common_utils::id_type::ProfileId,
    subscription_id: common_utils::id_type::SubscriptionId,
    request: subscription_types::CancelSubscriptionRequest,
) -> RouterResponse<subscription_types::CancelSubscriptionResponse> {
    let _profile = SubscriptionHandler::find_business_profile(&state, &platform, &profile_id)
        .await
        .attach_printable(
            "subscriptions: failed to find business profile in cancel_subscription",
        )?;

    let handler = SubscriptionHandler::new(&state, &platform);
    let mut subscription_entry = handler.find_subscription(subscription_id).await?;

    let billing_handler = BillingHandler::create(
        &state,
        platform.get_processor().get_account(),
        platform.get_processor().get_key_store(),
        _profile.clone(),
    )
    .await?;

    // Call the billing processor to cancel the subscription
    let cancel_response = billing_handler
        .cancel_subscription_on_connector(&state, &subscription_entry.subscription, &request)
        .await?;

    let status = SubscriptionStatus::from(cancel_response.status);
    // Update the subscription status in our database
    subscription_entry
        .update_subscription(SubscriptionUpdate::update_status(status.to_string()))
        .await?;

    let response = subscription_types::CancelSubscriptionResponse {
        id: subscription_entry.subscription.id.clone(),
        status,
        merchant_reference_id: subscription_entry
            .subscription
            .merchant_reference_id
            .clone(),
        profile_id: subscription_entry.subscription.profile_id.clone(),
        merchant_id: subscription_entry.subscription.merchant_id.clone(),
        customer_id: subscription_entry.subscription.customer_id.clone(),
        cancelled_at: cancel_response.cancelled_at,
    };

    Ok(ApplicationResponse::Json(response))
}

pub async fn update_subscription(
    state: SessionState,
    platform: Platform,
    profile_id: common_utils::id_type::ProfileId,
    subscription_id: common_utils::id_type::SubscriptionId,
    request: subscription_types::UpdateSubscriptionRequest,
) -> RouterResponse<SubscriptionResponse> {
    let profile = SubscriptionHandler::find_business_profile(&state, &platform, &profile_id)
        .await
        .attach_printable("subscriptions: failed to find business profile in get_subscription")?;

    let handler = SubscriptionHandler::new(&state, &platform);
    let mut subscription_entry = handler.find_subscription(subscription_id).await?;

    let invoice_handler = subscription_entry.get_invoice_handler(profile.clone());
    let invoice = invoice_handler
        .get_latest_invoice(&state)
        .await
        .attach_printable("subscriptions: failed to get latest invoice")?;

    let subscription = subscription_entry.subscription.clone();

    subscription_entry
        .update_subscription(SubscriptionUpdate::new(
            None,
            None,
            None,
            Some(request.plan_id.clone()),
            Some(request.item_price_id.clone()),
        ))
        .await?;

    let billing_handler = BillingHandler::create(
        &state,
        platform.get_processor().get_account(),
        platform.get_processor().get_key_store(),
        profile.clone(),
    )
    .await?;

    let estimate_request = subscription_types::EstimateSubscriptionQuery {
        plan_id: Some(request.plan_id.clone()),
        item_price_id: request.item_price_id.clone(),
        coupon_code: None,
    };

    let estimate = billing_handler
        .get_subscription_estimate(&state, estimate_request)
        .await?;

    let update_request = InvoiceUpdateRequest::update_amount_and_currency(
        estimate.total,
        estimate.currency.to_string(),
    );

    let invoice_entry = invoice_handler
        .update_invoice(&state, invoice.id, update_request)
        .await?;

    let _payment_response = invoice_handler
        .update_payment(
            &state,
            estimate.total,
            estimate.currency,
            invoice_entry.payment_intent_id.ok_or(
                errors::ApiErrorResponse::MissingRequiredField {
                    field_name: "payment_intent_id",
                },
            )?,
        )
        .await?;

    Box::pin(get_subscription(
        state,
        platform,
        profile_id,
        subscription.id,
    ))
    .await
}
