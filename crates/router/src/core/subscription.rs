use super::errors::{self, RouterResponse};
use crate::{core::payouts::helpers, routes::SessionState, services::api as service_api};
use api_models::subscription::{self as subscription_types, SUBSCRIPTION_ID_PREFIX};
use common_utils::{events::ApiEventMetric, generate_id_with_default_len, id_type};
use diesel_models::subscription::{SubscriptionNew, SubscriptionUpdate};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    merchant_context::MerchantContext, router_request_types::CustomerDetails,
};
use payment_methods::helpers::StorageErrorExt;
use std::str::FromStr;

pub async fn create_subscription(
    state: SessionState,
    merchant_context: MerchantContext,
    _authentication_profile_id: Option<common_utils::id_type::ProfileId>,
    request: subscription_types::CreateSubscriptionRequest,
) -> RouterResponse<CreateSubscriptionResponse> {
    let db = state.store.as_ref();
    // let key_manager_state = &(&state).into();

    let id = generate_id_with_default_len(SUBSCRIPTION_ID_PREFIX);

    // Fetch customer details from request and create new or else use existing customer that was attached
    let customer = get_customer_details_from_request(request.clone());
    let result_customer;
    let customer_id = if customer.customer_id.is_some()
        || customer.name.is_some()
        || customer.email.is_some()
        || customer.phone.is_some()
        || customer.phone_country_code.is_some()
    {
        result_customer =
            helpers::get_or_create_customer_details(&state, &customer, &merchant_context)
                .await
                .change_context(errors::ApiErrorResponse::CustomerRedacted)
                .attach_printable("Unable to retrieve or create customer")?
                .ok_or(errors::ApiErrorResponse::CustomerNotFound)?;
        result_customer.customer_id
    } else {
        request
            .customer_id
            .ok_or(errors::ApiErrorResponse::CustomerNotFound)?
    };

    // If provided we can strore plan_id, coupon_code etc as metadata
    // let metadata;
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
    let client_secret = subscription.generate_client_secret();
    let record = db
        .insert_subscription_entry(subscription)
        .await
        .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)?;

    let subscription_details = Subscription::new(record.id, "created", None);

    let result = CreateSubscriptionResponse::new(
        subscription_details,
        client_secret,
        merchant_context
            .get_merchant_account()
            .get_id()
            .get_string_repr(),
        None,
        None,
        customer,
        None,
    );

    Ok(service_api::ApplicationResponse::Json(result))
}

pub fn get_customer_details_from_request(
    request: subscription_types::CreateSubscriptionRequest,
) -> CustomerDetails {
    let customer_id = request.get_customer_id().map(ToOwned::to_owned);

    let customer_name = request
        .customer
        .as_ref()
        .and_then(|customer_details| customer_details.name.clone());

    let customer_email = request
        .customer
        .as_ref()
        .and_then(|customer_details| customer_details.email.clone());

    let customer_phone = request
        .customer
        .as_ref()
        .and_then(|customer_details| customer_details.phone.clone());

    let customer_phone_code = request
        .customer
        .as_ref()
        .and_then(|customer_details| customer_details.phone_country_code.clone());

    let tax_registration_id = request
        .customer
        .as_ref()
        .and_then(|customer_details| customer_details.tax_registration_id.clone());

    CustomerDetails {
        customer_id,
        name: customer_name,
        email: customer_email,
        phone: customer_phone,
        phone_country_code: customer_phone_code,
        tax_registration_id,
    }
}

pub async fn confirm_subscription(
    state: SessionState,
    merchant_context: MerchantContext,
    _authentication_profile_id: Option<common_utils::id_type::ProfileId>,
    _request: subscription_types::ConfirmSubscriptionRequest,
    subscription_id: String,
) -> RouterResponse<()> {
    let db = state.store.as_ref();
    // Fetch subscription from DB
    let subscription = db
        .find_subscription_by_id(subscription_id)
        .await
        .change_context(errors::ApiErrorResponse::GenericNotFoundError {
            message: "Subscription not found".to_string(),
        })?;

    let mercahnt_account = merchant_context.get_merchant_account();
    let key_store = merchant_context.get_merchant_key_store();
    let mca_id = subscription
        .mca_id
        .as_ref()
        .map(|id| id_type::MerchantConnectorAccountId::wrap(id.clone()))
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)?
        .ok_or(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: "No mca_id associated with this subscription".to_string(),
        })?;

    let billing_processor_mca = db
        .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
            &(&state).into(),
            &mercahnt_account.get_id(),
            &mca_id,
            key_store,
        )
        .await
        .change_context(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: mca_id.get_string_repr().to_string(),
        })?;

    let connector_name = billing_processor_mca.connector_name.clone();

    let connector_enum =
        common_enums::connector_enums::Connector::from_str(connector_name.as_str())
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Cannot find connector from the connector_name")?;

    let _connector_params =
        hyperswitch_domain_models::connector_endpoints::Connectors::get_connector_params(
            &state.conf.connectors,
            connector_enum,
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(format!(
            "cannot find connector params for this connector {connector_name} in this flow",
        ))?;

    // Create Subscription at billing processor
    // Update subscription with billing processor subscription_id
    let udpate = SubscriptionUpdate::new(Some("Some_id".to_string()), None);

    db.update_subscription_entry(subscription.id.clone(), udpate)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to update subscription with billing processor subscription_id")?;
    // Form Payments Request with billing processor details
    // Call Payments Core
    // Semd back response

    Ok(service_api::ApplicationResponse::Json(()))
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CreateSubscriptionResponse {
    pub subscription: Subscription,
    pub client_secret: String,
    pub merchant_id: String,
    pub plan_id: Option<String>,
    pub coupon_code: Option<String>,
    pub customer: CustomerDetails,
    pub invoice: Option<Invoice>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Subscription {
    pub id: String,
    pub status: String,
    pub plan_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Invoice {
    pub id: String,
    pub total: u64,
}

impl Subscription {
    pub fn new(id: impl Into<String>, status: impl Into<String>, plan_id: Option<String>) -> Self {
        Self {
            id: id.into(),
            status: status.into(),
            plan_id,
        }
    }
}

impl Invoice {
    pub fn new(id: impl Into<String>, total: u64) -> Self {
        Self {
            id: id.into(),
            total,
        }
    }
}
impl CreateSubscriptionResponse {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        subscription: Subscription,
        client_secret: impl Into<String>,
        merchant_id: impl Into<String>,
        plan_id: Option<String>,
        coupon_code: Option<String>,
        customer: CustomerDetails,
        invoice: Option<Invoice>,
    ) -> Self {
        Self {
            subscription,
            client_secret: client_secret.into(),
            merchant_id: merchant_id.into(),
            plan_id,
            coupon_code,
            customer,
            invoice,
        }
    }
}

impl ApiEventMetric for CreateSubscriptionResponse {}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ConfirmSubscriptionResponse {
    pub subscription: Subscription,
    pub customer_id: Option<common_utils::id_type::CustomerId>,
    pub invoice: Option<Invoice>,
}

impl ApiEventMetric for ConfirmSubscriptionResponse {}
