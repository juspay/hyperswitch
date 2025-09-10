pub mod utils;
use api_models::subscription::{
    self as subscription_types, CreateSubscriptionResponse, Subscription, SubscriptionStatus,
    SUBSCRIPTION_ID_PREFIX,
};
use common_utils::generate_id_with_default_len;
use diesel_models::subscription::SubscriptionNew;
use error_stack::ResultExt;
use hyperswitch_domain_models::{api::ApplicationResponse, merchant_context::MerchantContext};
use payment_methods::helpers::StorageErrorExt;
use utils::{get_customer_details_from_request, get_or_create_customer};

use super::errors::{self, RouterResponse};
use crate::{core::payments as payments_core, routes::SessionState, types::api as api_types};
use common_utils::ext_traits::ValueExt;
use std::str::FromStr;

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
    let subscription_details = Subscription::new(&id, SubscriptionStatus::Created, None);
    let mut response = CreateSubscriptionResponse::new(
        subscription_details,
        request.profile_id.clone(),
        merchant_context
            .get_merchant_account()
            .get_id()
            .get_string_repr(),
        request.merchant_connector_account_id.clone(),
    );

    let customer = get_customer_details_from_request(request.clone());
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
    .ok_or(errors::ApiErrorResponse::CustomerNotFound)
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
    response.client_secret = subscription.generate_and_set_client_secret();
    db.insert_subscription_entry(subscription)
        .await
        .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)
        .attach_printable("subscriptions: unable to insert subscription entry to database")?;

    Ok(ApplicationResponse::Json(response))
}

pub async fn confirm_subscription(
    state: SessionState,
    merchant_context: MerchantContext,
    _authentication_profile_id: Option<common_utils::id_type::ProfileId>,
    request: subscription_types::ConfirmSubscriptionRequest,
    subscription_id: String,
) -> RouterResponse<subscription_types::ConfirmSubscriptionResponse> {
    let db = state.store.as_ref();
    // Fetch subscription from DB
    let mercahnt_account = merchant_context.get_merchant_account();
    let key_store = merchant_context.get_merchant_key_store();
    let subscription = state
        .store
        .find_by_merchant_id_subscription_id(mercahnt_account.get_id(), subscription_id.clone())
        .await
        .change_context(errors::ApiErrorResponse::GenericNotFoundError {
            message: format!("subscription not found for id: {subscription_id}"),
        })?;

    let mca_id = subscription.merchant_connector_id.clone().ok_or(
        errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: "No mca_id associated with this subscription".to_string(),
        },
    )?;

    let billing_processor_mca = db
        .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
            &(&state).into(),
            mercahnt_account.get_id(),
            &mca_id,
            key_store,
        )
        .await
        .change_context(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: mca_id.get_string_repr().to_string(),
        })?;

    let connector_name = billing_processor_mca.connector_name.clone();

    let auth_type: hyperswitch_domain_models::router_data::ConnectorAuthType =
        payments_core::helpers::MerchantConnectorAccountType::DbVal(Box::new(
            billing_processor_mca.clone(),
        ))
        .get_connector_account_details()
        .parse_value("ConnectorAuthType")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

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

    // Create customer at billing processor
    // Create subscription at billing processor
    let create_subscription_connector_resp = create_subscription_at_billing_processor(
        &state,
        &request,
        &subscription,
        &connector_name,
        &auth_type,
        &connector_data,
        connector_params,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed while creating subscription at billing processor")?;
    // Create Invoice DB record
    // Create CIT payment
    // Create Invoice job entry based on payment status
    // Update Invoice DB record accordingly

    let response = subscription_types::ConfirmSubscriptionResponse {
        subscription: Subscription::new(
            subscription.subscription_id.clone(),
            SubscriptionStatus::get_status_from_connector_status(
                &create_subscription_connector_resp.status,
            ),
            None,
        ), // ?!?
        customer_id: Some(subscription.customer_id.to_owned()),
        invoice: None,
        payment: None,
    };

    Ok(ApplicationResponse::Json(response))
}

async fn create_subscription_at_billing_processor(
    _state: &SessionState,
    _request: &subscription_types::ConfirmSubscriptionRequest,
    _subscription: &diesel_models::subscription::Subscription,
    _connector_name: &str,
    _auth_type: &hyperswitch_domain_models::router_data::ConnectorAuthType,
    _connector_data: &api_types::ConnectorData,
    _connector_params: hyperswitch_domain_models::connector_endpoints::ConnectorParams,
) -> errors::RouterResult<subscription_types::SubscriptionCreateResponse> {
    Ok(subscription_types::SubscriptionCreateResponse::default())
}
