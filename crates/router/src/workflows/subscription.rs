use api_models::payments::BillingConnectorDetails;
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
};
pub struct ExecuteSubscriptionWorkflow;

#[async_trait]
impl ProcessTrackerWorkflow<SessionState> for ExecuteSubscriptionWorkflow {
    #[cfg(feature = "v1")]
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        let tracking_data = process
            .tracking_data
            .clone()
            .parse_value::<api_models::process_tracker::subscription::SubscriptionWorkflowTrackingData>(
                "SubscriptionWorkflowTrackingData",
            )?;

        match process.name.as_deref() {
            Some("SUBSCRIPTION_MIT_PAYMENT") => {
                Box::pin(perform_subscription_mit_payment(
                    state,
                    &process,
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
async fn perform_subscription_mit_payment(
    state: &SessionState,
    process: &storage::ProcessTracker,
    tracking_data: &api_models::process_tracker::subscription::SubscriptionWorkflowTrackingData,
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

    let profile = state
        .store
        .find_business_profile_by_profile_id(
            key_manager_state,
            &key_store,
            &tracking_data.profile_id,
        )
        .await?;

    let merchant_context = domain::MerchantContext::NormalMerchant(Box::new(domain::Context(
        merchant_account,
        key_store,
    )));

    let profile_id = profile.get_id().clone();

    let billing_connector_details = BillingConnectorDetails {
        processor_mca: tracking_data.billing_connector_mca_id.clone(),
        subscription_id: tracking_data.subscription_id.clone().ok_or_else(|| {
            errors::ProcessTrackerError::SerializationFailed
        })?,
        invoice_id: tracking_data.invoice_id.clone(),
    };

    logger::debug!(
        "Executing subscription MIT payment for process: {:?}, tracking_data: {:?}",
        process.id,
        tracking_data
    );


    // Create MIT payment request with the determined payment_method_id
    let mut payment_request = api_types::PaymentsRequest {
        amount: Some(api_types::Amount::from(tracking_data.amount)),
        currency: Some(tracking_data.currency),
        customer_id: tracking_data.customer_id.clone(),
        recurring_details: Some(api_models::mandates::RecurringDetails::PaymentMethodId(
            tracking_data.payment_method_id.clone(),
        )),
        merchant_id: Some(tracking_data.merchant_id.clone()),
        billing_processor_details: Some(billing_connector_details),
        confirm: Some(true),
        off_session: Some(true),
        ..Default::default()
    };

    logger::debug!(
        "payment_request for subscription MIT payment: {:?}, process_id: {:?}, tracking_data: {:?}",
        payment_request,
        process.id,
        payment_request
    );

    if let Err(err) = get_or_generate_payment_id(&mut payment_request) {
        return Err(err.into());
    }

    // Execute MIT payment
    let payment_response = payments::payments_core::<
        api_types::Authorize,
        api_types::PaymentsResponse,
        _,
        _,
        _,
        payments::PaymentData<api_types::Authorize>,
    >(
        state.clone(),
        state.get_req_state(),
        merchant_context,
        Some(profile_id),
        payments::PaymentCreate,
        payment_request,
        services::api::AuthFlow::Merchant,
        payments::CallConnectorAction::Trigger,
        None,
        hyperswitch_domain_models::payments::HeaderPayload::with_source(
            common_enums::PaymentSource::Webhook,
        ),
    )
    .await;

    let payment_res = match payment_response {
        Ok(services::ApplicationResponse::JsonWithHeaders((pi, _))) => Ok(pi),
        Ok(_) => Err(errors::ProcessTrackerError::FlowExecutionError {
            flow: "SUBSCRIPTION_MIT_PAYMENT",
        }),
        Err(error) => {
            logger::error!(?error);
            Err(errors::ProcessTrackerError::FlowExecutionError {
                flow: "SUBSCRIPTION_MIT_PAYMENT",
            })
        }
    }?;

    if payment_res.status == common_enums::IntentStatus::Succeeded {
        // Update the process tracker with the payment response
        let updated_process = storage::ProcessTracker {
            id: process.id.clone(),
            status: common_enums::ProcessTrackerStatus::Finish,
            ..process.clone()
        };

        state
            .store
            .as_scheduler()
            .finish_process_with_business_status(
                updated_process.clone(),
                business_status::EXECUTE_WORKFLOW_COMPLETE,
            )
            .await
            .change_context(ProcessTrackerFailure)
            .attach_printable("Failed to update the process tracker")?;
    } else {
        // Handle payment failure - log the payment status and return appropriate error
        logger::error!(
            "Payment failed for subscription MIT payment. Payment ID: {:?}, Status: {:?}",
            payment_res.payment_id,
            payment_res.status
        );
        return Err(errors::ProcessTrackerError::FlowExecutionError {
            flow: "SUBSCRIPTION_MIT_PAYMENT",
        });
    }

    Ok(())
}
