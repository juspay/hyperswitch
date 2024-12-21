use std::str::FromStr;

use api_models::relay as relay_models;
use common_utils::{
    self,
    ext_traits::OptionExt,
    id_type::{self, GenerateId},
};
use error_stack::ResultExt;
use hyperswitch_domain_models::types;

use super::errors::{self, ConnectorErrorExt, RouterResponse, RouterResult, StorageErrorExt};
use crate::{
    core::payments,
    routes::SessionState,
    services,
    types::{
        api::{self},
        domain,
        transformers::ForeignFrom,
    },
};

const IRRELEVANT_PAYMENT_INTENT_ID: &str = "irrelevant_payment_intent_id";

const IRRELEVANT_PAYMENT_ATTEMPT_ID: &str = "irrelevant_payment_attempt_id";

pub async fn relay(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    profile_id_optional: Option<id_type::ProfileId>,
    key_store: domain::MerchantKeyStore,
    req: relay_models::RelayRequest,
) -> RouterResponse<relay_models::RelayResponse> {
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();
    let merchant_id = merchant_account.get_id();

    let profile_id_from_auth_layer = profile_id_optional
        .get_required_value("ProfileId")
        .change_context(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "profile id",
        })?;

    let profile = db
        .find_business_profile_by_merchant_id_profile_id(
            key_manager_state,
            &key_store,
            merchant_id,
            &profile_id_from_auth_layer,
        )
        .await
        .change_context(errors::ApiErrorResponse::ProfileNotFound {
            id: profile_id_from_auth_layer.get_string_repr().to_owned(),
        })?;

    let relay_response = match req.relay_type {
        common_enums::RelayType::Refund => {
            Box::pin(relay_refund(
                state,
                merchant_account,
                profile,
                key_store,
                &req,
            ))
            .await?
        }
    };
    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        relay_response,
    ))
}

pub async fn relay_refund(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    profile: domain::Profile,
    key_store: domain::MerchantKeyStore,
    req: &relay_models::RelayRequest,
) -> RouterResult<relay_models::RelayResponse> {
    validate_relay_refund_request(req).attach_printable("Invalid relay refund request")?;

    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();
    let connector_id = &req.connector_id;

    let merchant_id = merchant_account.get_id();

    let connector_account = db
        .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
            key_manager_state,
            merchant_account.get_id(),
            connector_id,
            &key_store,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: connector_id.get_string_repr().to_string(),
        })?;

    let connector_data = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &connector_account.connector_name,
        api::GetToken::Connector,
        Some(connector_id.clone()),
    )?;

    let connector_integration: services::BoxedRefundConnectorIntegrationInterface<
        api::Execute,
        hyperswitch_domain_models::router_request_types::RefundsData,
        hyperswitch_domain_models::router_response_types::RefundsResponseData,
    > = connector_data.connector.get_connector_integration();

    let relay_id = id_type::RelayId::generate();

    let relay_domain =
        get_relay_domain_model(req, merchant_account.get_id(), profile.get_id(), &relay_id);

    let relay_record = db
        .insert_relay(key_manager_state, &key_store, relay_domain)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let router_data = construct_relay_refund_router_data(
        &state,
        &connector_account.connector_name,
        merchant_id,
        &connector_account,
        &relay_record,
    )
    .await?;

    let router_data_res = services::execute_connector_processing_step(
        &state,
        connector_integration,
        &router_data,
        payments::CallConnectorAction::Trigger,
        None,
    )
    .await
    .to_refund_failed_response()?;

    let relay_response = match router_data_res.response {
        Err(error) => hyperswitch_domain_models::relay::RelayUpdate::ErrorUpdate {
            error_code: error.code,
            error_reason: error.message,
            status: common_enums::RelayStatus::Failure,
        },
        Ok(response) => hyperswitch_domain_models::relay::RelayUpdate::StatusUpdate {
            connector_reference_id: Some(response.connector_refund_id),
            status: common_enums::RelayStatus::from(response.refund_status),
        },
    };

    let relay_update = db
        .update_relay(key_manager_state, &key_store, relay_record, relay_response)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let response = relay_models::RelayResponse::from(relay_update);

    Ok(response)
}

pub async fn construct_relay_refund_router_data<'a, F>(
    state: &'a SessionState,
    connector_name: &str,
    merchant_id: &id_type::MerchantId,
    connector_account: &domain::MerchantConnectorAccount,
    relay_record: &hyperswitch_domain_models::relay::Relay,
) -> RouterResult<types::RefundsRouterData<F>> {
    let connector_auth_type = connector_account
        .get_connector_account_details()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while parsing value for ConnectorAuthType")?;

    let webhook_url = Some(payments::helpers::create_webhook_url(
        &state.base_url.clone(),
        merchant_id,
        connector_name,
    ));

    let supported_connector = &state
        .conf
        .multiple_api_version_supported_connectors
        .supported_connectors;

    let connector_enum = api_models::enums::Connector::from_str(connector_name)
        .change_context(errors::ConnectorError::InvalidConnectorName)
        .change_context(errors::ApiErrorResponse::InvalidDataValue {
            field_name: "connector",
        })
        .attach_printable_lazy(|| format!("unable to parse connector name {connector_name:?}"))?;

    let connector_api_version = if supported_connector.contains(&connector_enum) {
        state
            .store
            .find_config_by_key(&format!("connector_api_version_{connector_name}"))
            .await
            .map(|value| value.config)
            .ok()
    } else {
        None
    };

    let hyperswitch_domain_models::relay::RelayData::Refund(relay_refund_data) = relay_record
        .request_data
        .clone()
        .get_required_value("refund relay data")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let relay_id_string = relay_record.id.get_string_repr().to_string();

    let router_data = hyperswitch_domain_models::router_data::RouterData {
        flow: std::marker::PhantomData,
        merchant_id: merchant_id.clone(),
        customer_id: None,
        connector: connector_name.to_string(),
        payment_id: IRRELEVANT_PAYMENT_INTENT_ID.to_string(),
        attempt_id: IRRELEVANT_PAYMENT_ATTEMPT_ID.to_string(),
        status: common_enums::AttemptStatus::Charged,
        payment_method: common_enums::PaymentMethod::default(),
        connector_auth_type,
        description: None,
        return_url: None,
        address: hyperswitch_domain_models::payment_address::PaymentAddress::default(),
        auth_type: common_enums::AuthenticationType::default(),
        connector_meta_data: None,
        connector_wallets_details: None,
        amount_captured: None,
        payment_method_status: None,
        minor_amount_captured: None,
        request: hyperswitch_domain_models::router_request_types::RefundsData {
            refund_id: relay_id_string.clone(),
            connector_transaction_id: relay_record.connector_resource_id.clone(),
            refund_amount: relay_refund_data.amount.get_amount_as_i64(),
            minor_refund_amount: relay_refund_data.amount,
            currency: relay_refund_data.currency,
            payment_amount: relay_refund_data.amount.get_amount_as_i64(),
            minor_payment_amount: relay_refund_data.amount,
            webhook_url,
            connector_metadata: None,
            reason: relay_refund_data.reason,
            connector_refund_id: relay_record.connector_reference_id.clone(),
            browser_info: None,
            split_refunds: None,
            integrity_object: None,
            refund_status: common_enums::RefundStatus::from(relay_record.status),
        },

        response: Ok(
            hyperswitch_domain_models::router_response_types::RefundsResponseData {
                connector_refund_id: relay_record.connector_resource_id.clone(),
                refund_status: common_enums::RefundStatus::default(),
            },
        ),
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_token: None,
        connector_customer: None,
        recurring_mandate_payment_data: None,
        preprocessing_id: None,
        connector_request_reference_id: relay_id_string.clone(),
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        test_mode: connector_account.test_mode,
        payment_method_balance: None,
        connector_api_version,
        connector_http_status_code: None,
        external_latency: None,
        apple_pay_flow: None,
        frm_metadata: None,
        refund_id: Some(relay_id_string),
        dispute_id: None,
        connector_response: None,
        integrity_check: Ok(()),
        additional_merchant_data: None,
        header_payload: None,
        connector_mandate_request_reference_id: None,
        authentication_id: None,
        psd2_sca_exemption_type: None,
    };

    Ok(router_data)
}

// validate relay request
pub fn validate_relay_refund_request(
    relay_request: &relay_models::RelayRequest,
) -> RouterResult<()> {
    match (relay_request.relay_type, &relay_request.data) {
        (common_enums::RelayType::Refund, Some(relay_models::RelayData::Refund(ref_data))) => {
            validate_relay_refund_data(ref_data)
        }
        (common_enums::RelayType::Refund, None) => {
            Err(errors::ApiErrorResponse::PreconditionFailed {
                message: "Relay data is required for refund relay".to_string(),
            })?
        }
    }
}

pub fn validate_relay_refund_data(
    refund_data: &relay_models::RelayRefundRequest,
) -> RouterResult<()> {
    if refund_data.amount.get_amount_as_i64().is_positive() {
        Err(errors::ApiErrorResponse::PreconditionFailed {
            message: "Amount should be greater than 0".to_string(),
        })?
    }
    Ok(())
}

pub fn get_relay_domain_model(
    relay_request: &relay_models::RelayRequest,
    merchant_id: &id_type::MerchantId,
    profile_id: &id_type::ProfileId,
    relay_id: &id_type::RelayId,
) -> hyperswitch_domain_models::relay::Relay {
    hyperswitch_domain_models::relay::Relay {
        id: relay_id.clone(),
        connector_resource_id: relay_request.connector_resource_id.clone(),
        connector_id: relay_request.connector_id.clone(),
        profile_id: profile_id.clone(),
        merchant_id: merchant_id.clone(),
        relay_type: common_enums::RelayType::Refund,
        request_data: relay_request.data.clone().map(ForeignFrom::foreign_from),
        status: common_enums::RelayStatus::Created,
        connector_reference_id: None,
        error_code: None,
        error_reason: None,
        created_at: common_utils::date_time::now(),
        modified_at: common_utils::date_time::now(),
        response_data: None,
    }
}

impl ForeignFrom<relay_models::RelayData> for hyperswitch_domain_models::relay::RelayData {
    fn foreign_from(relay: relay_models::RelayData) -> Self {
        match relay {
            relay_models::RelayData::Refund(relay_refund_request) => {
                Self::Refund(hyperswitch_domain_models::relay::RelayRefundRequest {
                    amount: relay_refund_request.amount,
                    currency: relay_refund_request.currency,
                    reason: relay_refund_request.reason,
                })
            }
        }
    }
}
