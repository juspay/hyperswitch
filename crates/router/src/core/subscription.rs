use std::str::FromStr;

use api_models::subscription::{self as subscription_types, CreateSubscriptionResponse};
use common_utils::id_type::GenerateId;
use diesel_models::subscription::SubscriptionNew;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    api::ApplicationResponse, merchant_context::MerchantContext,
    subscriptions::CreateSubscriptionRequest,
};

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

    let merchant_id = merchant_context.get_merchant_account().get_id().clone();
    let domain_request: CreateSubscriptionRequest = request.clone().into();
    let mut subscription: SubscriptionNew =
        domain_request.to_subscription_new(id.clone(), profile_id.clone(), merchant_id.clone());

    subscription.generate_and_set_client_secret();
    let subscription_response = db
        .insert_subscription_entry(subscription)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("subscriptions: unable to insert subscription entry to database")?;

    let domain_response =
        hyperswitch_domain_models::subscriptions::CreateSubscriptionResponse::from_subscription_db(
            subscription_response,
            request.customer_id,
        );
        
    let response = CreateSubscriptionResponse::from(domain_response);

    Ok(ApplicationResponse::Json(response))
}
