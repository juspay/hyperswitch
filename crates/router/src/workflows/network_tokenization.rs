use common_utils::ext_traits::ValueExt;
use error_stack::Report;
use scheduler::{
    consumer::{self, types::process_data},
    utils as pt_utils,
    workflows::ProcessTrackerWorkflow,
};

#[cfg(feature = "v2")]
use crate::core::payment_methods;
#[cfg(feature = "v1")]
use crate::core::payments::tokenization;
use crate::{
    errors, logger,
    routes::SessionState,
    types::{domain, storage, storage::NetworkTokenizationTrackingData},
};

const NETWORK_TOKENIZATION_DISABLED_STATUS: &str = "SKIPPED_NT_DISABLED";
const ALREADY_TOKENIZED_STATUS: &str = "ALREADY_TOKENIZED";
const COMPLETED_BY_PT_STATUS: &str = "COMPLETED_BY_PT";
const RETRIES_EXCEEDED_STATUS: &str = "RETRIES_EXCEEDED";
#[cfg(feature = "v2")]
const NOT_APPLICABLE_STATUS: &str = "SKIPPED_NT_NOT_APPLICABLE";

/// Result of one run of the workflow, before it is recorded on the process tracker entry.
enum TaskOutcome {
    /// Nothing left to do: finish the task with this business status.
    Finished(&'static str),
    /// The attempt failed and is worth retrying while the retry budget lasts.
    Failed(Report<errors::ApiErrorResponse>),
}

pub struct NetworkTokenizationWorkflow;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<SessionState> for NetworkTokenizationWorkflow {
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        let tracking_data: NetworkTokenizationTrackingData = process
            .tracking_data
            .clone()
            .parse_value("NetworkTokenizationTrackingData")?;

        let payment_method_id = get_payment_method_id(&tracking_data);

        let outcome = Box::pin(generate_network_token(state, &tracking_data)).await?;

        finish_task(state, process, &payment_method_id, outcome).await
    }

    async fn error_handler<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
        error: errors::ProcessTrackerError,
    ) -> errors::CustomResult<(), errors::ProcessTrackerError> {
        consumer::consumer_error_handler(state.store.as_scheduler(), process, error).await
    }
}

#[cfg(feature = "v1")]
fn get_payment_method_id(tracking_data: &NetworkTokenizationTrackingData) -> String {
    tracking_data.payment_method_id.clone()
}

#[cfg(feature = "v2")]
fn get_payment_method_id(tracking_data: &NetworkTokenizationTrackingData) -> String {
    tracking_data.payment_method_id.get_string_repr().to_owned()
}

/// Generates and persists the network token for the payment method the task was scheduled for.
///
/// Errors that the task can recover from by running again are returned as
/// [`TaskOutcome::Failed`]; errors in reading the merchant, profile or payment method are
/// propagated, since they leave the task in an unknown state.
#[cfg(feature = "v1")]
async fn generate_network_token(
    state: &SessionState,
    tracking_data: &NetworkTokenizationTrackingData,
) -> Result<TaskOutcome, errors::ProcessTrackerError> {
    let db = &*state.store;
    let merchant_id = tracking_data.merchant_id.clone();

    let key_store = db
        .get_merchant_key_store_by_merchant_id(&merchant_id, &db.get_master_key().to_vec().into())
        .await
        .inspect_err(|error| {
            logger::error!(
                merchant_id=%merchant_id.get_string_repr(),
                ?error,
                "Failed to fetch merchant key store for network tokenization workflow"
            );
        })?;

    let merchant_account = db
        .find_merchant_account_by_merchant_id(&merchant_id, &key_store)
        .await
        .inspect_err(|error| {
            logger::error!(
                merchant_id=%merchant_id.get_string_repr(),
                ?error,
                "Failed to fetch merchant account for network tokenization workflow"
            );
        })?;

    let business_profile = db
        .find_business_profile_by_merchant_id_profile_id(
            &key_store,
            &merchant_id,
            &tracking_data.profile_id,
        )
        .await
        .inspect_err(|error| {
            logger::error!(
                merchant_id=%merchant_id.get_string_repr(),
                profile_id=%tracking_data.profile_id.get_string_repr(),
                ?error,
                "Failed to fetch business profile for network tokenization workflow"
            );
        })?;

    if !business_profile.is_network_tokenization_enabled {
        // A skip is a terminal success, not a failure, so finish with a business status rather
        // than mapping it to an error state (which would trigger retries).
        logger::info!(
            payment_method_id=%tracking_data.payment_method_id,
            "Network tokenization is disabled for this profile, skipping"
        );
        Ok(TaskOutcome::Finished(NETWORK_TOKENIZATION_DISABLED_STATUS))
    } else {
        let payment_method = db
            .find_payment_method(
                &key_store,
                &tracking_data.payment_method_id,
                merchant_account.storage_scheme,
            )
            .await
            .inspect_err(|error| {
                logger::error!(
                    payment_method_id=%get_payment_method_id(tracking_data),
                    ?error,
                    "Failed to fetch payment method for network tokenization workflow"
                );
            })?;

        if payment_method
            .network_token_requestor_reference_id
            .is_some()
        {
            // Already tokenized — another terminal success, no retry needed.
            logger::info!(
                payment_method_id=%tracking_data.payment_method_id,
                "Payment method already has a network token, skipping"
            );
            Ok(TaskOutcome::Finished(ALREADY_TOKENIZED_STATUS))
        } else {
            let platform = domain::Platform::new(
                merchant_account.clone(),
                key_store.clone(),
                merchant_account,
                key_store,
                None,
            );

            let result = Box::pin(tokenization::generate_network_token_for_payment_method(
                state,
                &platform,
                tracking_data,
                payment_method,
            ))
            .await;

            Ok(match result {
                Ok(()) => TaskOutcome::Finished(COMPLETED_BY_PT_STATUS),
                Err(err) => TaskOutcome::Failed(err),
            })
        }
    }
}

/// Generates and persists the network token for the payment method the task was scheduled for.
///
/// Errors that the task can recover from by running again are returned as
/// [`TaskOutcome::Failed`]; errors in reading the merchant, profile or payment method are
/// propagated, since they leave the task in an unknown state.
#[cfg(feature = "v2")]
async fn generate_network_token(
    state: &SessionState,
    tracking_data: &NetworkTokenizationTrackingData,
) -> Result<TaskOutcome, errors::ProcessTrackerError> {
    let db = &*state.store;
    let merchant_id = tracking_data.merchant_id.clone();

    let key_store = db
        .get_merchant_key_store_by_merchant_id(&merchant_id, &db.get_master_key().to_vec().into())
        .await
        .inspect_err(|error| {
            logger::error!(
                merchant_id=%merchant_id.get_string_repr(),
                ?error,
                "Failed to fetch merchant key store for network tokenization workflow"
            );
        })?;

    let merchant_account = db
        .find_merchant_account_by_merchant_id(&merchant_id, &key_store)
        .await
        .inspect_err(|error| {
            logger::error!(
                merchant_id=%merchant_id.get_string_repr(),
                ?error,
                "Failed to fetch merchant account for network tokenization workflow"
            );
        })?;

    let business_profile = db
        .find_business_profile_by_merchant_id_profile_id(
            &key_store,
            &merchant_id,
            &tracking_data.profile_id,
        )
        .await
        .inspect_err(|error| {
            logger::error!(
                merchant_id=%merchant_id.get_string_repr(),
                profile_id=%tracking_data.profile_id.get_string_repr(),
                ?error,
                "Failed to fetch business profile for network tokenization workflow"
            );
        })?;

    if !business_profile.is_network_tokenization_enabled {
        // A skip is a terminal success, not a failure, so finish with a business status rather
        // than mapping it to an error state (which would trigger retries).
        logger::info!(
            payment_method_id=%tracking_data.payment_method_id.get_string_repr(),
            "Network tokenization is disabled for this profile, skipping"
        );
        Ok(TaskOutcome::Finished(NETWORK_TOKENIZATION_DISABLED_STATUS))
    } else {
        let payment_method = db
            .find_payment_method(
                &key_store,
                &tracking_data.payment_method_id,
                merchant_account.storage_scheme,
            )
            .await
            .inspect_err(|error| {
                logger::error!(
                    payment_method_id=%get_payment_method_id(tracking_data),
                    ?error,
                    "Failed to fetch payment method for network tokenization workflow"
                );
            })?;

        if payment_method
            .network_token_requestor_reference_id
            .is_some()
        {
            // Already tokenized — another terminal success, no retry needed.
            logger::info!(
                payment_method_id=%tracking_data.payment_method_id.get_string_repr(),
                "Payment method already has a network token, skipping"
            );
            Ok(TaskOutcome::Finished(ALREADY_TOKENIZED_STATUS))
        } else {
            let platform = domain::Platform::new(
                merchant_account.clone(),
                key_store.clone(),
                merchant_account,
                key_store,
                None,
            );

            let result = Box::pin(payment_methods::generate_network_token_for_payment_method(
                state,
                &platform,
                &business_profile,
                payment_method,
            ))
            .await;

            Ok(match result {
                Ok(payment_methods::NetworkTokenGenerationOutcome::Generated) => {
                    TaskOutcome::Finished(COMPLETED_BY_PT_STATUS)
                }
                Ok(payment_methods::NetworkTokenGenerationOutcome::NotApplicable) => {
                    TaskOutcome::Finished(NOT_APPLICABLE_STATUS)
                }
                Err(err) => TaskOutcome::Failed(err),
            })
        }
    }
}

/// Records the outcome of a run on the process tracker entry: a terminal outcome finishes the
/// task, a failure is retried until the retry budget for payment methods is exhausted.
async fn finish_task(
    state: &SessionState,
    process: storage::ProcessTracker,
    payment_method_id: &str,
    outcome: TaskOutcome,
) -> Result<(), errors::ProcessTrackerError> {
    let db = &*state.store;
    let retry_count = process.retry_count;

    match outcome {
        TaskOutcome::Finished(business_status) => {
            db.as_scheduler()
                .finish_process_with_business_status(process, business_status)
                .await?;
        }
        TaskOutcome::Failed(err) => {
            logger::error!(
                payment_method_id=%payment_method_id,
                ?err,
                "Failed to generate network token in process tracker workflow"
            );

            let mapping = process_data::PaymentMethodsPTMapping::default();
            let time_delta = if retry_count == 0 {
                Some(mapping.default_mapping.start_after)
            } else {
                pt_utils::get_delay(retry_count + 1, &mapping.default_mapping.frequencies)
            };

            match pt_utils::get_time_from_delta(time_delta) {
                Some(schedule_time) => {
                    db.as_scheduler()
                        .retry_process(process, schedule_time)
                        .await?;
                }
                None => {
                    logger::error!(
                        payment_method_id=%payment_method_id,
                        "Exhausted all retries to generate a network token for this payment method"
                    );
                    db.as_scheduler()
                        .finish_process_with_business_status(process, RETRIES_EXCEEDED_STATUS)
                        .await?;
                }
            }
        }
    }

    Ok(())
}
