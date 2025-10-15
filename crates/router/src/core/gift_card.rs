use std::marker::PhantomData;

use api_models::payments::{
    GetPaymentMethodType, PaymentMethodBalanceCheckRequest, PaymentMethodBalanceCheckResponse,
};
use common_enums::CallConnectorAction;
use common_utils::{ext_traits::Encode, id_type, types::MinorUnit};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payments::HeaderPayload,
    router_data_v2::{flow_common_types::GiftCardBalanceCheckFlowData, RouterDataV2},
    router_flow_types::GiftCardBalanceCheck,
    router_request_types::GiftCardBalanceCheckRequestData,
    router_response_types::GiftCardBalanceCheckResponseData,
};
use hyperswitch_interfaces::connector_integration_interface::RouterDataConversion;
use masking::ExposeInterface;
use router_env::{instrument, tracing};

use crate::{
    consts,
    core::{
        errors::{self, RouterResponse},
        payments::helpers,
    },
    db::errors::StorageErrorExt,
    routes::{app::ReqState, SessionState},
    services,
    services::logger,
    types::{api, domain},
};

#[allow(clippy::too_many_arguments)]
pub async fn payments_check_gift_card_balance_core(
    state: SessionState,
    merchant_context: domain::MerchantContext,
    profile: domain::Profile,
    _req_state: ReqState,
    req: PaymentMethodBalanceCheckRequest,
    _header_payload: HeaderPayload,
    payment_id: id_type::GlobalPaymentId,
) -> RouterResponse<PaymentMethodBalanceCheckResponse> {
    let db = state.store.as_ref();

    let key_manager_state = &(&state).into();

    let storage_scheme = merchant_context.get_merchant_account().storage_scheme;
    let payment_intent = db
        .find_payment_intent_by_id(
            key_manager_state,
            &payment_id,
            merchant_context.get_merchant_key_store(),
            storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    let redis_conn = db
        .get_redis_conn()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Could not get redis connection")?;

    let gift_card_connector_id: String = redis_conn
        .get_key(&payment_id.get_gift_card_connector_key().as_str().into())
        .await
        .attach_printable("Failed to fetch gift card connector from redis")
        .change_context(errors::ApiErrorResponse::GenericNotFoundError {
            message: "No connector found with Gift Card Support".to_string(),
        })?;

    let gift_card_connector_id = id_type::MerchantConnectorAccountId::wrap(gift_card_connector_id)
        .attach_printable("Failed to deserialize MCA")
        .change_context(errors::ApiErrorResponse::GenericNotFoundError {
            message: "No connector found with Gift Card Support".to_string(),
        })?;

    let merchant_connector_account =
        domain::MerchantConnectorAccountTypeDetails::MerchantConnectorAccount(Box::new(
            helpers::get_merchant_connector_account_v2(
                &state,
                merchant_context.get_merchant_key_store(),
                Some(&gift_card_connector_id),
            )
            .await
            .attach_printable(
                "failed to fetch merchant connector account for gift card balance check",
            )?,
        ));

    let connector_name = merchant_connector_account
        .get_connector_name()
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Connector name not present for gift card balance check")?; // always get the connector name from this call

    let connector_data = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &connector_name.to_string(),
        api::GetToken::Connector,
        merchant_connector_account.get_mca_id(),
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to get the connector data")?;

    let connector_auth_type = merchant_connector_account
        .get_connector_account_details()
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let resource_common_data = GiftCardBalanceCheckFlowData;

    let api_models::payments::BalanceCheckPaymentMethodData::GiftCard(gift_card_data) =
        req.payment_method_data;

    let router_data: RouterDataV2<
        GiftCardBalanceCheck,
        GiftCardBalanceCheckFlowData,
        GiftCardBalanceCheckRequestData,
        GiftCardBalanceCheckResponseData,
    > = RouterDataV2 {
        flow: PhantomData,
        resource_common_data,
        tenant_id: state.tenant.tenant_id.clone(),
        connector_auth_type,
        request: GiftCardBalanceCheckRequestData {
            payment_method_data: domain::PaymentMethodData::GiftCard(Box::new(
                gift_card_data.clone().into(),
            )),
            currency: Some(payment_intent.amount_details.currency),
            minor_amount: Some(payment_intent.amount_details.order_amount),
        },
        response: Err(hyperswitch_domain_models::router_data::ErrorResponse::default()),
    };

    let old_router_data = GiftCardBalanceCheckFlowData::to_old_router_data(router_data)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(
            "Cannot construct router data for making the gift card balance check api call",
        )?;
    let connector_integration: services::BoxedGiftCardBalanceCheckIntegrationInterface<
        GiftCardBalanceCheck,
        GiftCardBalanceCheckRequestData,
        GiftCardBalanceCheckResponseData,
    > = connector_data.connector.get_connector_integration();

    let connector_response = services::execute_connector_processing_step(
        &state,
        connector_integration,
        &old_router_data,
        CallConnectorAction::Trigger,
        None,
        None,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed while calling gift card balance check connector api")?;

    let gift_card_balance = connector_response
        .response
        .map_err(|_| errors::ApiErrorResponse::UnprocessableEntity {
            message: "Error while fetching gift card balance".to_string(),
        })
        .attach_printable("Connector returned invalid response")?;

    let balance = gift_card_balance.balance;
    let currency = gift_card_balance.currency;

    let balance_data = domain::PaymentMethodBalanceData {
        payment_intent_id: &payment_intent.id,
        pm_balance_data: vec![(
            domain::PaymentMethodBalanceKey {
                payment_method_type: common_enums::PaymentMethod::GiftCard,
                payment_method_subtype: gift_card_data.get_payment_method_type(),
                payment_method_key: domain::GiftCardData::from(gift_card_data)
                    .get_payment_method_key()
                    .expose(),
            },
            domain::PaymentMethodBalanceValue { balance, currency },
        )]
        .into_iter()
        .collect(),
    };

    persist_individual_pm_balance_details_in_redis(&state, &profile, &balance_data)
        .await
        .attach_printable("Failed to persist gift card balance details in redis")?;

    let resp = PaymentMethodBalanceCheckResponse {
        payment_id: payment_intent.id.clone(),
        balance,
        currency,
    };

    Ok(services::ApplicationResponse::Json(resp))
}

#[instrument(skip_all)]
pub async fn persist_individual_pm_balance_details_in_redis<'a>(
    state: &SessionState,
    business_profile: &domain::Profile,
    pm_balance_data: &domain::PaymentMethodBalanceData<'_>,
) -> errors::RouterResult<()> {
    if !pm_balance_data.is_empty() {
        let redis_conn = state
            .store
            .get_redis_conn()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to get redis connection")?;
        let redis_key = pm_balance_data.get_pm_balance_redis_key();

        let value_list = pm_balance_data
            .get_individual_pm_balance_key_value_pairs()
            .into_iter()
            .map(|(key, value)| {
                value
                    .encode_to_string_of_json()
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to encode to string of json")
                    .map(|encoded_value| (key, encoded_value))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let intent_fulfillment_time = business_profile
            .get_order_fulfillment_time()
            .unwrap_or(consts::DEFAULT_FULFILLMENT_TIME);

        redis_conn
            .set_hash_fields(
                &redis_key.as_str().into(),
                value_list,
                Some(intent_fulfillment_time),
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to write to redis")?;
        logger::debug!("Surcharge results stored in redis with key = {}", redis_key);
    }
    Ok(())
}
