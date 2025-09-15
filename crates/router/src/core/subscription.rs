pub mod utils;
use api_models::subscription::{
    self as subscription_types, CreateSubscriptionResponse, SubscriptionStatus,
    SUBSCRIPTION_ID_PREFIX,
};
use common_utils::generate_id_with_default_len;
use diesel_models::subscription::SubscriptionNew;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    api::ApplicationResponse, merchant_context::MerchantContext,
    router_request_types::CustomerDetails,
};
use utils::get_or_create_customer;

use super::errors::{self, RouterResponse};
use crate::routes::SessionState;

pub async fn create_subscription(
    state: SessionState,
    merchant_context: MerchantContext,
    request: subscription_types::CreateSubscriptionRequest,
) -> RouterResponse<CreateSubscriptionResponse> {
    let store = state.store.clone();
    let db = store.as_ref();
    let id = request
        .subscription_id
        .clone()
        .unwrap_or(generate_id_with_default_len(SUBSCRIPTION_ID_PREFIX));
    let mut response = CreateSubscriptionResponse::new(
        id.clone(),
        SubscriptionStatus::Created,
        None,
        request.profile_id.clone(),
        merchant_context.get_merchant_account().get_id().clone(),
        request.merchant_connector_account_id.clone(),
    );

    let customer = CustomerDetails::from(request.clone());
    let customer_id = if customer.customer_id.is_some()
        || customer.name.is_some()
        || customer.email.is_some()
        || customer.phone.is_some()
        || customer.phone_country_code.is_some()
    {
        let customer = get_or_create_customer(state, request.customer, merchant_context.clone())
            .await
            .map_err(|e| e.change_context(errors::ApiErrorResponse::CustomerNotFound))
            .attach_printable("subscriptions: unable to process customer")?;

        let customer_table_response = match &customer {
            ApplicationResponse::Json(inner) => {
                Some(subscription_types::map_customer_resp_to_details(inner))
            }
            _ => None,
        };
        response.customer = customer_table_response;
        response
            .customer
            .as_ref()
            .and_then(|customer| customer.id.clone())
    } else {
        request.customer_id.clone()
    }
    .ok_or(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("subscriptions: unable to create a customer")?;

    // If provided we can strore plan_id, coupon_code etc as metadata
    let mut subscription = SubscriptionNew::new(
        id,
        SubscriptionStatus::Created.to_string(),
        None,
        None,
        request.merchant_connector_account_id,
        None,
        None,
        merchant_context.get_merchant_account().get_id().clone(),
        customer_id,
        None,
        request.profile_id,
    );
    response.client_secret = Some(subscription.generate_and_set_client_secret());
    db.insert_subscription_entry(subscription)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("subscriptions: unable to insert subscription entry to database")?;

    Ok(ApplicationResponse::Json(response))
}
