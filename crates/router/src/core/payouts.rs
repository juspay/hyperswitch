pub mod helpers;
pub mod validator;

use api_models::enums as api_enums;
use common_utils::{crypto::Encryptable, ext_traits::ValueExt, pii};
use diesel_models::enums as storage_enums;
use error_stack::{report, ResultExt};
use router_env::{instrument, tracing};
use serde_json;

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
    pub profile_id: String,
}

// ********************************************** CORE FLOWS **********************************************
#[cfg(feature = "payouts")]
pub async fn get_connector_data(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    routed_through: Option<String>,
    routing_algorithm: Option<serde_json::Value>,
) -> RouterResult<api::PayoutConnectorData> {
    let mut routing_data = storage::PayoutRoutingData {
        routed_through,
        algorithm: None,
    };
    let connector_choice = helpers::get_default_payout_connector(state, routing_algorithm).await?;
    let connector_details = match connector_choice {
        api::PayoutConnectorChoice::SessionMultiple(session_connectors) => {
            api::PayoutConnectorCallType::Multiple(session_connectors)
        }

        api::PayoutConnectorChoice::StraightThrough(straight_through) => {
            let request_straight_through: Option<api::PayoutStraightThroughAlgorithm> =
                Some(straight_through)
                    .map(|val| val.parse_value("StraightThroughAlgorithm"))
                    .transpose()
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Invalid straight through routing rules format")?;
            helpers::decide_payout_connector(
                state,
                merchant_account,
                request_straight_through,
                &mut routing_data,
            )?
        }

        api::PayoutConnectorChoice::Decide => {
            helpers::decide_payout_connector(state, merchant_account, None, &mut routing_data)?
        }
    };
    let connector_data = match connector_details {
        api::PayoutConnectorCallType::Single(connector) => connector,

        api::PayoutConnectorCallType::Multiple(connectors) => {
            // TODO: route through actual multiple connectors.
            connectors.first().map_or(
                Err(errors::ApiErrorResponse::IncorrectConnectorNameGiven),
                |c| Ok(c.connector.to_owned()),
            )?
        }
    };

    Ok(connector_data)
}

#[cfg(feature = "payouts")]
#[instrument(skip_all)]
pub async fn payouts_create_core(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    req: payouts::PayoutCreateRequest,
) -> RouterResponse<payouts::PayoutCreateResponse> {
    // Form connector data
    let connector_data = get_connector_data(
        &state,
        &merchant_account,
        req.connector
            .clone()
            .and_then(|c| c.first().map(|c| c.to_string())),
        req.routing.clone(),
    )
    .await?;

    // Validate create request
    let (payout_id, payout_method_data, profile_id) =
        validator::validate_create_request(&state, &merchant_account, &req, &key_store).await?;

    // Create DB entries
    let mut payout_data = payout_create_db_entries(
        &state,
        &merchant_account,
        &key_store,
        &req,
        &payout_id,
        &profile_id,
        &connector_data.connector_name,
        payout_method_data.as_ref(),
    )
    .await?;

    call_connector_payout(
        &state,
        &merchant_account,
        &key_store,
        &req,
        connector_data,
        &mut payout_data,
    )
    .await
}

#[cfg(feature = "payouts")]
pub async fn payouts_update_core(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    req: payouts::PayoutCreateRequest,
) -> RouterResponse<payouts::PayoutCreateResponse> {
    let mut payout_data = make_payout_data(
        &state,
        &merchant_account,
        &key_store,
        &payouts::PayoutRequest::PayoutCreateRequest(req.to_owned()),
    )
    .await?;

    let payout_attempt = payout_data.payout_attempt.to_owned();
    let status = payout_attempt.status;

    // Verify update feasibility
    if helpers::is_payout_terminal_state(status) || helpers::is_payout_initiated(status) {
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
        destination_currency: req.currency.unwrap_or(payouts.destination_currency),
        source_currency: req.currency.unwrap_or(payouts.source_currency),
        description: req.description.clone().or(payouts.description),
        recurring: req.recurring.unwrap_or(payouts.recurring),
        auto_fulfill: req.auto_fulfill.unwrap_or(payouts.auto_fulfill),
        return_url: req.return_url.clone().or(payouts.return_url),
        entity_type: req.entity_type.unwrap_or(payouts.entity_type),
        metadata: req.metadata.clone().or(payouts.metadata),
        last_modified_at: Some(common_utils::date_time::now()),
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
                last_modified_at: Some(common_utils::date_time::now()),
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
    let connector_data: api::PayoutConnectorData = api::PayoutConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &payout_data.payout_attempt.connector,
        api::GetToken::Connector,
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to get the connector data")?;

    call_connector_payout(
        &state,
        &merchant_account,
        &key_store,
        &req,
        connector_data,
        &mut payout_data,
    )
    .await
}

#[cfg(feature = "payouts")]
#[instrument(skip_all)]
pub async fn payouts_retrieve_core(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    req: payouts::PayoutRetrieveRequest,
) -> RouterResponse<payouts::PayoutCreateResponse> {
    let payout_data = make_payout_data(
        &state,
        &merchant_account,
        &key_store,
        &payouts::PayoutRequest::PayoutRetrieveRequest(req.to_owned()),
    )
    .await?;

    response_handler(
        &state,
        &merchant_account,
        &payouts::PayoutRequest::PayoutRetrieveRequest(req.to_owned()),
        &payout_data,
    )
    .await
}

#[cfg(feature = "payouts")]
#[instrument(skip_all)]
pub async fn payouts_cancel_core(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    req: payouts::PayoutActionRequest,
) -> RouterResponse<payouts::PayoutCreateResponse> {
    let mut payout_data = make_payout_data(
        &state,
        &merchant_account,
        &key_store,
        &payouts::PayoutRequest::PayoutActionRequest(req.to_owned()),
    )
    .await?;

    let payout_attempt = payout_data.payout_attempt.to_owned();
    let connector_payout_id = payout_attempt.connector_payout_id.to_owned();
    let status = payout_attempt.status;

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
            last_modified_at: Some(common_utils::date_time::now()),
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
        // Form connector data
        let connector_data = get_connector_data(
            &state,
            &merchant_account,
            Some(payout_attempt.connector),
            None,
        )
        .await?;

        payout_data = cancel_payout(
            &state,
            &merchant_account,
            &key_store,
            &payouts::PayoutRequest::PayoutActionRequest(req.to_owned()),
            &connector_data,
            &mut payout_data,
        )
        .await
        .attach_printable("Payout cancellation failed for given Payout request")?;
    }

    response_handler(
        &state,
        &merchant_account,
        &payouts::PayoutRequest::PayoutActionRequest(req.to_owned()),
        &payout_data,
    )
    .await
}

#[cfg(feature = "payouts")]
#[instrument(skip_all)]
pub async fn payouts_fulfill_core(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    req: payouts::PayoutActionRequest,
) -> RouterResponse<payouts::PayoutCreateResponse> {
    let mut payout_data = make_payout_data(
        &state,
        &merchant_account,
        &key_store,
        &payouts::PayoutRequest::PayoutActionRequest(req.to_owned()),
    )
    .await?;

    let payout_attempt = payout_data.payout_attempt.to_owned();
    let status = payout_attempt.status;

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

    // Form connector data
    let connector_data = get_connector_data(
        &state,
        &merchant_account,
        Some(payout_attempt.connector.clone()),
        None,
    )
    .await?;

    // Trigger fulfillment
    payout_data.payout_method_data = Some(
        helpers::make_payout_method_data(
            &state,
            None,
            payout_attempt.payout_token.as_deref(),
            &payout_attempt.customer_id,
            &payout_attempt.merchant_id,
            &payout_attempt.payout_id,
            Some(&payout_data.payouts.payout_type),
            &key_store,
        )
        .await?
        .get_required_value("payout_method_data")?,
    );
    payout_data = fulfill_payout(
        &state,
        &merchant_account,
        &key_store,
        &payouts::PayoutRequest::PayoutActionRequest(req.to_owned()),
        &connector_data,
        &mut payout_data,
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
        &state,
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
    key_store: &domain::MerchantKeyStore,
    req: &payouts::PayoutCreateRequest,
    connector_data: api::PayoutConnectorData,
    payout_data: &mut PayoutData,
) -> RouterResponse<payouts::PayoutCreateResponse> {
    let payout_attempt = &payout_data.payout_attempt.to_owned();
    let payouts: &diesel_models::payouts::Payouts = &payout_data.payouts.to_owned();
    // Fetch / store payout_method_data
    if payout_data.payout_method_data.is_none() || payout_attempt.payout_token.is_none() {
        payout_data.payout_method_data = Some(
            helpers::make_payout_method_data(
                state,
                req.payout_method_data.as_ref(),
                payout_attempt.payout_token.as_deref(),
                &payout_attempt.customer_id,
                &payout_attempt.merchant_id,
                &payout_attempt.payout_id,
                Some(&payouts.payout_type),
                key_store,
            )
            .await?
            .get_required_value("payout_method_data")?,
        );
    }
    if let Some(true) = req.confirm {
        // Eligibility flow
        if payouts.payout_type == storage_enums::PayoutType::Card
            && payout_attempt.is_eligible.is_none()
        {
            *payout_data = check_payout_eligibility(
                state,
                merchant_account,
                key_store,
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
            // Create customer flow
            *payout_data = create_recipient(
                state,
                merchant_account,
                key_store,
                req,
                &connector_data,
                payout_data,
            )
            .await
            .attach_printable("Creation of customer failed")?;

            // Create payout flow
            *payout_data = create_payout(
                state,
                merchant_account,
                key_store,
                req,
                &connector_data,
                payout_data,
            )
            .await
            .attach_printable("Payout creation failed for given Payout request")?;
        }

        if payout_data.payouts.payout_type == storage_enums::PayoutType::Wallet
            && payout_data.payout_attempt.status == storage_enums::PayoutStatus::RequiresCreation
        {
            // Create payout flow
            *payout_data = create_payout(
                state,
                merchant_account,
                key_store,
                req,
                &connector_data,
                payout_data,
            )
            .await
            .attach_printable("Payout creation failed for given Payout request")?;
        }
    };

    // Auto fulfillment flow
    let status = payout_data.payout_attempt.status;
    if payouts.auto_fulfill && status == storage_enums::PayoutStatus::RequiresFulfillment {
        *payout_data = fulfill_payout(
            state,
            merchant_account,
            key_store,
            &payouts::PayoutRequest::PayoutCreateRequest(req.to_owned()),
            &connector_data,
            payout_data,
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
pub async fn create_recipient(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    req: &payouts::PayoutCreateRequest,
    connector_data: &api::PayoutConnectorData,
    payout_data: &mut PayoutData,
) -> RouterResult<PayoutData> {
    let customer_details = payout_data.customer_details.to_owned();
    let connector_name = connector_data.connector_name.to_string();

    // Create the connector label using {profile_id}_{connector_name}
    let connector_label = format!("{}_{}", payout_data.profile_id, connector_name);

    let (should_call_connector, _connector_customer_id) =
        helpers::should_call_payout_connector_create_customer(
            state,
            connector_data,
            &customer_details,
            &connector_label,
        );
    if should_call_connector {
        // 1. Form router data
        let customer_router_data = core_utils::construct_payout_router_data(
            state,
            &connector_name,
            merchant_account,
            key_store,
            &payouts::PayoutRequest::PayoutCreateRequest(req.to_owned()),
            payout_data,
        )
        .await?;

        // 2. Fetch connector integration details
        let connector_integration: services::BoxedConnectorIntegration<
            '_,
            api::PoRecipient,
            types::PayoutsData,
            types::PayoutsResponseData,
        > = connector_data.connector.get_connector_integration();

        // 3. Call connector service
        let router_resp = services::execute_connector_processing_step(
            state,
            connector_integration,
            &customer_router_data,
            payments::CallConnectorAction::Trigger,
            None,
        )
        .await
        .to_payout_failed_response()?;

        match router_resp.response {
            Ok(recipient_create_data) => {
                if let Some(customer) = customer_details {
                    let db = &*state.store;
                    let customer_id = customer.customer_id.to_owned();
                    let merchant_id = merchant_account.merchant_id.to_owned();
                    let updated_customer = storage::CustomerUpdate::ConnectorCustomer {
                        connector_customer: Some(
                            serde_json::json!({connector_label: recipient_create_data.connector_payout_id}),
                        ),
                    };
                    payout_data.customer_details = Some(
                        db.update_customer_by_customer_id_merchant_id(
                            customer_id,
                            merchant_id,
                            updated_customer,
                            key_store,
                        )
                        .await
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Error updating customers in db")?,
                    )
                }
            }
            Err(err) => Err(errors::ApiErrorResponse::PayoutFailed {
                data: serde_json::to_value(err).ok(),
            })?,
        }
    }
    Ok(payout_data.clone())
}

#[cfg(feature = "payouts")]
pub async fn check_payout_eligibility(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    req: &payouts::PayoutCreateRequest,
    connector_data: &api::PayoutConnectorData,
    payout_data: &mut PayoutData,
) -> RouterResult<PayoutData> {
    // 1. Form Router data
    let router_data = core_utils::construct_payout_router_data(
        state,
        &connector_data.connector_name.to_string(),
        merchant_account,
        key_store,
        &payouts::PayoutRequest::PayoutCreateRequest(req.to_owned()),
        payout_data,
    )
    .await?;

    // 2. Fetch connector integration details
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        api::PoEligibility,
        types::PayoutsData,
        types::PayoutsResponseData,
    > = connector_data.connector.get_connector_integration();

    // 3. Call connector service
    let router_data_resp = services::execute_connector_processing_step(
        state,
        connector_integration,
        &router_data,
        payments::CallConnectorAction::Trigger,
        None,
    )
    .await
    .to_payout_failed_response()?;

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
                    last_modified_at: Some(common_utils::date_time::now()),
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
            if helpers::is_payout_err_state(status) {
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
                    last_modified_at: Some(common_utils::date_time::now()),
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
    key_store: &domain::MerchantKeyStore,
    req: &payouts::PayoutCreateRequest,
    connector_data: &api::PayoutConnectorData,
    payout_data: &mut PayoutData,
) -> RouterResult<PayoutData> {
    // 1. Form Router data
    let mut router_data = core_utils::construct_payout_router_data(
        state,
        &connector_data.connector_name.to_string(),
        merchant_account,
        key_store,
        &payouts::PayoutRequest::PayoutCreateRequest(req.to_owned()),
        payout_data,
    )
    .await?;

    // 2. Fetch connector integration details
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        api::PoCreate,
        types::PayoutsData,
        types::PayoutsResponseData,
    > = connector_data.connector.get_connector_integration();

    // 3. Execute pretasks
    connector_integration
        .execute_pretasks(&mut router_data, state)
        .await
        .to_payout_failed_response()?;

    // 4. Call connector service
    let router_data_resp = services::execute_connector_processing_step(
        state,
        connector_integration,
        &router_data,
        payments::CallConnectorAction::Trigger,
        None,
    )
    .await
    .to_payout_failed_response()?;

    // 5. Process data returned by the connector
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
                    last_modified_at: Some(common_utils::date_time::now()),
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
            if helpers::is_payout_err_state(status) {
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
                    last_modified_at: Some(common_utils::date_time::now()),
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
pub async fn cancel_payout(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    req: &payouts::PayoutRequest,
    connector_data: &api::PayoutConnectorData,
    payout_data: &mut PayoutData,
) -> RouterResult<PayoutData> {
    // 1. Form Router data
    let router_data = core_utils::construct_payout_router_data(
        state,
        &connector_data.connector_name.to_string(),
        merchant_account,
        key_store,
        req,
        payout_data,
    )
    .await?;

    // 2. Fetch connector integration details
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        api::PoCancel,
        types::PayoutsData,
        types::PayoutsResponseData,
    > = connector_data.connector.get_connector_integration();

    // 3. Call connector service
    let router_data_resp = services::execute_connector_processing_step(
        state,
        connector_integration,
        &router_data,
        payments::CallConnectorAction::Trigger,
        None,
    )
    .await
    .to_payout_failed_response()?;

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
                    last_modified_at: Some(common_utils::date_time::now()),
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
                    last_modified_at: Some(common_utils::date_time::now()),
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
    key_store: &domain::MerchantKeyStore,
    req: &payouts::PayoutRequest,
    connector_data: &api::PayoutConnectorData,
    payout_data: &mut PayoutData,
) -> RouterResult<PayoutData> {
    // 1. Form Router data
    let router_data = core_utils::construct_payout_router_data(
        state,
        &connector_data.connector_name.to_string(),
        merchant_account,
        key_store,
        req,
        payout_data,
    )
    .await?;

    // 2. Fetch connector integration details
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        api::PoFulfill,
        types::PayoutsData,
        types::PayoutsResponseData,
    > = connector_data.connector.get_connector_integration();

    // 3. Call connector service
    let router_data_resp = services::execute_connector_processing_step(
        state,
        connector_integration,
        &router_data,
        payments::CallConnectorAction::Trigger,
        None,
    )
    .await
    .to_payout_failed_response()?;

    // 4. Process data returned by the connector
    let db = &*state.store;
    let merchant_id = &merchant_account.merchant_id;
    let payout_attempt = &payout_data.payout_attempt;
    let payout_id = &payout_attempt.payout_id;
    match router_data_resp.response {
        Ok(payout_response_data) => {
            if payout_data.payouts.recurring && payout_data.payouts.payout_method_id.is_none() {
                helpers::save_payout_data_to_locker(
                    state,
                    payout_attempt,
                    &payout_data
                        .payout_method_data
                        .clone()
                        .get_required_value("payout_method_data")?,
                    merchant_account,
                    key_store,
                )
                .await?;
            }
            let status = payout_response_data
                .status
                .unwrap_or(payout_attempt.status.to_owned());
            let updated_payouts = storage::payout_attempt::PayoutAttemptUpdate::StatusUpdate {
                connector_payout_id: payout_attempt.connector_payout_id.to_owned(),
                status,
                error_code: None,
                error_message: None,
                is_eligible: payout_response_data.payout_eligible,
                last_modified_at: Some(common_utils::date_time::now()),
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
            if helpers::is_payout_err_state(status) {
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
                last_modified_at: Some(common_utils::date_time::now()),
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
            email: a.email.to_owned().map(|inner| pii::Email::from(inner)),
        }
    });

    let response = api::PayoutCreateResponse {
        payout_id: payouts.payout_id.to_owned(),
        merchant_id: merchant_account.merchant_id.to_owned(),
        amount: payouts.amount.to_owned(),
        currency: payouts.destination_currency.to_owned(),
        connector: Some(payout_attempt.connector.to_owned()),
        payout_type: payouts.payout_type.to_owned(),
        billing: address,
        customer_id,
        auto_fulfill: payouts.auto_fulfill,
        email,
        name,
        phone,
        phone_country_code,
        client_secret: None,
        return_url: payouts.return_url.to_owned(),
        business_country: payout_attempt.business_country,
        business_label: payout_attempt.business_label,
        description: payouts.description.to_owned(),
        entity_type: payouts.entity_type.to_owned(),
        recurring: payouts.recurring,
        metadata: payouts.metadata,
        status: payout_attempt.status.to_owned(),
        error_message: payout_attempt.error_message.to_owned(),
        error_code: payout_attempt.error_code,
        profile_id: payout_attempt.profile_id,
    };
    Ok(services::ApplicationResponse::Json(response))
}

// DB entries
#[allow(clippy::too_many_arguments)]
#[cfg(feature = "payouts")]
pub async fn payout_create_db_entries(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    req: &payouts::PayoutCreateRequest,
    payout_id: &String,
    profile_id: &String,
    connector_name: &api_enums::PayoutConnectors,
    stored_payout_method_data: Option<&payouts::PayoutMethodData>,
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
    let customer = helpers::get_or_create_customer_details(
        state,
        &customer_details,
        merchant_account,
        key_store,
    )
    .await?;
    let customer_id = customer
        .to_owned()
        .ok_or_else(|| {
            report!(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "customer_id",
            })
        })?
        .customer_id;

    // Get or create address
    let billing_address = payment_helpers::create_or_find_address_for_payment_by_request(
        db,
        req.billing.as_ref(),
        None,
        merchant_id,
        Some(&customer_id.to_owned()),
        key_store,
        payout_id,
        merchant_account.storage_scheme,
    )
    .await?;
    let address_id = billing_address
        .to_owned()
        .ok_or_else(|| {
            report!(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "billing.address",
            })
        })?
        .address_id;

    // Make payouts entry
    let currency = req.currency.to_owned().get_required_value("currency")?;
    let payout_type = req
        .payout_type
        .to_owned()
        .get_required_value("payout_type")?;

    let payout_method_id = if stored_payout_method_data.is_some() {
        req.payout_token.to_owned()
    } else {
        None
    };

    let payouts_req = storage::PayoutsNew::default()
        .set_payout_id(payout_id.to_owned())
        .set_merchant_id(merchant_id.to_owned())
        .set_customer_id(customer_id.to_owned())
        .set_address_id(address_id.to_owned())
        .set_payout_type(payout_type)
        .set_amount(req.amount.unwrap_or(api::Amount::Zero).into())
        .set_destination_currency(currency)
        .set_source_currency(currency)
        .set_description(req.description.to_owned())
        .set_recurring(req.recurring.unwrap_or(false))
        .set_auto_fulfill(req.auto_fulfill.unwrap_or(false))
        .set_return_url(req.return_url.to_owned())
        .set_entity_type(req.entity_type.unwrap_or_default())
        .set_metadata(req.metadata.to_owned())
        .set_created_at(Some(common_utils::date_time::now()))
        .set_last_modified_at(Some(common_utils::date_time::now()))
        .set_payout_method_id(payout_method_id)
        .to_owned();
    let payouts = db
        .insert_payout(payouts_req)
        .await
        .to_duplicate_response(errors::ApiErrorResponse::DuplicatePayout {
            payout_id: payout_id.to_owned(),
        })
        .attach_printable("Error inserting payouts in db")?;

    // Make payout_attempt entry
    let status = if req.payout_method_data.is_some()
        || req.payout_token.is_some()
        || stored_payout_method_data.is_some()
    {
        storage_enums::PayoutStatus::RequiresCreation
    } else {
        storage_enums::PayoutStatus::RequiresPayoutMethodData
    };
    let payout_attempt_id = utils::get_payment_attempt_id(payout_id, 1);

    let payout_attempt_req = storage::PayoutAttemptNew::default()
        .set_payout_attempt_id(payout_attempt_id.to_string())
        .set_payout_id(payout_id.to_owned())
        .set_customer_id(customer_id.to_owned())
        .set_merchant_id(merchant_id.to_owned())
        .set_address_id(address_id.to_owned())
        .set_connector(connector_name.to_string())
        .set_status(status)
        .set_business_country(req.business_country.to_owned())
        .set_business_label(req.business_label.to_owned())
        .set_payout_token(req.payout_token.to_owned())
        .set_created_at(Some(common_utils::date_time::now()))
        .set_last_modified_at(Some(common_utils::date_time::now()))
        .set_profile_id(Some(profile_id.to_string()))
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
        payout_method_data: req
            .payout_method_data
            .as_ref()
            .cloned()
            .or(stored_payout_method_data.cloned()),
        merchant_connector_account: None,
        profile_id: profile_id.to_owned(),
    })
}

#[cfg(feature = "payouts")]
pub async fn make_payout_data(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
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

    let billing_address = payment_helpers::create_or_find_address_for_payment_by_request(
        db,
        None,
        Some(&payouts.address_id.to_owned()),
        merchant_id,
        Some(&payouts.customer_id.to_owned()),
        key_store,
        &payouts.payout_id,
        merchant_account.storage_scheme,
    )
    .await?;

    let customer_details = db
        .find_customer_optional_by_customer_id_merchant_id(
            &payouts.customer_id.to_owned(),
            merchant_id,
            key_store,
        )
        .await
        .map_or(None, |c| c);

    let profile_id = payout_attempt.profile_id.clone();

    Ok(PayoutData {
        billing_address,
        customer_details,
        payouts,
        payout_attempt,
        payout_method_data: None,
        merchant_connector_account: None,
        profile_id,
    })
}
