use std::str::FromStr;

#[cfg(feature = "v1")]
use api_models::subscription as subscription_types;
use common_utils::{errors::CustomResult, ext_traits::StringExt};
use error_stack::ResultExt;
use hyperswitch_domain_models::{self as domain, invoice::InvoiceUpdateRequest};
use router_env::logger;
use scheduler::{
    errors,
    types::process_data,
    utils as scheduler_utils,
    workflows::storage::{business_status, ProcessTracker, ProcessTrackerNew},
};
use storage_impl::StorageError;

use crate::{
    core::{
        billing_processor_handler as billing, errors as router_errors, invoice_handler,
        payments_api_client,
    },
    helpers::ForeignTryFrom,
    state::{SubscriptionState as SessionState, SubscriptionStorageInterface as StorageInterface},
    types::storage,
};

const INVOICE_SYNC_WORKFLOW: &str = "INVOICE_SYNC";
const INVOICE_SYNC_WORKFLOW_TAG: &str = "INVOICE";

pub struct InvoiceSyncHandler<'a> {
    pub state: &'a SessionState,
    pub tracking_data: storage::invoice_sync::InvoiceSyncTrackingData,
    pub key_store: hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
    pub merchant_account: hyperswitch_domain_models::merchant_account::MerchantAccount,
    pub customer: hyperswitch_domain_models::customer::Customer,
    pub profile: hyperswitch_domain_models::business_profile::Profile,
    pub subscription: hyperswitch_domain_models::subscription::Subscription,
    pub invoice: hyperswitch_domain_models::invoice::Invoice,
}

#[cfg(feature = "v1")]
impl<'a> InvoiceSyncHandler<'a> {
    pub async fn create(
        state: &'a SessionState,
        tracking_data: storage::invoice_sync::InvoiceSyncTrackingData,
    ) -> Result<Self, errors::ProcessTrackerError> {
        let key_store = state
            .store
            .get_merchant_key_store_by_merchant_id(
                &tracking_data.merchant_id,
                &state.store.get_master_key().to_vec().into(),
            )
            .await
            .attach_printable("Failed to fetch Merchant key store from DB")?;

        let merchant_account = state
            .store
            .find_merchant_account_by_merchant_id(&tracking_data.merchant_id, &key_store)
            .await
            .attach_printable("Subscriptions: Failed to fetch Merchant Account from DB")?;

        let profile = state
            .store
            .find_business_profile_by_profile_id(&key_store, &tracking_data.profile_id)
            .await
            .attach_printable("Subscriptions: Failed to fetch Business Profile from DB")?;

        let customer = state
            .store
            .find_customer_by_customer_id_merchant_id(
                &tracking_data.customer_id,
                merchant_account.get_id(),
                &key_store,
                merchant_account.storage_scheme,
            )
            .await
            .attach_printable("Subscriptions: Failed to fetch Customer from DB")?;

        let subscription = state
            .store
            .find_by_merchant_id_subscription_id(
                &key_store,
                merchant_account.get_id(),
                tracking_data.subscription_id.get_string_repr().to_string(),
            )
            .await
            .attach_printable("Subscriptions: Failed to fetch subscription from DB")?;

        let invoice = state
            .store
            .find_invoice_by_invoice_id(
                &key_store,
                tracking_data.invoice_id.get_string_repr().to_string(),
            )
            .await
            .attach_printable("invoices: unable to get latest invoice from database")?;

        Ok(Self {
            state,
            tracking_data,
            key_store,
            merchant_account,
            customer,
            profile,
            subscription,
            invoice,
        })
    }

    async fn finish_process_with_business_status(
        &self,
        process: &ProcessTracker,
        business_status: &'static str,
    ) -> CustomResult<(), router_errors::ApiErrorResponse> {
        self.state
            .store
            .as_scheduler()
            .finish_process_with_business_status(process.clone(), business_status)
            .await
            .change_context(router_errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to update process tracker status")
    }

    pub async fn perform_payments_sync(
        state: &SessionState,
        payment_intent_id: Option<&common_utils::id_type::PaymentId>,
        profile_id: &common_utils::id_type::ProfileId,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<subscription_types::PaymentResponseData, router_errors::ApiErrorResponse>
    {
        let payment_id =
            payment_intent_id.ok_or(router_errors::ApiErrorResponse::SubscriptionError {
                operation: "Invoice_sync: Missing Payment Intent ID in Invoice".to_string(),
            })?;
        let payments_response = payments_api_client::PaymentsApiClient::sync_payment(
            state,
            payment_id.get_string_repr().to_string(),
            merchant_id.get_string_repr(),
            profile_id.get_string_repr(),
        )
        .await
        .change_context(router_errors::ApiErrorResponse::SubscriptionError {
            operation: "Invoice_sync: Failed to sync payment status from payments microservice"
                .to_string(),
        })
        .attach_printable("Failed to sync payment status from payments microservice")?;
        Ok(payments_response)
    }

    pub fn generate_response(
        subscription: &hyperswitch_domain_models::subscription::Subscription,
        invoice: &hyperswitch_domain_models::invoice::Invoice,
        payment_response: &subscription_types::PaymentResponseData,
    ) -> CustomResult<
        subscription_types::ConfirmSubscriptionResponse,
        router_errors::ApiErrorResponse,
    > {
        subscription_types::ConfirmSubscriptionResponse::foreign_try_from((
            subscription,
            invoice,
            payment_response,
        ))
    }

    pub async fn form_response_for_retry_outgoing_webhook_task(
        state: SessionState,
        key_store: &domain::merchant_key_store::MerchantKeyStore,
        invoice_id: String,
        profile_id: &common_utils::id_type::ProfileId,
        merchant_account: &domain::merchant_account::MerchantAccount,
    ) -> Result<subscription_types::ConfirmSubscriptionResponse, errors::ProcessTrackerError> {
        let invoice = state
            .store
            .find_invoice_by_invoice_id(key_store, invoice_id.clone())
            .await
            .map_err(|err| {
                logger::error!(
                    ?err,
                    "invoices: unable to get latest invoice with id {invoice_id} from database"
                );
                errors::ProcessTrackerError::ResourceFetchingFailed {
                    resource_name: "Invoice".to_string(),
                }
            })?;

        let subscription = state
            .store
            .find_by_merchant_id_subscription_id(
                key_store,
                merchant_account.get_id(),
                invoice.subscription_id.get_string_repr().to_string(),
            )
            .await
            .map_err(|err| {
                logger::error!(
                    ?err,
                    "subscription: unable to get subscription from database"
                );
                errors::ProcessTrackerError::ResourceFetchingFailed {
                    resource_name: "Subscription".to_string(),
                }
            })?;

        let payments_response = InvoiceSyncHandler::perform_payments_sync(
            &state,
            invoice.payment_intent_id.as_ref(),
            profile_id,
            merchant_account.get_id(),
        )
        .await
        .map_err(|err| {
            logger::error!(
                ?err,
                "subscription: unable to make PSync Call to payments microservice"
            );
            errors::ProcessTrackerError::EApiErrorResponse
        })?;

        let response = Self::generate_response(&subscription, &invoice, &payments_response)
            .map_err(|err| {
                logger::error!(
                    ?err,
                    "subscription: unable to form ConfirmSubscriptionResponse from foreign types"
                );
                errors::ProcessTrackerError::DeserializationFailed
            })?;

        Ok(response)
    }

    pub async fn perform_billing_processor_record_back_if_possible(
        &self,
        payment_response: subscription_types::PaymentResponseData,
        payment_status: common_enums::AttemptStatus,
        connector_invoice_id: Option<common_utils::id_type::InvoiceId>,
        invoice_sync_status: storage::invoice_sync::InvoiceSyncPaymentStatus,
    ) -> CustomResult<hyperswitch_domain_models::invoice::Invoice, router_errors::ApiErrorResponse>
    {
        if let Some(connector_invoice_id) = connector_invoice_id {
            Box::pin(self.perform_billing_processor_record_back(
                payment_response,
                payment_status,
                connector_invoice_id,
                invoice_sync_status,
            ))
            .await
            .attach_printable("Failed to record back to billing processor")
        } else {
            Ok(self.invoice.clone())
        }
    }

    pub async fn perform_billing_processor_record_back(
        &self,
        payment_response: subscription_types::PaymentResponseData,
        payment_status: common_enums::AttemptStatus,
        connector_invoice_id: common_utils::id_type::InvoiceId,
        invoice_sync_status: storage::invoice_sync::InvoiceSyncPaymentStatus,
    ) -> CustomResult<hyperswitch_domain_models::invoice::Invoice, router_errors::ApiErrorResponse>
    {
        logger::info!("perform_billing_processor_record_back");

        let billing_handler = billing::BillingHandler::create(
            self.state,
            &self.merchant_account,
            &self.key_store,
            self.profile.clone(),
        )
        .await
        .attach_printable("Failed to create billing handler")?;

        let invoice_handler = invoice_handler::InvoiceHandler::new(
            self.subscription.clone(),
            self.merchant_account.clone(),
            self.profile.clone(),
            self.key_store.clone(),
        );

        // TODO: Handle retries here on failure
        billing_handler
            .record_back_to_billing_processor(
                self.state,
                connector_invoice_id.clone(),
                payment_response.payment_id.to_owned(),
                payment_status,
                payment_response.amount,
                payment_response.currency,
                payment_response.payment_method_type,
            )
            .await
            .attach_printable("Failed to record back to billing processor")?;

        let update_request = InvoiceUpdateRequest::update_connector_and_status(
            connector_invoice_id,
            common_enums::connector_enums::InvoiceStatus::from(invoice_sync_status),
        );

        invoice_handler
            .update_invoice(self.state, self.invoice.id.to_owned(), update_request)
            .await
            .attach_printable("Failed to update invoice in DB")
    }

    pub async fn transition_workflow_state(
        &self,
        process: ProcessTracker,
        payment_response: subscription_types::PaymentResponseData,
        connector_invoice_id: Option<common_utils::id_type::InvoiceId>,
    ) -> CustomResult<hyperswitch_domain_models::invoice::Invoice, router_errors::ApiErrorResponse>
    {
        logger::info!(
            "transition_workflow_state called with status: {:?}",
            payment_response.status
        );
        let invoice_sync_status =
            storage::invoice_sync::InvoiceSyncPaymentStatus::from(payment_response.status);
        let (payment_status, status) = match invoice_sync_status {
            storage::invoice_sync::InvoiceSyncPaymentStatus::PaymentSucceeded => (common_enums::AttemptStatus::Charged, "succeeded"),
            storage::invoice_sync::InvoiceSyncPaymentStatus::PaymentFailed => (common_enums::AttemptStatus::Failure, "failed"),
            storage::invoice_sync::InvoiceSyncPaymentStatus::PaymentProcessing => return Err(
                router_errors::ApiErrorResponse::SubscriptionError {
                    operation: "Invoice_sync: Payment in processing state, cannot transition workflow state"
                        .to_string(),
                },
            )?,
        };
        logger::info!("Performing billing processor record back for status: {status}");
        let invoice = Box::pin(self.perform_billing_processor_record_back_if_possible(
            payment_response.clone(),
            payment_status,
            connector_invoice_id,
            invoice_sync_status.clone(),
        ))
        .await
        .attach_printable(format!(
            "Failed to record back {status} status to billing processor"
        ))?;

        self.finish_process_with_business_status(&process, business_status::COMPLETED_BY_PT)
            .await
            .change_context(router_errors::ApiErrorResponse::SubscriptionError {
                operation: "Invoice_sync process_tracker task completion".to_string(),
            })
            .attach_printable("Failed to update process tracker status")?;
        Ok(invoice)
    }
}

#[cfg(feature = "v1")]
pub async fn perform_subscription_invoice_sync(
    state: &SessionState,
    process: ProcessTracker,
    tracking_data: storage::invoice_sync::InvoiceSyncTrackingData,
) -> Result<
    (
        InvoiceSyncHandler<'_>,
        subscription_types::PaymentResponseData,
    ),
    errors::ProcessTrackerError,
> {
    let mut handler = InvoiceSyncHandler::create(state, tracking_data).await?;

    let payments_response = InvoiceSyncHandler::perform_payments_sync(
        handler.state,
        handler.invoice.payment_intent_id.as_ref(),
        handler.profile.get_id(),
        handler.merchant_account.get_id(),
    )
    .await?;

    match Box::pin(handler.transition_workflow_state(
        process.clone(),
        payments_response.clone(),
        handler.tracking_data.connector_invoice_id.clone(),
    ))
    .await
    {
        Err(e) => {
            logger::error!(?e, "Error in transitioning workflow state");
            retry_subscription_invoice_sync_task(
                &*handler.state.store,
                handler.tracking_data.connector_name.to_string().clone(),
                handler.merchant_account.get_id().to_owned(),
                process,
            )
            .await
            .change_context(router_errors::ApiErrorResponse::SubscriptionError {
                operation: "Invoice_sync process_tracker task retry".to_string(),
            })
            .attach_printable("Failed to update process tracker status")?;
        }
        Ok(invoice) => {
            handler.invoice = invoice.clone();
        }
    }
    Ok((handler, payments_response))
}

pub async fn create_invoice_sync_job(
    state: &SessionState,
    request: storage::invoice_sync::InvoiceSyncRequest,
) -> CustomResult<(), router_errors::ApiErrorResponse> {
    let tracking_data = storage::invoice_sync::InvoiceSyncTrackingData::from(request);

    let process_tracker_entry = ProcessTrackerNew::new(
        common_utils::generate_id(common_utils::consts::ID_LENGTH, "proc"),
        INVOICE_SYNC_WORKFLOW.to_string(),
        common_enums::ProcessTrackerRunner::InvoiceSyncflow,
        vec![INVOICE_SYNC_WORKFLOW_TAG.to_string()],
        tracking_data,
        Some(0),
        common_utils::date_time::now(),
        common_types::consts::API_VERSION,
    )
    .change_context(router_errors::ApiErrorResponse::InternalServerError)
    .attach_printable("subscriptions: unable to form process_tracker type")?;

    state
        .store
        .insert_process(process_tracker_entry)
        .await
        .change_context(router_errors::ApiErrorResponse::InternalServerError)
        .attach_printable("subscriptions: unable to insert process_tracker entry in DB")?;

    Ok(())
}

pub async fn get_subscription_invoice_sync_process_schedule_time(
    db: &dyn StorageInterface,
    connector: &str,
    merchant_id: &common_utils::id_type::MerchantId,
    retry_count: i32,
) -> Result<Option<time::PrimitiveDateTime>, errors::ProcessTrackerError> {
    let mapping: CustomResult<process_data::SubscriptionInvoiceSyncPTMapping, StorageError> = db
        .find_config_by_key(&format!("invoice_sync_pt_mapping_{connector}"))
        .await
        .map(|value| value.config)
        .and_then(|config| {
            config
                .parse_struct("SubscriptionInvoiceSyncPTMapping")
                .change_context(StorageError::DeserializationFailed)
                .attach_printable("Failed to deserialize invoice_sync_pt_mapping config to struct")
        });
    let mapping = match mapping {
        Ok(x) => x,
        Err(error) => {
            logger::info!(?error, "Redis Mapping Error");
            process_data::SubscriptionInvoiceSyncPTMapping::default()
        }
    };

    let time_delta = scheduler_utils::get_subscription_invoice_sync_retry_schedule_time(
        mapping,
        merchant_id,
        retry_count,
    );

    Ok(scheduler_utils::get_time_from_delta(time_delta))
}

pub async fn retry_subscription_invoice_sync_task(
    db: &dyn StorageInterface,
    connector: String,
    merchant_id: common_utils::id_type::MerchantId,
    pt: ProcessTracker,
) -> Result<(), errors::ProcessTrackerError> {
    let schedule_time = get_subscription_invoice_sync_process_schedule_time(
        db,
        connector.as_str(),
        &merchant_id,
        pt.retry_count + 1,
    )
    .await?;

    match schedule_time {
        Some(s_time) => {
            db.as_scheduler()
                .retry_process(pt, s_time)
                .await
                .attach_printable("Failed to retry subscription invoice sync task")?;
        }
        None => {
            db.as_scheduler()
                .finish_process_with_business_status(pt, business_status::RETRIES_EXCEEDED)
                .await
                .attach_printable("Failed to finish subscription invoice sync task")?;
        }
    }

    Ok(())
}

impl
    ForeignTryFrom<(
        &domain::subscription::Subscription,
        &domain::invoice::Invoice,
        &subscription_types::PaymentResponseData,
    )> for subscription_types::ConfirmSubscriptionResponse
{
    type Error = error_stack::Report<router_errors::ApiErrorResponse>;

    fn foreign_try_from(
        value: (
            &domain::subscription::Subscription,
            &domain::invoice::Invoice,
            &subscription_types::PaymentResponseData,
        ),
    ) -> Result<Self, Self::Error> {
        let (subscription, invoice, payment_response) = value;
        let status = common_enums::SubscriptionStatus::from_str(subscription.status.as_str())
            .map_err(|_| router_errors::ApiErrorResponse::SubscriptionError {
                operation: "Failed to parse subscription status".to_string(),
            })
            .attach_printable("Failed to parse subscription status")?;

        Ok(Self {
            id: subscription.id.clone(),
            merchant_reference_id: subscription.merchant_reference_id.clone(),
            status,
            plan_id: subscription.plan_id.clone(),
            profile_id: subscription.profile_id.to_owned(),
            payment: Some(payment_response.clone()),
            customer_id: Some(subscription.customer_id.clone()),
            item_price_id: subscription.item_price_id.clone(),
            coupon: None,
            billing_processor_subscription_id: subscription.connector_subscription_id.clone(),
            invoice: Some(subscription_types::Invoice::foreign_try_from(invoice)?),
        })
    }
}
