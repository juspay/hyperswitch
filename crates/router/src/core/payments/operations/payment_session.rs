use std::marker::PhantomData;

use api_models::admin::PaymentMethodsEnabled;
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
    db::StorageInterface,
    routes::AppState,
    types::{
        api::{self, PaymentIdTypeExt},
        domain,
        storage::{self, enums as storage_enums},
    },
    utils::OptionExt,
};

#[derive(Debug, Clone, Copy, PaymentOperation)]
#[operation(ops = "all", flow = "session")]
pub struct PaymentSession;

#[async_trait]
impl<F: Send + Clone> GetTracker<F, PaymentData<F>, api::PaymentsSessionRequest>
    for PaymentSession
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a AppState,
        payment_id: &api::PaymentIdType,
        request: &api::PaymentsSessionRequest,
        _mandate_type: Option<api::MandateTransactionType>,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsSessionRequest>,
        PaymentData<F>,
        Option<payments::CustomerDetails>,
    )> {
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

        let mut payment_attempt = db
            .find_payment_attempt_by_payment_id_merchant_id_attempt_id(
                payment_intent.payment_id.as_str(),
                merchant_id,
                payment_intent.active_attempt_id.as_str(),
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        let currency = payment_intent.currency.get_required_value("currency")?;

        payment_attempt.payment_method = Some(storage_enums::PaymentMethod::Wallet);

        let amount = payment_intent.amount.into();

        let shipping_address = helpers::get_address_for_payment_request(
            db,
            None,
            payment_intent.shipping_address_id.as_deref(),
            merchant_id,
            payment_intent.customer_id.as_ref(),
            key_store,
        )
        .await?;

        let billing_address = helpers::get_address_for_payment_request(
            db,
            None,
            payment_intent.billing_address_id.as_deref(),
            merchant_id,
            payment_intent.customer_id.as_ref(),
            key_store,
        )
        .await?;

        payment_intent.shipping_address_id = shipping_address.clone().map(|x| x.address_id);
        payment_intent.billing_address_id = billing_address.clone().map(|x| x.address_id);

        let connector_response = db
            .find_connector_response_by_payment_id_merchant_id_attempt_id(
                &payment_intent.payment_id,
                &payment_intent.merchant_id,
                &payment_attempt.attempt_id,
                storage_scheme,
            )
            .await
            .map_err(|error| {
                error
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Database error when finding connector response")
            })?;

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

        Ok((
            Box::new(self),
            PaymentData {
                flow: PhantomData,
                payment_intent,
                payment_attempt,
                currency,
                amount,
                email: None,
                mandate_id: None,
                mandate_connector: None,
                token: None,
                setup_mandate: None,
                address: payments::PaymentAddress {
                    shipping: shipping_address.as_ref().map(|a| a.into()),
                    billing: billing_address.as_ref().map(|a| a.into()),
                },
                confirm: None,
                payment_method_data: None,
                force_sync: None,
                refunds: vec![],
                disputes: vec![],
                sessions_token: vec![],
                connector_response,
                card_cvc: None,
                creds_identifier,
                pm_token: None,
                connector_customer_id: None,
                ephemeral_key: None,
                redirect_response: None,
            },
            Some(customer_details),
        ))
    }
}

#[async_trait]
impl<F: Clone> UpdateTracker<F, PaymentData<F>, api::PaymentsSessionRequest> for PaymentSession {
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        db: &dyn StorageInterface,
        mut payment_data: PaymentData<F>,
        _customer: Option<domain::Customer>,
        storage_scheme: storage_enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        _mechant_key_store: &domain::MerchantKeyStore,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::PaymentsSessionRequest>,
        PaymentData<F>,
    )>
    where
        F: 'b + Send,
    {
        let metadata = payment_data.payment_intent.metadata.clone();
        payment_data.payment_intent = match metadata {
            Some(metadata) => db
                .update_payment_intent(
                    payment_data.payment_intent,
                    storage::PaymentIntentUpdate::MetadataUpdate { metadata },
                    storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?,
            None => payment_data.payment_intent,
        };

        Ok((Box::new(self), payment_data))
    }
}

impl<F: Send + Clone> ValidateRequest<F, api::PaymentsSessionRequest> for PaymentSession {
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        request: &api::PaymentsSessionRequest,
        merchant_account: &'a domain::MerchantAccount,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::PaymentsSessionRequest>,
        operations::ValidateResult<'a>,
    )> {
        //paymentid is already generated and should be sent in the request
        let given_payment_id = request.payment_id.clone();

        Ok((
            Box::new(self),
            operations::ValidateResult {
                merchant_id: &merchant_account.merchant_id,
                payment_id: api::PaymentIdType::PaymentIntentId(given_payment_id),
                mandate_type: None,
                storage_scheme: merchant_account.storage_scheme,
                requeue: false,
            },
        ))
    }
}

#[async_trait]
impl<F: Clone + Send, Op: Send + Sync + Operation<F, api::PaymentsSessionRequest>>
    Domain<F, api::PaymentsSessionRequest> for Op
where
    for<'a> &'a Op: Operation<F, api::PaymentsSessionRequest>,
{
    #[instrument(skip_all)]
    async fn get_or_create_customer_details<'a>(
        &'a self,
        db: &dyn StorageInterface,
        payment_data: &mut PaymentData<F>,
        request: Option<payments::CustomerDetails>,
        key_store: &domain::MerchantKeyStore,
    ) -> errors::CustomResult<
        (
            BoxedOperation<'a, F, api::PaymentsSessionRequest>,
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
        )
        .await
    }

    #[instrument(skip_all)]
    async fn make_pm_data<'b>(
        &'b self,
        _state: &'b AppState,
        _payment_data: &mut PaymentData<F>,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::PaymentsSessionRequest>,
        Option<api::PaymentMethodData>,
    )> {
        //No payment method data for this operation
        Ok((Box::new(self), None))
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
        payment_intent: &storage::payment_intent::PaymentIntent,
        key_store: &domain::MerchantKeyStore,
    ) -> RouterResult<api::ConnectorChoice> {
        let connectors = &state.conf.connectors;
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

        let filtered_connector_accounts = helpers::filter_mca_based_on_business_details(
            all_connector_accounts,
            Some(payment_intent),
        );

        let requested_payment_method_types = request.wallets.clone();
        let mut connector_and_supporting_payment_method_type = Vec::new();

        filtered_connector_accounts
            .into_iter()
            .for_each(|connector_account| {
                let res = connector_account
                    .payment_methods_enabled
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
                                (
                                    connector_account.connector_name.to_owned(),
                                    payment_method_type.payment_method_type,
                                    connector_account.business_sub_label.to_owned(),
                                )
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>();
                connector_and_supporting_payment_method_type.extend(res);
            });

        let mut session_connector_data =
            Vec::with_capacity(connector_and_supporting_payment_method_type.len());

        for (connector, payment_method_type, business_sub_label) in
            connector_and_supporting_payment_method_type
        {
            let connector_type =
                get_connector_type_for_session_token(payment_method_type, request, &connector);
            if let Ok(connector_data) =
                api::ConnectorData::get_connector_by_name(connectors, &connector, connector_type)
                    .map_err(|err| {
                        logger::error!(session_token_error=?err);
                        err
                    })
            {
                session_connector_data.push(api::SessionConnectorData {
                    payment_method_type,
                    connector: connector_data,
                    business_sub_label,
                })
            };
        }

        Ok(api::ConnectorChoice::SessionMultiple(
            session_connector_data,
        ))
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

pub fn get_connector_type_for_session_token(
    payment_method_type: api_models::enums::PaymentMethodType,
    request: &api::PaymentsSessionRequest,
    connector: &str,
) -> api::GetToken {
    if payment_method_type == api_models::enums::PaymentMethodType::ApplePay {
        if is_apple_pay_get_token_connector(connector, request) {
            api::GetToken::Connector
        } else {
            api::GetToken::ApplePayMetadata
        }
    } else {
        api::GetToken::from(payment_method_type)
    }
}

pub fn is_apple_pay_get_token_connector(
    connector: &str,
    _request: &api::PaymentsSessionRequest,
) -> bool {
    // Add connectors here, which all are required to hit connector for session call
    matches!(connector, "bluesnap")
}
