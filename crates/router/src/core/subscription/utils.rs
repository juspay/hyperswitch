use api_models::{customers::CustomerRequest, subscription::CreateSubscriptionRequest};
use common_utils::{ext_traits::OptionExt, id_type::GenerateId};
use diesel_models::subscription::Subscription;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    api::ApplicationResponse, merchant_context::MerchantContext,
    router_request_types::CustomerDetails,
};
use router_env::instrument;

use crate::{
    consts,
    core::customers::create_customer,
    db::{errors, StorageInterface},
    routes::SessionState,
    types::{api::CustomerResponse, transformers::ForeignInto},
};

pub async fn get_or_create_customer(
    state: SessionState,
    customer_request: Option<CustomerRequest>,
    merchant_context: MerchantContext,
) -> errors::CustomerResponse<CustomerResponse> {
    let db: &dyn StorageInterface = &*state.store;

    // Create customer_id if not passed in request
    let customer_id = customer_request
        .as_ref()
        .and_then(|c| c.customer_id.clone())
        .unwrap_or_else(common_utils::id_type::CustomerId::generate);

    let merchant_id = merchant_context.get_merchant_account().get_id();
    let key_manager_state = &(&state).into();

    match db
        .find_customer_optional_by_customer_id_merchant_id(
            key_manager_state,
            &customer_id,
            merchant_id,
            merchant_context.get_merchant_key_store(),
            merchant_context.get_merchant_account().storage_scheme,
        )
        .await
        .change_context(errors::CustomersErrorResponse::InternalServerError)
        .attach_printable("subscription: unable to perform db read query")?
    {
        // Customer found
        Some(customer) => {
            let api_customer: CustomerResponse =
                (customer, None::<api_models::payments::AddressDetails>).foreign_into();
            Ok(ApplicationResponse::Json(api_customer))
        }

        // Customer not found
        None => Ok(create_customer(
            state,
            merchant_context,
            customer_request.ok_or(errors::CustomersErrorResponse::CustomerNotFound)?,
            None,
        )
        .await?),
    }
}

pub fn get_customer_details_from_request(request: CreateSubscriptionRequest) -> CustomerDetails {
    let customer_id = request.get_customer_id().map(ToOwned::to_owned);
    let customer = request.customer.as_ref();
    CustomerDetails {
        customer_id,
        name: customer.and_then(|cus| cus.name.clone()),
        email: customer.and_then(|cus| cus.email.clone()),
        phone: customer.and_then(|cus| cus.phone.clone()),
        phone_country_code: customer.and_then(|cus| cus.phone_country_code.clone()),
        tax_registration_id: customer.and_then(|cus| cus.tax_registration_id.clone()),
    }
}

#[instrument(skip_all)]
pub fn authenticate_subscription_client_secret_and_check_expiry(
    req_client_secret: &String,
    subscription: &Subscription,
) -> errors::CustomResult<bool, errors::ApiErrorResponse> {
    let stored_client_secret = subscription
        .client_secret
        .clone()
        .get_required_value("client_secret")
        .change_context(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "client_secret",
        })
        .attach_printable("client secret not found in db")?;

    if req_client_secret != &stored_client_secret {
        Err((errors::ApiErrorResponse::ClientSecretInvalid).into())
    } else {
        let current_timestamp = common_utils::date_time::now();
        let session_expiry = subscription
            .created_at
            .saturating_add(time::Duration::seconds(consts::DEFAULT_SESSION_EXPIRY));

        let expired = current_timestamp > session_expiry;
        Ok(expired)
    }
}
