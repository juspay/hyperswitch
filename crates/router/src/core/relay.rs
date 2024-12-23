use api_models::relay as relay_models;
use common_enums::RelayStatus;
use common_utils::{self, ext_traits::OptionExt, id_type};
use error_stack::ResultExt;

use super::errors::{self, ConnectorErrorExt, RouterResponse, RouterResult, StorageErrorExt};
use crate::{
    core::payments,
    routes::SessionState,
    services,
    types::{
        api::{self},
        domain,
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
    let connector_id = &req.connector_id;

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

    #[cfg(feature = "v1")]
    let connector_account = db
        .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
            key_manager_state,
            merchant_id,
            connector_id,
            &key_store,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: connector_id.get_string_repr().to_string(),
        })?;

    #[cfg(feature = "v2")]
    let connector_account = db
        .find_merchant_connector_account_by_id(key_manager_state, connector_id, &key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: connector_id.get_string_repr().to_string(),
        })?;

    validate_relay_refund_request(&req).attach_printable("Invalid relay refund request")?;

    let relay_domain =
        hyperswitch_domain_models::relay::Relay::new(&req, merchant_id, profile.get_id());

    let relay_record = db
        .insert_relay(key_manager_state, &key_store, relay_domain)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to insert a relay record in db")?;

    let relay_response = match req.relay_type {
        common_enums::RelayType::Refund => {
            Box::pin(relay_refund(
                &state,
                merchant_account,
                connector_account,
                &relay_record,
            ))
            .await?
        }
    };

    let relay_update_record = db
        .update_relay(key_manager_state, &key_store, relay_record, relay_response)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let response = relay_models::RelayResponse::from(relay_update_record);

    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        response,
    ))
}

pub async fn relay_refund(
    state: &SessionState,
    merchant_account: domain::MerchantAccount,
    connector_account: domain::MerchantConnectorAccount,
    relay_record: &hyperswitch_domain_models::relay::Relay,
) -> RouterResult<hyperswitch_domain_models::relay::RelayUpdate> {
    let connector_id = &relay_record.connector_id;

    let merchant_id = merchant_account.get_id();

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

    let router_data = utils::construct_relay_refund_router_data(
        state,
        &connector_account.connector_name,
        merchant_id,
        &connector_account,
        relay_record,
    )
    .await?;

    let router_data_res = services::execute_connector_processing_step(
        state,
        connector_integration,
        &router_data,
        payments::CallConnectorAction::Trigger,
        None,
    )
    .await
    .to_refund_failed_response()?;

    let relay_response =
        hyperswitch_domain_models::relay::RelayUpdate::from(router_data_res.response);

    Ok(relay_response)
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

pub async fn relay_retrieve(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    profile_id_optional: Option<id_type::ProfileId>,
    key_store: domain::MerchantKeyStore,
    req: relay_models::RelayRetrieveRequest,
) -> RouterResponse<relay_models::RelayResponse> {
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();
    let merchant_id = merchant_account.get_id();
    let relay_id = &req.id;

    let profile_id_from_auth_layer = profile_id_optional
        .get_required_value("ProfileId")
        .change_context(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "profile id",
        })?;

    db.find_business_profile_by_merchant_id_profile_id(
        key_manager_state,
        &key_store,
        merchant_id,
        &profile_id_from_auth_layer,
    )
    .await
    .change_context(errors::ApiErrorResponse::ProfileNotFound {
        id: profile_id_from_auth_layer.get_string_repr().to_owned(),
    })?;

    let relay_record_result = db
        .find_relay_by_id_merchant_id(key_manager_state, &key_store, merchant_id, relay_id)
        .await;

    let relay_record = match relay_record_result {
        Err(error) => {
            if error.current_context().is_db_not_found() {
                Err(error).change_context(errors::ApiErrorResponse::GenericNotFoundError {
                    message: "relay not found".to_string(),
                })?
            } else {
                Err(error)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("error while fetch relay record")?
            }
        }
        Ok(relay) => relay,
    };

    #[cfg(feature = "v1")]
    let connector_account = db
        .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
            key_manager_state,
            merchant_id,
            &relay_record.connector_id,
            &key_store,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: relay_record.connector_id.get_string_repr().to_string(),
        })?;

    #[cfg(feature = "v2")]
    let connector_account = db
        .find_merchant_connector_account_by_id(
            key_manager_state,
            &relay_record.connector_id,
            &key_store,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: relay_record.connector_id.get_string_repr().to_string(),
        })?;

    let relay_response = match relay_record.relay_type {
        common_enums::RelayType::Refund => {
            if should_call_connector_for_relay_refund_status(&relay_record, req.force_sync) {
                let relay_response = sync_relay_refund_with_gateway(
                    &state,
                    &merchant_account,
                    &relay_record,
                    connector_account,
                )
                .await?;

                db.update_relay(key_manager_state, &key_store, relay_record, relay_response)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to update the relay record")?
            } else {
                relay_record
            }
        }
    };

    let response = relay_models::RelayResponse::from(relay_response);

    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        response,
    ))
}

fn should_call_connector_for_relay_refund_status(
    relay: &hyperswitch_domain_models::relay::Relay,
    force_sync: bool,
) -> bool {
    // This allows refund sync at connector level if force_sync is enabled, or
    // checks if the refund has failed
    !matches!(relay.status, RelayStatus::Failure | RelayStatus::Success) && force_sync
}

pub async fn sync_relay_refund_with_gateway(
    state: &SessionState,
    merchant_account: &domain::MerchantAccount,
    relay_record: &hyperswitch_domain_models::relay::Relay,
    connector_account: domain::MerchantConnectorAccount,
) -> RouterResult<hyperswitch_domain_models::relay::RelayUpdate> {
    let connector_id = &relay_record.connector_id;
    let merchant_id = merchant_account.get_id();

    let connector_data: api::ConnectorData = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &connector_account.connector_name,
        api::GetToken::Connector,
        Some(connector_id.clone()),
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to get the connector")?;

    let router_data = utils::construct_relay_refund_router_data(
        state,
        &connector_account.connector_name,
        merchant_id,
        &connector_account,
        relay_record,
    )
    .await?;

    let connector_integration: services::BoxedRefundConnectorIntegrationInterface<
        api::RSync,
        hyperswitch_domain_models::router_request_types::RefundsData,
        hyperswitch_domain_models::router_response_types::RefundsResponseData,
    > = connector_data.connector.get_connector_integration();

    let router_data_res = services::execute_connector_processing_step(
        state,
        connector_integration,
        &router_data,
        payments::CallConnectorAction::Trigger,
        None,
    )
    .await
    .to_refund_failed_response()?;

    let relay_response =
        hyperswitch_domain_models::relay::RelayUpdate::from(router_data_res.response);

    Ok(relay_response)
}
