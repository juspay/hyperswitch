use std::marker::PhantomData;

use api_models::relay as relay_api_models;
use async_trait::async_trait;
use common_enums::RelayStatus;
use common_utils::{
    self, fp_utils,
    id_type::{self, GenerateId},
};
use error_stack::ResultExt;
use hyperswitch_domain_models::relay;

use super::errors::{self, ConnectorErrorExt, RouterResponse, RouterResult, StorageErrorExt};
use crate::{
    core::payments,
    routes::SessionState,
    services,
    types::{
        api::{self},
        domain,
    },
    utils::OptionExt,
};

pub mod utils;

pub trait Validate {
    type Error: error_stack::Context;
    fn validate(&self) -> Result<(), Self::Error>;
}

impl Validate for relay_api_models::RelayRefundRequestData {
    type Error = errors::ApiErrorResponse;
    fn validate(&self) -> Result<(), Self::Error> {
        fp_utils::when(self.amount.get_amount_as_i64() <= 0, || {
            Err(errors::ApiErrorResponse::PreconditionFailed {
                message: "Amount should be greater than 0".to_string(),
            })
        })?;
        Ok(())
    }
}

#[async_trait]
pub trait RelayInterface {
    type Request: Validate;
    fn validate_relay_request(req: &Self::Request) -> RouterResult<()> {
        req.validate()
            .change_context(errors::ApiErrorResponse::PreconditionFailed {
                message: "Invalid relay request".to_string(),
            })
    }

    fn get_domain_models(
        relay_request: RelayRequestInner<Self>,
        merchant_id: &id_type::MerchantId,
        profile_id: &id_type::ProfileId,
    ) -> relay::Relay;

    async fn process_relay(
        state: &SessionState,
        merchant_account: domain::MerchantAccount,
        connector_account: domain::MerchantConnectorAccount,
        relay_record: &relay::Relay,
    ) -> RouterResult<relay::RelayUpdate>;

    fn generate_response(value: relay::Relay) -> RouterResult<api_models::relay::RelayResponse>;
}

pub struct RelayRequestInner<T: RelayInterface + ?Sized> {
    pub connector_resource_id: String,
    pub connector_id: id_type::MerchantConnectorAccountId,
    pub relay_type: PhantomData<T>,
    pub data: T::Request,
}

impl RelayRequestInner<RelayRefund> {
    pub fn from_relay_request(relay_request: relay_api_models::RelayRequest) -> RouterResult<Self> {
        match relay_request.data {
            Some(relay_api_models::RelayData::Refund(ref_data)) => Ok(Self {
                connector_resource_id: relay_request.connector_resource_id,
                connector_id: relay_request.connector_id,
                relay_type: PhantomData,
                data: ref_data,
            }),
            None => Err(errors::ApiErrorResponse::InvalidRequestData {
                message: "Relay data is required for relay type refund".to_string(),
            })?,
        }
    }
}

pub struct RelayRefund;

#[async_trait]
impl RelayInterface for RelayRefund {
    type Request = relay_api_models::RelayRefundRequestData;

    fn get_domain_models(
        relay_request: RelayRequestInner<Self>,
        merchant_id: &id_type::MerchantId,
        profile_id: &id_type::ProfileId,
    ) -> relay::Relay {
        let relay_id = id_type::RelayId::generate();
        let relay_refund: relay::RelayRefundData = relay_request.data.into();
        relay::Relay {
            id: relay_id.clone(),
            connector_resource_id: relay_request.connector_resource_id.clone(),
            connector_id: relay_request.connector_id.clone(),
            profile_id: profile_id.clone(),
            merchant_id: merchant_id.clone(),
            relay_type: common_enums::RelayType::Refund,
            request_data: Some(relay::RelayData::Refund(relay_refund)),
            status: RelayStatus::Created,
            connector_reference_id: None,
            error_code: None,
            error_message: None,
            created_at: common_utils::date_time::now(),
            modified_at: common_utils::date_time::now(),
            response_data: None,
        }
    }

    async fn process_relay(
        state: &SessionState,
        merchant_account: domain::MerchantAccount,
        connector_account: domain::MerchantConnectorAccount,
        relay_record: &relay::Relay,
    ) -> RouterResult<relay::RelayUpdate> {
        let connector_id = &relay_record.connector_id;

        let merchant_id = merchant_account.get_id();

        let connector_name = &connector_account.get_connector_name_as_string();

        let connector_data = api::ConnectorData::get_connector_by_name(
            &state.conf.connectors,
            connector_name,
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

        let relay_update = relay::RelayUpdate::from(router_data_res.response);

        Ok(relay_update)
    }

    fn generate_response(value: relay::Relay) -> RouterResult<api_models::relay::RelayResponse> {
        let error = value
            .error_code
            .zip(value.error_message)
            .map(
                |(error_code, error_message)| api_models::relay::RelayError {
                    code: error_code,
                    message: error_message,
                },
            );

        let data =
            api_models::relay::RelayData::from(value.request_data.get_required_value("RelayData")?);

        Ok(api_models::relay::RelayResponse {
            id: value.id,
            status: value.status,
            error,
            connector_resource_id: value.connector_resource_id,
            connector_id: value.connector_id,
            profile_id: value.profile_id,
            relay_type: value.relay_type,
            data: Some(data),
            connector_reference_id: value.connector_reference_id,
        })
    }
}

pub async fn relay_flow_decider(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    profile_id_optional: Option<id_type::ProfileId>,
    key_store: domain::MerchantKeyStore,
    request: relay_api_models::RelayRequest,
) -> RouterResponse<relay_api_models::RelayResponse> {
    let relay_flow_request = match request.relay_type {
        common_enums::RelayType::Refund => {
            RelayRequestInner::<RelayRefund>::from_relay_request(request)?
        }
    };
    relay(
        state,
        merchant_account,
        profile_id_optional,
        key_store,
        relay_flow_request,
    )
    .await
}

pub async fn relay<T: RelayInterface>(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    profile_id_optional: Option<id_type::ProfileId>,
    key_store: domain::MerchantKeyStore,
    req: RelayRequestInner<T>,
) -> RouterResponse<relay_api_models::RelayResponse> {
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();
    let merchant_id = merchant_account.get_id();
    let connector_id = &req.connector_id;

    let profile_id_from_auth_layer = profile_id_optional.get_required_value("ProfileId")?;

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

    T::validate_relay_request(&req.data)?;

    let relay_domain = T::get_domain_models(req, merchant_id, profile.get_id());

    let relay_record = db
        .insert_relay(key_manager_state, &key_store, relay_domain)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to insert a relay record in db")?;

    let relay_response =
        T::process_relay(&state, merchant_account, connector_account, &relay_record)
            .await
            .attach_printable("Failed to process relay")?;

    let relay_update_record = db
        .update_relay(key_manager_state, &key_store, relay_record, relay_response)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let response = T::generate_response(relay_update_record)
        .attach_printable("Failed to generate relay response")?;

    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        response,
    ))
}

pub async fn relay_retrieve(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    profile_id_optional: Option<id_type::ProfileId>,
    key_store: domain::MerchantKeyStore,
    req: relay_api_models::RelayRetrieveRequest,
) -> RouterResponse<relay_api_models::RelayResponse> {
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();
    let merchant_id = merchant_account.get_id();
    let relay_id = &req.id;

    let profile_id_from_auth_layer = profile_id_optional.get_required_value("ProfileId")?;

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
        .find_relay_by_id(key_manager_state, &key_store, relay_id)
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

    let response = relay_api_models::RelayResponse::from(relay_response);

    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        response,
    ))
}

fn should_call_connector_for_relay_refund_status(relay: &relay::Relay, force_sync: bool) -> bool {
    // This allows refund sync at connector level if force_sync is enabled, or
    // check if the refund is in terminal state
    !matches!(relay.status, RelayStatus::Failure | RelayStatus::Success) && force_sync
}

pub async fn sync_relay_refund_with_gateway(
    state: &SessionState,
    merchant_account: &domain::MerchantAccount,
    relay_record: &relay::Relay,
    connector_account: domain::MerchantConnectorAccount,
) -> RouterResult<relay::RelayUpdate> {
    let connector_id = &relay_record.connector_id;
    let merchant_id = merchant_account.get_id();

    #[cfg(feature = "v1")]
    let connector_name = &connector_account.connector_name;

    #[cfg(feature = "v2")]
    let connector_name = &connector_account.connector_name.to_string();

    let connector_data: api::ConnectorData = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        connector_name,
        api::GetToken::Connector,
        Some(connector_id.clone()),
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to get the connector")?;

    let router_data = utils::construct_relay_refund_router_data(
        state,
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

    let relay_response = relay::RelayUpdate::from(router_data_res.response);

    Ok(relay_response)
}
