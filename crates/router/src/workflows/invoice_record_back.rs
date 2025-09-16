use async_trait::async_trait;
use common_utils::ext_traits::ValueExt;
use diesel_models::process_tracker::business_status;
use error_stack::ResultExt;
use router_env::logger;
use scheduler::{consumer::workflows::ProcessTrackerWorkflow, errors};

#[cfg(feature = "v1")]
use crate::routes::payments::get_or_generate_payment_id;
use crate::{
    core::{errors::RecoveryError::ProcessTrackerFailure, payments},
    routes::SessionState,
    services,
    types::{api as api_types, domain, storage},
    workflows,
};
pub struct InvoiceRecordBack;

#[async_trait]
impl ProcessTrackerWorkflow<SessionState> for InvoiceRecordBack {
    #[cfg(feature = "v1")]
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        let tracking_data = process
            .tracking_data
            .clone()
            .parse_value::<api_models::process_tracker::invoice_record_back::InvoiceRecordBackTrackingData>(
                "InvoiceRecordBackTrackingData",
            )?;

        match process.name.as_deref() {
            Some("INVOICE_RECORD_BACK") => {
                Box::pin(perform_subscription_invoice_record_back(
                    state,
                    process,
                    &tracking_data,
                ))
                .await
            }
            _ => Err(errors::ProcessTrackerError::JobNotFound),
        }
    }
    #[cfg(feature = "v2")]
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        todo!()
    }
}
#[cfg(feature = "v1")]
async fn perform_subscription_invoice_record_back(
    state: &SessionState,
    process: storage::ProcessTracker,
    tracking_data: &api_models::process_tracker::invoice_record_back::InvoiceRecordBackTrackingData,
) -> Result<(), errors::ProcessTrackerError> {
    // Extract merchant context
    let key_manager_state = &state.into();
    let key_store = state
        .store
        .get_merchant_key_store_by_merchant_id(
            key_manager_state,
            &tracking_data.merchant_id,
            &state.store.get_master_key().to_vec().into(),
        )
        .await?;

    let merchant_account = state
        .store
        .find_merchant_account_by_merchant_id(
            key_manager_state,
            &tracking_data.merchant_id,
            &key_store,
        )
        .await?;

    let billing_processor_mca = state
        .store
        .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
            key_manager_state,
            merchant_account.get_id(),
            &tracking_data.billing_processor_mca_id,
            &key_store,
        )
        .await?;

    let profile = state
        .store
        .find_business_profile_by_profile_id(
            key_manager_state,
            &key_store,
            &tracking_data.profile_id,
        )
        .await?;

    // let merchant_context = domain::MerchantContext::NormalMerchant(Box::new(domain::Context(
    //     merchant_account,
    //     key_store,
    // )));

    let profile_id = profile.get_id().clone();

    let should_refund_payment = tracking_data.should_refund;

    // Call Payemnt Sync API
    // if refund flag is on, call refund payment API on payment success
    // If payment is successful, record back to billing processor
    // else if pending, schedule a retry

    let status = common_enums::IntentStatus::Succeeded;

    if status == common_enums::IntentStatus::Succeeded {
        if should_refund_payment {
            // Call refund payment API
            // Mark process as complete
        } else {
            // Record back to billing processor
        }
        state
            .store
            .as_scheduler()
            .finish_process_with_business_status(process.clone(), business_status::COMPLETED_BY_PT)
            .await?
    } else if status == common_enums::IntentStatus::Processing {
        let db = &*state.store;
        let connector = billing_processor_mca.connector_name.clone();
        let is_last_retry = workflows::payment_sync::retry_sync_task(
            db,
            connector,
            merchant_account.get_id().to_owned(),
            process.clone(),
        )
        .await?;

        // Map out all cases here
        if is_last_retry {
            // Perform payment ops
            state
                .store
                .as_scheduler()
                .finish_process_with_business_status(process, business_status::GLOBAL_FAILURE)
                .await?
        }
    } else {
        // Handle payment failure - log the payment status and return appropriate error
        logger::error!(
            "Payment failed for invoice record back. Payment ID: {:?}, Status: {:?}",
            tracking_data.payment_id,
            status
        );
        return Err(errors::ProcessTrackerError::FlowExecutionError {
            flow: "INVOICE_RECORD_BACK",
        });
    }

    // let billing_connector_details = BillingConnectorDetails {
    //     processor_mca: tracking_data.billing_connector_mca_id.clone(),
    //     subscription_id: tracking_data
    //         .subscription_id
    //         .clone()
    //         .ok_or_else(|| errors::ProcessTrackerError::SerializationFailed)?,
    //     invoice_id: tracking_data.invoice_id.clone(),
    // };

    // logger::debug!(
    //     "Executing subscription MIT payment for process: {:?}, tracking_data: {:?}",
    //     process.id,
    //     tracking_data
    // );

    // // Create MIT payment request with the determined payment_method_id
    // let mut payment_request = api_types::PaymentsRequest {
    //     amount: Some(api_types::Amount::from(tracking_data.amount)),
    //     currency: Some(tracking_data.currency),
    //     customer_id: tracking_data.customer_id.clone(),
    //     recurring_details: Some(api_models::mandates::RecurringDetails::PaymentMethodId(
    //         tracking_data.payment_method_id.clone(),
    //     )),
    //     merchant_id: Some(tracking_data.merchant_id.clone()),
    //     billing_processor_details: Some(billing_connector_details),
    //     confirm: Some(true),
    //     off_session: Some(true),
    //     ..Default::default()
    // };

    // logger::debug!(
    //     "payment_request for subscription MIT payment: {:?}, process_id: {:?}, tracking_data: {:?}",
    //     payment_request,
    //     process.id,
    //     payment_request
    // );

    // if let Err(err) = get_or_generate_payment_id(&mut payment_request) {
    //     return Err(err.into());
    // }

    // // Execute MIT payment
    // let payment_response = payments::payments_core::<
    //     api_types::Authorize,
    //     api_types::PaymentsResponse,
    //     _,
    //     _,
    //     _,
    //     payments::PaymentData<api_types::Authorize>,
    // >(
    //     state.clone(),
    //     state.get_req_state(),
    //     merchant_context,
    //     Some(profile_id),
    //     payments::PaymentCreate,
    //     payment_request,
    //     services::api::AuthFlow::Merchant,
    //     payments::CallConnectorAction::Trigger,
    //     None,
    //     hyperswitch_domain_models::payments::HeaderPayload::with_source(
    //         common_enums::PaymentSource::Webhook,
    //     ),
    // )
    // .await;

    // let payment_res = match payment_response {
    //     Ok(services::ApplicationResponse::JsonWithHeaders((pi, _))) => Ok(pi),
    //     Ok(_) => Err(errors::ProcessTrackerError::FlowExecutionError {
    //         flow: "SUBSCRIPTION_MIT_PAYMENT",
    //     }),
    //     Err(error) => {
    //         logger::error!(?error);
    //         Err(errors::ProcessTrackerError::FlowExecutionError {
    //             flow: "SUBSCRIPTION_MIT_PAYMENT",
    //         })
    //     }
    // }?;

    // if payment_res.status == common_enums::IntentStatus::Succeeded {
    //     // Update the process tracker with the payment response
    //     let updated_process = storage::ProcessTracker {
    //         id: process.id.clone(),
    //         status: common_enums::ProcessTrackerStatus::Finish,
    //         ..process.clone()
    //     };

    //     state
    //         .store
    //         .as_scheduler()
    //         .finish_process_with_business_status(
    //             updated_process.clone(),
    //             business_status::EXECUTE_WORKFLOW_COMPLETE,
    //         )
    //         .await
    //         .change_context(ProcessTrackerFailure)
    //         .attach_printable("Failed to update the process tracker")?;
    // } else {
    //     // Handle payment failure - log the payment status and return appropriate error
    //     logger::error!(
    //         "Payment failed for subscription MIT payment. Payment ID: {:?}, Status: {:?}",
    //         payment_res.payment_id,
    //         payment_res.status
    //     );
    //     return Err(errors::ProcessTrackerError::FlowExecutionError {
    //         flow: "SUBSCRIPTION_MIT_PAYMENT",
    //     });
    // }

    Ok(())
}
