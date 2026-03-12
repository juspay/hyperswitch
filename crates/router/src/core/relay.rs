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
    connector::utils::RouterData,
    core::payments,
    routes::SessionState,
    services::{self, api::ConnectorValidation},
    types::{
        api::{self},
        domain,
    },
    utils::OptionExt,
};

pub mod utils;

pub trait Validate {
    type Error: error_stack::Context;
    fn validate(&self) -> Result<(), Self::Error> {
        Ok(())
    }
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

impl Validate for relay_api_models::RelayCaptureRequestData {
    type Error = errors::ApiErrorResponse;
    fn validate(&self) -> Result<(), Self::Error> {
        fp_utils::when(self.amount_to_capture.get_amount_as_i64() <= 0, || {
            Err(errors::ApiErrorResponse::PreconditionFailed {
                message: "Amount should be greater than 0".to_string(),
            })
        })?;

        fp_utils::when(
            self.amount_to_capture.get_amount_as_i64() > self.authorized_amount.get_amount_as_i64(),
            || {
                Err(errors::ApiErrorResponse::PreconditionFailed {
                    message: "Capture Amount should be less than or equal to Authorized Amount"
                        .to_string(),
                })
            },
        )?;
        Ok(())
    }
}

impl Validate for relay_api_models::RelayIncrementalAuthorizationRequestData {
    type Error = errors::ApiErrorResponse;
    fn validate(&self) -> Result<(), Self::Error> {
        fp_utils::when(self.additional_amount.get_amount_as_i64() <= 0, || {
            Err(errors::ApiErrorResponse::PreconditionFailed {
                message: "Amount should be greater than 0".to_string(),
            })
        })?;

        Ok(())
    }
}

impl Validate for relay_api_models::RelayVoidRequestData {
    type Error = errors::ApiErrorResponse;
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
        platform: domain::Platform,
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
            Some(relay_api_models::RelayData::Capture(_))
            | Some(relay_api_models::RelayData::Void(_))
            | Some(relay_api_models::RelayData::IncrementalAuthorization(_))
            | None => Err(errors::ApiErrorResponse::InvalidRequestData {
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
        platform: domain::Platform,
        connector_account: domain::MerchantConnectorAccount,
        relay_record: &relay::Relay,
    ) -> RouterResult<relay::RelayUpdate> {
        let connector_id = &relay_record.connector_id;

        let merchant_id = platform.get_processor().get_account().get_id();

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
            None,
        )
        .await
        .to_refund_failed_response()?;

        let relay_update = relay::RelayUpdate::from_refund_response(router_data_res.response);

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

impl RelayRequestInner<RelayCapture> {
    pub fn from_relay_request(relay_request: relay_api_models::RelayRequest) -> RouterResult<Self> {
        match relay_request.data {
            Some(relay_api_models::RelayData::Capture(ref_data)) => Ok(Self {
                connector_resource_id: relay_request.connector_resource_id,
                connector_id: relay_request.connector_id,
                relay_type: PhantomData,
                data: ref_data,
            }),
            Some(relay_api_models::RelayData::Refund(_))
            | Some(relay_api_models::RelayData::Void(_))
            | Some(relay_api_models::RelayData::IncrementalAuthorization(_))
            | None => Err(errors::ApiErrorResponse::InvalidRequestData {
                message: "Relay data is required for relay type capture".to_string(),
            })?,
        }
    }
}

pub struct RelayCapture;

#[async_trait]
impl RelayInterface for RelayCapture {
    type Request = relay_api_models::RelayCaptureRequestData;

    fn get_domain_models(
        relay_request: RelayRequestInner<Self>,
        merchant_id: &id_type::MerchantId,
        profile_id: &id_type::ProfileId,
    ) -> relay::Relay {
        let relay_id = id_type::RelayId::generate();
        let relay_capture: relay::RelayCaptureData = relay_request.data.into();
        relay::Relay {
            id: relay_id.clone(),
            connector_resource_id: relay_request.connector_resource_id.clone(),
            connector_id: relay_request.connector_id.clone(),
            profile_id: profile_id.clone(),
            merchant_id: merchant_id.clone(),
            relay_type: common_enums::RelayType::Capture,
            request_data: Some(relay::RelayData::Capture(relay_capture)),
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
        platform: domain::Platform,
        connector_account: domain::MerchantConnectorAccount,
        relay_record: &relay::Relay,
    ) -> RouterResult<relay::RelayUpdate> {
        let connector_id = &relay_record.connector_id;

        let merchant_id = platform.get_processor().get_account().get_id();

        let connector_name = &connector_account.get_connector_name_as_string();

        let connector_data = api::ConnectorData::get_connector_by_name(
            &state.conf.connectors,
            connector_name,
            api::GetToken::Connector,
            Some(connector_id.clone()),
        )?;
        let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
            api::Capture,
            hyperswitch_domain_models::router_request_types::PaymentsCaptureData,
            hyperswitch_domain_models::router_response_types::PaymentsResponseData,
        > = connector_data.connector.get_connector_integration();

        let router_data = utils::construct_relay_capture_router_data(
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
            None,
        )
        .await
        .to_payment_failed_response()?;

        let relay_update = relay::RelayUpdate::try_from_capture_response((
            router_data_res.status,
            relay_record.connector_resource_id.to_owned(),
            router_data_res.response,
        ))?;

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

impl RelayRequestInner<RelayIncrementalAuthorization> {
    pub fn from_relay_request(relay_request: relay_api_models::RelayRequest) -> RouterResult<Self> {
        match relay_request.data {
            Some(relay_api_models::RelayData::IncrementalAuthorization(ref_data)) => Ok(Self {
                connector_resource_id: relay_request.connector_resource_id,
                connector_id: relay_request.connector_id,
                relay_type: PhantomData,
                data: ref_data,
            }),
            Some(relay_api_models::RelayData::Refund(_))
            | Some(relay_api_models::RelayData::Void(_))
            | Some(relay_api_models::RelayData::Capture(_))
            | None => Err(errors::ApiErrorResponse::InvalidRequestData {
                message: "Relay data is required for relay type capture".to_string(),
            })?,
        }
    }
}

pub struct RelayIncrementalAuthorization;

#[async_trait]
impl RelayInterface for RelayIncrementalAuthorization {
    type Request = relay_api_models::RelayIncrementalAuthorizationRequestData;

    fn get_domain_models(
        relay_request: RelayRequestInner<Self>,
        merchant_id: &id_type::MerchantId,
        profile_id: &id_type::ProfileId,
    ) -> relay::Relay {
        let relay_id = id_type::RelayId::generate();
        let relay_incremental_authorization: relay::RelayIncrementalAuthorizationData =
            relay_request.data.into();
        relay::Relay {
            id: relay_id.clone(),
            connector_resource_id: relay_request.connector_resource_id.clone(),
            connector_id: relay_request.connector_id.clone(),
            profile_id: profile_id.clone(),
            merchant_id: merchant_id.clone(),
            relay_type: common_enums::RelayType::IncrementalAuthorization,
            request_data: Some(relay::RelayData::IncrementalAuthorization(
                relay_incremental_authorization,
            )),
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
        platform: domain::Platform,
        connector_account: domain::MerchantConnectorAccount,
        relay_record: &relay::Relay,
    ) -> RouterResult<relay::RelayUpdate> {
        let connector_id = &relay_record.connector_id;

        let merchant_id = platform.get_processor().get_account().get_id();

        let connector_name = &connector_account.get_connector_name_as_string();

        let connector_data = api::ConnectorData::get_connector_by_name(
            &state.conf.connectors,
            connector_name,
            api::GetToken::Connector,
            Some(connector_id.clone()),
        )?;
        let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
            api::IncrementalAuthorization,
            hyperswitch_domain_models::router_request_types::PaymentsIncrementalAuthorizationData,
            hyperswitch_domain_models::router_response_types::PaymentsResponseData,
        > = connector_data.connector.get_connector_integration();

        let router_data = utils::construct_relay_incremental_authorization_router_data(
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
            None,
        )
        .await
        .to_payment_failed_response()?;

        let relay_update = relay::RelayUpdate::try_from_incremental_authorization_response(
            router_data_res.response,
        )?;

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

impl RelayRequestInner<RelayVoid> {
    pub fn from_relay_request(relay_request: relay_api_models::RelayRequest) -> RouterResult<Self> {
        match relay_request.data {
            Some(relay_api_models::RelayData::Void(ref_data)) => Ok(Self {
                connector_resource_id: relay_request.connector_resource_id,
                connector_id: relay_request.connector_id,
                relay_type: PhantomData,
                data: ref_data,
            }),
            Some(relay_api_models::RelayData::Refund(_))
            | Some(relay_api_models::RelayData::IncrementalAuthorization(_))
            | Some(relay_api_models::RelayData::Capture(_))
            | None => Err(errors::ApiErrorResponse::InvalidRequestData {
                message: "Relay data is required for relay type void".to_string(),
            })?,
        }
    }
}

pub struct RelayVoid;

#[async_trait]
impl RelayInterface for RelayVoid {
    type Request = relay_api_models::RelayVoidRequestData;

    fn get_domain_models(
        relay_request: RelayRequestInner<Self>,
        merchant_id: &id_type::MerchantId,
        profile_id: &id_type::ProfileId,
    ) -> relay::Relay {
        let relay_id = id_type::RelayId::generate();
        let relay_void: relay::RelayVoidData = relay_request.data.into();
        relay::Relay {
            id: relay_id.clone(),
            connector_resource_id: relay_request.connector_resource_id.clone(),
            connector_id: relay_request.connector_id.clone(),
            profile_id: profile_id.clone(),
            merchant_id: merchant_id.clone(),
            relay_type: common_enums::RelayType::Void,
            request_data: Some(relay::RelayData::Void(relay_void)),
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
        platform: domain::Platform,
        connector_account: domain::MerchantConnectorAccount,
        relay_record: &relay::Relay,
    ) -> RouterResult<relay::RelayUpdate> {
        let connector_id = &relay_record.connector_id;

        let merchant_id = platform.get_processor().get_account().get_id();

        let connector_name = &connector_account.get_connector_name_as_string();

        let connector_data = api::ConnectorData::get_connector_by_name(
            &state.conf.connectors,
            connector_name,
            api::GetToken::Connector,
            Some(connector_id.clone()),
        )?;
        let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
            api::Void,
            hyperswitch_domain_models::router_request_types::PaymentsCancelData,
            hyperswitch_domain_models::router_response_types::PaymentsResponseData,
        > = connector_data.connector.get_connector_integration();

        let router_data = utils::construct_relay_void_router_data(
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
            None,
        )
        .await
        .to_payment_failed_response()?;

        let relay_update = relay::RelayUpdate::try_from_void_response((
            router_data_res.status,
            router_data_res.response,
        ))?;

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
    platform: domain::Platform,
    profile_id_optional: Option<id_type::ProfileId>,
    request: relay_api_models::RelayRequest,
) -> RouterResponse<relay_api_models::RelayResponse> {
    match request.relay_type {
        common_enums::RelayType::Refund => {
            let relay_refund_request =
                RelayRequestInner::<RelayRefund>::from_relay_request(request)?;
            relay(state, platform, profile_id_optional, relay_refund_request).await
        }
        common_enums::RelayType::Capture => {
            let relay_capture_request =
                RelayRequestInner::<RelayCapture>::from_relay_request(request)?;
            relay(state, platform, profile_id_optional, relay_capture_request).await
        }
        common_enums::RelayType::IncrementalAuthorization => {
            let relay_incremental_auth_request =
                RelayRequestInner::<RelayIncrementalAuthorization>::from_relay_request(request)?;
            relay(
                state,
                platform,
                profile_id_optional,
                relay_incremental_auth_request,
            )
            .await
        }
        common_enums::RelayType::Void => {
            let relay_capture_request =
                RelayRequestInner::<RelayVoid>::from_relay_request(request)?;
            relay(state, platform, profile_id_optional, relay_capture_request).await
        }
    }
}

pub async fn relay<T: RelayInterface>(
    state: SessionState,
    platform: domain::Platform,
    profile_id_optional: Option<id_type::ProfileId>,
    req: RelayRequestInner<T>,
) -> RouterResponse<relay_api_models::RelayResponse> {
    let db = state.store.as_ref();
    let merchant_id = platform.get_processor().get_account().get_id();
    let connector_id = &req.connector_id;

    let profile_id_from_auth_layer = profile_id_optional.get_required_value("ProfileId")?;

    let profile = db
        .find_business_profile_by_merchant_id_profile_id(
            platform.get_processor().get_key_store(),
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
            merchant_id,
            connector_id,
            platform.get_processor().get_key_store(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: connector_id.get_string_repr().to_string(),
        })?;

    #[cfg(feature = "v2")]
    let connector_account = db
        .find_merchant_connector_account_by_id(
            connector_id,
            platform.get_processor().get_key_store(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: connector_id.get_string_repr().to_string(),
        })?;

    T::validate_relay_request(&req.data)?;

    let relay_domain = T::get_domain_models(req, merchant_id, profile.get_id());

    let relay_record = db
        .insert_relay(platform.get_processor().get_key_store(), relay_domain)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to insert a relay record in db")?;

    let relay_response =
        T::process_relay(&state, platform.clone(), connector_account, &relay_record)
            .await
            .attach_printable("Failed to process relay")?;

    let relay_update_record = db
        .update_relay(
            platform.get_processor().get_key_store(),
            relay_record,
            relay_response,
        )
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
    platform: domain::Platform,
    profile_id_optional: Option<id_type::ProfileId>,
    req: relay_api_models::RelayRetrieveRequest,
) -> RouterResponse<relay_api_models::RelayResponse> {
    let db = state.store.as_ref();
    let merchant_id = platform.get_processor().get_account().get_id();
    let relay_id = &req.id;

    let profile_id_from_auth_layer = profile_id_optional.get_required_value("ProfileId")?;

    db.find_business_profile_by_merchant_id_profile_id(
        platform.get_processor().get_key_store(),
        merchant_id,
        &profile_id_from_auth_layer,
    )
    .await
    .change_context(errors::ApiErrorResponse::ProfileNotFound {
        id: profile_id_from_auth_layer.get_string_repr().to_owned(),
    })?;

    let relay_record_result = db
        .find_relay_by_id(platform.get_processor().get_key_store(), relay_id)
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
            merchant_id,
            &relay_record.connector_id,
            platform.get_processor().get_key_store(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: relay_record.connector_id.get_string_repr().to_string(),
        })?;

    #[cfg(feature = "v2")]
    let connector_account = db
        .find_merchant_connector_account_by_id(
            &relay_record.connector_id,
            platform.get_processor().get_key_store(),
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
                    &platform,
                    &relay_record,
                    connector_account,
                )
                .await?;

                db.update_relay(
                    platform.get_processor().get_key_store(),
                    relay_record,
                    relay_response,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to update the relay record")?
            } else {
                relay_record
            }
        }
        common_enums::RelayType::Capture => {
            if should_call_connector_for_relay_capture_status(&relay_record, req.force_sync) {
                let relay_response = sync_relay_capture_with_gateway(
                    &state,
                    &platform,
                    &relay_record,
                    connector_account,
                )
                .await?;

                db.update_relay(
                    platform.get_processor().get_key_store(),
                    relay_record,
                    relay_response,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to update the relay record")?
            } else {
                relay_record
            }
        }
        common_enums::RelayType::IncrementalAuthorization | common_enums::RelayType::Void => {
            relay_record
        }
    };

    let response = relay_api_models::RelayResponse::from(relay_response);

    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        response,
    ))
}

fn should_call_connector_for_relay_refund_status(relay: &relay::Relay, force_sync: bool) -> bool {
    // This allows refund sync at connector level if force_sync is enabled, or
    // check if the refund is in non terminal state
    !matches!(relay.status, RelayStatus::Failure | RelayStatus::Success) && force_sync
}

fn should_call_connector_for_relay_capture_status(relay: &relay::Relay, force_sync: bool) -> bool {
    // This allows capture sync at connector level if force_sync is enabled, or
    // check if the capture is in non terminal state
    !matches!(relay.status, RelayStatus::Failure | RelayStatus::Success) && force_sync
}

pub async fn sync_relay_refund_with_gateway(
    state: &SessionState,
    platform: &domain::Platform,
    relay_record: &relay::Relay,
    connector_account: domain::MerchantConnectorAccount,
) -> RouterResult<relay::RelayUpdate> {
    let connector_id = &relay_record.connector_id;
    let merchant_id = platform.get_processor().get_account().get_id();

    let connector_name = &connector_account.get_connector_name_as_string();

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
        None,
    )
    .await
    .to_refund_failed_response()?;

    let relay_response = relay::RelayUpdate::from_refund_response(router_data_res.response);

    Ok(relay_response)
}

pub async fn sync_relay_capture_with_gateway(
    state: &SessionState,
    platform: &domain::Platform,
    relay_record: &relay::Relay,
    connector_account: domain::MerchantConnectorAccount,
) -> RouterResult<relay::RelayUpdate> {
    let connector_id = &relay_record.connector_id;
    let merchant_id = platform.get_processor().get_account().get_id();

    let connector_name = &connector_account.get_connector_name_as_string();

    let connector_data: api::ConnectorData = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        connector_name,
        api::GetToken::Connector,
        Some(connector_id.clone()),
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to get the connector")?;

    let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
        api::PSync,
        hyperswitch_domain_models::router_request_types::PaymentsSyncData,
        hyperswitch_domain_models::router_response_types::PaymentsResponseData,
    > = connector_data.connector.get_connector_integration();

    let capture_method_type = connector_integration
        .get_multiple_capture_sync_method()
        .map_err(|err| {
            router_env::logger::error!(error=?err);
        })
        .ok();

    let router_data = utils::construct_relay_payments_retrieve_router_data(
        state,
        merchant_id,
        &connector_account,
        relay_record,
        capture_method_type,
    )
    .await?;

    //validate_psync_reference_id if call_connector_action is trigger
    let router_data_res = if connector_data
        .connector
        .validate_psync_reference_id(
            &router_data.request,
            router_data.is_three_ds(),
            router_data.status,
            router_data.connector_meta_data.clone(),
        )
        .is_err()
    {
        router_env::logger::warn!(
            "validate_psync_reference_id failed, hence skipping call to connector"
        );

        router_data
    } else {
        services::execute_connector_processing_step(
            state,
            connector_integration,
            &router_data,
            payments::CallConnectorAction::Trigger,
            None,
            None,
        )
        .await
        .to_payment_failed_response()?
    };

    let relay_response = relay::RelayUpdate::try_from_capture_response((
        router_data_res.status,
        relay_record.connector_resource_id.to_owned(),
        router_data_res.response,
    ))?;

    Ok(relay_response)
}
