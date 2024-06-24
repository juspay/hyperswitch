use crate::{
    configs::settings,
    core::{errors, payments::helpers},
    types::{self, domain, storage, PaymentAddress},
};
use api_models::{enums, payment_methods::CardDetailUpdate};
use common_utils::id_type;
use common_utils::{errors::CustomResult, ext_traits::ValueExt};
use error_stack::ResultExt;
use std::marker::PhantomData;
const IRRELEVANT_PAYMENT_ID_IN_MANDATE_UPDATE_FLOW: &str =
    "irrelevant_payment_id_in_mandate_update_flow";

const IRRELEVANT_ATTEMPT_ID_IN_MANDATE_UPDATE_FLOW: &str =
    "irrelevant_attempt_id_in_mandate_update_flow";

const IRRELEVANT_CONNECTOR_REQUEST_REFERENCE_ID_IN_MANDATE_UPDATE_FLOW: &str =
    "irrelevant_connector_request_reference_id_in_mandate_update_flow";

pub fn is_conector_update_possible(
    supported_payment_methods_for_mandate: &settings::SupportedPaymentMethodsForMandate,
    payment_method: Option<&enums::PaymentMethod>,
    payment_method_type: Option<&enums::PaymentMethodType>,
    connector: enums::Connector,
) -> bool {
    payment_method
        .map(|payment_method| {
            if payment_method == &enums::PaymentMethod::Card {
                supported_payment_methods_for_mandate
                    .0
                    .get(payment_method)
                    .map(|payment_method_type_hm| {
                        let pm_credit = payment_method_type_hm
                            .0
                            .get(&enums::PaymentMethodType::Credit)
                            .map(|conn| conn.connector_list.clone())
                            .unwrap_or_default();
                        let pm_debit = payment_method_type_hm
                            .0
                            .get(&enums::PaymentMethodType::Debit)
                            .map(|conn| conn.connector_list.clone())
                            .unwrap_or_default();
                        &pm_credit | &pm_debit
                    })
                    .map(|supported_connectors| supported_connectors.contains(&connector))
                    .unwrap_or(false)
            } else if let Some(payment_method_type) = payment_method_type {
                supported_payment_methods_for_mandate
                    .0
                    .get(payment_method)
                    .and_then(|payment_method_type_hm| {
                        payment_method_type_hm.0.get(payment_method_type)
                    })
                    .map(|supported_connectors| {
                        supported_connectors.connector_list.contains(&connector)
                    })
                    .unwrap_or(false)
            } else {
                false
            }
        })
        .unwrap_or(false)
}

// use std::marker::PhantomData;

// use common_utils::{errors::CustomResult, ext_traits::ValueExt};
// use diesel_models::Mandate;
// use error_stack::ResultExt;

// use crate::{
//     core::{errors, payments::helpers},
//     types::{self, domain, PaymentAddress},
// };

pub async fn construct_mandate_update_router_data(
    merchant_connector_account: helpers::MerchantConnectorAccountType,
    merchant_account: &domain::MerchantAccount,
    update_mandate: storage::UpdateMandate,
    updation_obj: CardDetailUpdate,
    customer_id: id_type::CustomerId,
) -> CustomResult<types::UpdateMandateDetailsRouterData, errors::ApiErrorResponse> {
    let auth_type = merchant_connector_account
        .get_connector_account_details()
        .parse_value::<types::ConnectorAuthType>("ConnectorAuthType")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    let router_data = types::RouterData {
        flow: PhantomData,
        merchant_id: merchant_account.merchant_id.clone(),
        customer_id: Some(customer_id),
        connector_customer: None,
        connector: update_mandate.connector_variant.to_string(),
        payment_id: IRRELEVANT_PAYMENT_ID_IN_MANDATE_UPDATE_FLOW.to_string(),
        attempt_id: IRRELEVANT_ATTEMPT_ID_IN_MANDATE_UPDATE_FLOW.to_string(),
        status: diesel_models::enums::AttemptStatus::default(),
        payment_method: diesel_models::enums::PaymentMethod::default(),
        connector_auth_type: auth_type,
        description: None,
        return_url: None,
        address: PaymentAddress::default(),
        auth_type: diesel_models::enums::AuthenticationType::default(),
        connector_meta_data: None,
        connector_wallets_details: None,
        amount_captured: None,
        minor_amount_captured: None,
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_token: None,
        recurring_mandate_payment_data: None,
        preprocessing_id: None,
        payment_method_balance: None,
        connector_api_version: None,
        payment_method_status: None,
        request: types::MandateDetailsUpdateData {
            updation_obj,
            connector_mandate_id: update_mandate.connector_mandate_id,
        },
        response: Err(types::ErrorResponse::get_not_implemented()),
        connector_request_reference_id:
            IRRELEVANT_CONNECTOR_REQUEST_REFERENCE_ID_IN_MANDATE_UPDATE_FLOW.to_string(),
        test_mode: None,
        connector_http_status_code: None,
        external_latency: None,
        apple_pay_flow: None,
        frm_metadata: None,
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        refund_id: None,
        dispute_id: None,
        connector_response: None,
    };

    Ok(router_data)
}
