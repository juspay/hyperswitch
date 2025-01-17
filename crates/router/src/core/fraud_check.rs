use std::fmt::Debug;

use api_models::{self, enums as api_enums};
use common_enums::CaptureMethod;
use error_stack::ResultExt;
use masking::PeekInterface;
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
        payments::{self, flows::ConstructFlowSpecificData, operations::BoxedOperation},
    },
    db::StorageInterface,
    routes::{app::ReqState, SessionState},
    services,
    types::{
        self as oss_types,
        api::{
            fraud_check as frm_api, routing::FrmRoutingAlgorithm, Connector,
            FraudCheckConnectorData, Fulfillment,
        },
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

#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn call_frm_service<D: Clone, F, Req, OperationData>(
    state: &SessionState,
    payment_data: &OperationData,
    frm_data: &mut FrmData,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    customer: &Option<domain::Customer>,
) -> RouterResult<oss_types::RouterData<F, Req, frm_types::FraudCheckResponseData>>
where
    F: Send + Clone,

    OperationData: payments::OperationSessionGetters<D> + Send + Sync + Clone,

    // To create connector flow specific interface data
    FrmData: ConstructFlowSpecificData<F, Req, frm_types::FraudCheckResponseData>,
    oss_types::RouterData<F, Req, frm_types::FraudCheckResponseData>: FeatureFrm<F, Req> + Send,

    // To construct connector flow specific api
    dyn Connector: services::api::ConnectorIntegration<F, Req, frm_types::FraudCheckResponseData>,
{
    todo!()
}

#[cfg(feature = "v1")]
#[instrument(skip_all)]
pub async fn call_frm_service<D: Clone, F, Req, OperationData>(
    state: &SessionState,
    payment_data: &OperationData,
    frm_data: &mut FrmData,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    customer: &Option<domain::Customer>,
) -> RouterResult<oss_types::RouterData<F, Req, frm_types::FraudCheckResponseData>>
where
    F: Send + Clone,

    OperationData: payments::OperationSessionGetters<D> + Send + Sync + Clone,

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

    frm_data
        .payment_attempt
        .connector_transaction_id
        .clone_from(&payment_data.get_payment_attempt().connector_transaction_id);

    let mut router_data = frm_data
        .construct_router_data(
            state,
            &frm_data.connector_details.connector_name,
            merchant_account,
            key_store,
            customer,
            &merchant_connector_account,
            None,
            None,
        )
        .await?;

    router_data.status = payment_data.get_payment_attempt().status;
    if matches!(
        frm_data.fraud_check.frm_transaction_type,
        FraudCheckType::PreFrm
    ) && matches!(
        frm_data.fraud_check.last_step,
        FraudCheckLastStep::CheckoutOrSale
    ) {
        frm_data.fraud_check.last_step = FraudCheckLastStep::TransactionOrRecordRefund
    }

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

#[cfg(feature = "v2")]
pub async fn should_call_frm<F, D>(
    _merchant_account: &domain::MerchantAccount,
    _payment_data: &D,
    _state: &SessionState,
    _key_store: domain::MerchantKeyStore,
) -> RouterResult<(
    bool,
    Option<FrmRoutingAlgorithm>,
    Option<common_utils::id_type::ProfileId>,
    Option<FrmConfigsObject>,
)>
where
    F: Send + Clone,
    D: payments::OperationSessionGetters<F> + Send + Sync + Clone,
{
    // Frm routing algorithm is not present in the merchant account
    // it has to be fetched from the business profile
    todo!()
}

#[cfg(feature = "v1")]
pub async fn should_call_frm<F, D>(
    merchant_account: &domain::MerchantAccount,
    payment_data: &D,
    state: &SessionState,
    key_store: domain::MerchantKeyStore,
) -> RouterResult<(
    bool,
    Option<FrmRoutingAlgorithm>,
    Option<common_utils::id_type::ProfileId>,
    Option<FrmConfigsObject>,
)>
where
    F: Send + Clone,
    D: payments::OperationSessionGetters<F> + Send + Sync + Clone,
{
    use common_utils::ext_traits::OptionExt;
    use masking::ExposeInterface;

    let db = &*state.store;
    match merchant_account.frm_routing_algorithm.clone() {
        Some(frm_routing_algorithm_value) => {
            let frm_routing_algorithm_struct: FrmRoutingAlgorithm = frm_routing_algorithm_value
                .clone()
                .parse_value("FrmRoutingAlgorithm")
                .change_context(errors::ApiErrorResponse::MissingRequiredField {
                    field_name: "frm_routing_algorithm",
                })
                .attach_printable("Data field not found in frm_routing_algorithm")?;

            let profile_id = payment_data
                .get_payment_intent()
                .profile_id
                .as_ref()
                .get_required_value("profile_id")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("profile_id is not set in payment_intent")?
                .clone();

            #[cfg(feature = "v1")]
            let merchant_connector_account_from_db_option = db
                .find_merchant_connector_account_by_profile_id_connector_name(
                    &state.into(),
                    &profile_id,
                    &frm_routing_algorithm_struct.data,
                    &key_store,
                )
                .await
                .map_err(|error| {
                    logger::error!(
                        "{:?}",
                        error.change_context(
                            errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                                id: merchant_account.get_id().get_string_repr().to_owned(),
                            }
                        )
                    )
                })
                .ok();
            let enabled_merchant_connector_account_from_db_option =
                merchant_connector_account_from_db_option.and_then(|mca| {
                    if mca.disabled.unwrap_or(false) {
                        logger::info!("No eligible connector found for FRM");
                        None
                    } else {
                        Some(mca)
                    }
                });

            #[cfg(feature = "v2")]
            let merchant_connector_account_from_db_option: Option<
                domain::MerchantConnectorAccount,
            > = {
                let _ = key_store;
                let _ = frm_routing_algorithm_struct;
                let _ = profile_id;
                todo!()
            };

            match enabled_merchant_connector_account_from_db_option {
                Some(merchant_connector_account_from_db) => {
                    let frm_configs_option = merchant_connector_account_from_db
                        .frm_configs
                        .ok_or(errors::ApiErrorResponse::MissingRequiredField {
                            field_name: "frm_configs",
                        })
                        .ok();
                    match frm_configs_option {
                        Some(frm_configs_value) => {
                            let frm_configs_struct: Vec<api_models::admin::FrmConfigs> = frm_configs_value
                                .into_iter()
                                .map(|config| { config
                                    .expose()
                                    .parse_value("FrmConfigs")
                                    .change_context(errors::ApiErrorResponse::InvalidDataFormat {
                                            field_name: "frm_configs".to_string(),
                                            expected_format: r#"[{ "gateway": "stripe", "payment_methods": [{ "payment_method": "card","flow": "post"}]}]"#.to_string(),
                                        })
                                })
                                .collect::<Result<Vec<_>, _>>()?;

                            let mut is_frm_connector_enabled = false;
                            let mut is_frm_pm_enabled = false;
                            let connector = payment_data.get_payment_attempt().connector.clone();
                            let filtered_frm_config = frm_configs_struct
                                .iter()
                                .filter(|frm_config| match (&connector, &frm_config.gateway) {
                                    (Some(current_connector), Some(configured_connector)) => {
                                        let is_enabled =
                                            *current_connector == configured_connector.to_string();
                                        if is_enabled {
                                            is_frm_connector_enabled = true;
                                        }
                                        is_enabled
                                    }
                                    (None, _) | (_, None) => true,
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
                                                payment_data.get_payment_attempt().payment_method,
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
                            let is_frm_enabled = is_frm_connector_enabled && is_frm_pm_enabled;
                            logger::debug!(
                                "is_frm_connector_enabled {:?}, is_frm_pm_enabled:  {:?}, is_frm_enabled :{:?}",
                                is_frm_connector_enabled,
                                is_frm_pm_enabled,
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
                                // flow type should be consumed from payment_method.flow. To provide backward compatibility, if we don't find it there, we consume it from payment_method.payment_method_types[0].flow_type.
                                frm_preferred_flow_type: filtered_payment_methods
                                    .first()
                                    .and_then(|pm| pm.flow.clone())
                                    .or(filtered_payment_methods.first().and_then(|pm| {
                                        pm.payment_method_types.as_ref().and_then(|pmt| {
                                            pmt.first().map(|pmts| pmts.flow.clone())
                                        })
                                    }))
                                    .ok_or(errors::ApiErrorResponse::InvalidDataFormat {
                                            field_name: "frm_configs".to_string(),
                                            expected_format: r#"[{ "gateway": "stripe", "payment_methods": [{ "payment_method": "card","flow": "post"}]}]"#.to_string(),
                                    })?,
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
                                Some(profile_id),
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

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
pub async fn make_frm_data_and_fraud_check_operation<F, D>(
    _db: &dyn StorageInterface,
    state: &SessionState,
    merchant_account: &domain::MerchantAccount,
    payment_data: D,
    frm_routing_algorithm: FrmRoutingAlgorithm,
    profile_id: common_utils::id_type::ProfileId,
    frm_configs: FrmConfigsObject,
    _customer: &Option<domain::Customer>,
) -> RouterResult<FrmInfo<F, D>>
where
    F: Send + Clone,
    D: payments::OperationSessionGetters<F>
        + payments::OperationSessionSetters<F>
        + Send
        + Sync
        + Clone,
{
    todo!()
}

#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
pub async fn make_frm_data_and_fraud_check_operation<F, D>(
    _db: &dyn StorageInterface,
    state: &SessionState,
    merchant_account: &domain::MerchantAccount,
    payment_data: D,
    frm_routing_algorithm: FrmRoutingAlgorithm,
    profile_id: common_utils::id_type::ProfileId,
    frm_configs: FrmConfigsObject,
    _customer: &Option<domain::Customer>,
) -> RouterResult<FrmInfo<F, D>>
where
    F: Send + Clone,
    D: payments::OperationSessionGetters<F>
        + payments::OperationSessionSetters<F>
        + Send
        + Sync
        + Clone,
{
    let order_details = payment_data
        .get_payment_intent()
        .order_details
        .clone()
        .or_else(||
            // when the order_details are present within the meta_data, we need to take those to support backward compatibility
            payment_data.get_payment_intent().metadata.clone().and_then(|meta| {
                let order_details = meta.get("order_details").to_owned();
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
        amount: payment_data.get_amount(),
        payment_intent: payment_data.get_payment_intent().to_owned(),
        payment_attempt: payment_data.get_payment_attempt().to_owned(),
        merchant_account: merchant_account.to_owned(),
        address: payment_data.get_address().clone(),
        connector_details: frm_connector_details.clone(),
        order_details,
        frm_metadata: payment_data.get_payment_intent().frm_metadata.clone(),
    };

    let fraud_check_operation: operation::BoxedFraudCheckOperation<F, D> =
        fraud_check_operation_by_frm_preferred_flow_type(frm_configs.frm_preferred_flow_type);
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

fn fraud_check_operation_by_frm_preferred_flow_type<F, D>(
    frm_preferred_flow_type: api_enums::FrmPreferredFlowTypes,
) -> operation::BoxedFraudCheckOperation<F, D>
where
    operation::FraudCheckPost: operation::FraudCheckOperation<F, D>,
    operation::FraudCheckPre: operation::FraudCheckOperation<F, D>,
{
    match frm_preferred_flow_type {
        api_enums::FrmPreferredFlowTypes::Pre => Box::new(operation::FraudCheckPre),
        api_enums::FrmPreferredFlowTypes::Post => Box::new(operation::FraudCheckPost),
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn pre_payment_frm_core<F, Req, D>(
    state: &SessionState,
    merchant_account: &domain::MerchantAccount,
    payment_data: &mut D,
    frm_info: &mut FrmInfo<F, D>,
    frm_configs: FrmConfigsObject,
    customer: &Option<domain::Customer>,
    should_continue_transaction: &mut bool,
    should_continue_capture: &mut bool,
    key_store: domain::MerchantKeyStore,
    operation: &BoxedOperation<'_, F, Req, D>,
) -> RouterResult<Option<FrmData>>
where
    F: Send + Clone,
    D: payments::OperationSessionGetters<F>
        + payments::OperationSessionSetters<F>
        + Send
        + Sync
        + Clone,
{
    let mut frm_data = None;
    if is_operation_allowed(operation) {
        frm_data = if let Some(frm_data) = &mut frm_info.frm_data {
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
                        key_store.clone(),
                    )
                    .await?;
                let _router_data = call_frm_service::<F, frm_api::Transaction, _, D>(
                    state,
                    payment_data,
                    frm_data,
                    merchant_account,
                    &key_store,
                    customer,
                )
                .await?;
                let frm_data_updated = fraud_check_operation
                    .to_update_tracker()?
                    .update_tracker(
                        state,
                        &key_store,
                        frm_data.clone(),
                        payment_data,
                        None,
                        frm_router_data,
                    )
                    .await?;
                let frm_fraud_check = frm_data_updated.fraud_check.clone();
                payment_data.set_frm_message(frm_fraud_check.clone());
                if matches!(frm_fraud_check.frm_status, FraudCheckStatus::Fraud) {
                    *should_continue_transaction = false;
                    frm_info.suggested_action = Some(FrmSuggestion::FrmCancelTransaction);
                }
                logger::debug!(
                    "frm_updated_data: {:?} {:?}",
                    frm_info.fraud_check_operation,
                    frm_info.suggested_action
                );
                Some(frm_data_updated)
            } else if matches!(
                frm_configs.frm_preferred_flow_type,
                api_enums::FrmPreferredFlowTypes::Post
            ) && !matches!(
                frm_data.fraud_check.frm_status,
                FraudCheckStatus::TransactionFailure // Incase of TransactionFailure frm status(No frm decision is taken by frm processor), if capture method is automatic we should not change it to manual.
            ) {
                *should_continue_capture = false;
                Some(frm_data.to_owned())
            } else {
                Some(frm_data.to_owned())
            }
        } else {
            None
        };
    }
    Ok(frm_data)
}

#[allow(clippy::too_many_arguments)]
pub async fn post_payment_frm_core<F, D>(
    state: &SessionState,
    req_state: ReqState,
    merchant_account: &domain::MerchantAccount,
    payment_data: &mut D,
    frm_info: &mut FrmInfo<F, D>,
    frm_configs: FrmConfigsObject,
    customer: &Option<domain::Customer>,
    key_store: domain::MerchantKeyStore,
    should_continue_capture: &mut bool,
    platform_merchant_account: Option<&domain::MerchantAccount>,
) -> RouterResult<Option<FrmData>>
where
    F: Send + Clone,
    D: payments::OperationSessionGetters<F>
        + payments::OperationSessionSetters<F>
        + Send
        + Sync
        + Clone,
{
    if let Some(frm_data) = &mut frm_info.frm_data {
        // Allow the Post flow only if the payment is authorized,
        // this logic has to be removed if we are going to call /sale or /transaction after failed transaction
        let fraud_check_operation = &mut frm_info.fraud_check_operation;
        if payment_data.get_payment_attempt().status == AttemptStatus::Authorized {
            let frm_router_data_opt = fraud_check_operation
                .to_domain()?
                .post_payment_frm(
                    state,
                    req_state.clone(),
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
                        state,
                        &key_store,
                        frm_data.to_owned(),
                        payment_data,
                        None,
                        frm_router_data.to_owned(),
                    )
                    .await?;
                let frm_fraud_check = frm_data.fraud_check.clone();
                let mut frm_suggestion = None;
                payment_data.set_frm_message(frm_fraud_check.clone());
                if matches!(frm_fraud_check.frm_status, FraudCheckStatus::Fraud) {
                    frm_info.suggested_action = Some(FrmSuggestion::FrmCancelTransaction);
                } else if matches!(frm_fraud_check.frm_status, FraudCheckStatus::ManualReview) {
                    frm_info.suggested_action = Some(FrmSuggestion::FrmManualReview);
                }
                fraud_check_operation
                    .to_domain()?
                    .execute_post_tasks(
                        state,
                        req_state,
                        &mut frm_data,
                        merchant_account,
                        frm_configs,
                        &mut frm_suggestion,
                        key_store.clone(),
                        payment_data,
                        customer,
                        should_continue_capture,
                        platform_merchant_account,
                    )
                    .await?;
                logger::debug!("frm_post_tasks_data: {:?}", frm_data);
                let updated_frm_data = fraud_check_operation
                    .to_update_tracker()?
                    .update_tracker(
                        state,
                        &key_store,
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
pub async fn call_frm_before_connector_call<F, Req, D>(
    operation: &BoxedOperation<'_, F, Req, D>,
    merchant_account: &domain::MerchantAccount,
    payment_data: &mut D,
    state: &SessionState,
    frm_info: &mut Option<FrmInfo<F, D>>,
    customer: &Option<domain::Customer>,
    should_continue_transaction: &mut bool,
    should_continue_capture: &mut bool,
    key_store: domain::MerchantKeyStore,
) -> RouterResult<Option<FrmConfigsObject>>
where
    F: Send + Clone,
    D: payments::OperationSessionGetters<F>
        + payments::OperationSessionSetters<F>
        + Send
        + Sync
        + Clone,
{
    let (is_frm_enabled, frm_routing_algorithm, frm_connector_label, frm_configs) =
        should_call_frm(merchant_account, payment_data, state, key_store.clone()).await?;
    if let Some((frm_routing_algorithm_val, profile_id)) =
        frm_routing_algorithm.zip(frm_connector_label)
    {
        if let Some(frm_configs) = frm_configs.clone() {
            let mut updated_frm_info = Box::pin(make_frm_data_and_fraud_check_operation(
                &*state.store,
                state,
                merchant_account,
                payment_data.to_owned(),
                frm_routing_algorithm_val,
                profile_id,
                frm_configs.clone(),
                customer,
            ))
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
                    operation,
                )
                .await?;
            }
            *frm_info = Some(updated_frm_info);
        }
    }
    let fraud_capture_method = frm_info.as_ref().and_then(|frm_info| {
        frm_info
            .frm_data
            .as_ref()
            .map(|frm_data| frm_data.fraud_check.payment_capture_method)
    });
    if matches!(fraud_capture_method, Some(Some(CaptureMethod::Manual)))
        && matches!(
            payment_data.get_payment_attempt().status,
            AttemptStatus::Unresolved
        )
    {
        if let Some(info) = frm_info {
            info.suggested_action = Some(FrmSuggestion::FrmAuthorizeTransaction)
        };
        *should_continue_transaction = false;
        logger::debug!(
            "skipping connector call since payment_capture_method is already {:?}",
            fraud_capture_method
        );
    };
    logger::debug!("frm_configs: {:?} {:?}", frm_configs, is_frm_enabled);
    Ok(frm_configs)
}

pub fn is_operation_allowed<Op: Debug>(operation: &Op) -> bool {
    ![
        "PaymentSession",
        "PaymentApprove",
        "PaymentReject",
        "PaymentCapture",
        "PaymentsCancel",
    ]
    .contains(&format!("{operation:?}").as_str())
}

#[cfg(feature = "v1")]
impl From<PaymentToFrmData> for PaymentDetails {
    fn from(payment_data: PaymentToFrmData) -> Self {
        Self {
            amount: common_utils::types::MinorUnit::from(payment_data.amount).get_amount_as_i64(),
            currency: payment_data.payment_attempt.currency,
            payment_method: payment_data.payment_attempt.payment_method,
            payment_method_type: payment_data.payment_attempt.payment_method_type,
            refund_transaction_id: None,
        }
    }
}

#[cfg(feature = "v1")]
#[instrument(skip_all)]
pub async fn frm_fulfillment_core(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    req: frm_core_types::FrmFulfillmentRequest,
) -> RouterResponse<frm_types::FraudCheckResponseData> {
    let db = &*state.clone().store;
    let payment_intent = db
        .find_payment_intent_by_payment_id_merchant_id(
            &(&state).into(),
            &req.payment_id.clone(),
            merchant_account.get_id(),
            &key_store,
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
                    merchant_account.get_id().clone(),
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

#[cfg(feature = "v1")]
#[instrument(skip_all)]
pub async fn make_fulfillment_api_call(
    db: &dyn StorageInterface,
    fraud_check: FraudCheck,
    payment_intent: PaymentIntent,
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    req: frm_core_types::FrmFulfillmentRequest,
) -> RouterResponse<frm_types::FraudCheckResponseData> {
    let payment_attempt = db
        .find_payment_attempt_by_attempt_id_merchant_id(
            &payment_intent.active_attempt.get_id(),
            merchant_account.get_id(),
            merchant_account.storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::PaymentNotFound)?;
    let connector_data = FraudCheckConnectorData::get_connector_by_name(&fraud_check.frm_name)?;
    let connector_integration: services::BoxedFrmConnectorIntegrationInterface<
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
        payment_capture_method: fraud_check.payment_capture_method,
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
