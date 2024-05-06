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
        payment_methods::PaymentMethodRetrieve,
        payments::{self, helpers, operations, PaymentData},
    },
    db::StorageInterface,
    routes::{app::ReqState, AppState},
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

#[async_trait]
impl<F: Send + Clone, Ctx: PaymentMethodRetrieve>
    GetTracker<F, PaymentData<F>, api::PaymentsSessionRequest, Ctx> for PaymentSession
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a AppState,
        payment_id: &api::PaymentIdType,
        request: &api::PaymentsSessionRequest,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        _auth_flow: services::AuthFlow,
        _payment_confirm_source: Option<common_enums::PaymentSource>,
    ) -> RouterResult<operations::GetTrackerResponse<'a, F, api::PaymentsSessionRequest, Ctx>> {
        let payment_id = payment_id
            .get_payment_intent_id()
            .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

        let db = &*state.store;
        let merchant_id = &merchant_account.merchant_id;
        let storage_scheme = merchant_account.storage_scheme;

        let mut payment_intent = db
            .find_payment_intent_by_payment_id_merchant_id(&payment_id, merchant_id, storage_scheme)
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        helpers::validate_payment_status_against_not_allowed_statuses(
            &payment_intent.status,
            &[
                storage_enums::IntentStatus::Failed,
                storage_enums::IntentStatus::Succeeded,
            ],
            "create a session token for",
        )?;

        helpers::authenticate_client_secret(Some(&request.client_secret), &payment_intent)?;

        let mut payment_attempt = db
            .find_payment_attempt_by_payment_id_merchant_id_attempt_id(
                payment_intent.payment_id.as_str(),
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
            db,
            payment_intent.shipping_address_id.clone(),
            key_store,
            &payment_intent.payment_id,
            merchant_id,
            merchant_account.storage_scheme,
        )
        .await?;

        let billing_address = helpers::get_address_by_id(
            db,
            payment_intent.billing_address_id.clone(),
            key_store,
            &payment_intent.payment_id,
            merchant_id,
            merchant_account.storage_scheme,
        )
        .await?;

        let payment_method_billing = helpers::get_address_by_id(
            db,
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
                    merchant_account.merchant_id.as_str(),
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
            .find_business_profile_by_profile_id(profile_id)
            .await
            .to_not_found_response(errors::ApiErrorResponse::BusinessProfileNotFound {
                id: profile_id.to_string(),
            })?;

        let payment_data = PaymentData {
            flow: PhantomData,
            money_amount: api_models::payments::Money{
                money_in_minor: payment_intent.amount.clone(),
                currency
            },
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
            frm_metadata: None,
            recurring_details: None,
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
impl<F: Clone, Ctx: PaymentMethodRetrieve>
    UpdateTracker<F, PaymentData<F>, api::PaymentsSessionRequest, Ctx> for PaymentSession
{
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        state: &'b AppState,
        _req_state: ReqState,
        mut payment_data: PaymentData<F>,
        _customer: Option<domain::Customer>,
        storage_scheme: storage_enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        _mechant_key_store: &domain::MerchantKeyStore,
        _frm_suggestion: Option<FrmSuggestion>,
        _header_payload: api::HeaderPayload,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::PaymentsSessionRequest, Ctx>,
        PaymentData<F>,
    )>
    where
        F: 'b + Send,
    {
        let metadata = payment_data.payment_intent.metadata.clone();
        payment_data.payment_intent = match metadata {
            Some(metadata) => state
                .store
                .update_payment_intent(
                    payment_data.payment_intent,
                    storage::PaymentIntentUpdate::MetadataUpdate {
                        metadata,
                        updated_by: storage_scheme.to_string(),
                    },
                    storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?,
            None => payment_data.payment_intent,
        };

        Ok((Box::new(self), payment_data))
    }
}

impl<F: Send + Clone, Ctx: PaymentMethodRetrieve>
    ValidateRequest<F, api::PaymentsSessionRequest, Ctx> for PaymentSession
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        request: &api::PaymentsSessionRequest,
        merchant_account: &'a domain::MerchantAccount,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::PaymentsSessionRequest, Ctx>,
        operations::ValidateResult<'a>,
    )> {
        //paymentid is already generated and should be sent in the request
        let given_payment_id = request.payment_id.clone();

        Ok((
            Box::new(self),
            operations::ValidateResult {
                merchant_id: &merchant_account.merchant_id,
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
        Ctx: PaymentMethodRetrieve,
        Op: Send + Sync + Operation<F, api::PaymentsSessionRequest, Ctx>,
    > Domain<F, api::PaymentsSessionRequest, Ctx> for Op
where
    for<'a> &'a Op: Operation<F, api::PaymentsSessionRequest, Ctx>,
{
    #[instrument(skip_all)]
    async fn get_or_create_customer_details<'a>(
        &'a self,
        db: &dyn StorageInterface,
        payment_data: &mut PaymentData<F>,
        request: Option<payments::CustomerDetails>,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: common_enums::enums::MerchantStorageScheme,
    ) -> errors::CustomResult<
        (
            BoxedOperation<'a, F, api::PaymentsSessionRequest, Ctx>,
            Option<domain::Customer>,
        ),
        errors::StorageError,
    > {
        helpers::create_customer_if_not_exist(
            Box::new(self),
            db,
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
        _state: &'b AppState,
        _payment_data: &mut PaymentData<F>,
        _storage_scheme: storage_enums::MerchantStorageScheme,
        _merchant_key_store: &domain::MerchantKeyStore,
        _customer: &Option<domain::Customer>,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::PaymentsSessionRequest, Ctx>,
        Option<api::PaymentMethodData>,
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
        state: &AppState,
        request: &api::PaymentsSessionRequest,
        payment_intent: &storage::PaymentIntent,
        key_store: &domain::MerchantKeyStore,
    ) -> RouterResult<api::ConnectorChoice> {
        let db = &state.store;

        let all_connector_accounts = db
            .find_merchant_connector_account_by_merchant_id_and_disabled_list(
                &merchant_account.merchant_id,
                false,
                key_store,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Database error when querying for merchant connector accounts")?;

        let profile_id = crate::core::utils::get_profile_id_from_business_details(
            payment_intent.business_country,
            payment_intent.business_label.as_ref(),
            merchant_account,
            payment_intent.profile_id.as_ref(),
            &*state.store,
            false,
        )
        .await
        .attach_printable("Could not find profile id from business details")?;

        let filtered_connector_accounts =
            helpers::filter_mca_based_on_business_profile(all_connector_accounts, Some(profile_id));

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
                            .map_err(|err| {
                                logger::error!(session_token_parsing_error=?err);
                                err
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
                Some(merchant_connector_account.merchant_connector_id.clone()),
            )
            .map_err(|err| {
                logger::error!(session_token_error=?err);
                err
            }) {
                session_connector_data.push(api::SessionConnectorData {
                    payment_method_type,
                    connector: connector_data,
                    business_sub_label: merchant_connector_account.business_sub_label.clone(),
                })
            };
        }

        Ok(api::ConnectorChoice::SessionMultiple(
            session_connector_data,
        ))
    }

    #[instrument(skip_all)]
    async fn guard_payment_against_blocklist<'a>(
        &'a self,
        _state: &AppState,
        _merchant_account: &domain::MerchantAccount,
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
            _ => Self::Connector,
        }
    }
}
