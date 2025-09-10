pub mod utils;
use api_models::subscription::{
    self as subscription_types, CreateSubscriptionResponse, Subscription, SubscriptionStatus,
    SUBSCRIPTION_ID_PREFIX,
};
use common_utils::{ext_traits::ValueExt, generate_id_with_default_len};
use diesel_models::subscription::SubscriptionNew;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    api::ApplicationResponse, merchant_context::MerchantContext, router_data::ConnectorAuthType,
};
use payment_methods::helpers::StorageErrorExt;
use utils::{
    authenticate_subscription_client_secret_and_check_expiry, get_customer_details_from_request,
    get_or_create_customer,
};

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

pub async fn get_subscription_plans(
    state: SessionState,
    merchant_context: MerchantContext,
    _authentication_profile_id: Option<common_utils::id_type::ProfileId>,
    client_secret: String,
) -> RouterResponse<Vec<subscription_types::GetPlansResponse>> {
    let db = state.store.as_ref();
    let key_store = merchant_context.get_merchant_key_store();
    let sub_vec = client_secret.split("_secret").collect::<Vec<&str>>();
    let subscription_id =
        sub_vec
            .first()
            .ok_or(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "client_secret",
            })?;

    let subscription = db
        .find_by_merchant_id_subscription_id(
            merchant_context.get_merchant_account().get_id(),
            subscription_id.to_string(),
        )
        .await
        .change_context(errors::ApiErrorResponse::GenericNotFoundError {
            message: "Subscription not found".to_string(),
        })
        .attach_printable("Unable to find subscription")?;

    authenticate_subscription_client_secret_and_check_expiry(&client_secret, &subscription)?;

    let mca_id = subscription.merchant_connector_id.ok_or(
        errors::ApiErrorResponse::GenericNotFoundError {
            message: "merchant_connector_id not found".to_string(),
        },
    )?;

    let billing_processor_mca = db
        .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
            &(&state).into(),
            merchant_context.get_merchant_account().get_id(),
            &mca_id,
            key_store,
        )
        .await
        .change_context(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: mca_id.get_string_repr().to_string(),
        })?;

    let auth_type: ConnectorAuthType =
        super::payments::helpers::MerchantConnectorAccountType::DbVal(Box::new(
            billing_processor_mca.clone(),
        ))
        .get_connector_account_details()
        .parse_value("ConnectorAuthType")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let connector = &billing_processor_mca.connector_name;

    let connector_data = crate::types::api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &billing_processor_mca.connector_name,
        crate::types::api::GetToken::Connector,
        Some(billing_processor_mca.get_id()),
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("invalid connector name received in billing merchant connector account")?;

    let connector_integration_for_get_subscription_plans: crate::services::BoxedGetSubscriptionPlansInterface<
        hyperswitch_domain_models::router_flow_types::subscriptions::GetSubscriptionPlans,
        hyperswitch_domain_models::router_request_types::subscriptions::GetSubscriptionPlansRequest,
        hyperswitch_domain_models::router_response_types::subscriptions::GetSubscriptionPlansResponse,
        > = connector_data.connector.get_connector_integration();

    let get_plans_request =
        hyperswitch_domain_models::router_request_types::subscriptions::GetSubscriptionPlansRequest;
    let payment_id = common_utils::id_type::PaymentId::wrap("NA".to_string())
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to nullify payment_id")?;

    let router_data = utils::create_subscription_router_data::<
        hyperswitch_domain_models::router_flow_types::subscriptions::GetSubscriptionPlans,
        hyperswitch_domain_models::router_request_types::subscriptions::GetSubscriptionPlansRequest,
        hyperswitch_domain_models::router_response_types::subscriptions::GetSubscriptionPlansResponse,
    >(
        &state,
        subscription.merchant_id.to_owned(),
        Some(subscription.customer_id.to_owned()),
        connector.clone(),
        auth_type.clone(),
        get_plans_request,
        payment_id
    )?;

    let response = crate::services::api::execute_connector_processing_step::<
        hyperswitch_domain_models::router_flow_types::subscriptions::GetSubscriptionPlans,
        _,
        hyperswitch_domain_models::router_request_types::subscriptions::GetSubscriptionPlansRequest,
        hyperswitch_domain_models::router_response_types::subscriptions::GetSubscriptionPlansResponse,
    >(
        &state,
        connector_integration_for_get_subscription_plans,
        &router_data,
        common_enums::CallConnectorAction::Trigger,
        None,
        None,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed while handling response of subscription_create")?;

    let get_plans_response = response.response.map_err(|err| {
        crate::logger::error!(?err);
        errors::ApiErrorResponse::InternalServerError
    })?;

    let plans: Vec<subscription_types::GetPlansResponse> = get_plans_response
        .list
        .clone()
        .into_iter()
        .map(|p| subscription_types::GetPlansResponse {
            plan_id: p.subscription_provider_plan_id,
            name: p.name,
            description: p.description.unwrap_or_default(),
        })
        .collect();
    crate::logger::debug!("get_plans_response: {:?}", get_plans_response);
    Ok(ApplicationResponse::Json(plans))
}
