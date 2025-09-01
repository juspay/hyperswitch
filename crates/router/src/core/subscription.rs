pub mod utils;
use api_models::payments::CustomerDetailsResponse;
use common_utils::generate_id_with_default_len;
use diesel_models::subscription::SubscriptionNew;
use error_stack::ResultExt;
use hyperswitch_domain_models::merchant_context::MerchantContext;
use payment_methods::helpers::StorageErrorExt;
use utils::{
    self as subscription_types, get_customer_details_from_request, get_or_create_customer,
    CreateSubscriptionResponse, Subscription, SubscriptionStatus, SUBSCRIPTION_ID_PREFIX,
};

use super::errors::{self, RouterResponse};
use crate::{routes::SessionState, services::api as service_api, types::transformers::ForeignInto};

pub async fn create_subscription(
    state: SessionState,
    merchant_context: MerchantContext,
    request: subscription_types::CreateSubscriptionRequest,
) -> RouterResponse<CreateSubscriptionResponse> {
    let db = state.store.as_ref();
    let id = generate_id_with_default_len(SUBSCRIPTION_ID_PREFIX);
    let subscription_details = Subscription::new(&id, SubscriptionStatus::Created, None);
    let mut response = CreateSubscriptionResponse::new(
        subscription_details,
        merchant_context
            .get_merchant_account()
            .get_id()
            .get_string_repr(),
        request.mca_id.clone(),
    );

    let customer = get_customer_details_from_request(request.clone());
    let customer_id = if customer.customer_id.is_some()
        || customer.name.is_some()
        || customer.email.is_some()
        || customer.phone.is_some()
        || customer.phone_country_code.is_some()
    {
        let customer = get_or_create_customer(&state, &customer, &merchant_context).await?;
        let customer_table_response: Option<CustomerDetailsResponse> =
            customer.as_ref().map(ForeignInto::foreign_into);
        response.customer = customer_table_response;
        response
            .customer
            .as_ref()
            .and_then(|customer| customer.id.clone())
    } else {
        request.customer_id.clone()
    }
    .ok_or(errors::ApiErrorResponse::CustomerNotFound)
    .attach_printable("subscriptions: unable to create a customer")?;

    // If provided we can strore plan_id, coupon_code etc as metadata
    let subscription = SubscriptionNew::new(
        id,
        None,
        None,
        None,
        request.mca_id,
        None,
        customer_id,
        merchant_context.get_merchant_account().get_id().clone(),
        None,
    );
    response.client_secret = subscription.generate_client_secret();
    db.insert_subscription_entry(subscription)
        .await
        .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)
        .attach_printable("subscriptions: unable to insert subscription entry to database")?;

    Ok(service_api::ApplicationResponse::Json(response))
}
