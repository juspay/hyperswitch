use std::str::FromStr;

use api_models::subscription::{
    self as subscription_types, CreateSubscriptionResponse, SubscriptionStatus,
};
use common_utils::id_type::GenerateId;
use diesel_models::subscription::SubscriptionNew;
use error_stack::ResultExt;
use hyperswitch_domain_models::{api::ApplicationResponse, merchant_context::MerchantContext};
use masking::Secret;

use super::errors::{self, RouterResponse};
use crate::{core::utils::subscription as subscription_utils, routes::SessionState};

pub async fn create_subscription(
    state: SessionState,
    merchant_context: MerchantContext,
    profile_id: String,
    request: subscription_types::CreateSubscriptionRequest,
) -> RouterResponse<CreateSubscriptionResponse> {
    let store = state.store.clone();
    let db = store.as_ref();
    let id = common_utils::id_type::SubscriptionId::generate();
    let profile_id = common_utils::id_type::ProfileId::from_str(&profile_id).change_context(
        errors::ApiErrorResponse::InvalidDataValue {
            field_name: "X-Profile-Id",
        },
    )?;

    let mut subscription = SubscriptionNew::new(
        id,
        SubscriptionStatus::Created.to_string(),
        None,
        None,
        None,
        None,
        None,
        merchant_context.get_merchant_account().get_id().clone(),
        request.customer_id.clone(),
        None,
        profile_id,
        request.merchant_reference_id,
    );

    subscription.generate_and_set_client_secret();
    let subscription_response = db
        .insert_subscription_entry(subscription)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("subscriptions: unable to insert subscription entry to database")?;

    let response = CreateSubscriptionResponse::new(
        subscription_response.id.clone(),
        subscription_response.merchant_reference_id,
        SubscriptionStatus::from_str(&subscription_response.status)
            .unwrap_or(SubscriptionStatus::Created),
        None,
        subscription_response.profile_id,
        subscription_response.merchant_id,
        subscription_response.client_secret.map(Secret::new),
        request.customer_id,
    );

    Ok(ApplicationResponse::Json(response))
}

pub async fn get_subscription_plans(
    state: SessionState,
    merchant_context: MerchantContext,
    profile_id: common_utils::id_type::ProfileId,
    query: subscription_types::GetPlansQuery,
) -> RouterResponse<Vec<subscription_types::GetPlansResponse>> {
    let key_manager_state = &(&state).into();
    let merchant_key_store = merchant_context.get_merchant_key_store();

    let profile = state
        .store
        .find_business_profile_by_profile_id(key_manager_state, merchant_key_store, &profile_id)
        .await
        .change_context(errors::ApiErrorResponse::ProfileNotFound {
            id: profile_id.get_string_repr().to_string(),
        })?;

    let subscription_handler = subscription_utils::SubscriptionHandler::new(
        state.clone(),
        merchant_context.clone(),
        profile,
    );

    if let Some(client_secret) = query.client_secret {
        subscription_handler
            .find_and_validate_subscription(&client_secret.into())
            .await?
    };

    let subscription_with_handler =
        subscription_utils::SubscriptionWithHandler::new(&subscription_handler, None);

    let billing_handler = subscription_with_handler
        .get_billing_handler(None, None)
        .await?;
    let get_plans_response = billing_handler
        .get_subscription_plans(&state, query.limit)
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

pub async fn confirm_subscription(
    state: SessionState,
    merchant_context: MerchantContext,
    profile_id: String,
    request: subscription_types::ConfirmSubscriptionRequest,
    subscription_id: common_utils::id_type::SubscriptionId,
) -> RouterResponse<subscription_types::ConfirmSubscriptionResponse> {
    let profile_id = common_utils::id_type::ProfileId::from_str(&profile_id).change_context(
        errors::ApiErrorResponse::InvalidDataValue {
            field_name: "X-Profile-Id",
        },
    )?;

    let key_manager_state = &(&state).into();
    let merchant_key_store = merchant_context.get_merchant_key_store();

    let profile = state
        .store
        .find_business_profile_by_profile_id(key_manager_state, merchant_key_store, &profile_id)
        .await
        .change_context(errors::ApiErrorResponse::ProfileNotFound {
            id: profile_id.get_string_repr().to_string(),
        })?;

    let customer = state
        .store
        .find_customer_by_customer_id_merchant_id(
            key_manager_state,
            &request.customer_id,
            merchant_context.get_merchant_account().get_id(),
            merchant_key_store,
            merchant_context.get_merchant_account().storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::CustomerNotFound)
        .attach_printable("subscriptions: unable to fetch customer from database")?;

    let handler = subscription_utils::SubscriptionHandler::new(state, merchant_context, profile);

    let mut subscription_entry = handler
        .find_subscription(subscription_id.get_string_repr().to_string())
        .await?;

    let billing_handler = subscription_entry
        .get_billing_handler(Some(customer), Some(&request))
        .await?;
    let invoice_handler = subscription_entry.get_invoice_handler().await?;

    let _customer_create_response = billing_handler
        .create_customer_on_connector(&handler.state)
        .await?;

    let subscription_create_response = billing_handler
        .create_subscription_on_connector(&handler.state)
        .await?;

    // let payment_response = invoice_handler.create_cit_payment().await?;

    let invoice_entry = invoice_handler
        .create_invoice_entry(
            &handler.state,
            subscription_entry.profile.get_billing_processor_id()?,
            None,
            request.amount,
            request.currency.to_string(),
            common_enums::connector_enums::InvoiceStatus::InvoiceCreated,
            billing_handler.connector_data.connector_name,
            None,
        )
        .await?;

    // invoice_entry
    //     .create_invoice_record_back_job(&payment_response)
    //     .await?;

    subscription_entry
        .update_subscription_status(
            SubscriptionStatus::from(subscription_create_response.status).to_string(),
        )
        .await?;

    let response = subscription_entry
        .generate_response(&invoice_entry, subscription_create_response.status)?;

    Ok(ApplicationResponse::Json(response))
}
