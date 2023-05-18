pub mod helpers;
pub mod validator;

use api_models::enums as api_enums;
use common_utils::crypto::Encryptable;
use error_stack::{report, ResultExt};
use router_env::{instrument, tracing};
use serde_json::{self};
use storage_models::enums as storage_enums;

use super::errors::{ConnectorErrorExt, StorageErrorExt};
use crate::{
    core::{
        errors::{self, RouterResponse, RouterResult},
        payments::{self, helpers as payment_helpers},
        utils as core_utils,
    },
    routes::AppState,
    services,
    types::{
        self,
        api::{self, payouts},
        domain, storage,
        transformers::{ForeignFrom, ForeignInto},
    },
    utils::{self, OptionExt},
};

// ********************************************** TYPES **********************************************
#[cfg(feature = "payouts")]
#[derive(Clone)]
pub struct PayoutData {
    pub billing_address: Option<domain::Address>,
    pub customer_details: Option<domain::Customer>,
    pub payouts: storage::Payouts,
    pub payout_attempt: storage::PayoutAttempt,
    pub payout_method_data: Option<payouts::PayoutMethodData>,
    pub merchant_connector_account: Option<payment_helpers::MerchantConnectorAccountType>,
}

// ********************************************** CORE FLOWS **********************************************

#[cfg(feature = "payouts")]
#[instrument(skip_all)]
pub async fn payouts_create_core(
    state: &AppState,
    merchant_account: domain::MerchantAccount,
    req: payouts::PayoutCreateRequest,
) -> RouterResponse<payouts::PayoutCreateResponse>
where
{
    // TODO: Remove hardcoded connector
    let connector_name = api_enums::Connector::Adyen;

    // Validate create request
    let payout_id = validator::validate_create_request(state, &merchant_account, &req).await?;

    // Create DB entries
    let mut payout_data =
        payout_create_db_entries(state, &merchant_account, &req, &payout_id, &connector_name)
            .await?;

    // Form connector data
    let connector_data: api::ConnectorData = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &connector_name.to_string(),
        api::GetToken::Connector,
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to get the connector data")?;

    call_connector_payout(
        state,
        &merchant_account,
        &req,
        connector_data,
        &mut payout_data,
    )
    .await
}

#[cfg(feature = "payouts")]
pub async fn payouts_update_core(
    state: &AppState,
    merchant_account: domain::MerchantAccount,
    req: payouts::PayoutCreateRequest,
) -> RouterResponse<payouts::PayoutCreateResponse> {
    let mut payout_data = make_payout_data(
        state,
        &merchant_account,
        &payouts::PayoutRequest::PayoutCreateRequest(req.to_owned()),
    )
    .await?;

    let payout_attempt = payout_data.payout_attempt.to_owned();
    let status = payout_attempt.status.foreign_into();

    // Verify update feasibility
    if helpers::is_payout_terminal_state(status) {
        return Err(report!(errors::ApiErrorResponse::InvalidRequestData {
            message: format!(
                "Payout {} cannot be updated for status {}",
                payout_attempt.payout_id, status
            ),
        }));
    }

    // Update DB with new data
    let payouts = payout_data.payouts.to_owned();
    let updated_payouts = storage::PayoutsUpdate::Update {
        amount: req.amount.unwrap_or(payouts.amount.into()).into(),
        destination_currency: req
            .currency
            .unwrap_or(payouts.destination_currency.foreign_into())
            .foreign_into(),
        source_currency: req
            .currency
            .unwrap_or(payouts.source_currency.foreign_into())
            .foreign_into(),
        description: req.description.clone().or(payouts.description),
        recurring: req.recurring.unwrap_or(payouts.recurring),
        auto_fulfill: req.auto_fulfill.unwrap_or(payouts.auto_fulfill),
        return_url: req.return_url.clone().or(payouts.return_url),
        entity_type: req
            .entity_type
            .unwrap_or(payouts.entity_type.foreign_into())
            .foreign_into(),
        metadata: req.metadata.clone().or(payouts.metadata),
        last_modified_at: Some(common_utils::date_time::now()),
        payout_method_data: None,
    };

    let db = &*state.store;
    let payout_id = req.payout_id.clone().get_required_value("payout_id")?;
    let merchant_id = &merchant_account.merchant_id;
    payout_data.payouts = db
        .update_payout_by_merchant_id_payout_id(merchant_id, &payout_id, updated_payouts)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error updating payouts")?;

    let payout_attempt = payout_data.payout_attempt.to_owned();
    let updated_business_country =
        payout_attempt
            .business_country
            .map_or(req.business_country.to_owned(), |c| {
                req.business_country
                    .to_owned()
                    .and_then(|nc| if nc != c { Some(nc) } else { None })
            });
    let updated_business_label =
        payout_attempt
            .business_label
            .map_or(req.business_label.to_owned(), |l| {
                req.business_label
                    .to_owned()
                    .and_then(|nl| if nl != l { Some(nl) } else { None })
            });
    match (updated_business_country, updated_business_label) {
        (None, None) => {}
        (business_country, business_label) => {
            let update_payout_attempt = storage::PayoutAttemptUpdate::BusinessUpdate {
                business_country,
                business_label,
            };
            payout_data.payout_attempt = db
                .update_payout_attempt_by_merchant_id_payout_id(
                    merchant_id,
                    &payout_id,
                    update_payout_attempt,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating payout_attempt")?;
        }
    }

    // Form connector data
    let connector_data: api::ConnectorData = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &payout_data.payout_attempt.connector,
        api::GetToken::Connector,
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to get the connector data")?;

    call_connector_payout(
        state,
        &merchant_account,
        &req,
        connector_data,
        &mut payout_data,
    )
    .await
}

#[cfg(feature = "payouts")]
#[instrument(skip_all)]
pub async fn payouts_retrieve_core(
    state: &AppState,
    merchant_account: domain::MerchantAccount,
    req: payouts::PayoutRetrieveRequest,
) -> RouterResponse<payouts::PayoutCreateResponse> {
    let payout_data = make_payout_data(
        state,
        &merchant_account,
        &payouts::PayoutRequest::PayoutRetrieveRequest(req.to_owned()),
    )
    .await?;

    response_handler(
        state,
        &merchant_account,
        &payouts::PayoutRequest::PayoutRetrieveRequest(req.to_owned()),
        &payout_data,
    )
    .await
}

#[cfg(feature = "payouts")]
#[instrument(skip_all)]
pub async fn payouts_cancel_core(
    state: &AppState,
    merchant_account: domain::MerchantAccount,
    req: payouts::PayoutActionRequest,
) -> RouterResponse<payouts::PayoutCreateResponse> {
    let mut payout_data = make_payout_data(
        state,
        &merchant_account,
        &payouts::PayoutRequest::PayoutActionRequest(req.to_owned()),
    )
    .await?;

    let payout_attempt = payout_data.payout_attempt.to_owned();
    let connector_payout_id = payout_attempt.connector_payout_id.to_owned();
    let status = payout_attempt.status.foreign_into();

    // Verify if cancellation can be triggered
    if helpers::is_payout_terminal_state(status) {
        return Err(report!(errors::ApiErrorResponse::InvalidRequestData {
            message: format!(
                "Payout {} cannot be cancelled for status {}",
                payout_attempt.payout_id, status
            ),
        }));

    // Make local cancellation
    } else if helpers::is_eligible_for_local_payout_cancellation(status) {
        let updated_payout_attempt = storage::PayoutAttemptUpdate::StatusUpdate {
            connector_payout_id: connector_payout_id.to_owned(),
            status: storage_enums::PayoutStatus::Cancelled,
            error_message: Some("Cancelled by user".to_string()),
            error_code: None,
            is_eligible: None,
        };
        payout_data.payout_attempt = state
            .store
            .update_payout_attempt_by_merchant_id_payout_id(
                &merchant_account.merchant_id,
                &payout_attempt.payout_id,
                updated_payout_attempt,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error updating payout_attempt in db")?;

    // Trigger connector's cancellation
    } else {
        // TODO: Remove hardcoded connector
        let connector_name = api_enums::Connector::Adyen;
        let connector_data: api::ConnectorData = api::ConnectorData::get_connector_by_name(
            &state.conf.connectors,
            &connector_name.to_string(),
            api::GetToken::Connector,
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get the connector data")?;

        payout_data = cancel_payout(
            state,
            &merchant_account,
            &payouts::PayoutRequest::PayoutActionRequest(req.to_owned()),
            &connector_data,
            &mut payout_data,
        )
        .await
        .attach_printable("Payout fulfillment failed for given Payout request")?;
    }

    response_handler(
        state,
        &merchant_account,
        &payouts::PayoutRequest::PayoutActionRequest(req.to_owned()),
        &payout_data,
    )
    .await
}

#[cfg(feature = "payouts")]
#[instrument(skip_all)]
pub async fn payouts_fulfill_core(
    state: &AppState,
    merchant_account: domain::MerchantAccount,
    req: payouts::PayoutActionRequest,
) -> RouterResponse<payouts::PayoutCreateResponse> {
    let mut payout_data = make_payout_data(
        state,
        &merchant_account,
        &payouts::PayoutRequest::PayoutActionRequest(req.to_owned()),
    )
    .await?;

    let payout_attempt = payout_data.payout_attempt.to_owned();
    let status = payout_attempt.status.foreign_into();

    // Verify if fulfillment can be triggered
    if helpers::is_payout_terminal_state(status)
        || status != api_enums::PayoutStatus::RequiresFulfillment
    {
        return Err(report!(errors::ApiErrorResponse::InvalidRequestData {
            message: format!(
                "Payout {} cannot be fulfilled for status {}",
                payout_attempt.payout_id, status
            ),
        }));
    }

    // TODO: Remove hardcoded connector
    let connector_name = api_enums::Connector::Adyen;

    // Form connector data
    let connector_data: api::ConnectorData = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &connector_name.to_string(),
        api::GetToken::Connector,
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to get the connector data")?;

    // Trigger fulfillment
    let payout_method_data = helpers::make_payout_method_data(state, &None, &payout_attempt)
        .await?
        .get_required_value("payout_method_data")?;
    payout_data = fulfill_payout(
        state,
        &merchant_account,
        &payouts::PayoutRequest::PayoutActionRequest(req.to_owned()),
        &connector_data,
        &mut payout_data,
        &payout_method_data,
    )
    .await
    .attach_printable("Payout fulfillment failed for given Payout request")?;

    if helpers::is_payout_err_state(status) {
        return Err(report!(errors::ApiErrorResponse::PayoutFailed {
            data: Some(
                serde_json::json!({"payout_status": status.to_string(), "error_message": payout_attempt.error_message, "error_code": payout_attempt.error_code})
            ),
        }));
    }

    response_handler(
        state,
        &merchant_account,
        &payouts::PayoutRequest::PayoutActionRequest(req.to_owned()),
        &payout_data,
    )
    .await
}

// ********************************************** HELPERS **********************************************
#[cfg(feature = "payouts")]
pub async fn call_connector_payout(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    req: &payouts::PayoutCreateRequest,
    connector_data: api::ConnectorData,
    payout_data: &mut PayoutData,
) -> RouterResponse<payouts::PayoutCreateResponse> {
    let payout_attempt = &payout_data.payout_attempt.to_owned();
    let payouts: &storage_models::payouts::Payouts = &payout_data.payouts.to_owned();
    if let Some(true) = req.create_payout {
        payout_data.payout_method_data = Some(
            helpers::make_payout_method_data(state, &req.payout_method_data, payout_attempt)
                .await?
                .get_required_value("payout_method_data")?,
        );
        // Eligibility flow
        if payouts.payout_type == storage_enums::PayoutType::Card
            && payout_attempt.is_eligible.is_none()
        {
            *payout_data = check_payout_eligibility(
                state,
                merchant_account,
                req,
                &connector_data,
                payout_data,
            )
            .await
            .attach_printable("Eligibility failed for given Payout request")?;
        }

        // Payout creation flow
        utils::when(
            !payout_attempt
                .is_eligible
                .unwrap_or(state.conf.payouts.payout_eligibility),
            || {
                Err(report!(errors::ApiErrorResponse::PayoutFailed {
                    data: Some(serde_json::json!({
                        "message": "Payout method data is invalid"
                    }))
                })
                .attach_printable("Payout data provided is invalid"))
            },
        )?;
        if payout_data.payouts.payout_type == storage_enums::PayoutType::Bank
            && payout_data.payout_attempt.status == storage_enums::PayoutStatus::RequiresCreation
        {
            *payout_data =
                create_payout(state, merchant_account, req, &connector_data, payout_data)
                    .await
                    .attach_printable("Payout creation failed for given Payout request")?;
        }
    };

    // Auto fulfillment flow
    let status = payout_data.payout_attempt.status;
    if payouts.auto_fulfill && status == storage_enums::PayoutStatus::RequiresFulfillment {
        if payout_data.payout_method_data.is_none() {
            payout_data.payout_method_data =
                helpers::make_payout_method_data(state, &req.payout_method_data, payout_attempt)
                    .await?;
        }
        *payout_data = fulfill_payout(
            state,
            merchant_account,
            &payouts::PayoutRequest::PayoutCreateRequest(req.to_owned()),
            &connector_data,
            payout_data,
            &payout_data
                .payout_method_data
                .to_owned()
                .get_required_value("payout_method_data")?,
        )
        .await
        .attach_printable("Payout fulfillment failed for given Payout request")?;
    }

    response_handler(
        state,
        merchant_account,
        &payouts::PayoutRequest::PayoutCreateRequest(req.to_owned()),
        payout_data,
    )
    .await
}

#[cfg(feature = "payouts")]
pub async fn check_payout_eligibility(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    req: &payouts::PayoutCreateRequest,
    connector_data: &api::ConnectorData,
    payout_data: &mut PayoutData,
) -> RouterResult<PayoutData> {
    // 1. Form Router data
    let router_data = core_utils::construct_payout_router_data(
        state,
        &connector_data.connector_name.to_string(),
        merchant_account,
        &payouts::PayoutRequest::PayoutCreateRequest(req.to_owned()),
        payout_data,
    )
    .await?;

    // 2. Fetch connector integration details
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        api::PEligibility,
        types::PayoutsData,
        types::PayoutsResponseData,
    > = connector_data.connector.get_connector_integration();

    // 3. Call connector service
    let router_data_resp = services::execute_connector_processing_step(
        state,
        connector_integration,
        &router_data,
        payments::CallConnectorAction::Trigger,
    )
    .await
    .map_err(|error| error.to_payout_failed_response())?;

    // 4. Process data returned by the connector
    let db = &*state.store;
    let merchant_id = &merchant_account.merchant_id;
    let payout_id = &payout_data.payouts.payout_id;
    match router_data_resp.response {
        Ok(payout_response_data) => {
            let payout_attempt = &payout_data.payout_attempt;
            let status = payout_response_data
                .status
                .unwrap_or(payout_attempt.status.to_owned());
            let updated_payout_attempt =
                storage::payout_attempt::PayoutAttemptUpdate::StatusUpdate {
                    connector_payout_id: payout_response_data.connector_payout_id,
                    status,
                    error_code: None,
                    error_message: None,
                    is_eligible: payout_response_data.payout_eligible,
                };
            payout_data.payout_attempt = db
                .update_payout_attempt_by_merchant_id_payout_id(
                    merchant_id,
                    payout_id,
                    updated_payout_attempt,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating payout_attempt in db")?;
            if helpers::is_payout_err_state(status.foreign_into()) {
                return Err(report!(errors::ApiErrorResponse::PayoutFailed {
                    data: Some(
                        serde_json::json!({"payout_status": status.to_string(), "error_message": payout_data.payout_attempt.error_message.as_ref(), "error_code": payout_data.payout_attempt.error_code.as_ref()})
                    ),
                }));
            }
        }
        Err(err) => {
            let updated_payout_attempt =
                storage::payout_attempt::PayoutAttemptUpdate::StatusUpdate {
                    connector_payout_id: String::default(),
                    status: storage_enums::PayoutStatus::Failed,
                    error_code: Some(err.code),
                    error_message: Some(err.message),
                    is_eligible: None,
                };
            payout_data.payout_attempt = db
                .update_payout_attempt_by_merchant_id_payout_id(
                    merchant_id,
                    payout_id,
                    updated_payout_attempt,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating payout_attempt in db")?;
        }
    };

    Ok(payout_data.clone())
}

#[cfg(feature = "payouts")]
pub async fn create_payout(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    req: &payouts::PayoutCreateRequest,
    connector_data: &api::ConnectorData,
    payout_data: &mut PayoutData,
) -> RouterResult<PayoutData> {
    // 1. Form Router data
    let router_data = core_utils::construct_payout_router_data(
        state,
        &connector_data.connector_name.to_string(),
        merchant_account,
        &payouts::PayoutRequest::PayoutCreateRequest(req.to_owned()),
        payout_data,
    )
    .await?;

    // 2. Fetch connector integration details
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        api::PCreate,
        types::PayoutsData,
        types::PayoutsResponseData,
    > = connector_data.connector.get_connector_integration();

    // 3. Call connector service
    let router_data_resp = services::execute_connector_processing_step(
        state,
        connector_integration,
        &router_data,
        payments::CallConnectorAction::Trigger,
    )
    .await
    .map_err(|error| error.to_payout_failed_response())?;

    // 4. Process data returned by the connector
    let db = &*state.store;
    let merchant_id = &merchant_account.merchant_id;
    let payout_id = &payout_data.payouts.payout_id;
    match router_data_resp.response {
        Ok(payout_response_data) => {
            let payout_attempt = &payout_data.payout_attempt;
            let status = payout_response_data
                .status
                .unwrap_or(payout_attempt.status.to_owned());
            let updated_payout_attempt =
                storage::payout_attempt::PayoutAttemptUpdate::StatusUpdate {
                    connector_payout_id: payout_response_data.connector_payout_id,
                    status,
                    error_code: None,
                    error_message: None,
                    is_eligible: payout_response_data.payout_eligible,
                };
            payout_data.payout_attempt = db
                .update_payout_attempt_by_merchant_id_payout_id(
                    merchant_id,
                    payout_id,
                    updated_payout_attempt,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating payout_attempt in db")?;
            if helpers::is_payout_err_state(status.foreign_into()) {
                return Err(report!(errors::ApiErrorResponse::PayoutFailed {
                    data: Some(
                        serde_json::json!({"payout_status": status.to_string(), "error_message": payout_data.payout_attempt.error_message.as_ref(), "error_code": payout_data.payout_attempt.error_code.as_ref()})
                    ),
                }));
            }
        }
        Err(err) => {
            let updated_payout_attempt =
                storage::payout_attempt::PayoutAttemptUpdate::StatusUpdate {
                    connector_payout_id: String::default(),
                    status: storage_enums::PayoutStatus::Failed,
                    error_code: Some(err.code),
                    error_message: Some(err.message),
                    is_eligible: None,
                };
            payout_data.payout_attempt = db
                .update_payout_attempt_by_merchant_id_payout_id(
                    merchant_id,
                    payout_id,
                    updated_payout_attempt,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating payout_attempt in db")?;
        }
    };

    Ok(payout_data.clone())
}

pub async fn cancel_payout(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    req: &payouts::PayoutRequest,
    connector_data: &api::ConnectorData,
    payout_data: &mut PayoutData,
) -> RouterResult<PayoutData> {
    // 1. Form Router data
    let router_data = core_utils::construct_payout_router_data(
        state,
        &connector_data.connector_name.to_string(),
        merchant_account,
        req,
        payout_data,
    )
    .await?;

    // 2. Fetch connector integration details
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        api::PCancel,
        types::PayoutsData,
        types::PayoutsResponseData,
    > = connector_data.connector.get_connector_integration();

    // 3. Call connector service
    let router_data_resp = services::execute_connector_processing_step(
        state,
        connector_integration,
        &router_data,
        payments::CallConnectorAction::Trigger,
    )
    .await
    .map_err(|error| error.to_payout_failed_response())?;

    // 4. Process data returned by the connector
    let db = &*state.store;
    let merchant_id = &merchant_account.merchant_id;
    let payout_id = &payout_data.payout_attempt.payout_id;
    match router_data_resp.response {
        Ok(payout_response_data) => {
            let status = payout_response_data
                .status
                .unwrap_or(payout_data.payout_attempt.status.to_owned());
            let updated_payout_attempt =
                storage::payout_attempt::PayoutAttemptUpdate::StatusUpdate {
                    connector_payout_id: payout_response_data.connector_payout_id,
                    status,
                    error_code: None,
                    error_message: None,
                    is_eligible: payout_response_data.payout_eligible,
                };
            payout_data.payout_attempt = db
                .update_payout_attempt_by_merchant_id_payout_id(
                    merchant_id,
                    payout_id,
                    updated_payout_attempt,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating payout_attempt in db")?
        }
        Err(err) => {
            let updated_payouts_create =
                storage::payout_attempt::PayoutAttemptUpdate::StatusUpdate {
                    connector_payout_id: String::default(),
                    status: storage_enums::PayoutStatus::Failed,
                    error_code: Some(err.code),
                    error_message: Some(err.message),
                    is_eligible: None,
                };
            payout_data.payout_attempt = db
                .update_payout_attempt_by_merchant_id_payout_id(
                    merchant_id,
                    payout_id,
                    updated_payouts_create,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating payout_attempt in db")?
        }
    };

    Ok(payout_data.clone())
}

#[cfg(feature = "payouts")]
pub async fn fulfill_payout(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    req: &payouts::PayoutRequest,
    connector_data: &api::ConnectorData,
    payout_data: &mut PayoutData,
    payout_method_data: &api::PayoutMethodData,
) -> RouterResult<PayoutData> {
    // 1. Form Router data
    let router_data = core_utils::construct_payout_router_data(
        state,
        &connector_data.connector_name.to_string(),
        merchant_account,
        req,
        payout_data,
    )
    .await?;

    // 2. Fetch connector integration details
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        api::PFulfill,
        types::PayoutsData,
        types::PayoutsResponseData,
    > = connector_data.connector.get_connector_integration();

    // 3. Call connector service
    let router_data_resp = services::execute_connector_processing_step(
        state,
        connector_integration,
        &router_data,
        payments::CallConnectorAction::Trigger,
    )
    .await
    .map_err(|error| error.to_payout_failed_response())?;

    // 4. Process data returned by the connector
    let db = &*state.store;
    let merchant_id = &merchant_account.merchant_id;
    let payout_attempt = &payout_data.payout_attempt;
    let payout_id = &payout_attempt.payout_id;
    match router_data_resp.response {
        Ok(payout_response_data) => {
            if payout_data.payouts.recurring {
                helpers::save_payout_data_to_locker(
                    state,
                    payout_attempt,
                    payout_method_data,
                    merchant_account,
                )
                .await?;
            }
            let status = payout_response_data
                .status
                .unwrap_or(payout_attempt.status.to_owned());
            let updated_payouts = storage::payout_attempt::PayoutAttemptUpdate::StatusUpdate {
                connector_payout_id: payout_response_data.connector_payout_id,
                status,
                error_code: None,
                error_message: None,
                is_eligible: payout_response_data.payout_eligible,
            };
            payout_data.payout_attempt = db
                .update_payout_attempt_by_merchant_id_payout_id(
                    merchant_id,
                    payout_id,
                    updated_payouts,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating payout_attempt in db")?;
            if helpers::is_payout_err_state(status.foreign_into()) {
                return Err(report!(errors::ApiErrorResponse::PayoutFailed {
                    data: Some(
                        serde_json::json!({"payout_status": status.to_string(), "error_message": payout_data.payout_attempt.error_message.as_ref(), "error_code": payout_data.payout_attempt.error_code.as_ref()})
                    ),
                }));
            }
        }
        Err(err) => {
            let updated_payouts = storage::payout_attempt::PayoutAttemptUpdate::StatusUpdate {
                connector_payout_id: String::default(),
                status: storage_enums::PayoutStatus::Failed,
                error_code: Some(err.code),
                error_message: Some(err.message),
                is_eligible: None,
            };
            payout_data.payout_attempt = db
                .update_payout_attempt_by_merchant_id_payout_id(
                    merchant_id,
                    payout_id,
                    updated_payouts,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating payout_attempt in db")?
        }
    };

    Ok(payout_data.clone())
}

#[cfg(feature = "payouts")]
pub async fn response_handler(
    _state: &AppState,
    merchant_account: &domain::MerchantAccount,
    _req: &payouts::PayoutRequest,
    payout_data: &PayoutData,
) -> RouterResponse<payouts::PayoutCreateResponse> {
    let payout_attempt = payout_data.payout_attempt.to_owned();
    let payouts = payout_data.payouts.to_owned();
    let billing_address = payout_data.billing_address.to_owned();
    let customer_details = payout_data.customer_details.to_owned();

    let status = api_enums::PayoutStatus::foreign_from(payout_attempt.status.to_owned());
    let currency = api_enums::Currency::foreign_from(payouts.destination_currency.to_owned());
    let entity_type = api_enums::EntityType::foreign_from(payouts.entity_type.to_owned());
    let payout_type = api_enums::PayoutType::foreign_from(payouts.payout_type.to_owned());

    let customer_id = payouts.customer_id;

    let (email, name, phone, phone_country_code) = customer_details
        .map_or((None, None, None, None), |c| {
            (c.email, c.name, c.phone, c.phone_country_code)
        });

    let address = billing_address.as_ref().map(|a| {
        let phone_details = api_models::payments::PhoneDetails {
            number: a.phone_number.to_owned().map(Encryptable::into_inner),
            country_code: a.country_code.to_owned(),
        };
        let address_details = api_models::payments::AddressDetails {
            city: a.city.to_owned(),
            country: a.country.to_owned(),
            line1: a.line1.to_owned().map(Encryptable::into_inner),
            line2: a.line2.to_owned().map(Encryptable::into_inner),
            line3: a.line3.to_owned().map(Encryptable::into_inner),
            zip: a.zip.to_owned().map(Encryptable::into_inner),
            first_name: a.first_name.to_owned().map(Encryptable::into_inner),
            last_name: a.last_name.to_owned().map(Encryptable::into_inner),
            state: a.state.to_owned().map(Encryptable::into_inner),
        };
        api::payments::Address {
            phone: Some(phone_details),
            address: Some(address_details),
        }
    });

    let response = api::PayoutCreateResponse {
        payout_id: payouts.payout_id.to_owned(),
        merchant_id: merchant_account.merchant_id.to_owned(),
        amount: payouts.amount.to_owned(),
        currency,
        connector: Some(payout_attempt.connector.to_owned()),
        payout_type,
        billing: address,
        customer_id,
        auto_fulfill: payouts.auto_fulfill,
        email,
        name,
        phone,
        phone_country_code,
        client_secret: None,
        return_url: payouts.return_url.to_owned(),
        business_country: None, // FIXME: Fetch from MCA
        business_label: None,   // FIXME: Fetch from MCA
        description: payouts.description.to_owned(),
        entity_type,
        recurring: payouts.recurring,
        metadata: payouts.metadata,
        status,
        error_message: payout_attempt.error_message.to_owned(),
        error_code: payout_attempt.error_code,
    };
    Ok(services::ApplicationResponse::Json(response))
}

// DB entries
#[cfg(feature = "payouts")]
pub async fn payout_create_db_entries(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    req: &payouts::PayoutCreateRequest,
    payout_id: &String,
    connector_name: &api_enums::Connector,
) -> RouterResult<PayoutData> {
    let db = &*state.store;
    let merchant_id = &merchant_account.merchant_id;

    // Get or create customer
    let customer_details = payments::CustomerDetails {
        customer_id: req.customer_id.to_owned(),
        name: req.name.to_owned(),
        email: req.email.to_owned(),
        phone: req.phone.to_owned(),
        phone_country_code: req.phone_country_code.to_owned(),
    };
    let customer =
        helpers::get_or_create_customer_details(state, &customer_details, merchant_account).await?;
    let customer_id = customer.to_owned().map_or(
        Err(report!(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "customer_id",
        }))?,
        |c| c.customer_id,
    );

    // Get or create address
    let billing_address = payment_helpers::get_address_for_payment_request(
        db,
        req.billing.as_ref(),
        None,
        merchant_id,
        &Some(customer_id.to_owned()),
    )
    .await?;
    let address_id = billing_address.to_owned().map_or(
        Err(report!(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "billing.address"
        }))?,
        |b| b.address_id,
    );

    // Make payouts entry
    let currency = req
        .currency
        .to_owned()
        .map(ForeignInto::foreign_into)
        .get_required_value("currency")?;
    let payout_type = req
        .payout_type
        .to_owned()
        .get_required_value("payout_type")?;

    let payouts_req = storage::PayoutsNew::default()
        .set_payout_id(payout_id.to_owned())
        .set_merchant_id(merchant_id.to_owned())
        .set_customer_id(customer_id.to_owned())
        .set_address_id(address_id.to_owned())
        .set_payout_type(payout_type.foreign_into())
        .set_amount(req.amount.unwrap_or(api::Amount::Zero).into())
        .set_destination_currency(currency)
        .set_source_currency(currency)
        .set_description(req.description.to_owned())
        .set_recurring(req.recurring.unwrap_or(false))
        .set_auto_fulfill(req.auto_fulfill.unwrap_or(false))
        .set_return_url(req.return_url.to_owned())
        .set_entity_type(storage_enums::EntityType::foreign_from(
            req.entity_type.unwrap_or(api_enums::EntityType::default()),
        ))
        .set_metadata(req.metadata.to_owned())
        .set_created_at(Some(common_utils::date_time::now()))
        .set_last_modified_at(Some(common_utils::date_time::now()))
        .to_owned();
    let payouts = db
        .insert_payout(payouts_req)
        .await
        .to_duplicate_response(errors::ApiErrorResponse::DuplicatePayout {
            payout_id: payout_id.to_owned(),
        })
        .attach_printable("Error inserting payouts in db")?;

    // Make payout_attempt entry
    let status = if req.payout_method_data.is_some() {
        storage_enums::PayoutStatus::RequiresCreation
    } else {
        storage_enums::PayoutStatus::RequiresPayoutMethodData
    };
    let payout_attempt_req = storage::PayoutAttemptNew::default()
        .set_payout_id(payout_id.to_owned())
        .set_customer_id(customer_id.to_owned())
        .set_merchant_id(merchant_id.to_owned())
        .set_address_id(address_id.to_owned())
        .set_connector(connector_name.to_string())
        .set_status(status)
        .set_business_country(req.business_country.to_owned())
        .set_business_label(req.business_label.to_owned())
        .to_owned();
    let payout_attempt = db
        .insert_payout_attempt(payout_attempt_req)
        .await
        .to_duplicate_response(errors::ApiErrorResponse::DuplicatePayout {
            payout_id: payout_id.to_owned(),
        })
        .attach_printable("Error inserting payout_attempt in db")?;

    // Make PayoutData
    Ok(PayoutData {
        billing_address,
        customer_details: customer,
        payouts,
        payout_attempt,
        payout_method_data: None,
        merchant_connector_account: None,
    })
}

#[cfg(feature = "payouts")]
pub async fn make_payout_data(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    req: &payouts::PayoutRequest,
) -> RouterResult<PayoutData> {
    let db = &*state.store;
    let merchant_id = &merchant_account.merchant_id;
    let payout_id = match req {
        payouts::PayoutRequest::PayoutActionRequest(r) => r.payout_id.clone(),
        payouts::PayoutRequest::PayoutCreateRequest(r) => r.payout_id.clone().unwrap_or_default(),
        payouts::PayoutRequest::PayoutRetrieveRequest(r) => r.payout_id.clone(),
    };

    let payouts = db
        .find_payout_by_merchant_id_payout_id(merchant_id, &payout_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::PayoutNotFound)?;

    let payout_attempt = db
        .find_payout_attempt_by_merchant_id_payout_id(merchant_id, &payout_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::PayoutNotFound)?;

    let billing_address = payment_helpers::get_address_for_payment_request(
        db,
        None,
        Some(&payouts.address_id.to_owned()),
        merchant_id,
        &Some(payouts.customer_id.to_owned()),
    )
    .await?;

    let customer_details = db
        .find_customer_optional_by_customer_id_merchant_id(
            &payouts.customer_id.to_owned(),
            merchant_id,
        )
        .await
        .map_or(None, |c| c);

    Ok(PayoutData {
        billing_address,
        customer_details,
        payouts,
        payout_attempt,
        payout_method_data: None,
        merchant_connector_account: None,
    })
}
