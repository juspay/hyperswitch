use std::marker::PhantomData;

use api_models::{admin::PaymentMethodsEnabled, enums::FrmSuggestion};
use async_trait::async_trait;
use common_utils::ext_traits::{AsyncExt, ValueExt};
use error_stack::ResultExt;
use router_derive::PaymentOperation;
use router_env::{instrument, logger, tracing};

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        errors::{self, RouterResult, StorageErrorExt},
        payments::{self, helpers, operations, PaymentData},
    },
    routes::{app::ReqState, SessionState},
    services,
    types::{
        api::{self, PaymentIdTypeExt},
        domain,
        storage::{self, enums as storage_enums},
    },
    utils::OptionExt,
};

#[derive(Debug, Clone, Copy, PaymentOperation)]
#[operation(operations = "all", flow = "session")]
pub struct PaymentSession;

type PaymentSessionOperation<'b, F> =
    BoxedOperation<'b, F, api::PaymentsSessionRequest, PaymentData<F>>;

#[async_trait]
impl<F: Send + Clone + Sync> GetTracker<F, PaymentData<F>, api::PaymentsSessionRequest>
    for PaymentSession
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a SessionState,
        payment_id: &api::PaymentIdType,
        request: &api::PaymentsSessionRequest,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        _auth_flow: services::AuthFlow,
        _header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
        _platform_merchant_account: Option<&domain::MerchantAccount>,
    ) -> RouterResult<
        operations::GetTrackerResponse<'a, F, api::PaymentsSessionRequest, PaymentData<F>>,
    > {
        let payment_id = payment_id
            .get_payment_intent_id()
            .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

        let db = &*state.store;
        let key_manager_state = &state.into();
        let merchant_id = merchant_account.get_id();
        let storage_scheme = merchant_account.storage_scheme;

        let mut payment_intent = db
            .find_payment_intent_by_payment_id_merchant_id(
                &state.into(),
                &payment_id,
                merchant_id,
                key_store,
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        // TODO (#7195): Add platform merchant account validation once publishable key auth is solved

        helpers::validate_payment_status_against_not_allowed_statuses(
            payment_intent.status,
            &[
                storage_enums::IntentStatus::Failed,
                storage_enums::IntentStatus::Succeeded,
            ],
            "create a session token for",
        )?;

        helpers::authenticate_client_secret(Some(&request.client_secret), &payment_intent)?;

        let mut payment_attempt = db
            .find_payment_attempt_by_payment_id_merchant_id_attempt_id(
                &payment_intent.payment_id,
                merchant_id,
                payment_intent.active_attempt.get_id().as_str(),
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        let currency = payment_intent.currency.get_required_value("currency")?;

        payment_attempt.payment_method = Some(storage_enums::PaymentMethod::Wallet);

        let amount = payment_attempt.get_total_amount().into();

        let shipping_address = helpers::get_address_by_id(
            state,
            payment_intent.shipping_address_id.clone(),
            key_store,
            &payment_intent.payment_id,
            merchant_id,
            merchant_account.storage_scheme,
        )
        .await?;

        let billing_address = helpers::get_address_by_id(
            state,
            payment_intent.billing_address_id.clone(),
            key_store,
            &payment_intent.payment_id,
            merchant_id,
            merchant_account.storage_scheme,
        )
        .await?;

        let payment_method_billing = helpers::get_address_by_id(
            state,
            payment_attempt.payment_method_billing_address_id.clone(),
            key_store,
            &payment_intent.payment_id,
            merchant_id,
            merchant_account.storage_scheme,
        )
        .await?;

        payment_intent.shipping_address_id = shipping_address.clone().map(|x| x.address_id);
        payment_intent.billing_address_id = billing_address.clone().map(|x| x.address_id);

        let customer_details = payments::CustomerDetails {
            customer_id: payment_intent.customer_id.clone(),
            name: None,
            email: None,
            phone: None,
            phone_country_code: None,
        };

        let creds_identifier = request
            .merchant_connector_details
            .as_ref()
            .map(|mcd| mcd.creds_identifier.to_owned());
        request
            .merchant_connector_details
            .to_owned()
            .async_map(|mcd| async {
                helpers::insert_merchant_connector_creds_to_config(
                    db,
                    merchant_account.get_id(),
                    mcd,
                )
                .await
            })
            .await
            .transpose()?;

        let profile_id = payment_intent
            .profile_id
            .as_ref()
            .get_required_value("profile_id")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("'profile_id' not set in payment intent")?;

        let business_profile = db
            .find_business_profile_by_profile_id(key_manager_state, key_store, profile_id)
            .await
            .to_not_found_response(errors::ApiErrorResponse::ProfileNotFound {
                id: profile_id.get_string_repr().to_owned(),
            })?;

        let payment_data = PaymentData {
            flow: PhantomData,
            payment_intent,
            payment_attempt,
            currency,
            amount,
            email: None,
            mandate_id: None,
            mandate_connector: None,
            customer_acceptance: None,
            token: None,
            token_data: None,
            setup_mandate: None,
            address: payments::PaymentAddress::new(
                shipping_address.as_ref().map(From::from),
                billing_address.as_ref().map(From::from),
                payment_method_billing.as_ref().map(From::from),
                business_profile.use_billing_as_payment_method_billing,
            ),
            confirm: None,
            payment_method_data: None,
            payment_method_info: None,
            force_sync: None,
            refunds: vec![],
            disputes: vec![],
            attempts: None,
            sessions_token: vec![],
            card_cvc: None,
            creds_identifier,
            pm_token: None,
            connector_customer_id: None,
            recurring_mandate_payment_data: None,
            ephemeral_key: None,
            multiple_capture_data: None,
            redirect_response: None,
            surcharge_details: None,
            frm_message: None,
            payment_link_data: None,
            incremental_authorization_details: None,
            authorizations: vec![],
            authentication: None,
            recurring_details: None,
            poll_config: None,
            tax_data: None,
            session_id: None,
            service_details: None,
        };

        let get_trackers_response = operations::GetTrackerResponse {
            operation: Box::new(self),
            customer_details: Some(customer_details),
            payment_data,
            business_profile,
            mandate_type: None,
        };

        Ok(get_trackers_response)
    }
}

#[async_trait]
impl<F: Clone + Sync> UpdateTracker<F, PaymentData<F>, api::PaymentsSessionRequest>
    for PaymentSession
{
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        state: &'b SessionState,
        _req_state: ReqState,
        mut payment_data: PaymentData<F>,
        _customer: Option<domain::Customer>,
        storage_scheme: storage_enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        key_store: &domain::MerchantKeyStore,
        _frm_suggestion: Option<FrmSuggestion>,
        _header_payload: hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<(PaymentSessionOperation<'b, F>, PaymentData<F>)>
    where
        F: 'b + Send,
    {
        let metadata = payment_data.payment_intent.metadata.clone();
        payment_data.payment_intent = match metadata {
            Some(metadata) => state
                .store
                .update_payment_intent(
                    &state.into(),
                    payment_data.payment_intent,
                    storage::PaymentIntentUpdate::MetadataUpdate {
                        metadata,
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

impl<F: Send + Clone + Sync> ValidateRequest<F, api::PaymentsSessionRequest, PaymentData<F>>
    for PaymentSession
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        request: &api::PaymentsSessionRequest,
        merchant_account: &'a domain::MerchantAccount,
    ) -> RouterResult<(PaymentSessionOperation<'b, F>, operations::ValidateResult)> {
        //paymentid is already generated and should be sent in the request
        let given_payment_id = request.payment_id.clone();

        Ok((
            Box::new(self),
            operations::ValidateResult {
                merchant_id: merchant_account.get_id().to_owned(),
                payment_id: api::PaymentIdType::PaymentIntentId(given_payment_id),
                storage_scheme: merchant_account.storage_scheme,
                requeue: false,
            },
        ))
    }
}

#[async_trait]
impl<
        F: Clone + Send,
        Op: Send + Sync + Operation<F, api::PaymentsSessionRequest, Data = PaymentData<F>>,
    > Domain<F, api::PaymentsSessionRequest, PaymentData<F>> for Op
where
    for<'a> &'a Op: Operation<F, api::PaymentsSessionRequest, Data = PaymentData<F>>,
{
    #[instrument(skip_all)]
    async fn get_or_create_customer_details<'a>(
        &'a self,
        state: &SessionState,
        payment_data: &mut PaymentData<F>,
        request: Option<payments::CustomerDetails>,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: common_enums::enums::MerchantStorageScheme,
    ) -> errors::CustomResult<
        (PaymentSessionOperation<'a, F>, Option<domain::Customer>),
        errors::StorageError,
    > {
        helpers::create_customer_if_not_exist(
            state,
            Box::new(self),
            payment_data,
            request,
            &key_store.merchant_id,
            key_store,
            storage_scheme,
        )
        .await
    }

    #[instrument(skip_all)]
    async fn make_pm_data<'b>(
        &'b self,
        _state: &'b SessionState,
        _payment_data: &mut PaymentData<F>,
        _storage_scheme: storage_enums::MerchantStorageScheme,
        _merchant_key_store: &domain::MerchantKeyStore,
        _customer: &Option<domain::Customer>,
        _business_profile: &domain::Profile,
    ) -> RouterResult<(
        PaymentSessionOperation<'b, F>,
        Option<domain::PaymentMethodData>,
        Option<String>,
    )> {
        //No payment method data for this operation
        Ok((Box::new(self), None, None))
    }

    /// Returns `Vec<SessionConnectorData>`
    /// Steps carried out in this function
    /// Get all the `merchant_connector_accounts` which are not disabled
    /// Filter out connectors which have `invoke_sdk_client` enabled in `payment_method_types`
    /// If session token is requested for certain wallets only, then return them, else
    /// return all eligible connectors
    ///
    /// `GetToken` parameter specifies whether to get the session token from connector integration
    /// or from separate implementation ( for googlepay - from metadata and applepay - from metadata and call connector)
    async fn get_connector<'a>(
        &'a self,
        merchant_account: &domain::MerchantAccount,
        state: &SessionState,
        request: &api::PaymentsSessionRequest,
        payment_intent: &storage::PaymentIntent,
        key_store: &domain::MerchantKeyStore,
    ) -> RouterResult<api::ConnectorChoice> {
        let db = &state.store;

        let all_connector_accounts = db
            .find_merchant_connector_account_by_merchant_id_and_disabled_list(
                &state.into(),
                merchant_account.get_id(),
                false,
                key_store,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Database error when querying for merchant connector accounts")?;

        let profile_id = payment_intent
            .profile_id
            .clone()
            .get_required_value("profile_id")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("profile_id is not set in payment_intent")?;

        let filtered_connector_accounts = helpers::filter_mca_based_on_profile_and_connector_type(
            all_connector_accounts,
            &profile_id,
            common_enums::ConnectorType::PaymentProcessor,
        );

        let requested_payment_method_types = request.wallets.clone();
        let mut connector_and_supporting_payment_method_type = Vec::new();

        filtered_connector_accounts
            .iter()
            .for_each(|connector_account| {
                let res = connector_account
                    .payment_methods_enabled
                    .clone()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|payment_methods_enabled| {
                        payment_methods_enabled
                            .parse_value::<PaymentMethodsEnabled>("payment_methods_enabled")
                    })
                    .filter_map(|parsed_payment_method_result| {
                        parsed_payment_method_result
                            .inspect_err(|err| {
                                logger::error!(session_token_parsing_error=?err);
                            })
                            .ok()
                    })
                    .flat_map(|parsed_payment_methods_enabled| {
                        parsed_payment_methods_enabled
                            .payment_method_types
                            .unwrap_or_default()
                            .into_iter()
                            .filter(|payment_method_type| {
                                let is_invoke_sdk_client = matches!(
                                    payment_method_type.payment_experience,
                                    Some(api_models::enums::PaymentExperience::InvokeSdkClient)
                                );

                                // If session token is requested for the payment method type,
                                // filter it out
                                // if not, then create all sessions tokens
                                let is_sent_in_request = requested_payment_method_types
                                    .contains(&payment_method_type.payment_method_type)
                                    || requested_payment_method_types.is_empty();

                                is_invoke_sdk_client && is_sent_in_request
                            })
                            .map(|payment_method_type| {
                                (connector_account, payment_method_type.payment_method_type)
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>();
                connector_and_supporting_payment_method_type.extend(res);
            });

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
                #[cfg(feature = "v1")]
                {
                    let new_session_connector_data = api::SessionConnectorData::new(
                        payment_method_type,
                        connector_data,
                        merchant_connector_account.business_sub_label.clone(),
                    );
                    session_connector_data.push(new_session_connector_data)
                }
                #[cfg(feature = "v2")]
                {
                    let new_session_connector_data =
                        api::SessionConnectorData::new(payment_method_type, connector_data, None);
                    session_connector_data.push(new_session_connector_data)
                }
            };
        }

        Ok(api::ConnectorChoice::SessionMultiple(
            session_connector_data,
        ))
    }

    #[instrument(skip_all)]
    async fn guard_payment_against_blocklist<'a>(
        &'a self,
        _state: &SessionState,
        _merchant_account: &domain::MerchantAccount,
        _key_store: &domain::MerchantKeyStore,
        _payment_data: &mut PaymentData<F>,
    ) -> errors::CustomResult<bool, errors::ApiErrorResponse> {
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
