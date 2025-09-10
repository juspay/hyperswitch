pub mod utils;
use super::errors::{self, RouterResponse};
use crate::{
    core::payments as payments_core, db::errors::RouterResult, routes::SessionState,
    services as service_api, types::api as api_types,
};
use api_models::subscription::{
    self as subscription_types, CreateSubscriptionResponse, Subscription, SubscriptionStatus,
    SUBSCRIPTION_ID_PREFIX,
};
use common_utils::ext_traits::{BytesExt, ValueExt};
use common_utils::generate_id_with_default_len;
use diesel_models::subscription::SubscriptionNew;
use error_stack::ResultExt;
use hyperswitch_domain_models::{api::ApplicationResponse, merchant_context::MerchantContext};
use masking::ExposeInterface;
use payment_methods::helpers::StorageErrorExt;
use std::str::FromStr;
use utils::{get_customer_details_from_request, get_or_create_customer};

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
        request.mca_id.clone(),
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
        request
            .mca_id
            .map(|mca_id| {
                common_utils::id_type::MerchantConnectorAccountId::wrap(mca_id).change_context(
                    errors::ApiErrorResponse::InvalidRequestData {
                        message: "Invalid merchant_connector_account_id".to_string(),
                    },
                )
            })
            .transpose()?,
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
    authentication_profile_id: Option<common_utils::id_type::ProfileId>,
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
            &mercahnt_account.get_id(),
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

    // create_customer_at_billing_processor(
    //     &state,
    //     &request,
    //     &subscription,
    //     &connector_name,
    //     &auth_type,
    //     &connector_data,
    // )
    // .await
    // .change_context(errors::ApiErrorResponse::InternalServerError)
    // .attach_printable("Failed while creating customer at billing processor")?;

    // let create_subscription_connector_resp = create_subscription_at_billing_processor(
    //     &state,
    //     &request,
    //     &subscription,
    //     &connector_name,
    //     &auth_type,
    //     &connector_data,
    //     connector_params,
    // )
    // .await
    // .change_context(errors::ApiErrorResponse::InternalServerError)
    // .attach_printable("Failed while creating subscription at billing processor")?;

    let mut payment_request = subscription_types::SubscriptionPaymentsRequest {
        amount: Some(request.amount),
        currency: Some(request.currency.clone()),
        customer_id: Some(subscription.customer_id.clone()),
        merchant_id: Some(subscription.merchant_id.get_string_repr().to_string()),
        confirm: Some(true),
        payment_method_data: Some(request.payment_data.payment_method_data.clone()),
        setup_future_usage: request.payment_data.setup_future_usage,
        payment_id: Some(
            common_utils::id_type::PaymentId::default()
                .get_string_repr()
                .to_string(),
        ),
        payment_method: Some(request.payment_data.payment_method),
        payment_method_type: request.payment_data.payment_method_type,
        customer_acceptance: request.payment_data.customer_acceptance,
    };

    // if let Err(err) = crate::routes::payments::get_or_generate_payment_id(&mut payment_request) {
    //     return Err(err.into());
    // }

    crate::logger::debug!("payment_request: {:?}", (payment_request));

    // Call Payments Core
    // let payment_response = payments_core::payments_core::<
    //     api_types::Authorize,
    //     api_types::PaymentsResponse,
    //     _,
    //     _,
    //     _,
    //     payments_core::PaymentData<api_types::Authorize>,
    // >(
    //     state.clone(),
    //     state.get_req_state(),
    //     merchant_context,
    //     authentication_profile_id,
    //     payments_core::PaymentCreate,
    //     payment_request,
    //     service_api::AuthFlow::Merchant,
    //     payments_core::CallConnectorAction::Trigger,
    //     None,
    //     hyperswitch_domain_models::payments::HeaderPayload::with_source(
    //         common_enums::PaymentSource::Webhook,
    //     ),
    // )
    // .await;

    let payments_response: subscription_types::PaymentResponseData =
        build_and_send_payment_request(
            &state,
            service_api::Method::Post,
            "payments",
            payment_request,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while calling Payments API")?;
    // fix this error handling
    // let payment_res = match payment_response {
    //     Ok(service_api::ApplicationResponse::JsonWithHeaders((pi, _))) => Ok(pi),
    //     Ok(_) => Err(errors::ApiErrorResponse::InternalServerError),
    //     Err(error) => {
    //         crate::logger::error!(?error);
    //         Err(errors::ApiErrorResponse::InternalServerError)
    //     }
    // }?;

    // Create Invoice job entry

    let response = subscription_types::ConfirmSubscriptionResponse {
        subscription: Subscription::new(
            subscription.subscription_id.clone(),
            SubscriptionStatus::Active,
            None,
        ), // ?!?
        customer_id: Some(subscription.customer_id.to_owned()),
        invoice: None,
        payment: payments_response,
    };

    Ok(ApplicationResponse::Json(response))
}

async fn create_customer_at_billing_processor(
    state: &SessionState,
    request: &subscription_types::ConfirmSubscriptionRequest,
    subscription: &diesel_models::subscription::Subscription,
    connector_name: &String,
    auth_type: &hyperswitch_domain_models::router_data::ConnectorAuthType,
    connector_data: &api_types::ConnectorData,
) -> RouterResult<
    hyperswitch_domain_models::router_response_types::subscriptions::CreateCustomerResponse,
> {
    let connector_integration_for_create_customer: service_api::BoxedCreateCustomerConnectorIntegrationInterface<
            hyperswitch_domain_models::router_flow_types::subscriptions::CreateCustomer,
            hyperswitch_domain_models::router_request_types::subscriptions::CreateCustomerRequest,
            hyperswitch_domain_models::router_response_types::subscriptions::CreateCustomerResponse,
        > = connector_data.connector.get_connector_integration();

    let customer_req =
        hyperswitch_domain_models::router_request_types::subscriptions::CreateCustomerRequest {
            customer_id: subscription.customer_id.to_owned(),
            email: request
                .customer
                .as_ref()
                .and_then(|c| c.email.clone())
                .map(|email| email.expose())
                .map(|email| email.expose())
                .ok_or(errors::ApiErrorResponse::InvalidRequestData {
                    message: "Email is required to create customer".to_string(),
                })?,
            first_name: request
                .customer
                .as_ref()
                .and_then(|c| c.name.clone())
                .map(|name| name.expose())
                .ok_or(errors::ApiErrorResponse::InvalidRequestData {
                    message: "Name is required to create customer".to_string(),
                })?,
            last_name: request
                .customer
                .as_ref()
                .and_then(|c| c.name.clone())
                .map(|name| name.expose())
                .ok_or(errors::ApiErrorResponse::InvalidRequestData {
                    message: "Name is required to create customer".to_string(),
                })?, // Split name into first and last name if needed
            billing_address: None,
            locale: None,
        };

    let router_data = payments_core::helpers::create_subscription_router_data::<
        hyperswitch_domain_models::router_flow_types::subscriptions::CreateCustomer,
        hyperswitch_domain_models::router_request_types::subscriptions::CreateCustomerRequest,
        hyperswitch_domain_models::router_response_types::subscriptions::CreateCustomerResponse,
    >(
        state,
        subscription.merchant_id.to_owned(),
        Some(subscription.customer_id.to_owned()),
        connector_name.clone(),
        auth_type.clone(),
        customer_req,
        None,
    )?;
    let response = service_api::execute_connector_processing_step::<
        hyperswitch_domain_models::router_flow_types::subscriptions::CreateCustomer,
        _,
        hyperswitch_domain_models::router_request_types::subscriptions::CreateCustomerRequest,
        hyperswitch_domain_models::router_response_types::subscriptions::CreateCustomerResponse,
    >(
        state,
        connector_integration_for_create_customer,
        &router_data,
        common_enums::CallConnectorAction::Trigger,
        None,
        None,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed while handling response of subscription_create")?;

    let customer_create_connector_resp = response.response.map_err(|err| {
        crate::logger::error!(?err);
        errors::ApiErrorResponse::InternalServerError
    })?;

    crate::logger::debug!(
        "customer_create_connector_resp: {:?}",
        customer_create_connector_resp
    );
    Ok(customer_create_connector_resp)
}

async fn create_subscription_at_billing_processor(
    state: &SessionState,
    request: &subscription_types::ConfirmSubscriptionRequest,
    subscription: &diesel_models::subscription::Subscription,
    connector_name: &String,
    auth_type: &hyperswitch_domain_models::router_data::ConnectorAuthType,
    connector_data: &api_types::ConnectorData,
    connector_params: hyperswitch_domain_models::connector_endpoints::ConnectorParams,
) -> RouterResult<
    hyperswitch_domain_models::router_response_types::subscriptions::SubscriptionCreateResponse,
> {
    let connector_integration: service_api::BoxedSubscriptionConnectorIntegrationInterface<
        hyperswitch_domain_models::router_flow_types::subscriptions::SubscriptionCreate,
        hyperswitch_domain_models::router_request_types::subscriptions::SubscriptionCreateRequest,
        hyperswitch_domain_models::router_response_types::subscriptions::SubscriptionCreateResponse,
    > = connector_data.connector.get_connector_integration();

    // Create Subscription at billing processor
    let subscription_item =
        hyperswitch_domain_models::router_request_types::subscriptions::SubscriptionItem {
            item_price_id: request.item_price_id.clone().ok_or(
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

    crate::logger::debug!("conn_request_customer: {:?}", conn_request.customer_id);

    let router_data = payments_core::helpers::create_subscription_router_data::<
        hyperswitch_domain_models::router_flow_types::subscriptions::SubscriptionCreate,
        hyperswitch_domain_models::router_request_types::subscriptions::SubscriptionCreateRequest,
        hyperswitch_domain_models::router_response_types::subscriptions::SubscriptionCreateResponse,
    >(
        &state,
        subscription.merchant_id.to_owned(),
        Some(subscription.customer_id.to_owned()),
        connector_name.clone(),
        auth_type.clone(),
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

    Ok(connector_resp)
}

pub async fn build_and_send_payment_request<Req, Res>(
    state: &SessionState,
    http_method: service_api::Method,
    path: &str,
    request_body: Req,
) -> RouterResult<Res>
where
    Req: serde::Serialize + Send + Sync + 'static + Clone,
    Res: serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug + Clone + 'static,
{
    let base_url = &state.conf.open_router.url;
    let url = format!("{base_url}/{path}");

    let mut request_builder = service_api::RequestBuilder::new()
        .method(http_method)
        .url(&url);

    let body = common_utils::request::RequestContent::Json(Box::new(request_body));
    request_builder = request_builder.set_body(body);
    let http_request = request_builder.build();

    let response = service_api::call_connector_api(state, http_request, "Payments API call")
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let response = match response {
        Ok(resp) => {
            crate::logger::debug!("response from payments api: {:?}", resp);
            let resp: Res = resp
                .response
                .parse_struct(std::any::type_name::<Res>())
                .change_context(errors::ApiErrorResponse::InternalServerError)?;

            Ok(resp)
        }
        Err(err) => {
            crate::logger::error!(?err);
            return Err(errors::ApiErrorResponse::InternalServerError.into());
        }
    };

    response
}
