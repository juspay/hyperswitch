use api_models::{
    admin::ExtendedCardInfoConfig,
    enums::FrmSuggestion,
    payments::{ExtendedCardInfo, GetAddressFromPaymentMethodData, PaymentsConfirmIntentRequest},
};
use async_trait::async_trait;
use common_utils::{ext_traits::Encode, types::keymanager::ToEncryptable};
use error_stack::ResultExt;
use hyperswitch_domain_models::payments::PaymentConfirmData;
use masking::PeekInterface;
use router_env::{instrument, tracing};
use tracing_futures::Instrument;

use super::{Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        admin,
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        payments::{
            self, call_decision_manager, helpers,
            operations::{self, ValidateStatusForOperation},
            populate_surcharge_details, CustomerDetails, OperationSessionSetters, PaymentAddress,
            PaymentData,
        },
        utils as core_utils,
    },
    routes::{app::ReqState, SessionState},
    services,
    types::{
        self,
        api::{self, ConnectorCallType, PaymentIdTypeExt},
        domain::{self, types as domain_types},
        storage::{self, enums as storage_enums},
    },
    utils::{self, OptionExt},
};

#[derive(Debug, Clone, Copy)]
pub struct PaymentIntentConfirm;

impl ValidateStatusForOperation for PaymentIntentConfirm {
    /// Validate if the current operation can be performed on the current status of the payment intent
    fn validate_status_for_operation(
        &self,
        intent_status: common_enums::IntentStatus,
    ) -> Result<(), errors::ApiErrorResponse> {
        match intent_status {
            common_enums::IntentStatus::RequiresPaymentMethod => Ok(()),
            common_enums::IntentStatus::Succeeded
            | common_enums::IntentStatus::Failed
            | common_enums::IntentStatus::Cancelled
            | common_enums::IntentStatus::Processing
            | common_enums::IntentStatus::RequiresCustomerAction
            | common_enums::IntentStatus::RequiresMerchantAction
            | common_enums::IntentStatus::RequiresCapture
            | common_enums::IntentStatus::PartiallyCaptured
            | common_enums::IntentStatus::RequiresConfirmation
            | common_enums::IntentStatus::PartiallyCapturedAndCapturable => {
                Err(errors::ApiErrorResponse::PaymentUnexpectedState {
                    current_flow: format!("{self:?}"),
                    field_name: "status".to_string(),
                    current_value: intent_status.to_string(),
                    states: ["requires_payment_method".to_string()].join(", "),
                })
            }
        }
    }
}

type BoxedConfirmOperation<'b, F> =
    super::BoxedOperation<'b, F, PaymentsConfirmIntentRequest, PaymentConfirmData<F>>;

// TODO: change the macro to include changes for v2
// TODO: PaymentData in the macro should be an input
impl<F: Send + Clone + Sync> Operation<F, PaymentsConfirmIntentRequest> for &PaymentIntentConfirm {
    type Data = PaymentConfirmData<F>;
    fn to_validate_request(
        &self,
    ) -> RouterResult<
        &(dyn ValidateRequest<F, PaymentsConfirmIntentRequest, Self::Data> + Send + Sync),
    > {
        Ok(*self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<F, Self::Data, PaymentsConfirmIntentRequest> + Send + Sync)>
    {
        Ok(*self)
    }
    fn to_domain(
        &self,
    ) -> RouterResult<&(dyn Domain<F, PaymentsConfirmIntentRequest, Self::Data>)> {
        Ok(*self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, Self::Data, PaymentsConfirmIntentRequest> + Send + Sync)>
    {
        Ok(*self)
    }
}
#[automatically_derived]
impl<F: Send + Clone + Sync> Operation<F, PaymentsConfirmIntentRequest> for PaymentIntentConfirm {
    type Data = PaymentConfirmData<F>;
    fn to_validate_request(
        &self,
    ) -> RouterResult<
        &(dyn ValidateRequest<F, PaymentsConfirmIntentRequest, Self::Data> + Send + Sync),
    > {
        Ok(self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<F, Self::Data, PaymentsConfirmIntentRequest> + Send + Sync)>
    {
        Ok(self)
    }
    fn to_domain(&self) -> RouterResult<&dyn Domain<F, PaymentsConfirmIntentRequest, Self::Data>> {
        Ok(self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, Self::Data, PaymentsConfirmIntentRequest> + Send + Sync)>
    {
        Ok(self)
    }
}

impl<F: Send + Clone + Sync> ValidateRequest<F, PaymentsConfirmIntentRequest, PaymentConfirmData<F>>
    for PaymentIntentConfirm
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        request: &PaymentsConfirmIntentRequest,
        merchant_account: &'a domain::MerchantAccount,
    ) -> RouterResult<operations::ValidateResult> {
        let validate_result = operations::ValidateResult {
            merchant_id: merchant_account.get_id().to_owned(),
            storage_scheme: merchant_account.storage_scheme,
            requeue: false,
        };

        Ok(validate_result)
    }
}

#[async_trait]
impl<F: Send + Clone + Sync> GetTracker<F, PaymentConfirmData<F>, PaymentsConfirmIntentRequest>
    for PaymentIntentConfirm
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a SessionState,
        payment_id: &common_utils::id_type::GlobalPaymentId,
        request: &PaymentsConfirmIntentRequest,
        merchant_account: &domain::MerchantAccount,
        profile: &domain::Profile,
        key_store: &domain::MerchantKeyStore,
        header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
        _platform_merchant_account: Option<&domain::MerchantAccount>,
    ) -> RouterResult<operations::GetTrackerResponse<PaymentConfirmData<F>>> {
        let db = &*state.store;
        let key_manager_state = &state.into();

        let storage_scheme = merchant_account.storage_scheme;

        let payment_intent = db
            .find_payment_intent_by_id(key_manager_state, payment_id, key_store, storage_scheme)
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        // TODO (#7195): Add platform merchant account validation once publishable key auth is solved

        self.validate_status_for_operation(payment_intent.status)?;
        let client_secret = header_payload
            .client_secret
            .as_ref()
            .get_required_value("client_secret header")?;
        payment_intent.validate_client_secret(client_secret)?;

        let cell_id = state.conf.cell_information.id.clone();

        let batch_encrypted_data = domain_types::crypto_operation(
            key_manager_state,
            common_utils::type_name!(hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt),
            domain_types::CryptoOperation::BatchEncrypt(
                hyperswitch_domain_models::payments::payment_attempt::FromRequestEncryptablePaymentAttempt::to_encryptable(
                    hyperswitch_domain_models::payments::payment_attempt::FromRequestEncryptablePaymentAttempt {
                        payment_method_billing_address: request.payment_method_data.billing.as_ref().map(|address| address.clone().encode_to_value()).transpose().change_context(errors::ApiErrorResponse::InternalServerError).attach_printable("Failed to encode payment_method_billing address")?.map(masking::Secret::new),
                    },
                ),
            ),
            common_utils::types::keymanager::Identifier::Merchant(merchant_account.get_id().to_owned()),
            key_store.key.peek(),
        )
        .await
        .and_then(|val| val.try_into_batchoperation())
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while encrypting payment intent details".to_string())?;

        let encrypted_data =
             hyperswitch_domain_models::payments::payment_attempt::FromRequestEncryptablePaymentAttempt::from_encryptable(batch_encrypted_data)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed while encrypting payment intent details")?;

        let payment_attempt_domain_model =
            hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt::create_domain_model(
                &payment_intent,
                cell_id,
                storage_scheme,
                request,
                encrypted_data
            )
            .await?;

        let payment_attempt = db
            .insert_payment_attempt(
                key_manager_state,
                key_store,
                payment_attempt_domain_model,
                storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Could not insert payment attempt")?;

        let payment_method_data = request
            .payment_method_data
            .payment_method_data
            .clone()
            .map(hyperswitch_domain_models::payment_method_data::PaymentMethodData::from);

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

        let payment_data = PaymentConfirmData {
            flow: std::marker::PhantomData,
            payment_intent,
            payment_attempt,
            payment_method_data,
            payment_address,
        };

        let get_trackers_response = operations::GetTrackerResponse { payment_data };

        Ok(get_trackers_response)
    }
}

#[async_trait]
impl<F: Clone + Send + Sync> Domain<F, PaymentsConfirmIntentRequest, PaymentConfirmData<F>>
    for PaymentIntentConfirm
{
    async fn get_customer_details<'a>(
        &'a self,
        state: &SessionState,
        payment_data: &mut PaymentConfirmData<F>,
        merchant_key_store: &domain::MerchantKeyStore,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<(BoxedConfirmOperation<'a, F>, Option<domain::Customer>), errors::StorageError>
    {
        match payment_data.payment_intent.customer_id.clone() {
            Some(id) => {
                let customer = state
                    .store
                    .find_customer_by_global_id(
                        &state.into(),
                        &id,
                        &payment_data.payment_intent.merchant_id,
                        merchant_key_store,
                        storage_scheme,
                    )
                    .await?;

                Ok((Box::new(self), Some(customer)))
            }
            None => Ok((Box::new(self), None)),
        }
    }

    async fn run_decision_manager<'a>(
        &'a self,
        state: &SessionState,
        payment_data: &mut PaymentConfirmData<F>,
        business_profile: &domain::Profile,
    ) -> CustomResult<(), errors::ApiErrorResponse> {
        let authentication_type = payment_data.payment_intent.authentication_type;

        let authentication_type = match business_profile.three_ds_decision_manager_config.as_ref() {
            Some(three_ds_decision_manager_config) => call_decision_manager(
                state,
                three_ds_decision_manager_config.clone(),
                payment_data,
            )?,
            None => authentication_type,
        };

        if let Some(auth_type) = authentication_type {
            payment_data.payment_attempt.authentication_type = auth_type;
        }

        Ok(())
    }

    #[instrument(skip_all)]
    async fn make_pm_data<'a>(
        &'a self,
        state: &'a SessionState,
        payment_data: &mut PaymentConfirmData<F>,
        storage_scheme: storage_enums::MerchantStorageScheme,
        key_store: &domain::MerchantKeyStore,
        customer: &Option<domain::Customer>,
        business_profile: &domain::Profile,
    ) -> RouterResult<(
        BoxedConfirmOperation<'a, F>,
        Option<domain::PaymentMethodData>,
        Option<String>,
    )> {
        Ok((Box::new(self), None, None))
    }

    #[cfg(feature = "v2")]
    async fn perform_routing<'a>(
        &'a self,
        merchant_account: &domain::MerchantAccount,
        business_profile: &domain::Profile,
        state: &SessionState,
        // TODO: do not take the whole payment data here
        payment_data: &mut PaymentConfirmData<F>,
        mechant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<ConnectorCallType, errors::ApiErrorResponse> {
        use crate::core::payments::OperationSessionSetters;

        let fallback_config = admin::ProfileWrapper::new(business_profile.clone())
            .get_default_fallback_list_of_connector_under_profile()
            .change_context(errors::RoutingError::FallbackConfigFetchFailed)
            .change_context(errors::ApiErrorResponse::InternalServerError)?;

        let first_chosen_connector = fallback_config
            .first()
            .ok_or(errors::ApiErrorResponse::IncorrectPaymentMethodConfiguration)?;

        let connector_name = first_chosen_connector.connector.to_string();
        let merchant_connector_id = first_chosen_connector
            .merchant_connector_id
            .clone()
            .get_required_value("merchant_connector_id")?;

        payment_data.set_connector_in_payment_attempt(Some(connector_name.to_string()));
        payment_data.set_merchant_connector_id_in_attempt(Some(merchant_connector_id.clone()));

        let connector_data = api::ConnectorData::get_connector_by_name(
            &state.conf.connectors,
            &connector_name,
            api::GetToken::Connector,
            Some(merchant_connector_id),
        )?;

        Ok(ConnectorCallType::PreDetermined(connector_data))
    }
}

#[async_trait]
impl<F: Clone + Sync> UpdateTracker<F, PaymentConfirmData<F>, PaymentsConfirmIntentRequest>
    for PaymentIntentConfirm
{
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        state: &'b SessionState,
        req_state: ReqState,
        mut payment_data: PaymentConfirmData<F>,
        customer: Option<domain::Customer>,
        storage_scheme: storage_enums::MerchantStorageScheme,
        updated_customer: Option<storage::CustomerUpdate>,
        key_store: &domain::MerchantKeyStore,
        frm_suggestion: Option<FrmSuggestion>,
        header_payload: hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<(BoxedConfirmOperation<'b, F>, PaymentConfirmData<F>)>
    where
        F: 'b + Send,
    {
        let db = &*state.store;
        let key_manager_state = &state.into();

        let intent_status = common_enums::IntentStatus::Processing;
        let attempt_status = common_enums::AttemptStatus::Pending;

        let connector = payment_data
            .payment_attempt
            .connector
            .clone()
            .get_required_value("connector")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Connector is none when constructing response")?;

        let merchant_connector_id = payment_data
            .payment_attempt
            .merchant_connector_id
            .clone()
            .get_required_value("merchant_connector_id")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Merchant connector id is none when constructing response")?;

        let payment_intent_update =
            hyperswitch_domain_models::payments::payment_intent::PaymentIntentUpdate::ConfirmIntent {
                status: intent_status,
                updated_by: storage_scheme.to_string(),
                active_attempt_id: payment_data.payment_attempt.id.clone(),
            };

        let authentication_type = payment_data.payment_attempt.authentication_type;

        let payment_attempt_update = hyperswitch_domain_models::payments::payment_attempt::PaymentAttemptUpdate::ConfirmIntent {
            status: attempt_status,
            updated_by: storage_scheme.to_string(),
            connector,
            merchant_connector_id,
            authentication_type,
        };

        let updated_payment_intent = db
            .update_payment_intent(
                key_manager_state,
                payment_data.payment_intent.clone(),
                payment_intent_update,
                key_store,
                storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to update payment intent")?;

        payment_data.payment_intent = updated_payment_intent;

        let updated_payment_attempt = db
            .update_payment_attempt(
                key_manager_state,
                key_store,
                payment_data.payment_attempt.clone(),
                payment_attempt_update,
                storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to update payment attempt")?;

        payment_data.payment_attempt = updated_payment_attempt;

        if let Some((customer, updated_customer)) = customer.zip(updated_customer) {
            let customer_id = customer.get_id().clone();
            let customer_merchant_id = customer.merchant_id.clone();

            let _updated_customer = db
                .update_customer_by_global_id(
                    key_manager_state,
                    &customer_id,
                    customer,
                    &customer_merchant_id,
                    updated_customer,
                    key_store,
                    storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to update customer during `update_trackers`")?;
        }

        Ok((Box::new(self), payment_data))
    }
}
