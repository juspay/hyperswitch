#[cfg(feature = "v2")]
use api_models::payments::PaymentsGetIntentRequest;
#[cfg(feature = "v2")]
use common_utils::{
    ext_traits::{StringExt, ValueExt},
    id_type,
};
#[cfg(feature = "v2")]
use diesel_models::types::BillingConnectorPaymentMethodDetails;
#[cfg(feature = "v2")]
use error_stack::ResultExt;
#[cfg(all(feature = "revenue_recovery", feature = "v2"))]
use external_services::{
    date_time, grpc_client::revenue_recovery::recovery_decider_client as external_grpc_client,
};
#[cfg(feature = "v2")]
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    payments::{
        payment_attempt::PaymentAttempt, PaymentConfirmData, PaymentIntent, PaymentIntentData,
    },
    router_flow_types::Authorize,
};
#[cfg(feature = "v2")]
use masking::{ExposeInterface, PeekInterface, Secret};
#[cfg(feature = "v2")]
use router_env::logger;
use scheduler::{consumer::workflows::ProcessTrackerWorkflow, errors};
#[cfg(feature = "v2")]
use scheduler::{types::process_data, utils as scheduler_utils};
#[cfg(feature = "v2")]
use storage_impl::errors as storage_errors;

#[cfg(feature = "v2")]
use crate::{
    core::{
        payments,
        revenue_recovery::{self as pcr},
    },
    db::StorageInterface,
    errors::StorageError,
    types::{
        api::{self as api_types},
        domain,
        storage::revenue_recovery as pcr_storage_types,
    },
};
use crate::{routes::SessionState, types::storage};
pub struct ExecutePcrWorkflow;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<SessionState> for ExecutePcrWorkflow {
    #[cfg(feature = "v1")]
    async fn execute_workflow<'a>(
        &'a self,
        _state: &'a SessionState,
        _process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        Ok(())
    }
    #[cfg(feature = "v2")]
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        let tracking_data = process
            .tracking_data
            .clone()
            .parse_value::<pcr_storage_types::RevenueRecoveryWorkflowTrackingData>(
            "PCRWorkflowTrackingData",
        )?;
        let request = PaymentsGetIntentRequest {
            id: tracking_data.global_payment_id.clone(),
        };
        let revenue_recovery_payment_data =
            extract_data_and_perform_action(state, &tracking_data).await?;
        let merchant_context_from_revenue_recovery_payment_data =
            domain::MerchantContext::NormalMerchant(Box::new(domain::Context(
                revenue_recovery_payment_data.merchant_account.clone(),
                revenue_recovery_payment_data.key_store.clone(),
            )));
        let (payment_data, _, _) = payments::payments_intent_operation_core::<
            api_types::PaymentGetIntent,
            _,
            _,
            PaymentIntentData<api_types::PaymentGetIntent>,
        >(
            state,
            state.get_req_state(),
            merchant_context_from_revenue_recovery_payment_data,
            revenue_recovery_payment_data.profile.clone(),
            payments::operations::PaymentGetIntent,
            request,
            tracking_data.global_payment_id.clone(),
            hyperswitch_domain_models::payments::HeaderPayload::default(),
        )
        .await?;

        match process.name.as_deref() {
            Some("EXECUTE_WORKFLOW") => {
                Box::pin(pcr::perform_execute_payment(
                    state,
                    &process,
                    &tracking_data,
                    &revenue_recovery_payment_data,
                    &payment_data.payment_intent,
                ))
                .await
            }
            Some("PSYNC_WORKFLOW") => {
                Box::pin(pcr::perform_payments_sync(
                    state,
                    &process,
                    &tracking_data,
                    &revenue_recovery_payment_data,
                    &payment_data.payment_intent,
                ))
                .await?;
                Ok(())
            }

            _ => Err(errors::ProcessTrackerError::JobNotFound),
        }
    }
}
#[cfg(feature = "v2")]
pub(crate) async fn extract_data_and_perform_action(
    state: &SessionState,
    tracking_data: &pcr_storage_types::RevenueRecoveryWorkflowTrackingData,
) -> Result<pcr_storage_types::RevenueRecoveryPaymentData, errors::ProcessTrackerError> {
    let db = &state.store;

    let key_manager_state = &state.into();
    let key_store = db
        .get_merchant_key_store_by_merchant_id(
            key_manager_state,
            &tracking_data.merchant_id,
            &db.get_master_key().to_vec().into(),
        )
        .await?;

    let merchant_account = db
        .find_merchant_account_by_merchant_id(
            key_manager_state,
            &tracking_data.merchant_id,
            &key_store,
        )
        .await?;

    let profile = db
        .find_business_profile_by_profile_id(
            key_manager_state,
            &key_store,
            &tracking_data.profile_id,
        )
        .await?;

    let billing_mca = db
        .find_merchant_connector_account_by_id(
            key_manager_state,
            &tracking_data.billing_mca_id,
            &key_store,
        )
        .await?;

    let pcr_payment_data = pcr_storage_types::RevenueRecoveryPaymentData {
        merchant_account,
        profile,
        key_store,
        billing_mca,
        retry_algorithm: tracking_data.revenue_recovery_retry,
    };
    Ok(pcr_payment_data)
}

#[cfg(feature = "v2")]
pub(crate) async fn get_schedule_time_to_retry_mit_payments(
    db: &dyn StorageInterface,
    merchant_id: &id_type::MerchantId,
    retry_count: i32,
) -> Option<time::PrimitiveDateTime> {
    let key = "pt_mapping_pcr_retries";
    let result = db
        .find_config_by_key(key)
        .await
        .map(|value| value.config)
        .and_then(|config| {
            config
                .parse_struct("RevenueRecoveryPaymentProcessTrackerMapping")
                .change_context(StorageError::DeserializationFailed)
        });

    let mapping = result.map_or_else(
        |error| {
            if error.current_context().is_db_not_found() {
                logger::debug!("Revenue Recovery retry config `{key}` not found, ignoring");
            } else {
                logger::error!(
                    ?error,
                    "Failed to read Revenue Recovery retry config `{key}`"
                );
            }
            process_data::RevenueRecoveryPaymentProcessTrackerMapping::default()
        },
        |mapping| {
            logger::debug!(?mapping, "Using custom pcr payments retry config");
            mapping
        },
    );

    let time_delta =
        scheduler_utils::get_pcr_payments_retry_schedule_time(mapping, merchant_id, retry_count);

    scheduler_utils::get_time_from_delta(time_delta)
}

#[cfg(feature = "v2")]
pub(crate) async fn get_schedule_time_for_smart_retry(
    state: &SessionState,
    payment_attempt: &PaymentAttempt,
    payment_intent: &PaymentIntent,
    retry_count: i32,
) -> Option<time::PrimitiveDateTime> {
    let first_error_message = payment_attempt.error.as_ref().map_or(
        hyperswitch_interfaces::consts::NO_ERROR_MESSAGE.to_string(),
        |error| error.message.clone(),
    );

    let billing_state = payment_intent
        .billing_address
        .as_ref()
        .and_then(|addr_enc| addr_enc.get_inner().address.as_ref())
        .and_then(|details| details.state.as_ref())
        .cloned()?;

    // Check if payment_method_data itself is None
    if payment_attempt.payment_method_data.is_none() {
        logger::debug!(
            payment_intent_id = ?payment_intent.get_id(),
            attempt_id = ?payment_attempt.get_id(),
            message = "payment_attempt.payment_method_data is None"
        );
    }

    let billing_connector_payment_method_details = payment_intent
        .feature_metadata
        .as_ref()
        .and_then(|revenue_recovery_data| {
            revenue_recovery_data
                .payment_revenue_recovery_metadata
                .as_ref()
        })
        .and_then(|payment_metadata| {
            payment_metadata
                .billing_connector_payment_method_details
                .as_ref()
        });

    let card_network_str = billing_connector_payment_method_details
        .and_then(|details| match details {
            BillingConnectorPaymentMethodDetails::Card(card_info) => card_info.card_network.clone(),
        })
        .map(|cn| cn.to_string())?;

    let card_issuer_str =
        billing_connector_payment_method_details.and_then(|details| match details {
            BillingConnectorPaymentMethodDetails::Card(card_info) => card_info.card_issuer.clone(),
        })?;

    let card_funding_str = payment_intent
        .feature_metadata
        .as_ref()
        .and_then(|revenue_recovery_data| {
            revenue_recovery_data
                .payment_revenue_recovery_metadata
                .as_ref()
        })
        .map(|payment_metadata| payment_metadata.payment_method_subtype.to_string())?;

    let start_time_primitive = payment_intent.created_at;
    let recovery_timestamp_config = &state.conf.revenue_recovery.recovery_timestamp;

    let modified_start_time_primitive = start_time_primitive.saturating_add(time::Duration::hours(
        recovery_timestamp_config.initial_timestamp_in_hours,
    ));
    let start_time_proto = date_time::convert_to_prost_timestamp(modified_start_time_primitive);

    let end_time_primitive = start_time_primitive.saturating_add(time::Duration::hours(
        recovery_timestamp_config.final_timestamp_in_hours,
    ));
    let end_time_proto = date_time::convert_to_prost_timestamp(end_time_primitive);

    let decider_request = InternalDeciderRequest {
        first_error_message,
        billing_state,
        card_funding: card_funding_str,
        card_network: card_network_str,
        card_issuer: card_issuer_str,
        start_time: Some(start_time_proto),
        end_time: Some(end_time_proto),
        retry_count: retry_count.into(),
    };

    if let Some(mut client) = state.grpc_client.recovery_decider_client.clone() {
        match client
            .decide_on_retry(decider_request.into(), state.get_recovery_grpc_headers())
            .await
        {
            Ok(grpc_response) => grpc_response
                .retry_flag
                .then_some(())
                .and(grpc_response.retry_time)
                .and_then(|prost_ts| {
                    match date_time::convert_from_prost_timestamp(&prost_ts) {
                        Ok(pdt) => Some(pdt),
                        Err(e) => {
                            logger::error!(
                                "Failed to convert retry_time from prost::Timestamp: {e:?}"
                            );
                            None // If conversion fails, treat as no valid retry time
                        }
                    }
                }),

            Err(e) => {
                logger::error!("Recovery decider gRPC call failed: {e:?}");
                None
            }
        }
    } else {
        logger::debug!("Recovery decider client is not configured");
        None
    }
}

#[cfg(feature = "v2")]
#[derive(Debug)]
struct InternalDeciderRequest {
    first_error_message: String,
    billing_state: Secret<String>,
    card_funding: String,
    card_network: String,
    card_issuer: String,
    start_time: Option<prost_types::Timestamp>,
    end_time: Option<prost_types::Timestamp>,
    retry_count: f64,
}

#[cfg(feature = "v2")]
impl From<InternalDeciderRequest> for external_grpc_client::DeciderRequest {
    fn from(internal_request: InternalDeciderRequest) -> Self {
        Self {
            first_error_message: internal_request.first_error_message,
            billing_state: internal_request.billing_state.peek().to_string(),
            card_funding: internal_request.card_funding,
            card_network: internal_request.card_network,
            card_issuer: internal_request.card_issuer,
            start_time: internal_request.start_time,
            end_time: internal_request.end_time,
            retry_count: internal_request.retry_count,
        }
    }
}
