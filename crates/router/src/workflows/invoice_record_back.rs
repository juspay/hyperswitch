use std::str::FromStr;

use async_trait::async_trait;
#[cfg(feature = "v1")]
use common_utils::errors::CustomResult;
use common_utils::ext_traits::ValueExt;
use diesel_models::process_tracker::business_status;
use error_stack::ResultExt;
use router_env::logger;
use scheduler::{consumer::workflows::ProcessTrackerWorkflow, errors};

use crate::{
    core::payments,
    routes::SessionState,
    services,
    types::{api as api_types, domain, storage},
    workflows,
};
pub struct InvoiceRecordBackWorkflow;

#[async_trait]
impl ProcessTrackerWorkflow<SessionState> for InvoiceRecordBackWorkflow {
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

    // Call Payemnt Sync API
    // If payment is successful, record back to billing processor
    // else if pending, schedule a retry

    let status = common_enums::IntentStatus::Succeeded;

    if status == common_enums::IntentStatus::Succeeded {
        // Record back to billing processor
        perform_billing_processor_record_back(
            state,
            &key_store,
            tracking_data,
            &billing_processor_mca,
        )
        .await
        .attach_printable("Failed to record back to billing processor")?;

        state
            .store
            .as_scheduler()
            .finish_process_with_business_status(process.clone(), business_status::COMPLETED_BY_PT)
            .await?
    } else if status == common_enums::IntentStatus::Processing {
        let db = &*state.store;
        let connector = billing_processor_mca.connector_name.clone();
        let is_last_retry = workflows::payment_sync::retry_subscription_psync_task(
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

    Ok(())
}

#[cfg(feature = "v1")]
pub async fn perform_billing_processor_record_back(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    tracking_data: &api_models::process_tracker::invoice_record_back::InvoiceRecordBackTrackingData,
    billing_processor_mca: &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
) -> CustomResult<(), crate::errors::ApiErrorResponse> {
    logger::info!("perform_billing_processor_record_back");

    let handler = InvoiceRecordBackHandler::create(state, key_store, tracking_data, billing_processor_mca)
        .await?;

    handler.record_back_to_billing_processor().await?;
    handler.update_invoice_status().await?;

    Ok(())
}

pub async fn create_invoice_record_back_job(
    state: &SessionState,
    _key_store: &domain::MerchantKeyStore,
    payment_id: common_utils::id_type::PaymentId,
    subscription_id: String,
    billing_processor_mca_id: common_utils::id_type::MerchantConnectorAccountId,
    invoice_id: String,
    merchant_id: common_utils::id_type::MerchantId,
    profile_id: common_utils::id_type::ProfileId,
    customer_id: common_utils::id_type::CustomerId,
    amount: common_utils::types::MinorUnit,
    currency: common_enums::Currency,
    payment_method_type: Option<common_enums::PaymentMethodType>,
    attempt_status: common_enums::AttemptStatus,
) -> CustomResult<(), crate::errors::ApiErrorResponse> {
    let tracking_data =
        api_models::process_tracker::invoice_record_back::InvoiceRecordBackTrackingData::new(
            payment_id.clone(),
            subscription_id,
            billing_processor_mca_id,
            invoice_id,
            merchant_id.clone(),
            profile_id,
            customer_id.clone(),
            amount,
            currency,
            payment_method_type,
            attempt_status,
        );

    let process_tracker_entry = diesel_models::ProcessTrackerNew {
        id: common_utils::generate_id(crate::consts::ID_LENGTH, "proc"),
        name: Some("INVOICE_RECORD_BACK".to_string()),
        tag: vec!["INVOICE".to_string()],
        runner: Some(common_enums::ProcessTrackerRunner::InvoiceRecordBackflow.to_string()),
        retry_count: 5,
        schedule_time: Some(common_utils::date_time::now()),
        rule: String::new(),
        tracking_data: serde_json::to_value(&tracking_data)
            .change_context(crate::errors::ApiErrorResponse::InternalServerError)?,
        business_status: "Pending".to_string(),
        status: diesel_models::enums::ProcessTrackerStatus::New,
        event: vec![],
        created_at: common_utils::date_time::now(),
        updated_at: common_utils::date_time::now(),
        version: common_types::consts::API_VERSION,
    };

    state
        .store
        .insert_process(process_tracker_entry)
        .await
        .change_context(crate::errors::ApiErrorResponse::InternalServerError)?;

    Ok(())
}

pub struct InvoiceRecordBackHandler<'a> {
    pub state: &'a SessionState,
    pub key_store: &'a domain::MerchantKeyStore,
    pub tracking_data:
        &'a api_models::process_tracker::invoice_record_back::InvoiceRecordBackTrackingData,
    pub billing_processor_mca:
        &'a hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
    pub merchant_id: &'a common_utils::id_type::MerchantId,
    pub customer_id: &'a common_utils::id_type::CustomerId,
    pub router_data: hyperswitch_domain_models::router_data::RouterData<
        hyperswitch_domain_models::router_flow_types::InvoiceRecordBack,
        hyperswitch_domain_models::router_request_types::revenue_recovery::InvoiceRecordBackRequest,
        hyperswitch_domain_models::router_response_types::revenue_recovery::InvoiceRecordBackResponse>,
    pub invoice: diesel_models::invoice::Invoice,
    // To have an invoice object as well 
}

impl<'a> InvoiceRecordBackHandler<'a> {
    pub async fn create(
        state: &'a SessionState,
        key_store: &'a domain::MerchantKeyStore,
        tracking_data: &'a api_models::process_tracker::invoice_record_back::InvoiceRecordBackTrackingData,
        billing_processor_mca: &'a hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
    ) -> CustomResult<Self, crate::errors::ApiErrorResponse> {
        let auth_type = payments::helpers::MerchantConnectorAccountType::DbVal(Box::new(
            billing_processor_mca.clone(),
        ))
        .get_connector_account_details()
        .parse_value("ConnectorAuthType")
        .change_context(crate::errors::ApiErrorResponse::InternalServerError)?;

        let connector = billing_processor_mca.connector_name.clone();

        let connector_enum = common_enums::connector_enums::Connector::from_str(connector.as_str())
            .change_context(crate::errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Cannot find connector from the connector_name")?;

        let connector_params =
            hyperswitch_domain_models::connector_endpoints::Connectors::get_connector_params(
                &state.conf.connectors,
                connector_enum,
            )
            .change_context(crate::errors::ApiErrorResponse::InternalServerError)
            .attach_printable(format!(
                "cannot find connector params for this connector {connector} in this flow",
            ))?;

        let request = hyperswitch_domain_models::router_request_types::revenue_recovery::InvoiceRecordBackRequest {
            merchant_reference_id: common_utils::id_type::PaymentReferenceId::from_str(&tracking_data.invoice_id)
            .change_context(crate::errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to parse invoice id")?,
            amount:tracking_data.amount, 
            currency: tracking_data.currency, 
            payment_method_type: tracking_data.payment_method_type, 
            attempt_status: common_enums::AttemptStatus::Charged, 
            connector_transaction_id: None, 
            connector_params,
        };

        let router_data = hyperswitch_domain_models::router_data::RouterData {
            flow: std::marker::PhantomData,
            merchant_id: tracking_data.merchant_id.to_owned(),
            customer_id: Some(tracking_data.customer_id.to_owned()),
            connector_customer: None,
            connector,
            payment_id: "DefaultPaymentId".to_string(),
            tenant_id: state.tenant.tenant_id.clone(),
            attempt_id: "Subscriptions attempt".to_owned(),
            status: common_enums::AttemptStatus::default(),
            payment_method: common_enums::PaymentMethod::default(),
            connector_auth_type: auth_type,
            description: None,
            address: hyperswitch_domain_models::payment_address::PaymentAddress::default(),
            auth_type: common_enums::AuthenticationType::default(),
            connector_meta_data: None,
            connector_wallets_details: None,
            amount_captured: None,
            minor_amount_captured: None,
            access_token: None,
            session_token: None,
            reference_id: None,
            payment_method_token: None,
            recurring_mandate_payment_data: None,
            preprocessing_id: None,
            payment_method_balance: None,
            connector_api_version: None,
            request,
            response: Err(hyperswitch_domain_models::router_data::ErrorResponse::default()),
            connector_request_reference_id: "Notjing".to_owned(),
            #[cfg(feature = "payouts")]
            payout_method_data: None,
            #[cfg(feature = "payouts")]
            quote_id: None,
            test_mode: None,
            connector_http_status_code: None,
            external_latency: None,
            apple_pay_flow: None,
            frm_metadata: None,
            dispute_id: None,
            refund_id: None,
            payment_method_status: None,
            connector_response: None,
            integrity_check: Ok(()),
            additional_merchant_data: None,
            header_payload: None,
            connector_mandate_request_reference_id: None,
            authentication_id: None,
            psd2_sca_exemption_type: None,
            raw_connector_response: None,
            is_payment_id_from_merchant: None,
            l2_l3_data: None,
            minor_amount_capturable: None,
        };

        Ok(Self {
            state,
            key_store,
            tracking_data,
            billing_processor_mca,
            merchant_id: &tracking_data.merchant_id,
            customer_id: &tracking_data.customer_id,
            router_data,
        })
    }

    pub async fn record_back_to_billing_processor(
        &self,
    ) -> CustomResult<(), crate::errors::ApiErrorResponse> {
        let connector_data = api_types::ConnectorData::get_connector_by_name(
            &self.state.conf.connectors,
            &self.billing_processor_mca.connector_name,
            api_types::GetToken::Connector,
            Some(self.billing_processor_mca.get_id()),
        )
        .change_context(crate::errors::ApiErrorResponse::InternalServerError)
        .attach_printable(
            "invalid connector name received in billing merchant connector account",
        )?;

        let connector_integration: services::BoxedRevenueRecoveryRecordBackInterface<
        hyperswitch_domain_models::router_flow_types::InvoiceRecordBack,
        hyperswitch_domain_models::router_request_types::revenue_recovery::InvoiceRecordBackRequest,
        hyperswitch_domain_models::router_response_types::revenue_recovery::InvoiceRecordBackResponse,
    > = connector_data.connector.get_connector_integration();

        services::execute_connector_processing_step(
            self.state,
            connector_integration,
            &self.router_data,
            common_enums::CallConnectorAction::Trigger,
            None,
            None,
        )
        .await
        .inspect_err(|err| {
            logger::error!(
                "Error while handling response of record back to billing connector: {:?}",
                err
            );
        })
        .change_context(crate::errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while handling response of record back to billing connector")?;

        Ok(())
    }

    pub async fn update_invoice_status(
        &self,
    ) -> CustomResult<(), crate::errors::ApiErrorResponse> {
        // Update the invoice status in DB
        Ok(())
    }
}
