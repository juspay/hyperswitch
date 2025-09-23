use std::str::FromStr;

use api_models::subscription::{
    self as subscription_types, CreateSubscriptionResponse, SubscriptionStatus,
};
use common_utils::id_type::GenerateId;
use diesel_models::subscription::SubscriptionNew;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    api::ApplicationResponse, merchant_context::MerchantContext, subscription::ClientSecret,
};
use masking::Secret;

use super::{
    errors::{self, RouterResponse},
    utils::subscription::SubscriptionHandler,
};
use crate::routes::SessionState;

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
    _authentication_profile_id: Option<common_utils::id_type::ProfileId>,
    client_secret: ClientSecret,
) -> RouterResponse<Vec<subscription_types::GetPlansResponse>> {
    let subscription_handler = SubscriptionHandler::new(state.clone(), merchant_context.clone());

    let subscription = subscription_handler
        .find_and_validate_subscription(&client_secret)
        .await?;

    let billing_handler = subscription_handler
        .get_billing_handler(&subscription)
        .await?;

    let get_plans_response = billing_handler.get_subscription_plans(&state).await?;

    let plans: Vec<subscription_types::GetPlansResponse> = get_plans_response
        .list
        .into_iter()
        .map(|plan| subscription_types::GetPlansResponse {
            plan_id: plan.subscription_provider_plan_id,
            name: plan.name,
            description: plan.description.unwrap_or_default(),
        })
        .collect();

    Ok(ApplicationResponse::Json(plans))
}
