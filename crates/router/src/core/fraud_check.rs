use std::fmt::Debug;

use api_models::{admin::FrmConfigs, enums as api_enums, payments::AdditionalPaymentData};
use error_stack::ResultExt;
use masking::{ExposeInterface, PeekInterface};
use router_env::{
    logger,
    tracing::{self, instrument},
};

use self::{
    flows::{self as frm_flows, FeatureFrm},
    types::{
        self as frm_core_types, ConnectorDetailsCore, FrmConfigsObject, FrmData, FrmInfo,
        PaymentDetails, PaymentToFrmData,
    },
};
use super::errors::{ConnectorErrorExt, RouterResponse};
use crate::{
    core::{
        errors::{self, RouterResult},
        payments::{
            self, flows::ConstructFlowSpecificData, helpers::get_additional_payment_data,
            operations::BoxedOperation,
        },
        utils as core_utils,
    },
    db::StorageInterface,
    routes::AppState,
    services,
    types::{
        self as oss_types,
        api::{routing::FrmRoutingAlgorithm, Connector, FraudCheckConnectorData, Fulfillment},
        domain, fraud_check as frm_types,
        storage::{
            enums::{
                AttemptStatus, FraudCheckLastStep, FraudCheckStatus, FraudCheckType, FrmSuggestion,
                IntentStatus,
            },
            fraud_check::{FraudCheck, FraudCheckUpdate},
            PaymentIntent,
        },
    },
    utils::ValueExt,
};
pub mod flows;
pub mod operation;
pub mod types;

#[instrument(skip_all)]
pub async fn call_frm_service<D: Clone, F, Req>(
    state: &AppState,
    payment_data: &mut payments::PaymentData<D>,
    frm_data: &mut FrmData,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    customer: &Option<domain::Customer>,
) -> RouterResult<oss_types::RouterData<F, Req, frm_types::FraudCheckResponseData>>
where
    F: Send + Clone,

    // To create connector flow specific interface data
    FrmData: ConstructFlowSpecificData<F, Req, frm_types::FraudCheckResponseData>,
    oss_types::RouterData<F, Req, frm_types::FraudCheckResponseData>: FeatureFrm<F, Req> + Send,

    // To construct connector flow specific api
    dyn Connector: services::api::ConnectorIntegration<F, Req, frm_types::FraudCheckResponseData>,
{
    let merchant_connector_account = payments::construct_profile_id_and_get_mca(
        state,
        merchant_account,
        payment_data,
        &frm_data.connector_details.connector_name,
        None,
        key_store,
        false,
    )
    .await?;

    frm_data.payment_attempt.connector_transaction_id = payment_data
        .payment_attempt
        .connector_transaction_id
        .clone();

    let mut router_data = frm_data
        .construct_router_data(
            state,
            &frm_data.connector_details.connector_name,
            merchant_account,
            key_store,
            customer,
            &merchant_connector_account,
        )
        .await?;

    router_data.status = payment_data.payment_attempt.status;

    let connector =
        FraudCheckConnectorData::get_connector_by_name(&frm_data.connector_details.connector_name)?;
    let router_data_res = router_data
        .decide_frm_flows(
            state,
            &connector,
            payments::CallConnectorAction::Trigger,
            merchant_account,
        )
        .await?;

    Ok(router_data_res)
}

pub async fn should_call_frm<F>(
    merchant_account: &domain::MerchantAccount,
    payment_data: &payments::PaymentData<F>,
    db: &dyn StorageInterface,
    key_store: domain::MerchantKeyStore,
) -> RouterResult<(
    bool,
    Option<FrmRoutingAlgorithm>,
    Option<String>,
    Option<FrmConfigsObject>,
)>
where
    F: Send + Clone,
{
    match merchant_account.frm_routing_algorithm.clone() {
        Some(frm_routing_algorithm_value) => {
            let frm_routing_algorithm_struct: FrmRoutingAlgorithm = frm_routing_algorithm_value
                .clone()
                .parse_value("FrmRoutingAlgorithm")
                .change_context(errors::ApiErrorResponse::MissingRequiredField {
                    field_name: "frm_routing_algorithm",
                })
                .attach_printable("Data field not found in frm_routing_algorithm")?;

            let profile_id = core_utils::get_profile_id_from_business_details(
                payment_data.payment_intent.business_country,
                payment_data.payment_intent.business_label.as_ref(),
                merchant_account,
                payment_data.payment_intent.profile_id.as_ref(),
                db,
                false,
            )
            .await
            .attach_printable("Could not find profile id from business details")?;

            let merchant_connector_account_from_db_option = db
                .find_merchant_connector_account_by_profile_id_connector_name(
                    &profile_id,
                    &frm_routing_algorithm_struct.data,
                    &key_store,
                )
                .await
                .change_context(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                    id: merchant_account.merchant_id.clone(),
                })
                .ok();

            match merchant_connector_account_from_db_option {
                Some(merchant_connector_account_from_db) => {
                    let frm_configs_option = merchant_connector_account_from_db
                        .frm_configs
                        .ok_or(errors::ApiErrorResponse::MissingRequiredField {
                            field_name: "frm_configs",
                        })
                        .ok();
                    match frm_configs_option {
                        Some(frm_configs_value) => {
                            let frm_configs_struct: Vec<FrmConfigs> = frm_configs_value
                                .into_iter()
                                .map(|config| { config
                                    .expose()
                                    .parse_value("FrmConfigs")
                                    .change_context(errors::ApiErrorResponse::InvalidDataFormat {
                                        field_name: "frm_configs".to_string(),
                                        expected_format: r#"[{ "gateway": "stripe", "payment_methods": [{ "payment_method": "card","payment_method_types": [{"payment_method_type": "credit","card_networks": ["Visa"],"flow": "pre","action": "cancel_txn"}]}]}]"#.to_string(),
                                    })
                                })
                                .collect::<Result<Vec<_>, _>>()?;

                            let mut is_frm_connector_enabled = false;
                            let mut is_frm_pm_enabled = false;
                            let mut is_frm_pmt_enabled = false;
                            let filtered_frm_config = frm_configs_struct
                                .iter()
                                .filter(|frm_config| {
                                    match (
                                        &payment_data.clone().payment_attempt.connector,
                                        &frm_config.gateway,
                                    ) {
                                        (Some(current_connector), Some(configured_connector)) => {
                                            let is_enabled = *current_connector
                                                == configured_connector.to_string();
                                            if is_enabled {
                                                is_frm_connector_enabled = true;
                                            }
                                            is_enabled
                                        }
                                        (None, _) | (_, None) => true,
                                    }
                                })
                                .collect::<Vec<_>>();
                            let filtered_payment_methods = filtered_frm_config
                                .iter()
                                .map(|frm_config| {
                                    let filtered_frm_config_by_pm = frm_config
                                        .payment_methods
                                        .iter()
                                        .filter(|frm_config_pm| {
                                            match (
                                                payment_data.payment_attempt.payment_method,
                                                frm_config_pm.payment_method,
                                            ) {
                                                (
                                                    Some(current_pm),
                                                    Some(configured_connector_pm),
                                                ) => {
                                                    let is_enabled = current_pm.to_string()
                                                        == configured_connector_pm.to_string();
                                                    if is_enabled {
                                                        is_frm_pm_enabled = true;
                                                    }
                                                    is_enabled
                                                }
                                                (None, _) | (_, None) => true,
                                            }
                                        })
                                        .collect::<Vec<_>>();
                                    filtered_frm_config_by_pm
                                })
                                .collect::<Vec<_>>()
                                .concat();
                            let additional_payment_data = match &payment_data.payment_method_data {
                                Some(pmd) => {
                                    let additional_payment_data =
                                        get_additional_payment_data(pmd, db).await;
                                    Some(additional_payment_data)
                                }
                                None => payment_data
                                    .payment_attempt
                                    .payment_method_data
                                    .as_ref()
                                    .map(|pm_data| {
                                        pm_data.clone().parse_value::<AdditionalPaymentData>(
                                            "AdditionalPaymentData",
                                        )
                                    })
                                    .transpose()
                                    .unwrap_or_default(), // Making this default in case of error as we don't want to fail payment for frm errors
                            };
                            let filtered_payment_method_types = filtered_payment_methods
                                .iter()
                                .map(|frm_pm_config| {
                                    let filtered_pm_config_by_pmt = frm_pm_config
                                        .payment_method_types
                                        .iter()
                                        .filter(|frm_pm_config_by_pmt| {
                                            match (
                                                &payment_data
                                                    .clone()
                                                    .payment_attempt
                                                    .payment_method_type,
                                                frm_pm_config_by_pmt.payment_method_type,
                                            ) {
                                                (Some(curr), Some(conf))
                                                    if curr.to_string() == conf.to_string() =>
                                                {
                                                    is_frm_pmt_enabled = true;
                                                    true
                                                }
                                                (None, Some(conf)) => match additional_payment_data
                                                    .clone()
                                                {
                                                    Some(AdditionalPaymentData::Card(card)) => {
                                                        let card_type = card
                                                            .card_type
                                                            .unwrap_or_else(|| "debit".to_string());
                                                        let is_enabled = card_type.to_lowercase()
                                                            == conf.to_string().to_lowercase();
                                                        if is_enabled {
                                                            is_frm_pmt_enabled = true;
                                                        }
                                                        is_enabled
                                                    }
                                                    _ => false,
                                                },
                                                _ => false,
                                            }
                                        })
                                        .collect::<Vec<_>>();
                                    filtered_pm_config_by_pmt
                                })
                                .collect::<Vec<_>>()
                                .concat();
                            let is_frm_enabled =
                                is_frm_connector_enabled && is_frm_pm_enabled && is_frm_pmt_enabled;
                            logger::debug!(
                                "frm_configs {:?} {:?} {:?} {:?}",
                                is_frm_connector_enabled,
                                is_frm_pm_enabled,
                                is_frm_pmt_enabled,
                                is_frm_enabled
                            );
                            // filtered_frm_config...
                            // Panic Safety: we are first checking if the object is present... only if present, we try to fetch index 0
                            let frm_configs_object = FrmConfigsObject {
                                frm_enabled_gateway: filtered_frm_config
                                    .first()
                                    .and_then(|c| c.gateway),
                                frm_enabled_pm: filtered_payment_methods
                                    .first()
                                    .and_then(|pm| pm.payment_method),
                                frm_enabled_pm_type: filtered_payment_method_types
                                    .first()
                                    .and_then(|pmt| pmt.payment_method_type),
                                frm_action: filtered_payment_method_types
                                    // .clone()
                                    .first()
                                    .map(|pmt| pmt.action.clone())
                                    .unwrap_or(api_enums::FrmAction::ManualReview),
                                frm_preferred_flow_type: filtered_payment_method_types
                                    .first()
                                    .map(|pmt| pmt.flow.clone())
                                    .unwrap_or(api_enums::FrmPreferredFlowTypes::Pre),
                            };
                            logger::debug!(
                                "frm_routing_configs: {:?} {:?} {:?} {:?}",
                                frm_routing_algorithm_struct,
                                profile_id,
                                frm_configs_object,
                                is_frm_enabled
                            );
                            Ok((
                                is_frm_enabled,
                                Some(frm_routing_algorithm_struct),
                                Some(profile_id.to_string()),
                                Some(frm_configs_object),
                            ))
                        }
                        None => {
                            logger::error!("Cannot find frm_configs for FRM provider");
                            Ok((false, None, None, None))
                        }
                    }
                }
                None => {
                    logger::error!("Cannot find merchant connector account for FRM provider");
                    Ok((false, None, None, None))
                }
            }
        }
        _ => Ok((false, None, None, None)),
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn make_frm_data_and_fraud_check_operation<'a, F>(
    _db: &dyn StorageInterface,
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    payment_data: payments::PaymentData<F>,
    frm_routing_algorithm: FrmRoutingAlgorithm,
    profile_id: String,
    frm_configs: FrmConfigsObject,
    _customer: &Option<domain::Customer>,
) -> RouterResult<FrmInfo<F>>
where
    F: Send + Clone,
{
    let order_details = payment_data
        .payment_intent
        .order_details
        .clone()
        .or_else(||
            // when the order_details are present within the meta_data, we need to take those to support backward compatibility
            payment_data.payment_intent.metadata.clone().and_then(|meta| {
                let order_details = meta.peek().get("order_details").to_owned();
                order_details.map(|order| vec![masking::Secret::new(order.to_owned())])
            }))
        .map(|order_details_value| {
            order_details_value
                .into_iter()
                .map(|data| {
                    data.peek()
                        .to_owned()
                        .parse_value("OrderDetailsWithAmount")
                        .attach_printable("unable to parse OrderDetailsWithAmount")
                })
                .collect::<Result<Vec<_>, _>>()
                .unwrap_or_default()
        });

    let frm_connector_details = ConnectorDetailsCore {
        connector_name: frm_routing_algorithm.data,
        profile_id,
    };

    let payment_to_frm_data = PaymentToFrmData {
        amount: payment_data.amount,
        payment_intent: payment_data.payment_intent,
        payment_attempt: payment_data.payment_attempt,
        merchant_account: merchant_account.to_owned(),
        address: payment_data.address.clone(),
        connector_details: frm_connector_details.clone(),
        order_details,
        frm_metadata: payment_data.frm_metadata.clone(),
    };

    let fraud_check_operation: operation::BoxedFraudCheckOperation<F> =
        match frm_configs.frm_preferred_flow_type {
            api_enums::FrmPreferredFlowTypes::Pre => Box::new(operation::FraudCheckPre),
            api_enums::FrmPreferredFlowTypes::Post => Box::new(operation::FraudCheckPost),
        };
    let frm_data = fraud_check_operation
        .to_get_tracker()?
        .get_trackers(state, payment_to_frm_data, frm_connector_details)
        .await?;
    Ok(FrmInfo {
        fraud_check_operation,
        frm_data,
        suggested_action: None,
    })
}

#[allow(clippy::too_many_arguments)]
pub async fn pre_payment_frm_core<'a, F>(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    payment_data: &mut payments::PaymentData<F>,
    frm_info: &mut FrmInfo<F>,
    frm_configs: FrmConfigsObject,
    customer: &Option<domain::Customer>,
    should_continue_transaction: &mut bool,
    should_continue_capture: &mut bool,
    key_store: domain::MerchantKeyStore,
) -> RouterResult<Option<FrmData>>
where
    F: Send + Clone,
{
    if let Some(frm_data) = &mut frm_info.frm_data {
        if matches!(
            frm_configs.frm_preferred_flow_type,
            api_enums::FrmPreferredFlowTypes::Pre
        ) {
            let fraud_check_operation = &mut frm_info.fraud_check_operation;

            let frm_router_data = fraud_check_operation
                .to_domain()?
                .pre_payment_frm(
                    state,
                    payment_data,
                    frm_data,
                    merchant_account,
                    customer,
                    key_store,
                )
                .await?;
            let frm_data_updated = fraud_check_operation
                .to_update_tracker()?
                .update_tracker(
                    &*state.store,
                    frm_data.clone(),
                    payment_data,
                    None,
                    frm_router_data,
                )
                .await?;
            let frm_fraud_check = frm_data_updated.fraud_check.clone();
            payment_data.frm_message = Some(frm_fraud_check.clone());
            if matches!(frm_fraud_check.frm_status, FraudCheckStatus::Fraud) {
                if matches!(frm_configs.frm_action, api_enums::FrmAction::CancelTxn) {
                    *should_continue_transaction = false;
                    frm_info.suggested_action = Some(FrmSuggestion::FrmCancelTransaction);
                } else if matches!(frm_configs.frm_action, api_enums::FrmAction::ManualReview) {
                    *should_continue_capture = false;
                    frm_info.suggested_action = Some(FrmSuggestion::FrmManualReview);
                }
            }
            logger::debug!(
                "frm_updated_data: {:?} {:?}",
                frm_info.fraud_check_operation,
                frm_info.suggested_action
            );
            Ok(Some(frm_data_updated))
        } else {
            Ok(Some(frm_data.to_owned()))
        }
    } else {
        Ok(None)
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn post_payment_frm_core<'a, F>(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    payment_data: &mut payments::PaymentData<F>,
    frm_info: &mut FrmInfo<F>,
    frm_configs: FrmConfigsObject,
    customer: &Option<domain::Customer>,
    key_store: domain::MerchantKeyStore,
) -> RouterResult<Option<FrmData>>
where
    F: Send + Clone,
{
    if let Some(frm_data) = &mut frm_info.frm_data {
        // Allow the Post flow only if the payment is succeeded,
        // this logic has to be removed if we are going to call /sale or /transaction after failed transaction
        let fraud_check_operation = &mut frm_info.fraud_check_operation;
        if payment_data.payment_attempt.status == AttemptStatus::Charged {
            let frm_router_data_opt = fraud_check_operation
                .to_domain()?
                .post_payment_frm(
                    state,
                    payment_data,
                    frm_data,
                    merchant_account,
                    customer,
                    key_store.clone(),
                )
                .await?;
            if let Some(frm_router_data) = frm_router_data_opt {
                let mut frm_data = fraud_check_operation
                    .to_update_tracker()?
                    .update_tracker(
                        &*state.store,
                        frm_data.to_owned(),
                        payment_data,
                        None,
                        frm_router_data.to_owned(),
                    )
                    .await?;

                payment_data.frm_message = Some(frm_data.fraud_check.clone());
                logger::debug!(
                    "frm_updated_data: {:?} {:?}",
                    frm_data,
                    payment_data.frm_message
                );
                let mut frm_suggestion = None;
                fraud_check_operation
                    .to_domain()?
                    .execute_post_tasks(
                        state,
                        &mut frm_data,
                        merchant_account,
                        frm_configs,
                        &mut frm_suggestion,
                        key_store,
                        payment_data,
                        customer,
                    )
                    .await?;
                logger::debug!("frm_post_tasks_data: {:?}", frm_data);
                let updated_frm_data = fraud_check_operation
                    .to_update_tracker()?
                    .update_tracker(
                        &*state.store,
                        frm_data.to_owned(),
                        payment_data,
                        frm_suggestion,
                        frm_router_data.to_owned(),
                    )
                    .await?;
                return Ok(Some(updated_frm_data));
            }
        }

        Ok(Some(frm_data.to_owned()))
    } else {
        Ok(None)
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn call_frm_before_connector_call<'a, F, Req, Ctx>(
    db: &dyn StorageInterface,
    operation: &BoxedOperation<'_, F, Req, Ctx>,
    merchant_account: &domain::MerchantAccount,
    payment_data: &mut payments::PaymentData<F>,
    state: &AppState,
    frm_info: &mut Option<FrmInfo<F>>,
    customer: &Option<domain::Customer>,
    should_continue_transaction: &mut bool,
    should_continue_capture: &mut bool,
    key_store: domain::MerchantKeyStore,
) -> RouterResult<Option<FrmConfigsObject>>
where
    F: Send + Clone,
{
    if is_operation_allowed(operation) {
        let (is_frm_enabled, frm_routing_algorithm, frm_connector_label, frm_configs) =
            should_call_frm(merchant_account, payment_data, db, key_store.clone()).await?;
        if let Some((frm_routing_algorithm_val, profile_id)) =
            frm_routing_algorithm.zip(frm_connector_label)
        {
            if let Some(frm_configs) = frm_configs.clone() {
                let mut updated_frm_info = make_frm_data_and_fraud_check_operation(
                    db,
                    state,
                    merchant_account,
                    payment_data.to_owned(),
                    frm_routing_algorithm_val,
                    profile_id,
                    frm_configs.clone(),
                    customer,
                )
                .await?;

                if is_frm_enabled {
                    pre_payment_frm_core(
                        state,
                        merchant_account,
                        payment_data,
                        &mut updated_frm_info,
                        frm_configs,
                        customer,
                        should_continue_transaction,
                        should_continue_capture,
                        key_store,
                    )
                    .await?;
                }
                *frm_info = Some(updated_frm_info);
            }
        }
        logger::debug!("frm_configs: {:?} {:?}", frm_configs, is_frm_enabled);
        return Ok(frm_configs);
    }
    Ok(None)
}

pub fn is_operation_allowed<Op: Debug>(operation: &Op) -> bool {
    !["PaymentSession", "PaymentApprove", "PaymentReject"]
        .contains(&format!("{operation:?}").as_str())
}

impl From<PaymentToFrmData> for PaymentDetails {
    fn from(payment_data: PaymentToFrmData) -> Self {
        Self {
            amount: payment_data.amount.into(),
            currency: payment_data.payment_attempt.currency,
            payment_method: payment_data.payment_attempt.payment_method,
            payment_method_type: payment_data.payment_attempt.payment_method_type,
            refund_transaction_id: None,
        }
    }
}

#[instrument(skip_all)]
pub async fn frm_fulfillment_core(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    req: frm_core_types::FrmFulfillmentRequest,
) -> RouterResponse<frm_types::FraudCheckResponseData> {
    let db = &*state.clone().store;
    let payment_intent = db
        .find_payment_intent_by_payment_id_merchant_id(
            &req.payment_id.clone(),
            &merchant_account.merchant_id,
            merchant_account.storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::PaymentNotFound)?;
    match payment_intent.status {
        IntentStatus::Succeeded => {
            let invalid_request_error = errors::ApiErrorResponse::InvalidRequestData {
                message: "no fraud check entry found for this payment_id".to_string(),
            };
            let existing_fraud_check = db
                .find_fraud_check_by_payment_id_if_present(
                    req.payment_id.clone(),
                    merchant_account.merchant_id.clone(),
                )
                .await
                .change_context(invalid_request_error.to_owned())?;
            match existing_fraud_check {
                Some(fraud_check) => {
                    if (matches!(fraud_check.frm_transaction_type, FraudCheckType::PreFrm)
                        && fraud_check.last_step == FraudCheckLastStep::TransactionOrRecordRefund)
                        || (matches!(fraud_check.frm_transaction_type, FraudCheckType::PostFrm)
                            && fraud_check.last_step == FraudCheckLastStep::CheckoutOrSale)
                    {
                        Box::pin(make_fulfillment_api_call(
                            db,
                            fraud_check,
                            payment_intent,
                            state,
                            merchant_account,
                            key_store,
                            req,
                        ))
                        .await
                    } else {
                        Err(errors::ApiErrorResponse::PreconditionFailed {message:"Frm pre/post flow hasn't terminated yet, so fulfillment cannot be called".to_string(),}.into())
                    }
                }
                None => Err(invalid_request_error.into()),
            }
        }
        _ => Err(errors::ApiErrorResponse::PreconditionFailed {
            message: "Fulfillment can be performed only for succeeded payment".to_string(),
        }
        .into()),
    }
}

#[instrument(skip_all)]
pub async fn make_fulfillment_api_call(
    db: &dyn StorageInterface,
    fraud_check: FraudCheck,
    payment_intent: PaymentIntent,
    state: AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    req: frm_core_types::FrmFulfillmentRequest,
) -> RouterResponse<frm_types::FraudCheckResponseData> {
    let payment_attempt = db
        .find_payment_attempt_by_attempt_id_merchant_id(
            &payment_intent.active_attempt.get_id(),
            &merchant_account.merchant_id,
            merchant_account.storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::PaymentNotFound)?;
    let connector_data = FraudCheckConnectorData::get_connector_by_name(&fraud_check.frm_name)?;
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        Fulfillment,
        frm_types::FraudCheckFulfillmentData,
        frm_types::FraudCheckResponseData,
    > = connector_data.connector.get_connector_integration();
    let router_data = frm_flows::fulfillment_flow::construct_fulfillment_router_data(
        &state,
        &payment_intent,
        &payment_attempt,
        &merchant_account,
        &key_store,
        fraud_check.frm_name.clone(),
        req,
    )
    .await?;
    let response = services::execute_connector_processing_step(
        &state,
        connector_integration,
        &router_data,
        payments::CallConnectorAction::Trigger,
        None,
    )
    .await
    .to_payment_failed_response()?;
    let fraud_check_copy = fraud_check.clone();
    let fraud_check_update = FraudCheckUpdate::ResponseUpdate {
        frm_status: fraud_check.frm_status,
        frm_transaction_id: fraud_check.frm_transaction_id,
        frm_reason: fraud_check.frm_reason,
        frm_score: fraud_check.frm_score,
        metadata: fraud_check.metadata,
        modified_at: common_utils::date_time::now(),
        last_step: FraudCheckLastStep::Fulfillment,
    };
    let _updated = db
        .update_fraud_check_response_with_attempt_id(fraud_check_copy, fraud_check_update)
        .await
        .map_err(|error| error.change_context(errors::ApiErrorResponse::PaymentNotFound))?;
    let fulfillment_response =
        response
            .response
            .map_err(|err| errors::ApiErrorResponse::ExternalConnectorError {
                code: err.code,
                message: err.message,
                connector: connector_data.connector_name.clone().to_string(),
                status_code: err.status_code,
                reason: err.reason,
            })?;
    Ok(services::ApplicationResponse::Json(fulfillment_response))
}
