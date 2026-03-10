use std::marker::PhantomData;

use api_models::{enums::FrmSuggestion, payments::PaymentsAttemptRecordRequest};
use async_trait::async_trait;
use common_utils::{
    errors::CustomResult,
    ext_traits::{AsyncExt, Encode, ValueExt},
    types::keymanager::ToEncryptable,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::payments::PaymentAttemptRecordData;
use masking::PeekInterface;
use router_env::{instrument, tracing};

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        errors::{self, StorageErrorExt},
        payments::{
            self,
            cards::create_encrypted_data,
            helpers,
            operations::{self, ValidateStatusForOperation},
        },
    },
    db::{domain::types, errors::RouterResult},
    routes::{app::ReqState, SessionState},
    services,
    types::{
        api,
        domain::{self, types as domain_types},
        storage::{self, enums},
    },
    utils::{self, OptionExt},
};

#[derive(Debug, Clone, Copy)]
pub struct PaymentAttemptRecord;

type PaymentsAttemptRecordOperation<'b, F> =
    BoxedOperation<'b, F, PaymentsAttemptRecordRequest, PaymentAttemptRecordData<F>>;

impl<F: Send + Clone + Sync> Operation<F, PaymentsAttemptRecordRequest> for &PaymentAttemptRecord {
    type Data = PaymentAttemptRecordData<F>;
    fn to_validate_request(
        &self,
    ) -> RouterResult<
        &(dyn ValidateRequest<F, PaymentsAttemptRecordRequest, Self::Data> + Send + Sync),
    > {
        Ok(*self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<F, Self::Data, PaymentsAttemptRecordRequest> + Send + Sync)>
    {
        Ok(*self)
    }
    fn to_domain(
        &self,
    ) -> RouterResult<&(dyn Domain<F, PaymentsAttemptRecordRequest, Self::Data>)> {
        Ok(*self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, Self::Data, PaymentsAttemptRecordRequest> + Send + Sync)>
    {
        Ok(*self)
    }
}

impl ValidateStatusForOperation for PaymentAttemptRecord {
    fn validate_status_for_operation(
        &self,
        intent_status: common_enums::IntentStatus,
    ) -> Result<(), errors::ApiErrorResponse> {
        // need to verify this?
        match intent_status {
            // Payment attempt can be recorded for failed payment as well in revenue recovery flow.
            common_enums::IntentStatus::RequiresPaymentMethod
            | common_enums::IntentStatus::Failed => Ok(()),
            common_enums::IntentStatus::Succeeded
            | common_enums::IntentStatus::Cancelled
            | common_enums::IntentStatus::CancelledPostCapture
            | common_enums::IntentStatus::Processing
            | common_enums::IntentStatus::Conflicted
            | common_enums::IntentStatus::RequiresCustomerAction
            | common_enums::IntentStatus::RequiresMerchantAction
            | common_enums::IntentStatus::RequiresCapture
            | common_enums::IntentStatus::PartiallyAuthorizedAndRequiresCapture
            | common_enums::IntentStatus::PartiallyCaptured
            | common_enums::IntentStatus::RequiresConfirmation
            | common_enums::IntentStatus::PartiallyCapturedAndCapturable
            | common_enums::IntentStatus::Expired => {
                Err(errors::ApiErrorResponse::PaymentUnexpectedState {
                    current_flow: format!("{self:?}"),
                    field_name: "status".to_string(),
                    current_value: intent_status.to_string(),
                    states: [
                        common_enums::IntentStatus::RequiresPaymentMethod,
                        common_enums::IntentStatus::Failed,
                    ]
                    .map(|enum_value| enum_value.to_string())
                    .join(", "),
                })
            }
        }
    }
}

#[async_trait]
impl<F: Send + Clone + Sync>
    GetTracker<F, PaymentAttemptRecordData<F>, PaymentsAttemptRecordRequest>
    for PaymentAttemptRecord
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a SessionState,
        payment_id: &common_utils::id_type::GlobalPaymentId,
        request: &PaymentsAttemptRecordRequest,
        platform: &domain::Platform,
        _profile: &domain::Profile,
        _header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<operations::GetTrackerResponse<PaymentAttemptRecordData<F>>> {
        let db = &*state.store;
        let key_manager_state = &state.into();

        let storage_scheme = platform.get_processor().get_account().storage_scheme;

        let payment_intent = db
            .find_payment_intent_by_id(
                payment_id,
                platform.get_processor().get_key_store(),
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        self.validate_status_for_operation(payment_intent.status)?;
        let payment_method_billing_address = request
            .payment_method_data
            .as_ref()
            .and_then(|data| {
                data.billing
                    .as_ref()
                    .map(|address| address.clone().encode_to_value())
            })
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to encode payment_method_billing address")?
            .map(masking::Secret::new);

        let batch_encrypted_data = domain_types::crypto_operation(
                key_manager_state,
                common_utils::type_name!(hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt),
                domain_types::CryptoOperation::BatchEncrypt(
                    hyperswitch_domain_models::payments::payment_attempt::FromRequestEncryptablePaymentAttempt::to_encryptable(
                        hyperswitch_domain_models::payments::payment_attempt::FromRequestEncryptablePaymentAttempt {
                            payment_method_billing_address,
                        },
                    ),
                ),
                common_utils::types::keymanager::Identifier::Merchant(platform.get_processor().get_account().get_id().to_owned()),
                platform.get_processor().get_key_store().key.peek(),
            )
            .await
            .and_then(|val| val.try_into_batchoperation())
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed while encrypting payment intent details".to_string())?;

        let encrypted_data =
                 hyperswitch_domain_models::payments::payment_attempt::FromRequestEncryptablePaymentAttempt::from_encryptable(batch_encrypted_data)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed while encrypting payment intent details")?;
        let cell_id = state.conf.cell_information.id.clone();

        let payment_attempt_domain_model =
            hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt::create_domain_model_using_record_request(
                &payment_intent,
                cell_id,
                storage_scheme,
                request,
                encrypted_data,
            )
            .await?;

        let payment_attempt = db
            .insert_payment_attempt(
                platform.get_processor().get_key_store(),
                payment_attempt_domain_model,
                storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Could not insert payment attempt")?;
        let revenue_recovery_data = hyperswitch_domain_models::payments::RevenueRecoveryData {
            billing_connector_id: request.billing_connector_id.clone(),
            processor_payment_method_token: request.processor_payment_method_token.clone(),
            connector_customer_id: request.connector_customer_id.clone(),
            retry_count: request.retry_count,
            invoice_next_billing_time: request.invoice_next_billing_time,
            triggered_by: request.triggered_by,
            card_network: request.card_network.clone(),
            card_issuer: request.card_issuer.clone(),
        };
        let payment_address = hyperswitch_domain_models::payment_address::PaymentAddress::new(
            payment_intent
                .shipping_address
                .clone()
                .map(|address| address.into_inner()),
            payment_intent
                .billing_address
                .clone()
                .map(|address| address.into_inner()),
            payment_attempt
                .payment_method_billing_address
                .clone()
                .map(|address| address.into_inner()),
            Some(true),
        );

        let payment_data = PaymentAttemptRecordData {
            flow: PhantomData,
            payment_intent,
            payment_attempt,
            payment_address,
            revenue_recovery_data,
        };

        let get_trackers_response = operations::GetTrackerResponse { payment_data };

        Ok(get_trackers_response)
    }
}

#[async_trait]
impl<F: Clone + Sync> UpdateTracker<F, PaymentAttemptRecordData<F>, PaymentsAttemptRecordRequest>
    for PaymentAttemptRecord
{
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        state: &'b SessionState,
        _req_state: ReqState,
        mut payment_data: PaymentAttemptRecordData<F>,
        _customer: Option<domain::Customer>,
        storage_scheme: enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        key_store: &domain::MerchantKeyStore,
        _frm_suggestion: Option<FrmSuggestion>,
        _header_payload: hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<(
        PaymentsAttemptRecordOperation<'b, F>,
        PaymentAttemptRecordData<F>,
    )>
    where
        F: 'b + Send,
    {
        let feature_metadata = payment_data.get_updated_feature_metadata()?;
        let active_attempt_id = match payment_data.revenue_recovery_data.triggered_by {
            common_enums::TriggeredBy::Internal => Some(payment_data.payment_attempt.id.clone()),
            common_enums::TriggeredBy::External => None,
        };
        let payment_intent_update =

    hyperswitch_domain_models::payments::payment_intent::PaymentIntentUpdate::RecordUpdate
        {
            status: common_enums::IntentStatus::from(payment_data.payment_attempt.status),
            feature_metadata: Box::new(feature_metadata),
            updated_by: storage_scheme.to_string(),
            active_attempt_id
        }
    ;
        payment_data.payment_intent = state
            .store
            .update_payment_intent(
                payment_data.payment_intent,
                payment_intent_update,
                key_store,
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
        Ok((Box::new(self), payment_data))
    }
}

impl<F: Send + Clone> ValidateRequest<F, PaymentsAttemptRecordRequest, PaymentAttemptRecordData<F>>
    for PaymentAttemptRecord
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        _request: &PaymentsAttemptRecordRequest,
        platform: &'a domain::Platform,
    ) -> RouterResult<operations::ValidateResult> {
        Ok(operations::ValidateResult {
            merchant_id: platform.get_processor().get_account().get_id().to_owned(),
            storage_scheme: platform.get_processor().get_account().storage_scheme,
            requeue: false,
        })
    }
}

#[async_trait]
impl<F: Clone + Send + Sync> Domain<F, PaymentsAttemptRecordRequest, PaymentAttemptRecordData<F>>
    for PaymentAttemptRecord
{
    #[instrument(skip_all)]
    async fn get_customer_details<'a>(
        &'a self,
        _state: &SessionState,
        _payment_data: &mut PaymentAttemptRecordData<F>,
        _merchant_key_store: &domain::MerchantKeyStore,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<
        (
            BoxedOperation<'a, F, PaymentsAttemptRecordRequest, PaymentAttemptRecordData<F>>,
            Option<domain::Customer>,
        ),
        errors::StorageError,
    > {
        Ok((Box::new(self), None))
    }

    #[instrument(skip_all)]
    async fn make_pm_data<'a>(
        &'a self,
        _state: &'a SessionState,
        _payment_data: &mut PaymentAttemptRecordData<F>,
        _storage_scheme: enums::MerchantStorageScheme,
        _merchant_key_store: &domain::MerchantKeyStore,
        _customer: &Option<domain::Customer>,
        _business_profile: &domain::Profile,
        _should_retry_with_pan: bool,
    ) -> RouterResult<(
        PaymentsAttemptRecordOperation<'a, F>,
        Option<domain::PaymentMethodData>,
        Option<String>,
    )> {
        Ok((Box::new(self), None, None))
    }

    #[instrument(skip_all)]
    async fn perform_routing<'a>(
        &'a self,
        _platform: &domain::Platform,
        _business_profile: &domain::Profile,
        _state: &SessionState,
        _payment_data: &mut PaymentAttemptRecordData<F>,
    ) -> CustomResult<api::ConnectorCallType, errors::ApiErrorResponse> {
        Ok(api::ConnectorCallType::Skip)
    }
}
