use std::str::FromStr;

use api_models::subscription::{self as subscription_types, SubscriptionResponse};
use common_enums::connector_enums;
use common_utils::id_type::GenerateId;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    api::ApplicationResponse, invoice::InvoiceUpdateRequest, platform::Platform,
    router_response_types::subscriptions as subscription_response_types,
    subscription::SubscriptionUpdate,
};
use hyperswitch_masking::{PeekInterface, Secret};

pub type RouterResponse<T> =
    Result<ApplicationResponse<T>, error_stack::Report<errors::ApiErrorResponse>>;
use api_models::enums::SubscriptionStatus;

use crate::{
    core::{
        billing_processor_handler::BillingHandler,
        subscription_handler::{SubscriptionHandler, SubscriptionWithHandler},
    },
    state::SubscriptionState as SessionState,
};

pub mod billing_processor_handler;
pub mod errors;
pub mod invoice_handler;
pub mod payments_api_client;
pub mod subscription_handler;

pub const SUBSCRIPTIONS_MAX_LIST_COUNT: i64 = 10;
const STRIPE_SUBSCRIPTION_ID_PREFIX: &str = "sub_";

fn processor_payment_token(
    recurring_details: Option<&api_models::mandates::RecurringDetails>,
) -> Option<Secret<String>> {
    match recurring_details {
        Some(api_models::mandates::RecurringDetails::ProcessorPaymentToken(token)) => {
            Some(Secret::new(token.processor_payment_token.clone()))
        }
        _ => None,
    }
}

struct SubscriptionPaymentReference {
    payment_method_id: Option<Secret<String>>,
}

impl SubscriptionPaymentReference {
    fn from_payment(
        payment: &subscription_types::PaymentResponseData,
    ) -> errors::SubscriptionResult<Self> {
        Ok(Self {
            payment_method_id: payment.payment_method_id.clone(),
        })
    }

    fn invoice_reference_id(&self) -> Option<Secret<String>> {
        self.payment_method_id.clone()
    }
}

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

    if billing_handler.connector_name == connector_enums::Connector::Stripebilling {
        let requested_payment_connector_id = request
            .payment_merchant_connector_id
            .as_ref()
            .map(common_utils::id_type::MerchantConnectorAccountId::get_string_repr);
        if !matches!(
            (
                billing_handler.configured_payment_connector_id(),
                requested_payment_connector_id,
            ),
            (Some(configured), Some(requested)) if configured == requested
        ) {
            return Err(errors::ApiErrorResponse::InvalidRequestData {
                message: "payment_merchant_connector_id must match the Stripe payment connector configured on the Stripe Billing connector"
                    .to_string(),
            }
            .into());
        }
        billing_handler
            .validate_stripe_account_pair(
                &state,
                platform.get_processor().get_account(),
                platform.get_processor().get_key_store(),
                request.payment_merchant_connector_id.as_ref().ok_or(
                    errors::ApiErrorResponse::MissingRequiredField {
                        field_name: "payment_merchant_connector_id".into(),
                    },
                )?,
            )
            .await?;
    }

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
            request.payment_merchant_connector_id.clone(),
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

    let response = SubscriptionWithHandler::to_subscription_response(
        &subscription.subscription,
        Some(payment),
        Some(&invoice),
    )?;

    Ok(ApplicationResponse::Json(response))
}

pub async fn get_subscription_items(
    state: SessionState,
    platform: Platform,
    profile_id: common_utils::id_type::ProfileId,
    query: subscription_types::GetSubscriptionItemsQuery,
) -> RouterResponse<Vec<subscription_types::GetSubscriptionItemsResponse>> {
    let profile = SubscriptionHandler::find_business_profile(&state, &platform, &profile_id)
        .await
        .attach_printable("subscriptions: failed to find business profile")?;

    let subscription_handler = SubscriptionHandler::new(&state, &platform);

    if let Some(client_secret) = query.client_secret {
        subscription_handler
            .find_and_validate_subscription(&client_secret.into())
            .await?;
    };

    let billing_handler = BillingHandler::create(
        &state,
        platform.get_processor().get_account(),
        platform.get_processor().get_key_store(),
        profile.clone(),
    )
    .await?;

    let get_items_response = billing_handler
        .get_subscription_items(&state, query.limit, query.offset, query.item_type)
        .await?;

    let mut response = Vec::new();

    for item in &get_items_response.list {
        let item_price_response = billing_handler
            .get_subscription_item_prices(&state, item.subscription_provider_item_id.clone())
            .await?;

        response.push(subscription_types::GetSubscriptionItemsResponse {
            item_id: item.subscription_provider_item_id.clone(),
            name: item.name.clone(),
            description: item.description.clone(),
            price_id: item_price_response
                .list
                .into_iter()
                .map(subscription_types::SubscriptionItemPrices::from)
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
    if billing_handler.connector_name == connector_enums::Connector::Stripebilling {
        return Err(errors::ApiErrorResponse::NotSupported {
            message: "Stripe Billing subscriptions must be created through Hosted Checkout; direct confirmation is disabled because it can charge the first invoice twice"
                .to_string(),
        }
        .into());
    }
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
            None,
        )
        .await
        .attach_printable("subscriptions: failed to create subscription entry")?;
    let invoice_handler = subs_handler.get_invoice_handler(profile.clone());

    // 创建外部订阅前，先按计费平台价格完成首笔付款，避免支付失败后留下孤立订阅。
    let estimate = billing_handler
        .get_subscription_estimate(
            &state,
            subscription_types::EstimateSubscriptionQuery {
                plan_id: request.plan_id.clone(),
                item_price_id: request.item_price_id.clone(),
                coupon_code: request.coupon_code.clone(),
            },
        )
        .await?;
    let amount = estimate.total;
    let currency = estimate.currency;

    let payment_response = invoice_handler
        .create_and_confirm_payment(&state, &request, amount, currency)
        .await?;
    let payment_reference = SubscriptionPaymentReference::from_payment(&payment_response)?;

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
    let customer_updated_response = SubscriptionHandler::update_connector_customer_id_in_customer(
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
            customer_updated_response
                .get_connector_customer_map()
                .get(&billing_handler.merchant_connector_id)
                .cloned(),
            Some(request.item_price_id.clone()),
            payment_response.connector_mandate_id.clone().or_else(|| {
                processor_payment_token(request.payment_details.recurring_details.as_ref())
            }),
            request.get_billing_address(),
        )
        .await?;

    let invoice_details = subscription_create_response.invoice_details;
    if let Some(invoice) = invoice_details.as_ref() {
        if invoice.total != amount || invoice.currency_code != currency {
            return Err(errors::ApiErrorResponse::InvalidRequestData {
                message: "billing processor invoice does not match the confirmed payment"
                    .to_string(),
            }
            .into());
        }
    }

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
            None,
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
            payment_reference.payment_method_id,
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
    let mut subscription_entry = if let Some(client_secret) = request.client_secret.clone() {
        let subscription_entry = handler
            .find_and_validate_subscription(&client_secret.into())
            .await?;
        if subscription_entry.subscription.id != subscription_id {
            return Err(errors::ApiErrorResponse::ClientSecretInvalid.into());
        }
        subscription_entry
    } else {
        handler.find_subscription(subscription_id).await?
    };
    let invoice_handler = subscription_entry.get_invoice_handler(profile.clone());
    let invoice = invoice_handler
        .get_latest_invoice(&state)
        .await
        .attach_printable("subscriptions: failed to get latest invoice")?;
    if subscription_entry.subscription.billing_processor.as_deref() == Some("stripebilling")
        && subscription_entry
            .subscription
            .connector_subscription_id
            .is_none()
        && !request.payment_already_confirmed.unwrap_or(false)
    {
        return Err(errors::ApiErrorResponse::NotSupported {
            message: "Stripe Billing subscriptions must be created through Hosted Checkout; direct confirmation is disabled because it can charge the first invoice twice"
                .to_string(),
        }
        .into());
    }
    let payment_id = invoice.payment_intent_id.clone().ok_or(
        errors::ApiErrorResponse::MissingRequiredField {
            field_name: "payment_intent_id".into(),
        },
    )?;
    let payment_response = if request.payment_already_confirmed.unwrap_or(false) {
        let payment = invoice_handler
            .get_payment_details(&state, payment_id)
            .await?;
        if payment.status != common_enums::IntentStatus::Succeeded {
            return Err(errors::ApiErrorResponse::InvalidRequestData {
                message: format!(
                    "subscription payment must be succeeded before activation, current status: {}",
                    payment.status
                ),
            }
            .into());
        }
        payment
    } else {
        invoice_handler
            .confirm_payment(&state, payment_id, &request)
            .await?
    };

    if subscription_entry.subscription.billing_processor.as_deref() == Some("stripebilling") {
        let expected_payment_connector_id = subscription_entry
            .subscription
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.peek().get("payment_merchant_connector_id"))
            .and_then(serde_json::Value::as_str);
        let actual_payment_connector_id = payment_response
            .merchant_connector_id
            .as_ref()
            .map(common_utils::id_type::MerchantConnectorAccountId::get_string_repr);
        if expected_payment_connector_id.is_none()
            || expected_payment_connector_id != actual_payment_connector_id
        {
            return Err(errors::ApiErrorResponse::InvalidRequestData {
                message: "hosted checkout payment connector does not match the connector bound to this subscription"
                    .to_string(),
            }
            .into());
        }
    }

    let hosted_stripe_subscription_id = (payment_response.status
        == common_enums::IntentStatus::Succeeded
        && payment_response.connector.as_deref() == Some("stripe")
        && subscription_entry.subscription.billing_processor.as_deref() == Some("stripebilling"))
    .then(|| {
        payment_response
            .connector_response_reference_id
            .as_ref()
            .filter(|reference| reference.starts_with(STRIPE_SUBSCRIPTION_ID_PREFIX))
            .cloned()
    })
    .flatten();

    if let Some(connector_subscription_id) = hosted_stripe_subscription_id {
        if subscription_entry
            .subscription
            .connector_subscription_id
            .as_ref()
            .is_some_and(|existing_id| existing_id != &connector_subscription_id)
        {
            return Err(errors::ApiErrorResponse::InvalidRequestData {
                message: "hosted checkout subscription does not match the existing connector subscription"
                    .to_string(),
            }
            .into());
        }
        // Stripe Checkout 的 `mode=subscription` 已创建 Stripe 订阅，
        // 此处绑定 `sub_*` 并更新首张账单，不再通过计费接口重复创建。
        let mut current_invoice = invoice;
        for _ in 0..2 {
            if let Some(updated_invoice) = invoice_handler
                .update_invoice_if_status(
                    &state,
                    current_invoice.id.clone(),
                    current_invoice.status.clone(),
                    InvoiceUpdateRequest::update_payment_and_status(
                        None,
                        Some(payment_response.payment_id.clone()),
                        connector_enums::InvoiceStatus::InvoicePaid,
                        None,
                    ),
                )
                .await?
            {
                current_invoice = updated_invoice;
                break;
            }
            current_invoice = invoice_handler
                .get_invoice_by_id(&state, current_invoice.id.clone())
                .await?;
        }
        if current_invoice.status != connector_enums::InvoiceStatus::InvoicePaid {
            return Err(errors::ApiErrorResponse::InvalidRequestData {
                message: "hosted checkout invoice did not converge to paid".to_string(),
            }
            .into());
        }

        subscription_entry
            .bind_connector_subscription_id(connector_subscription_id.clone())
            .await?;

        if subscription_entry
            .subscription
            .connector_subscription_id
            .as_ref()
            != Some(&connector_subscription_id)
        {
            return Err(errors::ApiErrorResponse::InvalidRequestData {
                message: "hosted checkout subscription binding changed concurrently".to_string(),
            }
            .into());
        }

        if !matches!(
            SubscriptionStatus::from_str(&subscription_entry.subscription.status),
            Ok(SubscriptionStatus::Cancelled)
        ) {
            subscription_entry
                .update_subscription_if_status(
                    subscription_entry.subscription.status.clone(),
                    SubscriptionUpdate::new(
                        None,
                        None,
                        Some(SubscriptionStatus::Active.to_string()),
                        subscription_entry.subscription.plan_id.clone(),
                        subscription_entry.subscription.item_price_id.clone(),
                    ),
                )
                .await?;
        }

        let response_status =
            match SubscriptionStatus::from_str(&subscription_entry.subscription.status) {
                Ok(SubscriptionStatus::Pending) => {
                    subscription_response_types::SubscriptionStatus::Pending
                }
                Ok(SubscriptionStatus::Trial) => {
                    subscription_response_types::SubscriptionStatus::Trial
                }
                Ok(SubscriptionStatus::Paused) => {
                    subscription_response_types::SubscriptionStatus::Paused
                }
                Ok(SubscriptionStatus::Unpaid) => {
                    subscription_response_types::SubscriptionStatus::Unpaid
                }
                Ok(SubscriptionStatus::Onetime) => {
                    subscription_response_types::SubscriptionStatus::Onetime
                }
                Ok(SubscriptionStatus::Cancelled) => {
                    subscription_response_types::SubscriptionStatus::Cancelled
                }
                Ok(SubscriptionStatus::Failed) => {
                    subscription_response_types::SubscriptionStatus::Failed
                }
                Ok(SubscriptionStatus::Created | SubscriptionStatus::InActive) => {
                    subscription_response_types::SubscriptionStatus::Created
                }
                Ok(SubscriptionStatus::Active) => {
                    subscription_response_types::SubscriptionStatus::Active
                }
                Err(_) => {
                    return Err(errors::ApiErrorResponse::InternalServerError.into());
                }
            };

        let response = subscription_entry.generate_response(
            &current_invoice,
            &payment_response,
            response_status,
        )?;
        return Ok(ApplicationResponse::Json(response));
    }
    if subscription_entry.subscription.billing_processor.as_deref() == Some("stripebilling") {
        return Err(errors::ApiErrorResponse::NotSupported {
            message: "Stripe Billing subscriptions must be created through Hosted Checkout; direct confirmation is disabled because it can charge the first invoice twice"
                .to_string(),
        }
        .into());
    }
    if subscription_entry
        .subscription
        .connector_subscription_id
        .is_some()
    {
        return Err(errors::ApiErrorResponse::InvalidRequestData {
            message: "subscription is already active on the billing processor".to_string(),
        }
        .into());
    }
    let payment_reference = SubscriptionPaymentReference::from_payment(&payment_response)?;

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
    let customer_updated_response = SubscriptionHandler::update_connector_customer_id_in_customer(
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
            customer_updated_response
                .get_connector_customer_map()
                .get(&billing_handler.merchant_connector_id)
                .cloned(),
            subscription.item_price_id.clone(),
            payment_response.connector_mandate_id.clone().or_else(|| {
                processor_payment_token(request.payment_details.recurring_details.as_ref())
            }),
            payment_response.get_billing_address(),
        )
        .await?;

    let invoice_details = subscription_create_response.invoice_details;
    let update_request = InvoiceUpdateRequest::update_payment_and_status(
        payment_reference.invoice_reference_id(),
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
            payment_reference.payment_method_id,
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

    let response =
        SubscriptionWithHandler::to_subscription_response(&subscription.subscription, None, None)?;

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
    if matches!(
        SubscriptionStatus::from_str(&subscription_entry.subscription.status),
        Ok(SubscriptionStatus::Cancelled)
    ) {
        return Err(errors::ApiErrorResponse::InvalidRequestData {
            message: "cancelled subscriptions cannot be paused".to_string(),
        }
        .into());
    }
    let mut expected_status = subscription_entry.subscription.status.clone();

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
    let mut converged = false;
    for _ in 0..3 {
        if matches!(
            SubscriptionStatus::from_str(&subscription_entry.subscription.status),
            Ok(SubscriptionStatus::Cancelled)
        ) {
            return Err(errors::ApiErrorResponse::InvalidRequestData {
                message: "subscription was cancelled while it was being paused".to_string(),
            }
            .into());
        }
        if subscription_entry
            .update_subscription_if_status(
                expected_status,
                SubscriptionUpdate::update_status(status.to_string()),
            )
            .await?
        {
            converged = true;
            break;
        }
        // 连接器暂停成功后，若 webhook 抢先更新，则从最新本地状态收敛，且不得覆盖 cancelled 终态。
        expected_status = subscription_entry.subscription.status.clone();
    }
    if !converged {
        return Err(errors::ApiErrorResponse::InvalidRequestData {
            message: "subscription status could not converge after it was paused".to_string(),
        }
        .into());
    }

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
    if matches!(
        SubscriptionStatus::from_str(&subscription_entry.subscription.status),
        Ok(SubscriptionStatus::Cancelled)
    ) {
        return Err(errors::ApiErrorResponse::InvalidRequestData {
            message: "cancelled subscriptions cannot be resumed".to_string(),
        }
        .into());
    }
    let mut expected_status = subscription_entry.subscription.status.clone();

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
    let mut converged = false;
    for _ in 0..3 {
        if matches!(
            SubscriptionStatus::from_str(&subscription_entry.subscription.status),
            Ok(SubscriptionStatus::Cancelled)
        ) {
            return Err(errors::ApiErrorResponse::InvalidRequestData {
                message: "subscription was cancelled while it was being resumed".to_string(),
            }
            .into());
        }
        if subscription_entry
            .update_subscription_if_status(
                expected_status,
                SubscriptionUpdate::update_status(status.to_string()),
            )
            .await?
        {
            converged = true;
            break;
        }
        // 连接器恢复成功后，若 webhook 抢先更新，则从最新本地状态收敛，且不得覆盖 cancelled 终态。
        expected_status = subscription_entry.subscription.status.clone();
    }
    if !converged {
        return Err(errors::ApiErrorResponse::InvalidRequestData {
            message: "subscription status could not converge after it was resumed".to_string(),
        }
        .into());
    }

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

    if subscription_entry.subscription.billing_processor.as_deref() == Some("stripebilling") {
        return Err(errors::ApiErrorResponse::NotSupported {
            message:
                "Stripe Billing plan changes are not implemented; refusing a local-only update"
                    .to_string(),
        }
        .into());
    }

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
                    field_name: "payment_intent_id".into(),
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

pub async fn list_subscriptions(
    state: SessionState,
    platform: Platform,
    profile_id: common_utils::id_type::ProfileId,
    query: subscription_types::ListSubscriptionQuery,
) -> RouterResponse<Vec<SubscriptionResponse>> {
    SubscriptionHandler::find_business_profile(&state, &platform, &profile_id)
        .await
        .attach_printable("subscriptions: failed to find business profile in list_subscriptions")?;

    let handler = SubscriptionHandler::new(&state, &platform);

    let subscriptions = handler
        .list_subscriptions_by_profile_id(
            &profile_id,
            Some(query.limit.unwrap_or(SUBSCRIPTIONS_MAX_LIST_COUNT)),
            Some(query.offset.unwrap_or_default()),
        )
        .await
        .attach_printable("subscriptions: failed to list subscriptions by profile id")?;

    let mut subscriptions_response = Vec::new();
    for subscription in subscriptions {
        let response = SubscriptionWithHandler::to_subscription_response(&subscription, None, None)
            .attach_printable("subscriptions: failed to convert subscription entry to response")?;
        subscriptions_response.push(response);
    }

    Ok(ApplicationResponse::Json(subscriptions_response))
}

#[cfg(test)]
mod tests {
    use hyperswitch_masking::PeekInterface;

    use super::*;

    #[test]
    fn extracts_processor_token_for_stripe_billing_default_payment_method() {
        let details = api_models::mandates::RecurringDetails::ProcessorPaymentToken(
            api_models::mandates::ProcessorPaymentToken {
                processor_payment_token: "pm_stripe_saved".to_string(),
                merchant_connector_id: None,
            },
        );

        let token = processor_payment_token(Some(&details))
            .expect("connector-native payment credential should be extracted");

        assert_eq!(token.peek(), "pm_stripe_saved");
    }
}
