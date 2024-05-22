pub mod access_token;
pub mod helpers;
#[cfg(feature = "payout_retry")]
pub mod retry;
pub mod validator;
use std::vec::IntoIter;

use api_models::enums as api_enums;
use common_utils::{consts, crypto::Encryptable, ext_traits::ValueExt, pii, types::MinorUnit};
use diesel_models::enums as storage_enums;
use error_stack::{report, ResultExt};
#[cfg(feature = "olap")]
use futures::future::join_all;
#[cfg(feature = "olap")]
use hyperswitch_domain_models::errors::StorageError;
#[cfg(feature = "payout_retry")]
use retry::GsmValidation;
#[cfg(feature = "olap")]
use router_env::logger;
use router_env::{instrument, tracing};
use scheduler::utils as pt_utils;
use serde_json;

use super::{
    errors::{ConnectorErrorExt, StorageErrorExt},
    payments::customers,
};
#[cfg(feature = "olap")]
use crate::types::{domain::behaviour::Conversion, transformers::ForeignFrom};
use crate::{
    core::{
        errors::{self, CustomResult, RouterResponse, RouterResult},
        payments::{self, helpers as payment_helpers},
        utils as core_utils,
    },
    db::StorageInterface,
    routes::AppState,
    services,
    types::{
        self,
        api::{self, payouts},
        domain,
        storage::{self, PaymentRoutingInfo},
    },
    utils::{self, OptionExt},
};

// ********************************************** TYPES **********************************************
#[derive(Clone)]
pub struct PayoutData {
    pub billing_address: Option<domain::Address>,
    pub business_profile: storage::BusinessProfile,
    pub customer_details: Option<domain::Customer>,
    pub merchant_connector_account: Option<payment_helpers::MerchantConnectorAccountType>,
    pub payouts: storage::Payouts,
    pub payout_attempt: storage::PayoutAttempt,
    pub payout_method_data: Option<payouts::PayoutMethodData>,
    pub profile_id: String,
    pub should_terminate: bool,
}

// ********************************************** CORE FLOWS **********************************************
pub fn get_next_connector(
    connectors: &mut IntoIter<api::ConnectorData>,
) -> RouterResult<api::ConnectorData> {
    connectors
        .next()
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Connector not found in connectors iterator")
}

#[cfg(feature = "payouts")]
pub async fn get_connector_choice(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    connector: Option<String>,
    routing_algorithm: Option<serde_json::Value>,
    payout_data: &mut PayoutData,
    eligible_connectors: Option<Vec<api_models::enums::PayoutConnectors>>,
) -> RouterResult<api::ConnectorCallType> {
    let eligible_routable_connectors = eligible_connectors.map(|connectors| {
        connectors
            .into_iter()
            .map(api::enums::RoutableConnectors::from)
            .collect()
    });
    let connector_choice = helpers::get_default_payout_connector(state, routing_algorithm).await?;
    match connector_choice {
        api::ConnectorChoice::SessionMultiple(_) => {
            Err(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Invalid connector choice - SessionMultiple")?
        }

        api::ConnectorChoice::StraightThrough(straight_through) => {
            let request_straight_through: api::routing::StraightThroughAlgorithm = straight_through
                .clone()
                .parse_value("StraightThroughAlgorithm")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Invalid straight through routing rules format")?;
            payout_data.payout_attempt.routing_info = Some(straight_through);
            let mut routing_data = storage::RoutingData {
                routed_through: connector,
                #[cfg(feature = "connector_choice_mca_id")]
                merchant_connector_id: None,
                #[cfg(not(feature = "connector_choice_mca_id"))]
                business_sub_label: payout_data.payout_attempt.business_label.clone(),
                algorithm: Some(request_straight_through.clone()),
                routing_info: PaymentRoutingInfo {
                    algorithm: None,
                    pre_routing_results: None,
                },
            };
            helpers::decide_payout_connector(
                state,
                merchant_account,
                key_store,
                Some(request_straight_through),
                &mut routing_data,
                payout_data,
                eligible_routable_connectors,
            )
            .await
        }

        api::ConnectorChoice::Decide => {
            let mut routing_data = storage::RoutingData {
                routed_through: connector,
                #[cfg(feature = "connector_choice_mca_id")]
                merchant_connector_id: None,
                #[cfg(not(feature = "connector_choice_mca_id"))]
                business_sub_label: payout_data.payout_attempt.business_label.clone(),
                algorithm: None,
                routing_info: PaymentRoutingInfo {
                    algorithm: None,
                    pre_routing_results: None,
                },
            };
            helpers::decide_payout_connector(
                state,
                merchant_account,
                key_store,
                None,
                &mut routing_data,
                payout_data,
                eligible_routable_connectors,
            )
            .await
        }
    }
}

#[instrument(skip_all)]
pub async fn make_connector_decision(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    connector_call_type: api::ConnectorCallType,
    payout_data: &mut PayoutData,
) -> RouterResult<()> {
    match connector_call_type {
        api::ConnectorCallType::PreDetermined(connector_data) => {
            call_connector_payout(
                state,
                merchant_account,
                key_store,
                &connector_data,
                payout_data,
            )
            .await?;

            #[cfg(feature = "payout_retry")]
            {
                let config_bool = retry::config_should_call_gsm_payout(
                    &*state.store,
                    &merchant_account.merchant_id,
                    retry::PayoutRetryType::SingleConnector,
                )
                .await;

                if config_bool && payout_data.should_call_gsm() {
                    Box::pin(retry::do_gsm_single_connector_actions(
                        state,
                        connector_data,
                        payout_data,
                        merchant_account,
                        key_store,
                    ))
                    .await?;
                }
            }

            Ok(())
        }
        api::ConnectorCallType::Retryable(connectors) => {
            let mut connectors = connectors.into_iter();

            let connector_data = get_next_connector(&mut connectors)?;

            call_connector_payout(
                state,
                merchant_account,
                key_store,
                &connector_data,
                payout_data,
            )
            .await?;

            #[cfg(feature = "payout_retry")]
            {
                let config_multiple_connector_bool = retry::config_should_call_gsm_payout(
                    &*state.store,
                    &merchant_account.merchant_id,
                    retry::PayoutRetryType::MultiConnector,
                )
                .await;

                if config_multiple_connector_bool && payout_data.should_call_gsm() {
                    Box::pin(retry::do_gsm_multiple_connector_actions(
                        state,
                        connectors,
                        connector_data.clone(),
                        payout_data,
                        merchant_account,
                        key_store,
                    ))
                    .await?;
                }

                let config_single_connector_bool = retry::config_should_call_gsm_payout(
                    &*state.store,
                    &merchant_account.merchant_id,
                    retry::PayoutRetryType::SingleConnector,
                )
                .await;

                if config_single_connector_bool && payout_data.should_call_gsm() {
                    Box::pin(retry::do_gsm_single_connector_actions(
                        state,
                        connector_data,
                        payout_data,
                        merchant_account,
                        key_store,
                    ))
                    .await?;
                }
            }

            Ok(())
        }
        _ => Err(errors::ApiErrorResponse::InternalServerError)?,
    }
}

#[instrument(skip_all)]
pub async fn payouts_core(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    payout_data: &mut PayoutData,
    routing_algorithm: Option<serde_json::Value>,
    eligible_connectors: Option<Vec<api_models::enums::PayoutConnectors>>,
) -> RouterResult<()> {
    let payout_attempt = &payout_data.payout_attempt;

    // Form connector data
    let connector_call_type = get_connector_choice(
        state,
        merchant_account,
        key_store,
        payout_attempt.connector.clone(),
        routing_algorithm,
        payout_data,
        eligible_connectors,
    )
    .await?;

    // Call connector steps
    Box::pin(make_connector_decision(
        state,
        merchant_account,
        key_store,
        connector_call_type,
        payout_data,
    ))
    .await
}

#[instrument(skip_all)]
pub async fn payouts_create_core(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    req: payouts::PayoutCreateRequest,
) -> RouterResponse<payouts::PayoutCreateResponse> {
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
        payout_method_data.as_ref(),
    )
    .await?;

    let payout_attempt = payout_data.payout_attempt.to_owned();

    // Persist payout method data in temp locker
    payout_data.payout_method_data = helpers::make_payout_method_data(
        &state,
        req.payout_method_data.as_ref(),
        payout_attempt.payout_token.as_deref(),
        &payout_attempt.customer_id,
        &payout_attempt.merchant_id,
        Some(&payout_data.payouts.payout_type.clone()),
        &key_store,
        Some(&mut payout_data),
        merchant_account.storage_scheme,
    )
    .await?;

    payouts_core(
        &state,
        &merchant_account,
        &key_store,
        &mut payout_data,
        req.routing.clone(),
        req.connector.clone(),
    )
    .await?;

    response_handler(&merchant_account, &payout_data).await
}

pub async fn payouts_update_core(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    req: payouts::PayoutCreateRequest,
) -> RouterResponse<payouts::PayoutCreateResponse> {
    let payout_id = req.payout_id.clone().get_required_value("payout_id")?;
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
                payout_id, status
            ),
        }));
    }
    // Update DB with new data
    let payouts = payout_data.payouts.to_owned();
    let amount = MinorUnit::from(req.amount.unwrap_or(MinorUnit::new(payouts.amount).into()))
        .get_amount_as_i64();
    let updated_payouts = storage::PayoutsUpdate::Update {
        amount,
        destination_currency: req.currency.unwrap_or(payouts.destination_currency),
        source_currency: req.currency.unwrap_or(payouts.source_currency),
        description: req.description.clone().or(payouts.description.clone()),
        recurring: req.recurring.unwrap_or(payouts.recurring),
        auto_fulfill: req.auto_fulfill.unwrap_or(payouts.auto_fulfill),
        return_url: req.return_url.clone().or(payouts.return_url.clone()),
        entity_type: req.entity_type.unwrap_or(payouts.entity_type),
        metadata: req.metadata.clone().or(payouts.metadata.clone()),
        status: Some(status),
        profile_id: Some(payout_attempt.profile_id.clone()),
        confirm: req.confirm,
    };

    let db = &*state.store;
    payout_data.payouts = db
        .update_payout(
            &payouts,
            updated_payouts,
            &payout_attempt,
            merchant_account.storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error updating payouts")?;

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
            let payout_attempt = payout_data.payout_attempt;
            let updated_payout_attempt = storage::PayoutAttemptUpdate::BusinessUpdate {
                business_country,
                business_label,
            };
            payout_data.payout_attempt = db
                .update_payout_attempt(
                    &payout_attempt,
                    updated_payout_attempt,
                    &payout_data.payouts,
                    merchant_account.storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating payout_attempt")?;
        }
    }

    let payout_attempt = payout_data.payout_attempt.to_owned();

    if (req.connector.is_none(), payout_attempt.connector.is_some()) != (true, true) {
        // if the connector is not updated but was provided during payout create
        payout_data.payout_attempt.connector = None;
        payout_data.payout_attempt.routing_info = None;
    };

    // Update payout method data in temp locker
    payout_data.payout_method_data = helpers::make_payout_method_data(
        &state,
        req.payout_method_data.as_ref(),
        payout_attempt.payout_token.as_deref(),
        &payout_attempt.customer_id,
        &payout_attempt.merchant_id,
        Some(&payout_data.payouts.payout_type.clone()),
        &key_store,
        Some(&mut payout_data),
        merchant_account.storage_scheme,
    )
    .await?;

    payouts_core(
        &state,
        &merchant_account,
        &key_store,
        &mut payout_data,
        req.routing.clone(),
        req.connector.clone(),
    )
    .await?;

    response_handler(&merchant_account, &payout_data).await
}

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

    response_handler(&merchant_account, &payout_data).await
}

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
        let status = storage_enums::PayoutStatus::Cancelled;
        let updated_payout_attempt = storage::PayoutAttemptUpdate::StatusUpdate {
            connector_payout_id: connector_payout_id.to_owned(),
            status,
            error_message: Some("Cancelled by user".to_string()),
            error_code: None,
            is_eligible: None,
        };
        payout_data.payout_attempt = state
            .store
            .update_payout_attempt(
                &payout_attempt,
                updated_payout_attempt,
                &payout_data.payouts,
                merchant_account.storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error updating payout_attempt in db")?;
        payout_data.payouts = state
            .store
            .update_payout(
                &payout_data.payouts,
                storage::PayoutsUpdate::StatusUpdate { status },
                &payout_data.payout_attempt,
                merchant_account.storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error updating payouts in db")?;

    // Trigger connector's cancellation
    } else {
        // Form connector data
        let connector_data = match &payout_attempt.connector {
            Some(connector) => api::ConnectorData::get_payout_connector_by_name(
                &state.conf.connectors,
                connector,
                api::GetToken::Connector,
                payout_attempt.merchant_connector_id.clone(),
            )
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to get the connector data")?,
            _ => Err(errors::ApplicationError::InvalidConfigurationValueError(
                "Connector not found in payout_attempt - should not reach here".to_string(),
            ))
            .change_context(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "connector",
            })
            .attach_printable("Connector not found for payout cancellation")?,
        };

        cancel_payout(
            &state,
            &merchant_account,
            &key_store,
            &connector_data,
            &mut payout_data,
        )
        .await
        .attach_printable("Payout cancellation failed for given Payout request")?;
    }

    response_handler(&merchant_account, &payout_data).await
}

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
    let connector_data = match &payout_attempt.connector {
        Some(connector) => api::ConnectorData::get_payout_connector_by_name(
            &state.conf.connectors,
            connector,
            api::GetToken::Connector,
            payout_attempt.merchant_connector_id.clone(),
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get the connector data")?,
        _ => Err(errors::ApplicationError::InvalidConfigurationValueError(
            "Connector not found in payout_attempt - should not reach here.".to_string(),
        ))
        .change_context(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "connector",
        })
        .attach_printable("Connector not found for payout fulfillment")?,
    };

    // Trigger fulfillment
    payout_data.payout_method_data = Some(
        helpers::make_payout_method_data(
            &state,
            None,
            payout_attempt.payout_token.as_deref(),
            &payout_attempt.customer_id,
            &payout_attempt.merchant_id,
            Some(&payout_data.payouts.payout_type.clone()),
            &key_store,
            Some(&mut payout_data),
            merchant_account.storage_scheme,
        )
        .await?
        .get_required_value("payout_method_data")?,
    );
    fulfill_payout(
        &state,
        &merchant_account,
        &key_store,
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

    response_handler(&merchant_account, &payout_data).await
}

#[cfg(feature = "olap")]
pub async fn payouts_list_core(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    constraints: payouts::PayoutListConstraints,
) -> RouterResponse<payouts::PayoutListResponse> {
    validator::validate_payout_list_request(&constraints)?;
    let merchant_id = &merchant_account.merchant_id;
    let db = state.store.as_ref();
    let payouts = helpers::filter_by_constraints(
        db,
        &constraints,
        merchant_id,
        merchant_account.storage_scheme,
    )
    .await
    .to_not_found_response(errors::ApiErrorResponse::PayoutNotFound)?;

    let collected_futures = payouts.into_iter().map(|payouts| async {
        match db
            .find_payout_attempt_by_merchant_id_payout_attempt_id(
                merchant_id,
                &utils::get_payment_attempt_id(payouts.payout_id.clone(), payouts.attempt_count),
                storage_enums::MerchantStorageScheme::PostgresOnly,
            )
            .await
        {
            Ok(payout_attempt) => {
                match db
                    .find_customer_by_customer_id_merchant_id(
                        &payouts.customer_id,
                        merchant_id,
                        &key_store,
                        merchant_account.storage_scheme,
                    )
                    .await
                {
                    Ok(customer) => Some(Ok((payouts, payout_attempt, customer))),
                    Err(error) => {
                        if matches!(
                            error.current_context(),
                            storage_impl::errors::StorageError::ValueNotFound(_)
                        ) {
                            logger::warn!(
                                ?error,
                                "customer missing for customer_id : {}",
                                payouts.customer_id,
                            );
                            return None;
                        }
                        Some(Err(error.change_context(StorageError::ValueNotFound(
                            format!("customer missing for customer_id : {}", payouts.customer_id),
                        ))))
                    }
                }
            }
            Err(error) => {
                if matches!(error.current_context(), StorageError::ValueNotFound(_)) {
                    logger::warn!(
                        ?error,
                        "payout_attempt missing for payout_id : {}",
                        payouts.payout_id,
                    );
                    return None;
                }
                Some(Err(error))
            }
        }
    });

    let pi_pa_tuple_vec: Result<
        Vec<(storage::Payouts, storage::PayoutAttempt, domain::Customer)>,
        _,
    > = join_all(collected_futures)
        .await
        .into_iter()
        .flatten()
        .collect::<Result<Vec<(storage::Payouts, storage::PayoutAttempt, domain::Customer)>, _>>();

    let data: Vec<api::PayoutCreateResponse> = pi_pa_tuple_vec
        .change_context(errors::ApiErrorResponse::InternalServerError)?
        .into_iter()
        .map(ForeignFrom::foreign_from)
        .collect();

    Ok(services::ApplicationResponse::Json(
        api::PayoutListResponse {
            size: data.len(),
            data,
        },
    ))
}

#[cfg(feature = "olap")]
pub async fn payouts_filtered_list_core(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    filters: payouts::PayoutListFilterConstraints,
) -> RouterResponse<payouts::PayoutListResponse> {
    let limit = &filters.limit;
    validator::validate_payout_list_request_for_joins(*limit)?;
    let db = state.store.as_ref();
    let list: Vec<(
        storage::Payouts,
        storage::PayoutAttempt,
        diesel_models::Customer,
    )> = db
        .filter_payouts_and_attempts(
            &merchant_account.merchant_id,
            &filters.clone().into(),
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PayoutNotFound)?;

    let data: Vec<api::PayoutCreateResponse> = join_all(list.into_iter().map(|(p, pa, c)| async {
        match domain::Customer::convert_back(c, &key_store.key).await {
            Ok(domain_cust) => Some((p, pa, domain_cust)),
            Err(err) => {
                logger::warn!(?err, "failed to convert customer for id: {}", p.customer_id);
                None
            }
        }
    }))
    .await
    .into_iter()
    .flatten()
    .map(ForeignFrom::foreign_from)
    .collect();

    Ok(services::ApplicationResponse::Json(
        api::PayoutListResponse {
            size: data.len(),
            data,
        },
    ))
}

#[cfg(feature = "olap")]
pub async fn payouts_list_available_filters_core(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    time_range: api::TimeRange,
) -> RouterResponse<api::PayoutListFilters> {
    let db = state.store.as_ref();
    let payout = db
        .filter_payouts_by_time_range_constraints(
            &merchant_account.merchant_id,
            &time_range,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    let filters = db
        .get_filters_for_payouts(
            payout.as_slice(),
            &merchant_account.merchant_id,
            storage_enums::MerchantStorageScheme::PostgresOnly,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    Ok(services::ApplicationResponse::Json(
        api::PayoutListFilters {
            connector: filters.connector,
            currency: filters.currency,
            status: filters.status,
            payout_method: filters.payout_method,
        },
    ))
}

// ********************************************** HELPERS **********************************************
pub async fn call_connector_payout(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    connector_data: &api::ConnectorData,
    payout_data: &mut PayoutData,
) -> RouterResult<()> {
    let payout_attempt = &payout_data.payout_attempt.to_owned();
    let payouts = &payout_data.payouts.to_owned();

    // update connector_name
    if payout_data.payout_attempt.connector.is_none()
        || payout_data.payout_attempt.connector != Some(connector_data.connector_name.to_string())
    {
        payout_data.payout_attempt.connector = Some(connector_data.connector_name.to_string());
        let updated_payout_attempt = storage::PayoutAttemptUpdate::UpdateRouting {
            connector: connector_data.connector_name.to_string(),
            routing_info: payout_data.payout_attempt.routing_info.clone(),
        };
        let db = &*state.store;
        payout_data.payout_attempt = db
            .update_payout_attempt(
                &payout_data.payout_attempt,
                updated_payout_attempt,
                payouts,
                merchant_account.storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error updating routing info in payout_attempt")?;
    };

    // Fetch / store payout_method_data
    if payout_data.payout_method_data.is_none() || payout_attempt.payout_token.is_none() {
        payout_data.payout_method_data = Some(
            helpers::make_payout_method_data(
                state,
                payout_data.payout_method_data.to_owned().as_ref(),
                payout_attempt.payout_token.as_deref(),
                &payout_attempt.customer_id,
                &payout_attempt.merchant_id,
                Some(&payouts.payout_type),
                key_store,
                Some(payout_data),
                merchant_account.storage_scheme,
            )
            .await?
            .get_required_value("payout_method_data")?,
        );
    }

    if let Some(true) = payouts.confirm {
        // Eligibility flow
        complete_payout_eligibility(
            state,
            merchant_account,
            key_store,
            connector_data,
            payout_data,
        )
        .await?;

        // Create customer flow
        complete_create_recipient(
            state,
            merchant_account,
            key_store,
            connector_data,
            payout_data,
        )
        .await?;

        // Create customer's disbursement account flow
        complete_create_recipient_disburse_account(
            state,
            merchant_account,
            key_store,
            connector_data,
            payout_data,
        )
        .await?;

        // Payout creation flow
        complete_create_payout(
            state,
            merchant_account,
            key_store,
            connector_data,
            payout_data,
        )
        .await?;
    };

    // Auto fulfillment flow
    let status = payout_data.payout_attempt.status;
    if payouts.auto_fulfill && status == storage_enums::PayoutStatus::RequiresFulfillment {
        fulfill_payout(
            state,
            merchant_account,
            key_store,
            connector_data,
            payout_data,
        )
        .await
        .attach_printable("Payout fulfillment failed for given Payout request")?;
    }

    Ok(())
}

pub async fn complete_create_recipient(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    connector_data: &api::ConnectorData,
    payout_data: &mut PayoutData,
) -> RouterResult<()> {
    if !payout_data.should_terminate
        && payout_data.payout_attempt.status == storage_enums::PayoutStatus::RequiresCreation
        && connector_data
            .connector_name
            .supports_create_recipient(payout_data.payouts.payout_type)
    {
        create_recipient(
            state,
            merchant_account,
            key_store,
            connector_data,
            payout_data,
        )
        .await
        .attach_printable("Creation of customer failed")?;
    }

    Ok(())
}

pub async fn create_recipient(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    connector_data: &api::ConnectorData,
    payout_data: &mut PayoutData,
) -> RouterResult<()> {
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
        let router_data = core_utils::construct_payout_router_data(
            state,
            &connector_data.connector_name,
            merchant_account,
            key_store,
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
            connector_integration.to_owned(),
            &router_data,
            payments::CallConnectorAction::Trigger,
            None,
        )
        .await
        .to_payout_failed_response()?;

        match router_resp.response {
            Ok(recipient_create_data) => {
                let db = &*state.store;
                if let Some(customer) = customer_details {
                    let customer_id = customer.customer_id.to_owned();
                    let merchant_id = merchant_account.merchant_id.to_owned();
                    if let Some(updated_customer) =
                        customers::update_connector_customer_in_customers(
                            &connector_label,
                            Some(&customer),
                            &Some(recipient_create_data.connector_payout_id.clone()),
                        )
                        .await
                    {
                        payout_data.customer_details = Some(
                            db.update_customer_by_customer_id_merchant_id(
                                customer_id,
                                merchant_id,
                                customer,
                                updated_customer,
                                key_store,
                                merchant_account.storage_scheme,
                            )
                            .await
                            .change_context(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("Error updating customers in db")?,
                        )
                    }
                }

                // Add next step to ProcessTracker
                if recipient_create_data.should_add_next_step_to_process_tracker {
                    add_external_account_addition_task(
                        &*state.store,
                        payout_data,
                        common_utils::date_time::now().saturating_add(time::Duration::seconds(consts::STRIPE_ACCOUNT_ONBOARDING_DELAY_IN_SECONDS)),
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed while adding attach_payout_account_workflow workflow to process tracker")?;

                    // Update payout status in DB
                    let status = recipient_create_data
                        .status
                        .unwrap_or(api_enums::PayoutStatus::RequiresVendorAccountCreation);
                    let updated_payout_attempt = storage::PayoutAttemptUpdate::StatusUpdate {
                        connector_payout_id: recipient_create_data.connector_payout_id,
                        status,
                        error_code: None,
                        error_message: None,
                        is_eligible: recipient_create_data.payout_eligible,
                    };
                    payout_data.payout_attempt = db
                        .update_payout_attempt(
                            &payout_data.payout_attempt,
                            updated_payout_attempt,
                            &payout_data.payouts,
                            merchant_account.storage_scheme,
                        )
                        .await
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Error updating payout_attempt in db")?;
                    payout_data.payouts = db
                        .update_payout(
                            &payout_data.payouts,
                            storage::PayoutsUpdate::StatusUpdate { status },
                            &payout_data.payout_attempt,
                            merchant_account.storage_scheme,
                        )
                        .await
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Error updating payouts in db")?;

                    // Helps callee functions skip the execution
                    payout_data.should_terminate = true;
                }
            }
            Err(err) => Err(errors::ApiErrorResponse::PayoutFailed {
                data: serde_json::to_value(err).ok(),
            })?,
        }
    }
    Ok(())
}

pub async fn complete_payout_eligibility(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    connector_data: &api::ConnectorData,
    payout_data: &mut PayoutData,
) -> RouterResult<()> {
    let payout_attempt = &payout_data.payout_attempt.to_owned();

    if !payout_data.should_terminate
        && payout_attempt.is_eligible.is_none()
        && connector_data
            .connector_name
            .supports_payout_eligibility(payout_data.payouts.payout_type)
    {
        check_payout_eligibility(
            state,
            merchant_account,
            key_store,
            connector_data,
            payout_data,
        )
        .await
        .attach_printable("Eligibility failed for given Payout request")?;
    }

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

    Ok(())
}

pub async fn check_payout_eligibility(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    connector_data: &api::ConnectorData,
    payout_data: &mut PayoutData,
) -> RouterResult<()> {
    // 1. Form Router data
    let router_data = core_utils::construct_payout_router_data(
        state,
        &connector_data.connector_name,
        merchant_account,
        key_store,
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
    match router_data_resp.response {
        Ok(payout_response_data) => {
            let payout_attempt = &payout_data.payout_attempt;
            let status = payout_response_data
                .status
                .unwrap_or(payout_attempt.status.to_owned());
            let updated_payout_attempt = storage::PayoutAttemptUpdate::StatusUpdate {
                connector_payout_id: payout_response_data.connector_payout_id,
                status,
                error_code: None,
                error_message: None,
                is_eligible: payout_response_data.payout_eligible,
            };
            payout_data.payout_attempt = db
                .update_payout_attempt(
                    payout_attempt,
                    updated_payout_attempt,
                    &payout_data.payouts,
                    merchant_account.storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating payout_attempt in db")?;
            payout_data.payouts = db
                .update_payout(
                    &payout_data.payouts,
                    storage::PayoutsUpdate::StatusUpdate { status },
                    &payout_data.payout_attempt,
                    merchant_account.storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating payouts in db")?;
            if helpers::is_payout_err_state(status) {
                return Err(report!(errors::ApiErrorResponse::PayoutFailed {
                    data: Some(
                        serde_json::json!({"payout_status": status.to_string(), "error_message": payout_data.payout_attempt.error_message.as_ref(), "error_code": payout_data.payout_attempt.error_code.as_ref()})
                    ),
                }));
            }
        }
        Err(err) => {
            let status = storage_enums::PayoutStatus::Failed;
            let updated_payout_attempt = storage::PayoutAttemptUpdate::StatusUpdate {
                connector_payout_id: String::default(),
                status,
                error_code: Some(err.code),
                error_message: Some(err.message),
                is_eligible: Some(false),
            };
            payout_data.payout_attempt = db
                .update_payout_attempt(
                    &payout_data.payout_attempt,
                    updated_payout_attempt,
                    &payout_data.payouts,
                    merchant_account.storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating payout_attempt in db")?;
            payout_data.payouts = db
                .update_payout(
                    &payout_data.payouts,
                    storage::PayoutsUpdate::StatusUpdate { status },
                    &payout_data.payout_attempt,
                    merchant_account.storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating payouts in db")?;
        }
    };

    Ok(())
}

pub async fn complete_create_payout(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    connector_data: &api::ConnectorData,
    payout_data: &mut PayoutData,
) -> RouterResult<()> {
    if !payout_data.should_terminate
        && payout_data.payout_attempt.status == storage_enums::PayoutStatus::RequiresCreation
    {
        if connector_data
            .connector_name
            .supports_instant_payout(payout_data.payouts.payout_type)
        {
            // create payout_object only in router
            let db = &*state.store;
            let payout_attempt = &payout_data.payout_attempt;
            let updated_payout_attempt = storage::PayoutAttemptUpdate::StatusUpdate {
                connector_payout_id: "".to_string(),
                status: storage::enums::PayoutStatus::RequiresFulfillment,
                error_code: None,
                error_message: None,
                is_eligible: None,
            };
            payout_data.payout_attempt = db
                .update_payout_attempt(
                    payout_attempt,
                    updated_payout_attempt,
                    &payout_data.payouts,
                    merchant_account.storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating payout_attempt in db")?;
            payout_data.payouts = db
                .update_payout(
                    &payout_data.payouts,
                    storage::PayoutsUpdate::StatusUpdate {
                        status: storage::enums::PayoutStatus::RequiresFulfillment,
                    },
                    &payout_data.payout_attempt,
                    merchant_account.storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating payouts in db")?;
        } else {
            // create payout_object in connector as well as router
            create_payout(
                state,
                merchant_account,
                key_store,
                connector_data,
                payout_data,
            )
            .await
            .attach_printable("Payout creation failed for given Payout request")?;
        }
    }
    Ok(())
}

pub async fn create_payout(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    connector_data: &api::ConnectorData,
    payout_data: &mut PayoutData,
) -> RouterResult<()> {
    // 1. Form Router data
    let mut router_data = core_utils::construct_payout_router_data(
        state,
        &connector_data.connector_name,
        merchant_account,
        key_store,
        payout_data,
    )
    .await?;

    // 2. Get/Create access token
    access_token::create_access_token(
        state,
        connector_data,
        merchant_account,
        &mut router_data,
        payout_data.payouts.payout_type.to_owned(),
    )
    .await?;

    // 3. Fetch connector integration details
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        api::PoCreate,
        types::PayoutsData,
        types::PayoutsResponseData,
    > = connector_data.connector.get_connector_integration();

    // 4. Execute pretasks
    connector_integration
        .execute_pretasks(&mut router_data, state)
        .await
        .to_payout_failed_response()?;

    // 5. Call connector service
    let router_data_resp = services::execute_connector_processing_step(
        state,
        connector_integration,
        &router_data,
        payments::CallConnectorAction::Trigger,
        None,
    )
    .await
    .to_payout_failed_response()?;

    // 6. Process data returned by the connector
    let db = &*state.store;
    match router_data_resp.response {
        Ok(payout_response_data) => {
            let payout_attempt = &payout_data.payout_attempt;
            let status = payout_response_data
                .status
                .unwrap_or(payout_attempt.status.to_owned());
            let updated_payout_attempt = storage::PayoutAttemptUpdate::StatusUpdate {
                connector_payout_id: payout_response_data.connector_payout_id,
                status,
                error_code: None,
                error_message: None,
                is_eligible: payout_response_data.payout_eligible,
            };
            payout_data.payout_attempt = db
                .update_payout_attempt(
                    payout_attempt,
                    updated_payout_attempt,
                    &payout_data.payouts,
                    merchant_account.storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating payout_attempt in db")?;
            payout_data.payouts = db
                .update_payout(
                    &payout_data.payouts,
                    storage::PayoutsUpdate::StatusUpdate { status },
                    &payout_data.payout_attempt,
                    merchant_account.storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating payouts in db")?;
            if helpers::is_payout_err_state(status) {
                return Err(report!(errors::ApiErrorResponse::PayoutFailed {
                    data: Some(
                        serde_json::json!({"payout_status": status.to_string(), "error_message": payout_data.payout_attempt.error_message.as_ref(), "error_code": payout_data.payout_attempt.error_code.as_ref()})
                    ),
                }));
            }
        }
        Err(err) => {
            let status = storage_enums::PayoutStatus::Failed;
            let updated_payout_attempt = storage::PayoutAttemptUpdate::StatusUpdate {
                connector_payout_id: String::default(),
                status,
                error_code: Some(err.code),
                error_message: Some(err.message),
                is_eligible: None,
            };
            payout_data.payout_attempt = db
                .update_payout_attempt(
                    &payout_data.payout_attempt,
                    updated_payout_attempt,
                    &payout_data.payouts,
                    merchant_account.storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating payout_attempt in db")?;
            payout_data.payouts = db
                .update_payout(
                    &payout_data.payouts,
                    storage::PayoutsUpdate::StatusUpdate { status },
                    &payout_data.payout_attempt,
                    merchant_account.storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating payouts in db")?;
        }
    };

    Ok(())
}

pub async fn complete_create_recipient_disburse_account(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    connector_data: &api::ConnectorData,
    payout_data: &mut PayoutData,
) -> RouterResult<()> {
    if !payout_data.should_terminate
        && payout_data.payout_attempt.status
            == storage_enums::PayoutStatus::RequiresVendorAccountCreation
        && connector_data
            .connector_name
            .supports_vendor_disburse_account_create_for_payout()
    {
        create_recipient_disburse_account(
            state,
            merchant_account,
            key_store,
            connector_data,
            payout_data,
        )
        .await
        .attach_printable("Creation of customer failed")?;
    }
    Ok(())
}

pub async fn create_recipient_disburse_account(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    connector_data: &api::ConnectorData,
    payout_data: &mut PayoutData,
) -> RouterResult<()> {
    // 1. Form Router data
    let router_data = core_utils::construct_payout_router_data(
        state,
        &connector_data.connector_name,
        merchant_account,
        key_store,
        payout_data,
    )
    .await?;

    // 2. Fetch connector integration details
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        api::PoRecipientAccount,
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
    match router_data_resp.response {
        Ok(payout_response_data) => {
            let payout_attempt = &payout_data.payout_attempt;
            let status = payout_response_data
                .status
                .unwrap_or(payout_attempt.status.to_owned());
            let updated_payout_attempt = storage::PayoutAttemptUpdate::StatusUpdate {
                connector_payout_id: payout_response_data.connector_payout_id,
                status,
                error_code: None,
                error_message: None,
                is_eligible: payout_response_data.payout_eligible,
            };
            payout_data.payout_attempt = db
                .update_payout_attempt(
                    payout_attempt,
                    updated_payout_attempt,
                    &payout_data.payouts,
                    merchant_account.storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating payout_attempt in db")?;
        }
        Err(err) => {
            let updated_payout_attempt = storage::PayoutAttemptUpdate::StatusUpdate {
                connector_payout_id: String::default(),
                status: storage_enums::PayoutStatus::Failed,
                error_code: Some(err.code),
                error_message: Some(err.message),
                is_eligible: None,
            };
            payout_data.payout_attempt = db
                .update_payout_attempt(
                    &payout_data.payout_attempt,
                    updated_payout_attempt,
                    &payout_data.payouts,
                    merchant_account.storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating payout_attempt in db")?;
        }
    };

    Ok(())
}

pub async fn cancel_payout(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    connector_data: &api::ConnectorData,
    payout_data: &mut PayoutData,
) -> RouterResult<()> {
    // 1. Form Router data
    let router_data = core_utils::construct_payout_router_data(
        state,
        &connector_data.connector_name,
        merchant_account,
        key_store,
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
    match router_data_resp.response {
        Ok(payout_response_data) => {
            let status = payout_response_data
                .status
                .unwrap_or(payout_data.payout_attempt.status.to_owned());
            let updated_payout_attempt = storage::PayoutAttemptUpdate::StatusUpdate {
                connector_payout_id: payout_response_data.connector_payout_id,
                status,
                error_code: None,
                error_message: None,
                is_eligible: payout_response_data.payout_eligible,
            };
            payout_data.payout_attempt = db
                .update_payout_attempt(
                    &payout_data.payout_attempt,
                    updated_payout_attempt,
                    &payout_data.payouts,
                    merchant_account.storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating payout_attempt in db")?;
            payout_data.payouts = db
                .update_payout(
                    &payout_data.payouts,
                    storage::PayoutsUpdate::StatusUpdate { status },
                    &payout_data.payout_attempt,
                    merchant_account.storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating payouts in db")?;
        }
        Err(err) => {
            let status = storage_enums::PayoutStatus::Failed;
            let updated_payout_attempt = storage::PayoutAttemptUpdate::StatusUpdate {
                connector_payout_id: String::default(),
                status,
                error_code: Some(err.code),
                error_message: Some(err.message),
                is_eligible: None,
            };
            payout_data.payout_attempt = db
                .update_payout_attempt(
                    &payout_data.payout_attempt,
                    updated_payout_attempt,
                    &payout_data.payouts,
                    merchant_account.storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating payout_attempt in db")?;
            payout_data.payouts = db
                .update_payout(
                    &payout_data.payouts,
                    storage::PayoutsUpdate::StatusUpdate { status },
                    &payout_data.payout_attempt,
                    merchant_account.storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating payouts in db")?;
        }
    };

    Ok(())
}

pub async fn fulfill_payout(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    connector_data: &api::ConnectorData,
    payout_data: &mut PayoutData,
) -> RouterResult<()> {
    // 1. Form Router data
    let mut router_data = core_utils::construct_payout_router_data(
        state,
        &connector_data.connector_name,
        merchant_account,
        key_store,
        payout_data,
    )
    .await?;

    // 2. Get/Create access token
    access_token::create_access_token(
        state,
        connector_data,
        merchant_account,
        &mut router_data,
        payout_data.payouts.payout_type.to_owned(),
    )
    .await?;

    // 3. Fetch connector integration details
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        api::PoFulfill,
        types::PayoutsData,
        types::PayoutsResponseData,
    > = connector_data.connector.get_connector_integration();

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
    match router_data_resp.response {
        Ok(payout_response_data) => {
            let status = payout_response_data
                .status
                .unwrap_or(payout_data.payout_attempt.status.to_owned());
            payout_data.payouts.status = status;
            if payout_data.payouts.recurring
                && payout_data.payouts.payout_method_id.clone().is_none()
                && !helpers::is_payout_err_state(status)
            {
                helpers::save_payout_data_to_locker(
                    state,
                    payout_data,
                    &payout_data
                        .payout_method_data
                        .clone()
                        .get_required_value("payout_method_data")?,
                    merchant_account,
                    key_store,
                )
                .await?;
            }
            let updated_payout_attempt = storage::PayoutAttemptUpdate::StatusUpdate {
                connector_payout_id: payout_data.payout_attempt.connector_payout_id.to_owned(),
                status,
                error_code: None,
                error_message: None,
                is_eligible: payout_response_data.payout_eligible,
            };
            payout_data.payout_attempt = db
                .update_payout_attempt(
                    &payout_data.payout_attempt,
                    updated_payout_attempt,
                    &payout_data.payouts,
                    merchant_account.storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating payout_attempt in db")?;
            payout_data.payouts = db
                .update_payout(
                    &payout_data.payouts,
                    storage::PayoutsUpdate::StatusUpdate { status },
                    &payout_data.payout_attempt,
                    merchant_account.storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating payouts in db")?;
            if helpers::is_payout_err_state(status) {
                return Err(report!(errors::ApiErrorResponse::PayoutFailed {
                    data: Some(
                        serde_json::json!({"payout_status": status.to_string(), "error_message": payout_data.payout_attempt.error_message.as_ref(), "error_code": payout_data.payout_attempt.error_code.as_ref()})
                    ),
                }));
            }
        }
        Err(err) => {
            let status = storage_enums::PayoutStatus::Failed;
            let updated_payout_attempt = storage::PayoutAttemptUpdate::StatusUpdate {
                connector_payout_id: String::default(),
                status,
                error_code: Some(err.code),
                error_message: Some(err.message),
                is_eligible: None,
            };
            payout_data.payout_attempt = db
                .update_payout_attempt(
                    &payout_data.payout_attempt,
                    updated_payout_attempt,
                    &payout_data.payouts,
                    merchant_account.storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating payout_attempt in db")?;
            payout_data.payouts = db
                .update_payout(
                    &payout_data.payouts,
                    storage::PayoutsUpdate::StatusUpdate { status },
                    &payout_data.payout_attempt,
                    merchant_account.storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating payouts in db")?;
        }
    };

    Ok(())
}

pub async fn response_handler(
    merchant_account: &domain::MerchantAccount,
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
            email: a.email.to_owned().map(pii::Email::from),
        }
    });

    let response = api::PayoutCreateResponse {
        payout_id: payouts.payout_id.to_owned(),
        merchant_id: merchant_account.merchant_id.to_owned(),
        amount: payouts.amount.to_owned(),
        currency: payouts.destination_currency.to_owned(),
        connector: payout_attempt.connector.to_owned(),
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
        created: Some(payouts.created_at),
        attempts: None,
    };
    Ok(services::ApplicationResponse::Json(response))
}

// DB entries
#[allow(clippy::too_many_arguments)]
pub async fn payout_create_db_entries(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    req: &payouts::PayoutCreateRequest,
    payout_id: &String,
    profile_id: &String,
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
    let amount = MinorUnit::from(req.amount.unwrap_or(api::Amount::Zero)).get_amount_as_i64();
    let payouts_req = storage::PayoutsNew {
        payout_id: payout_id.to_string(),
        merchant_id: merchant_id.to_string(),
        customer_id: customer_id.to_owned(),
        address_id: address_id.to_owned(),
        payout_type,
        amount,
        destination_currency: currency,
        source_currency: currency,
        description: req.description.to_owned(),
        recurring: req.recurring.unwrap_or(false),
        auto_fulfill: req.auto_fulfill.unwrap_or(false),
        return_url: req.return_url.to_owned(),
        entity_type: req.entity_type.unwrap_or_default(),
        payout_method_id,
        profile_id: profile_id.to_string(),
        attempt_count: 1,
        metadata: req.metadata.clone(),
        confirm: req.confirm,
        ..Default::default()
    };
    let payouts = db
        .insert_payout(payouts_req, merchant_account.storage_scheme)
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

    let payout_attempt_req = storage::PayoutAttemptNew {
        payout_attempt_id: payout_attempt_id.to_string(),
        payout_id: payout_id.to_owned(),
        customer_id: customer_id.to_owned(),
        merchant_id: merchant_id.to_owned(),
        address_id: address_id.to_owned(),
        status,
        business_country: req.business_country.to_owned(),
        business_label: req.business_label.to_owned(),
        payout_token: req.payout_token.to_owned(),
        profile_id: profile_id.to_string(),
        ..Default::default()
    };
    let payout_attempt = db
        .insert_payout_attempt(
            payout_attempt_req,
            &payouts,
            merchant_account.storage_scheme,
        )
        .await
        .to_duplicate_response(errors::ApiErrorResponse::DuplicatePayout {
            payout_id: payout_id.to_owned(),
        })
        .attach_printable("Error inserting payout_attempt in db")?;

    // Validate whether profile_id passed in request is valid and is linked to the merchant
    let business_profile =
        validate_and_get_business_profile(state, profile_id, merchant_id).await?;

    // Make PayoutData
    Ok(PayoutData {
        billing_address,
        business_profile,
        customer_details: customer,
        merchant_connector_account: None,
        payouts,
        payout_attempt,
        payout_method_data: req
            .payout_method_data
            .as_ref()
            .cloned()
            .or(stored_payout_method_data.cloned()),
        should_terminate: false,
        profile_id: profile_id.to_owned(),
    })
}

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
        .find_payout_by_merchant_id_payout_id(
            merchant_id,
            &payout_id,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PayoutNotFound)?;

    let payout_attempt_id = utils::get_payment_attempt_id(payout_id, payouts.attempt_count);

    let payout_attempt = db
        .find_payout_attempt_by_merchant_id_payout_attempt_id(
            merchant_id,
            &payout_attempt_id,
            merchant_account.storage_scheme,
        )
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
            merchant_account.storage_scheme,
        )
        .await
        .map_or(None, |c| c);

    let profile_id = payout_attempt.profile_id.clone();

    // Validate whether profile_id passed in request is valid and is linked to the merchant
    let business_profile =
        validate_and_get_business_profile(state, &profile_id, merchant_id).await?;

    Ok(PayoutData {
        billing_address,
        business_profile,
        customer_details,
        payouts,
        payout_attempt,
        payout_method_data: None,
        merchant_connector_account: None,
        should_terminate: false,
        profile_id,
    })
}

pub async fn add_external_account_addition_task(
    db: &dyn StorageInterface,
    payout_data: &PayoutData,
    schedule_time: time::PrimitiveDateTime,
) -> CustomResult<(), errors::StorageError> {
    let runner = storage::ProcessTrackerRunner::AttachPayoutAccountWorkflow;
    let task = "STRPE_ATTACH_EXTERNAL_ACCOUNT";
    let tag = ["PAYOUTS", "STRIPE", "ACCOUNT", "CREATE"];
    let process_tracker_id = pt_utils::get_process_tracker_id(
        runner,
        task,
        &payout_data.payout_attempt.payout_attempt_id,
        &payout_data.payout_attempt.merchant_id,
    );
    let tracking_data = api::PayoutRetrieveRequest {
        payout_id: payout_data.payouts.payout_id.to_owned(),
        force_sync: None,
        merchant_id: Some(payout_data.payouts.merchant_id.to_owned()),
    };
    let process_tracker_entry = storage::ProcessTrackerNew::new(
        process_tracker_id,
        task,
        runner,
        tag,
        tracking_data,
        schedule_time,
    )
    .map_err(errors::StorageError::from)?;

    db.insert_process(process_tracker_entry).await?;
    Ok(())
}

async fn validate_and_get_business_profile(
    state: &AppState,
    profile_id: &String,
    merchant_id: &str,
) -> RouterResult<storage::BusinessProfile> {
    let db = &*state.store;
    if let Some(business_profile) =
        core_utils::validate_and_get_business_profile(db, Some(profile_id), merchant_id).await?
    {
        Ok(business_profile)
    } else {
        db.find_business_profile_by_profile_id(profile_id)
            .await
            .to_not_found_response(errors::ApiErrorResponse::BusinessProfileNotFound {
                id: profile_id.to_string(),
            })
    }
}
