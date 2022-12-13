use std::marker::PhantomData;

use async_trait::async_trait;
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
    pii::Secret,
    routes::AppState,
    types::{
        api::{self, PaymentIdTypeExt},
        storage::{self, enums},
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
        merchant_id: &str,
        request: &api::PaymentsSessionRequest,
        _mandate_type: Option<api::MandateTxnType>,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsSessionRequest>,
        PaymentData<F>,
        Option<payments::CustomerDetails>,
    )> {
        let payment_id = payment_id
            .get_payment_intent_id()
            .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

        let db = &*state.store;

        let mut payment_attempt = db
            .find_payment_attempt_by_payment_id_merchant_id(
                &payment_id,
                merchant_id,
                storage_scheme,
            )
            .await
            .map_err(|error| {
                error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
            })?;

        let mut payment_intent = db
            .find_payment_intent_by_payment_id_merchant_id(&payment_id, merchant_id, storage_scheme)
            .await
            .map_err(|error| {
                error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
            })?;

        let currency = payment_intent.currency.get_required_value("currency")?;

        payment_attempt.payment_method = Some(enums::PaymentMethodType::Wallet);

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

        let db = db as &dyn StorageInterface;
        let connector_response = db
            .find_connector_response_by_payment_id_merchant_id_txn_id(
                &payment_intent.payment_id,
                &payment_intent.merchant_id,
                &payment_attempt.txn_id,
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

        Ok((
            Box::new(self),
            PaymentData {
                flow: PhantomData,
                payment_intent,
                payment_attempt,
                currency,
                amount,
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
        _db: &dyn StorageInterface,
        _payment_id: &api::PaymentIdType,
        payment_data: PaymentData<F>,
        _customer: Option<storage::Customer>,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::PaymentsSessionRequest>,
        PaymentData<F>,
    )>
    where
        F: 'b + Send,
    {
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
        _payment_method: Option<enums::PaymentMethodType>,
        _txn_id: &str,
        _payment_attempt: &storage::PaymentAttempt,
        _request: &Option<api::PaymentMethod>,
        _token: &Option<String>,
        _card_cvc: Option<Secret<String>>,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::PaymentsSessionRequest>,
        Option<api::PaymentMethod>,
        Option<String>,
    )> {
        //No payment method data for this operation
        Ok((Box::new(self), None, None))
    }

    async fn get_connector<'a>(
        &'a self,
        merchant_account: &storage::MerchantAccount,
        state: &AppState,
    ) -> RouterResult<api::ConnectorCallType> {
        let connectors = &state.conf.connectors;
        let db = &state.store;

        let supported_connectors: &Vec<String> = state.conf.connectors.supported.wallets.as_ref();

        //FIXME: Check if merchant has enabled wallet through the connector
        let connector_names = db
            .find_merchant_connector_account_by_merchant_id_list(&merchant_account.merchant_id)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Database error when querying for merchant accounts")?
            .iter()
            .filter(|connector_account| {
                supported_connectors.contains(&connector_account.connector_name)
            })
            .map(|filtered_connector| filtered_connector.connector_name.clone())
            .collect::<Vec<String>>();

        let mut connectors_data = Vec::with_capacity(connector_names.len());

        for connector_name in connector_names {
            let connector_data =
                api::ConnectorData::get_connector_by_name(connectors, &connector_name)?;
            connectors_data.push(connector_data);
        }

        Ok(api::ConnectorCallType::Multiple(connectors_data))
    }
}
