use std::marker::PhantomData;

use api_models::payments::PaymentsSessionRequest;
use async_trait::async_trait;
use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use router_env::{instrument, logger, tracing};

use super::{BoxedOperation, Domain, GetTracker, Operation, ValidateRequest};
use crate::{
    core::{
        errors::{self, RouterResult, StorageErrorExt},
        payments::{self, helpers, operations},
    },
    routes::SessionState,
    types::{
        api,
        domain::{self, MerchantConnectorAccountListTrait},
        storage::enums,
    },
};

#[derive(Debug, Clone, Copy)]
pub struct PaymentSessionIntent;

impl<F: Send + Clone> Operation<F, PaymentsSessionRequest> for &PaymentSessionIntent {
    type Data = payments::PaymentIntentData<F>;
    fn to_validate_request(
        &self,
    ) -> RouterResult<&(dyn ValidateRequest<F, PaymentsSessionRequest, Self::Data> + Send + Sync)>
    {
        Ok(*self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<F, Self::Data, PaymentsSessionRequest> + Send + Sync)> {
        Ok(*self)
    }
    fn to_domain(&self) -> RouterResult<&(dyn Domain<F, PaymentsSessionRequest, Self::Data>)> {
        Ok(*self)
    }
}

impl<F: Send + Clone> Operation<F, PaymentsSessionRequest> for PaymentSessionIntent {
    type Data = payments::PaymentIntentData<F>;
    fn to_validate_request(
        &self,
    ) -> RouterResult<&(dyn ValidateRequest<F, PaymentsSessionRequest, Self::Data> + Send + Sync)>
    {
        Ok(self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<F, Self::Data, PaymentsSessionRequest> + Send + Sync)> {
        Ok(self)
    }
    fn to_domain(&self) -> RouterResult<&dyn Domain<F, PaymentsSessionRequest, Self::Data>> {
        Ok(self)
    }
}

type PaymentsCreateIntentOperation<'b, F> =
    BoxedOperation<'b, F, PaymentsSessionRequest, payments::PaymentIntentData<F>>;

#[async_trait]
impl<F: Send + Clone> GetTracker<F, payments::PaymentIntentData<F>, PaymentsSessionRequest>
    for PaymentSessionIntent
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a SessionState,
        payment_id: &common_utils::id_type::GlobalPaymentId,
        _request: &PaymentsSessionRequest,
        merchant_account: &domain::MerchantAccount,
        _profile: &domain::Profile,
        key_store: &domain::MerchantKeyStore,
        _header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<
        operations::GetTrackerResponse<
            'a,
            F,
            PaymentsSessionRequest,
            payments::PaymentIntentData<F>,
        >,
    > {
        let db = &*state.store;
        let key_manager_state = &state.into();
        let storage_scheme = merchant_account.storage_scheme;

        let payment_intent = db
            .find_payment_intent_by_id(key_manager_state, payment_id, key_store, storage_scheme)
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        helpers::validate_payment_status_against_not_allowed_statuses(
            &payment_intent.status,
            &[enums::IntentStatus::Failed, enums::IntentStatus::Succeeded],
            "create a session token for",
        )?;

        // do this in core function
        // helpers::authenticate_client_secret(Some(&request.client_secret), &payment_intent)?;

        let payment_data = payments::PaymentIntentData {
            flow: PhantomData,
            payment_intent,
            sessions_token: vec![],
        };

        let get_trackers_response = operations::GetTrackerResponse {
            operation: Box::new(self),
            payment_data,
        };

        Ok(get_trackers_response)
    }
}

impl<F: Send + Clone> ValidateRequest<F, PaymentsSessionRequest, payments::PaymentIntentData<F>>
    for PaymentSessionIntent
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        _request: &PaymentsSessionRequest,
        merchant_account: &'a domain::MerchantAccount,
    ) -> RouterResult<(
        PaymentsCreateIntentOperation<'b, F>,
        operations::ValidateResult,
    )> {
        Ok((
            Box::new(self),
            operations::ValidateResult {
                merchant_id: merchant_account.get_id().to_owned(),
                storage_scheme: merchant_account.storage_scheme,
                requeue: false,
            },
        ))
    }
}

#[async_trait]
impl<F: Clone + Send> Domain<F, PaymentsSessionRequest, payments::PaymentIntentData<F>>
    for PaymentSessionIntent
{
    #[instrument(skip_all)]
    async fn get_customer_details<'a>(
        &'a self,
        state: &SessionState,
        payment_data: &mut payments::PaymentIntentData<F>,
        merchant_key_store: &domain::MerchantKeyStore,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<
        (
            BoxedOperation<'a, F, PaymentsSessionRequest, payments::PaymentIntentData<F>>,
            Option<domain::Customer>,
        ),
        errors::StorageError,
    > {
        match payment_data.payment_intent.customer_id.clone() {
            Some(id) => {
                let customer = state
                    .store
                    .find_customer_by_global_id(
                        &state.into(),
                        id.get_string_repr(),
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

    #[instrument(skip_all)]
    async fn make_pm_data<'a>(
        &'a self,
        _state: &'a SessionState,
        _payment_data: &mut payments::PaymentIntentData<F>,
        _storage_scheme: enums::MerchantStorageScheme,
        _merchant_key_store: &domain::MerchantKeyStore,
        _customer: &Option<domain::Customer>,
        _business_profile: &domain::Profile,
    ) -> RouterResult<(
        PaymentsCreateIntentOperation<'a, F>,
        Option<domain::PaymentMethodData>,
        Option<String>,
    )> {
        Ok((Box::new(self), None, None))
    }

    async fn perform_routing<'a>(
        &'a self,
        merchant_account: &domain::MerchantAccount,
        _business_profile: &domain::Profile,
        state: &SessionState,
        payment_data: &mut payments::PaymentIntentData<F>,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<api::ConnectorCallType, errors::ApiErrorResponse> {
        let db = &state.store;
        let all_connector_accounts = db
            .find_merchant_connector_account_by_merchant_id_and_disabled_list(
                &state.into(),
                merchant_account.get_id(),
                false,
                merchant_key_store,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Database error when querying for merchant connector accounts")?;
        let all_connector_accounts =
            domain::MerchantConnectorAccountList::new(all_connector_accounts);
        let profile_id = &payment_data.payment_intent.profile_id;
        let filtered_connector_accounts = all_connector_accounts
            .filter_based_on_profile_and_connector_type(
                &profile_id,
                common_enums::ConnectorType::PaymentProcessor,
            );
        let connector_and_supporting_payment_method_type = filtered_connector_accounts
            .get_connector_and_supporting_payment_method_type_for_session_call();
        let mut session_connector_data =
            Vec::with_capacity(connector_and_supporting_payment_method_type.len());
        for (merchant_connector_account, payment_method_type) in
            connector_and_supporting_payment_method_type
        {
            let connector_type = api::GetToken::from(payment_method_type);
            if let Ok(connector_data) = api::ConnectorData::get_connector_by_name(
                &state.conf.connectors,
                &merchant_connector_account.connector_name.to_string(),
                connector_type,
                Some(merchant_connector_account.get_id()),
            )
            .inspect_err(|err| {
                logger::error!(session_token_error=?err);
            }) {
                let new_session_connector_data =
                    api::SessionConnectorData::new(payment_method_type, connector_data, None);
                session_connector_data.push(new_session_connector_data)
            };
        }

        Ok(api::ConnectorCallType::SessionMultiple(
            session_connector_data,
        ))
    }

    #[instrument(skip_all)]
    async fn guard_payment_against_blocklist<'a>(
        &'a self,
        _state: &SessionState,
        _merchant_account: &domain::MerchantAccount,
        _key_store: &domain::MerchantKeyStore,
        _payment_data: &mut payments::PaymentIntentData<F>,
    ) -> CustomResult<bool, errors::ApiErrorResponse> {
        Ok(false)
    }
}

impl From<api_models::enums::PaymentMethodType> for api::GetToken {
    fn from(value: api_models::enums::PaymentMethodType) -> Self {
        match value {
            api_models::enums::PaymentMethodType::GooglePay => Self::GpayMetadata,
            api_models::enums::PaymentMethodType::ApplePay => Self::ApplePayMetadata,
            api_models::enums::PaymentMethodType::SamsungPay => Self::SamsungPayMetadata,
            api_models::enums::PaymentMethodType::Paypal => Self::PaypalSdkMetadata,
            api_models::enums::PaymentMethodType::Paze => Self::PazeMetadata,
            _ => Self::Connector,
        }
    }
}
