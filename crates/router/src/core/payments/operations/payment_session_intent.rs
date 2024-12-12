use std::{collections::HashMap, marker::PhantomData};

use api_models::payments::PaymentsSessionRequest;
use async_trait::async_trait;
use common_utils::{errors::CustomResult, ext_traits::Encode};
use error_stack::ResultExt;
use hyperswitch_domain_models::customer;
use router_env::{instrument, logger, tracing};

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        errors::{self, RouterResult, StorageErrorExt},
        payments::{self, operations, operations::ValidateStatusForOperation},
    },
    routes::{app::ReqState, SessionState},
    types::{api, domain, storage, storage::enums},
    utils::ext_traits::OptionExt,
};

#[derive(Debug, Clone, Copy)]
pub struct PaymentSessionIntent;

type PaymentSessionOperation<'b, F> =
    BoxedOperation<'b, F, PaymentsSessionRequest, payments::PaymentIntentData<F>>;

impl ValidateStatusForOperation for PaymentSessionIntent {
    /// Validate if the current operation can be performed on the current status of the payment intent
    fn validate_status_for_operation(
        &self,
        intent_status: common_enums::IntentStatus,
    ) -> Result<(), errors::ApiErrorResponse> {
        match intent_status {
            common_enums::IntentStatus::RequiresPaymentMethod => Ok(()),
            common_enums::IntentStatus::Cancelled
            | common_enums::IntentStatus::Processing
            | common_enums::IntentStatus::RequiresCustomerAction
            | common_enums::IntentStatus::RequiresMerchantAction
            | common_enums::IntentStatus::RequiresCapture
            | common_enums::IntentStatus::PartiallyCaptured
            | common_enums::IntentStatus::RequiresConfirmation
            | common_enums::IntentStatus::PartiallyCapturedAndCapturable
            | common_enums::IntentStatus::Succeeded
            | common_enums::IntentStatus::Failed => {
                Err(errors::ApiErrorResponse::PreconditionFailed {
                    message: format!(
                        "You cannot create session token for this payment because it has status {intent_status}. Expected status is requires_payment_method.",
                    ),
                })
            }
        }
    }
}

impl<F: Send + Clone + Sync> Operation<F, PaymentsSessionRequest> for &PaymentSessionIntent {
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
    fn to_update_tracker(
        &self,
    ) -> RouterResult<
        &(dyn UpdateTracker<F, payments::PaymentIntentData<F>, PaymentsSessionRequest>
              + Send
              + Sync),
    > {
        Ok(*self)
    }
}

impl<F: Send + Clone + Sync> Operation<F, PaymentsSessionRequest> for PaymentSessionIntent {
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
    fn to_update_tracker(
        &self,
    ) -> RouterResult<
        &(dyn UpdateTracker<F, payments::PaymentIntentData<F>, PaymentsSessionRequest>
              + Send
              + Sync),
    > {
        Ok(self)
    }
}

type PaymentsCreateIntentOperation<'b, F> =
    BoxedOperation<'b, F, PaymentsSessionRequest, payments::PaymentIntentData<F>>;

#[async_trait]
impl<F: Send + Clone + Sync> GetTracker<F, payments::PaymentIntentData<F>, PaymentsSessionRequest>
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
        header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<operations::GetTrackerResponse<payments::PaymentIntentData<F>>> {
        let db = &*state.store;
        let key_manager_state = &state.into();
        let storage_scheme = merchant_account.storage_scheme;

        let payment_intent = db
            .find_payment_intent_by_id(key_manager_state, payment_id, key_store, storage_scheme)
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        self.validate_status_for_operation(payment_intent.status)?;

        let client_secret = header_payload
            .client_secret
            .as_ref()
            .get_required_value("client_secret header")?;
        payment_intent.validate_client_secret(client_secret)?;

        let payment_data = payments::PaymentIntentData {
            flow: PhantomData,
            payment_intent,
            sessions_token: vec![],
        };

        let get_trackers_response = operations::GetTrackerResponse { payment_data };

        Ok(get_trackers_response)
    }
}

#[async_trait]
impl<F: Clone + Sync> UpdateTracker<F, payments::PaymentIntentData<F>, PaymentsSessionRequest>
    for PaymentSessionIntent
{
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        state: &'b SessionState,
        _req_state: ReqState,
        mut payment_data: payments::PaymentIntentData<F>,
        _customer: Option<domain::Customer>,
        storage_scheme: enums::MerchantStorageScheme,
        _updated_customer: Option<customer::CustomerUpdate>,
        key_store: &domain::MerchantKeyStore,
        _frm_suggestion: Option<common_enums::FrmSuggestion>,
        _header_payload: hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<(
        PaymentSessionOperation<'b, F>,
        payments::PaymentIntentData<F>,
    )>
    where
        F: 'b + Send,
    {
        let prerouting_algorithm = payment_data.payment_intent.prerouting_algorithm.clone();
        payment_data.payment_intent = match prerouting_algorithm {
            Some(prerouting_algorithm) => state
                .store
                .update_payment_intent(
                    &state.into(),
                    payment_data.payment_intent,
                    storage::PaymentIntentUpdate::SessionIntentUpdate {
                        prerouting_algorithm,
                        updated_by: storage_scheme.to_string(),
                    },
                    key_store,
                    storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?,
            None => payment_data.payment_intent,
        };

        Ok((Box::new(self), payment_data))
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
    ) -> RouterResult<operations::ValidateResult> {
        Ok(operations::ValidateResult {
            merchant_id: merchant_account.get_id().to_owned(),
            storage_scheme: merchant_account.storage_scheme,
            requeue: false,
        })
    }
}

#[async_trait]
impl<F: Clone + Send + Sync> Domain<F, PaymentsSessionRequest, payments::PaymentIntentData<F>>
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
        let profile_id = &payment_data.payment_intent.profile_id;
        let filtered_connector_accounts = all_connector_accounts
            .filter_based_on_profile_and_connector_type(
                profile_id,
                common_enums::ConnectorType::PaymentProcessor,
            );
        let connector_and_supporting_payment_method_type = filtered_connector_accounts
            .get_connector_and_supporting_payment_method_type_for_session_call();

        let session_connector_data: Vec<_> = connector_and_supporting_payment_method_type
            .into_iter()
            .filter_map(
                |(merchant_connector_account, payment_method_type, payment_method)| {
                    let connector_type = api::GetToken::from(payment_method_type);

                    match api::ConnectorData::get_connector_by_name(
                        &state.conf.connectors,
                        &merchant_connector_account.connector_name.to_string(),
                        connector_type,
                        Some(merchant_connector_account.get_id()),
                    ) {
                        Ok(connector_data) => Some(api::SessionConnectorData::new(
                            payment_method_type,
                            connector_data,
                            None,
                            payment_method,
                        )),
                        Err(err) => {
                            logger::error!(session_token_error=?err);
                            None
                        }
                    }
                },
            )
            .collect();
        let session_token_routing_result = payments::perform_session_token_routing(
            state.clone(),
            merchant_account,
            merchant_key_store,
            payment_data,
            session_connector_data,
        )
        .await?;

        let pre_routing = storage::PaymentRoutingInfo {
            algorithm: None,
            pre_routing_results: Some((|| {
                let mut pre_routing_results: HashMap<
                    common_enums::PaymentMethodType,
                    storage::PreRoutingConnectorChoice,
                > = HashMap::new();
                for (pm_type, routing_choice) in session_token_routing_result.routing_result {
                    let mut routable_choice_list = vec![];
                    for choice in routing_choice {
                        let routable_choice = api::routing::RoutableConnectorChoice {
                            choice_kind: api::routing::RoutableChoiceKind::FullStruct,
                            connector: choice
                                .connector
                                .connector_name
                                .to_string()
                                .parse::<common_enums::RoutableConnectors>()
                                .change_context(errors::ApiErrorResponse::InternalServerError)?,
                            merchant_connector_id: choice.connector.merchant_connector_id.clone(),
                        };
                        routable_choice_list.push(routable_choice);
                    }
                    pre_routing_results.insert(
                        pm_type,
                        storage::PreRoutingConnectorChoice::Multiple(routable_choice_list),
                    );
                }
                Ok::<_, error_stack::Report<errors::ApiErrorResponse>>(pre_routing_results)
            })()?),
        };

        // Store the routing results in payment intent
        payment_data.payment_intent.prerouting_algorithm = Some(
            pre_routing
                .encode_to_value()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Unable to serialize payment routing info to value")?,
        );

        Ok(api::ConnectorCallType::SessionMultiple(
            session_token_routing_result.final_result,
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
