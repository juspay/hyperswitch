#[cfg(feature = "v2")]
use common_utils::id_type;
#[cfg(feature = "v2")]
use hyperswitch_domain_models::payments::HeaderPayload;

#[cfg(feature = "v2")]
use crate::{
    core::{
        errors::{self, RouterResponse},
        payments::helpers,
    },
    routes::{app::ReqState, SessionState},
    services,
    types::{api, domain},
};

use common_utils::types::MinorUnit;

use std::marker::PhantomData;

use common_enums::CallConnectorAction;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    router_data_v2::{flow_common_types::GiftCardBalanceCheckFlowData, RouterDataV2},
    router_flow_types::GiftCardBalanceCheck,
    router_request_types::GiftCardBalanceCheckRequestData,
    router_response_types::GiftCardBalanceCheckResponseData,
};

use hyperswitch_interfaces::connector_integration_interface::RouterDataConversion;

use crate::db::errors::StorageErrorExt;

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
pub async fn payments_check_gift_card_balance_core(
    state: SessionState,
    merchant_context: domain::MerchantContext,
    profile: domain::Profile,
    _req_state: ReqState,
    req: PaymentsGiftCardBalanceCheckRequest,
    _header_payload: HeaderPayload,
    payment_id: id_type::GlobalPaymentId,
) -> RouterResponse<GiftCardBalanceCheckResponse> {
    use api_models::payments::GiftCardBalanceCheckResponse;

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

    let payment_connector_accounts = db
        .list_enabled_connector_accounts_by_profile_id(
            key_manager_state,
            profile.get_id(),
            merchant_context.get_merchant_key_store(),
            common_enums::ConnectorType::PaymentProcessor,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("error when fetching merchant connector accounts")?;

    let gift_card_connector_id = payment_connector_accounts
        .iter()
        .find_map(|account| {
            account
                .payment_methods_enabled
                .as_ref()?
                .iter()
                .find(|payment_method| {
                    payment_method.payment_method_type == common_enums::PaymentMethod::GiftCard
                })
                .map(|_| account.get_id().clone())
        })
        .ok_or(errors::ApiErrorResponse::GenericNotFoundError {
            message: "No MCA found with Gift Card Support".to_string(),
        })
        .attach_printable("No configured MCA supports Gift Card")?;

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
                req.gift_card_data.into(),
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

    let payment_method_balance = connector_response
        .payment_method_balance
        .ok_or(errors::ApiErrorResponse::UnprocessableEntity {
            message: "Payment Method Balance cannot be None".to_string(),
        })
        .attach_printable("Payment Method Balance cannot be None")?;

    let balance = payment_method_balance.amount;
    let currency = payment_method_balance.currency;
    let remaining_amount =
        if (payment_intent.amount_details.order_amount - balance).is_greater_than(0) {
            payment_intent.amount_details.order_amount - balance
        } else {
            MinorUnit::zero()
        };
    let needs_additional_pm_data = remaining_amount.is_greater_than(0);

    let resp = GiftCardBalanceCheckResponse {
        payment_id: payment_intent.id.clone(),
        balance,
        currency,
        needs_additional_pm_data,
        remaining_amount,
    };

    Ok(services::ApplicationResponse::Json(resp))
}
