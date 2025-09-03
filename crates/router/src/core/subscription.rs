use super::errors::{self, RouterResponse};
use crate::{
    core::{payments as payments_core, payouts::helpers},
    routes::SessionState,
    services::{api as service_api, logger},
    types::api as api_types,
};
use api_models::{
    enums as api_enums,
    subscription::{self as subscription_types, SUBSCRIPTION_ID_PREFIX},
};
use common_types::payments::CustomerAcceptance;
use common_utils::ext_traits::ValueExt;
use common_utils::{events::ApiEventMetric, generate_id_with_default_len, id_type};
use diesel_models::subscription::{SubscriptionNew, SubscriptionUpdate};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    merchant_context::MerchantContext, router_request_types::CustomerDetails,
};
use payment_methods::helpers::StorageErrorExt;
use std::{num::NonZeroI64, str::FromStr};

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
    authentication_profile_id: Option<common_utils::id_type::ProfileId>,
    request: ConfirmSubscriptionRequest,
    subscription_id: String,
) -> RouterResponse<ConfirmSubscriptionResponse> {
    let db = state.store.as_ref();
    // Fetch subscription from DB
    let mercahnt_account = merchant_context.get_merchant_account();
    let key_store = merchant_context.get_merchant_key_store();
    let subscription = state
        .store
        .find_by_merchant_id_customer_id_subscription_id(
            merchant_id,
            &"cust_id".to_string(), // fix this
            subscription_id.clone(),
        )
        .await
        .change_context(errors::ApiErrorResponse::GenericNotFoundError {
            message: format!("subscription not found for id: {}", subscription_id),
        })?;

    logger::debug!("fetched_subscription: {:?}", subscription);

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

    let connector_data = api_types::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &connector_name,
        api_types::GetToken::Connector,
        Some(billing_processor_mca.get_id()),
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("invalid connector name received in billing merchant connector account")?;

    let connector_enum =
        common_enums::connector_enums::Connector::from_str(connector_name.as_str())
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Cannot find connector from the connector_name")?;

    let connector_params =
        hyperswitch_domain_models::connector_endpoints::Connectors::get_connector_params(
            &state.conf.connectors,
            connector_enum,
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(format!(
            "cannot find connector params for this connector {connector_name} in this flow",
        ))?;

    let connector_integration: service_api::BoxedSubscriptionConnectorIntegrationInterface<
        hyperswitch_domain_models::router_flow_types::subscriptions::SubscriptionCreate,
        hyperswitch_domain_models::router_request_types::subscriptions::SubscriptionCreateRequest,
        hyperswitch_domain_models::router_response_types::subscriptions::SubscriptionCreateResponse,
    > = connector_data.connector.get_connector_integration();

    // Create Subscription at billing processor
    let subscription_item =
        hyperswitch_domain_models::router_request_types::subscriptions::SubscriptionItem {
            item_price_id: request.item_price_id.ok_or(
                errors::ApiErrorResponse::InvalidRequestData {
                    message: "item_price_id is required".to_string(),
                },
            )?,
            quantity: Some(1),
        };

    let conn_request =
        hyperswitch_domain_models::router_request_types::subscriptions::SubscriptionCreateRequest {
            customer_id: subscription.customer_id.get_string_repr().to_string(),
            subscription_id: subscription.subscription_id.clone(),
            subscription_items: vec![subscription_item],
            billing_address: request.billing_address.clone().ok_or(
                errors::ApiErrorResponse::InvalidRequestData {
                    message: "Billing address is required".to_string(),
                },
            )?,
            auto_collection: "off".to_string(),
            connector_params,
        };

    logger::debug!("conn_request_customer: {:?}", conn_request.customer_id);

    let auth_type = payments_core::helpers::MerchantConnectorAccountType::DbVal(Box::new(
        billing_processor_mca.clone(),
    ))
    .get_connector_account_details()
    .parse_value("ConnectorAuthType")
    .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let router_data = payments_core::helpers::create_subscription_router_data::<
        hyperswitch_domain_models::router_flow_types::subscriptions::SubscriptionCreate,
        hyperswitch_domain_models::router_request_types::subscriptions::SubscriptionCreateRequest,
        hyperswitch_domain_models::router_response_types::subscriptions::SubscriptionCreateResponse,
    >(
        &state,
        subscription.merchant_id.to_owned(),
        Some(subscription.customer_id.to_owned()),
        connector_name,
        auth_type,
        conn_request,
        None,
    )?;

    let response = service_api::execute_connector_processing_step::<
        hyperswitch_domain_models::router_flow_types::subscriptions::SubscriptionCreate,
        _,
        hyperswitch_domain_models::router_request_types::subscriptions::SubscriptionCreateRequest,
        hyperswitch_domain_models::router_response_types::subscriptions::SubscriptionCreateResponse,
    >(
        &state,
        connector_integration,
        &router_data,
        common_enums::CallConnectorAction::Trigger,
        None,
        None,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed while handling response of subscription_create")?;

    let connector_resp = response.response.map_err(|err| {
        crate::logger::error!(?err);
        errors::ApiErrorResponse::InternalServerError
    })?;

    crate::logger::debug!("connector_resp: {:?}", connector_resp);

    // Form Payments Request with billing processor details
    let billing_connector_details = api_models::payments::BillingConnectorDetails {
        processor_mca: mca_id.to_owned(),
        subscription_id: connector_resp.subscription_id.clone(),
        invoice_id: connector_resp.invoice_id.clone(),
    };

    let mut payment_request = api_types::PaymentsRequest {
        amount: Some(api_types::Amount::Value(
            NonZeroI64::new(request.amount).unwrap(), // fix this
        )),
        currency: Some(request.currency),
        customer_id: Some(subscription.customer_id.to_owned()),
        merchant_id: Some(subscription.merchant_id.to_owned()),
        billing_processor_details: Some(billing_connector_details),
        confirm: Some(true),
        setup_future_usage: request.payment_data.setup_future_usage,
        payment_method: Some(request.payment_data.payment_method),
        payment_method_type: request.payment_data.payment_method_type,
        payment_method_data: Some(request.payment_data.payment_method_data),
        customer_acceptance: request.payment_data.customer_acceptance,
        ..Default::default()
    };

    if let Err(err) = crate::routes::payments::get_or_generate_payment_id(&mut payment_request) {
        return Err(err.into());
    }

    // Call Payments Core
    let payment_response = payments_core::payments_core::<
        api_types::Authorize,
        api_types::PaymentsResponse,
        _,
        _,
        _,
        payments_core::PaymentData<api_types::Authorize>,
    >(
        state.clone(),
        state.get_req_state(),
        merchant_context,
        authentication_profile_id,
        payments_core::PaymentCreate,
        payment_request,
        service_api::AuthFlow::Merchant,
        payments_core::CallConnectorAction::Trigger,
        None,
        hyperswitch_domain_models::payments::HeaderPayload::with_source(
            common_enums::PaymentSource::Webhook,
        ),
    )
    .await;

    // fix this error handling
    let payment_res = match payment_response {
        Ok(service_api::ApplicationResponse::JsonWithHeaders((pi, _))) => Ok(pi),
        Ok(_) => Err(errors::ApiErrorResponse::InternalServerError),
        Err(error) => {
            crate::logger::error!(?error);
            Err(errors::ApiErrorResponse::InternalServerError)
        }
    }?;
    // Semd back response
    let response = ConfirmSubscriptionResponse {
        subscription: Subscription::new(subscription.id.clone(), "active", None), // ?!?
        customer_id: Some(subscription.customer_id.to_owned()),
        invoice: None,
        payment: payment_res,
    };

    Ok(service_api::ApplicationResponse::Json(response))
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentData {
    pub payment_method: api_enums::PaymentMethod,
    pub payment_method_type: Option<api_enums::PaymentMethodType>,
    pub payment_method_data: api_models::payments::PaymentMethodDataRequest,
    pub setup_future_usage: Option<api_enums::FutureUsage>,
    pub customer_acceptance: Option<CustomerAcceptance>,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConfirmSubscriptionRequest {
    // pub client_secret: Option<String>,
    pub amount: i64,
    pub currency: api_enums::Currency,
    pub plan_id: Option<String>,
    pub item_price_id: Option<String>,
    pub coupon_code: Option<String>,
    pub customer: Option<api_models::payments::CustomerDetails>,
    pub billing_address: Option<api_models::payments::Address>,
    pub payment_data: PaymentData,
}

impl ApiEventMetric for ConfirmSubscriptionRequest {}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ConfirmSubscriptionResponse {
    pub subscription: Subscription,
    pub payment: api_models::payments::PaymentsResponse,
    pub customer_id: Option<common_utils::id_type::CustomerId>,
    pub invoice: Option<Invoice>,
}

impl ApiEventMetric for ConfirmSubscriptionResponse {}
