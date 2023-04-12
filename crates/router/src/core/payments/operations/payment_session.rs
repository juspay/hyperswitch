use std::{collections::HashSet, marker::PhantomData};

use api_models::admin::PaymentMethodsEnabled;
use async_trait::async_trait;
use common_utils::ext_traits::{AsyncExt, ValueExt};
use error_stack::ResultExt;
use router_derive::PaymentOperation;
use router_env::{instrument, tracing};

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        errors::{self, RouterResult, StorageErrorExt},
        payments::{self, helpers, operations, PaymentData},
    },
    db::StorageInterface,
    logger, pii,
    pii::Secret,
    routes::AppState,
    types::{
        api::{self, enums as api_enums, PaymentIdTypeExt},
        storage::{self, enums as storage_enums},
        transformers::ForeignInto,
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
        _mandate_type: Option<api::MandateTxnType>,
        merchant_account: &storage::MerchantAccount,
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
            .map_err(|error| {
                error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
            })?;

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
            .map_err(|error| {
                error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
            })?;

        let currency = payment_intent.currency.get_required_value("currency")?;

        payment_attempt.payment_method = Some(storage_enums::PaymentMethod::Wallet);

        let amount = payment_intent.amount.into();

        helpers::authenticate_client_secret(
            Some(&request.client_secret),
            payment_intent.client_secret.as_ref(),
        )?;

        let shipping_address = helpers::get_address_for_payment_request(
            db,
            None,
            payment_intent.shipping_address_id.as_deref(),
            merchant_id,
            &payment_intent.customer_id,
        )
        .await?;

        let billing_address = helpers::get_address_for_payment_request(
            db,
            None,
            payment_intent.billing_address_id.as_deref(),
            merchant_id,
            &payment_intent.customer_id,
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
                email: None::<Secret<String, pii::Email>>,
                mandate_id: None,
                token: None,
                setup_mandate: None,
                address: payments::PaymentAddress {
                    shipping: shipping_address.as_ref().map(|a| a.foreign_into()),
                    billing: billing_address.as_ref().map(|a| a.foreign_into()),
                },
                confirm: None,
                payment_method_data: None,
                force_sync: None,
                refunds: vec![],
                sessions_token: vec![],
                connector_response,
                card_cvc: None,
                creds_identifier,
                pm_token: None,
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
        _payment_id: &api::PaymentIdType,
        mut payment_data: PaymentData<F>,
        _customer: Option<storage::Customer>,
        storage_scheme: storage_enums::MerchantStorageScheme,
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
                .map_err(|error| {
                    error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
                })?,
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
        merchant_account: &'a storage::MerchantAccount,
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
        merchant_id: &str,
    ) -> errors::CustomResult<
        (
            BoxedOperation<'a, F, api::PaymentsSessionRequest>,
            Option<storage::Customer>,
        ),
        errors::StorageError,
    > {
        helpers::create_customer_if_not_exist(
            Box::new(self),
            db,
            payment_data,
            request,
            merchant_id,
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

    async fn get_connector<'a>(
        &'a self,
        merchant_account: &storage::MerchantAccount,
        state: &AppState,
        request: &api::PaymentsSessionRequest,
    ) -> RouterResult<api::ConnectorChoice> {
        let connectors = &state.conf.connectors;
        let db = &state.store;

        let supported_connectors: &Vec<String> = state.conf.connectors.supported.wallets.as_ref();

        let connector_accounts = db
            .find_merchant_connector_account_by_merchant_id_and_disabled_list(
                &merchant_account.merchant_id,
                false,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Database error when querying for merchant connector accounts")?;

        let normal_connector_names: HashSet<String> = connector_accounts
            .iter()
            .filter(|connector_account| {
                connector_account
                    .payment_methods_enabled
                    .clone()
                    .unwrap_or_default()
                    .iter()
                    .any(|payment_method| {
                        let parsed_payment_method_result: Result<
                            PaymentMethodsEnabled,
                            error_stack::Report<errors::ParsingError>,
                        > = payment_method.clone().parse_value("payment_method");

                        match parsed_payment_method_result {
                            Ok(parsed_payment_method) => parsed_payment_method
                                .payment_method_types
                                .map(|payment_method_types| {
                                    payment_method_types.iter().any(|payment_method_type| {
                                        matches!(
                                        payment_method_type.payment_experience,
                                        Some(api_models::enums::PaymentExperience::InvokeSdkClient)
                                    )
                                    })
                                })
                                .unwrap_or(false),
                            Err(parsing_error) => {
                                logger::debug!(session_token_parsing_error=?parsing_error);
                                false
                            }
                        }
                    })
            })
            .map(|filtered_connector| filtered_connector.connector_name.clone())
            .collect();

        // Parse the payment methods enabled to check if the merchant has enabled googlepay ( wallet ) using that connector.
        // A single connector can support creating session token from metadata as well as by calling the connector.
        let session_token_from_metadata_connectors = connector_accounts
            .iter()
            .filter(|connector_account| {
                connector_account
                    .payment_methods_enabled
                    .clone()
                    .unwrap_or_default()
                    .iter()
                    .any(|payment_method| {
                        let parsed_payment_method_result: Result<
                            PaymentMethodsEnabled,
                            error_stack::Report<errors::ParsingError>,
                        > = payment_method.clone().parse_value("payment_method");

                        match parsed_payment_method_result {
                            Ok(parsed_payment_method) => parsed_payment_method
                                .payment_method_types
                                .map(|payment_method_types| {
                                    payment_method_types.iter().any(|payment_method_type| {
                                        matches!(
                                            payment_method_type.payment_method_type,
                                            api_models::enums::PaymentMethodType::GooglePay
                                        )
                                    })
                                })
                                .unwrap_or(false),
                            Err(parsing_error) => {
                                logger::debug!(session_token_parsing_error=?parsing_error);
                                false
                            }
                        }
                    })
            })
            .map(|filtered_connector| filtered_connector.connector_name.clone())
            .collect::<HashSet<String>>();

        let given_wallets = request.wallets.clone();

        let connectors_data = if !given_wallets.is_empty() {
            // Create connectors for provided wallets
            let mut connectors_data = Vec::with_capacity(supported_connectors.len());
            for wallet in given_wallets {
                let (connector_name, connector_type) = match wallet {
                    api_enums::SupportedWallets::Gpay => ("adyen", api::GetToken::Metadata),
                    api_enums::SupportedWallets::ApplePay => ("applepay", api::GetToken::Connector),
                    api_enums::SupportedWallets::Paypal => ("braintree", api::GetToken::Connector),
                    api_enums::SupportedWallets::Klarna => ("klarna", api::GetToken::Connector),
                };

                // Check if merchant has enabled the required merchant connector account
                if session_token_from_metadata_connectors.contains(connector_name)
                    || normal_connector_names.contains(connector_name)
                {
                    connectors_data.push(api::ConnectorData::get_connector_by_name(
                        connectors,
                        connector_name,
                        connector_type,
                    )?);
                }
            }
            connectors_data
        } else {
            // Create connectors for all enabled wallets
            let mut connectors_data = Vec::with_capacity(
                normal_connector_names.len() + session_token_from_metadata_connectors.len(),
            );

            for connector_name in normal_connector_names {
                let connector_data = api::ConnectorData::get_connector_by_name(
                    connectors,
                    &connector_name,
                    api::GetToken::Connector,
                )?;
                connectors_data.push(connector_data);
            }

            for connector_name in session_token_from_metadata_connectors {
                let connector_data = api::ConnectorData::get_connector_by_name(
                    connectors,
                    &connector_name,
                    api::GetToken::Metadata,
                )?;
                connectors_data.push(connector_data);
            }
            connectors_data
        };

        Ok(api::ConnectorChoice::SessionMultiple(connectors_data))
    }
}
