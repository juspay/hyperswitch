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
