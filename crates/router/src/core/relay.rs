use api_models::relay as relay_models;
use common_utils::{
    self,
    ext_traits::OptionExt,
    id_type::{self, GenerateId},
};
use error_stack::ResultExt;

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

pub mod utils;

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

    let relay_domain = get_relay_domain_model(req, merchant_account.get_id(), profile.get_id());

    let relay_record = db
        .insert_relay(key_manager_state, &key_store, relay_domain)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to insert a relay record in db")?;

    let router_data = utils::construct_relay_refund_router_data(
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
            error_message: error.message,
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
    if refund_data.amount.get_amount_as_i64() <= 0 {
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
) -> hyperswitch_domain_models::relay::Relay {
    let relay_id = id_type::RelayId::generate();
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
        error_message: None,
        created_at: common_utils::date_time::now(),
        modified_at: common_utils::date_time::now(),
        response_data: None,
    }
}

impl ForeignFrom<relay_models::RelayData> for hyperswitch_domain_models::relay::RelayData {
    fn foreign_from(relay: relay_models::RelayData) -> Self {
        match relay {
            relay_models::RelayData::Refund(relay_refund_request) => {
                Self::Refund(hyperswitch_domain_models::relay::RelayRefundData {
                    amount: relay_refund_request.amount,
                    currency: relay_refund_request.currency,
                    reason: relay_refund_request.reason,
                })
            }
        }
    }
}
