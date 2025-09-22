use std::str::FromStr;

use api_models::subscription::{
    self as subscription_types, CreateSubscriptionResponse, SubscriptionStatus,
};
use common_utils::{ext_traits::ValueExt, id_type::GenerateId};
use diesel_models::subscription::SubscriptionNew;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    api::ApplicationResponse, merchant_context::MerchantContext, router_data::ConnectorAuthType,
    subscription::ClientSecret,
};
use masking::Secret;

use super::{
    errors::{self, RouterResponse},
    utils::subscription as utils,
};
use crate::{
    core::utils::subscription::authenticate_subscription_client_secret_and_check_expiry,
    routes::SessionState,
};

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
    let db = state.store.as_ref();
    let key_store = merchant_context.get_merchant_key_store();
    let subscription_id = client_secret.get_subscription_id()?;

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

    authenticate_subscription_client_secret_and_check_expiry(
        &client_secret.to_string(),
        &subscription,
    )?;

    let mca_id = subscription.get_merchant_connector_id().change_context(
        errors::ApiErrorResponse::GenericNotFoundError {
            message: "merchant_connector_id not found".to_string(),
        },
    )?;

    let billing_processor_mca = db
        .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
            &(&state).into(),
            merchant_context.get_merchant_account().get_id(),
            mca_id,
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
    .attach_printable("Invalid connector name received in billing merchant connector account")?;

    let connector_integration_for_get_subscription_plans: crate::services::BoxedGetSubscriptionPlansInterface<
        hyperswitch_domain_models::router_flow_types::subscriptions::GetSubscriptionPlans,
        hyperswitch_domain_models::router_request_types::subscriptions::GetSubscriptionPlansRequest,
        hyperswitch_domain_models::router_response_types::subscriptions::GetSubscriptionPlansResponse,
        > = connector_data.connector.get_connector_integration();

    let get_plans_request =
        hyperswitch_domain_models::router_request_types::subscriptions::GetSubscriptionPlansRequest::default();

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
        .map(|plan| subscription_types::GetPlansResponse {
            plan_id: plan.subscription_provider_plan_id,
            name: plan.name,
            description: plan.description.unwrap_or_default(),
        })
        .collect();
    crate::logger::debug!("get_plans_response: {:?}", get_plans_response);
    Ok(ApplicationResponse::Json(plans))
}
