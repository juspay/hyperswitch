use std::marker::PhantomData;

use async_trait::async_trait;
use common_utils::{date_time, errors::CustomResult};
use error_stack::ResultExt;
use router_derive::PaymentOperation;
use router_env::{instrument, tracing};
use uuid::Uuid;

use super::{BoxedOperation, Domain, GetTracker, PaymentCreate, UpdateTracker, ValidateRequest};
use crate::{
    consts,
    core::{
        errors::{self, RouterResult, StorageErrorExt},
        payments::{self, helpers, operations, Operation, PaymentData},
        utils as core_utils,
    },
    db::StorageInterface,
    routes::AppState,
    types::{
        self,
        api::{self, enums as api_enums, PaymentIdTypeExt},
        storage::{self, enums as storage_enums},
        transformers::ForeignInto,
    },
    utils,
};

#[derive(Debug, Clone, Copy, PaymentOperation)]
#[operation(ops = "all", flow = "verify")]
pub struct PaymentMethodValidate;

impl<F: Send + Clone> ValidateRequest<F, api::VerifyRequest> for PaymentMethodValidate {
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        request: &api::VerifyRequest,
        merchant_account: &'a types::storage::MerchantAccount,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::VerifyRequest>,
        operations::ValidateResult<'a>,
    )> {
        let request_merchant_id = request.merchant_id.as_deref();
        helpers::validate_merchant_id(&merchant_account.merchant_id, request_merchant_id)
            .change_context(errors::ApiErrorResponse::MerchantAccountNotFound)?;

        let mandate_type = helpers::validate_mandate(request)?;
        let validation_id = core_utils::get_or_generate_id("validation_id", &None, "val")?;

        Ok((
            Box::new(self),
            operations::ValidateResult {
                merchant_id: &merchant_account.merchant_id,
                payment_id: api::PaymentIdType::PaymentIntentId(validation_id),
                mandate_type,
                storage_scheme: merchant_account.storage_scheme,
            },
        ))
    }
}

#[async_trait]
impl<F: Send + Clone> GetTracker<F, PaymentData<F>, api::VerifyRequest> for PaymentMethodValidate {
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a AppState,
        payment_id: &api::PaymentIdType,
        request: &api::VerifyRequest,
        _mandate_type: Option<api::MandateTxnType>,
        merchant_account: &storage::MerchantAccount,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::VerifyRequest>,
        PaymentData<F>,
        Option<payments::CustomerDetails>,
    )> {
        let db = &state.store;

        let merchant_id = &merchant_account.merchant_id;
        let storage_scheme = merchant_account.storage_scheme;

        let (payment_intent, payment_attempt, connector_response);

        let payment_id = payment_id
            .get_payment_intent_id()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed while getting payment_intent_id from PaymentIdType")?;

        payment_attempt = match db
            .insert_payment_attempt(
                Self::make_payment_attempt(
                    &payment_id,
                    merchant_id,
                    request.payment_method,
                    request,
                ),
                storage_scheme,
            )
            .await
        {
            Ok(payment_attempt) => Ok(payment_attempt),
            Err(err) => {
                Err(err.change_context(errors::ApiErrorResponse::VerificationFailed { data: None }))
            }
        }?;

        payment_intent = match db
            .insert_payment_intent(
                Self::make_payment_intent(&payment_id, merchant_id, request),
                storage_scheme,
            )
            .await
        {
            Ok(payment_intent) => Ok(payment_intent),
            Err(err) => {
                Err(err.change_context(errors::ApiErrorResponse::VerificationFailed { data: None }))
            }
        }?;

        connector_response = match db
            .insert_connector_response(
                PaymentCreate::make_connector_response(&payment_attempt),
                storage_scheme,
            )
            .await
        {
            Ok(connector_resp) => Ok(connector_resp),
            Err(err) => {
                Err(err.change_context(errors::ApiErrorResponse::VerificationFailed { data: None }))
            }
        }?;

        Ok((
            Box::new(self),
            PaymentData {
                flow: PhantomData,
                payment_intent,
                payment_attempt,
                /// currency and amount are irrelevant in this scenario
                currency: storage_enums::Currency::default(),
                amount: api::Amount::Zero,
                email: None,
                mandate_id: None,
                setup_mandate: request.mandate_data.clone(),
                token: request.payment_token.clone(),
                connector_response,
                payment_method_data: request.payment_method_data.clone(),
                confirm: Some(true),
                address: types::PaymentAddress::default(),
                force_sync: None,
                refunds: vec![],
                sessions_token: vec![],
                card_cvc: None,
            },
            Some(payments::CustomerDetails {
                customer_id: request.customer_id.clone(),
                name: request.name.clone(),
                email: request.email.clone(),
                phone: request.phone.clone(),
                phone_country_code: request.phone_country_code.clone(),
            }),
        ))
    }
}

#[async_trait]
impl<F: Clone> UpdateTracker<F, PaymentData<F>, api::VerifyRequest> for PaymentMethodValidate {
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        db: &dyn StorageInterface,
        _payment_id: &api::PaymentIdType,
        mut payment_data: PaymentData<F>,
        _customer: Option<storage::Customer>,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> RouterResult<(BoxedOperation<'b, F, api::VerifyRequest>, PaymentData<F>)>
    where
        F: 'b + Send,
    {
        // There is no fsm involved in this operation all the change of states must happen in a single request
        let status = Some(storage_enums::IntentStatus::Processing);

        let customer_id = payment_data.payment_intent.customer_id.clone();

        payment_data.payment_intent = db
            .update_payment_intent(
                payment_data.payment_intent,
                storage::PaymentIntentUpdate::ReturnUrlUpdate {
                    return_url: None,
                    status,
                    customer_id,
                    shipping_address_id: None,
                    billing_address_id: None,
                },
                storage_scheme,
            )
            .await
            .map_err(|err| {
                err.to_not_found_response(errors::ApiErrorResponse::VerificationFailed {
                    data: None,
                })
            })?;

        Ok((Box::new(self), payment_data))
    }
}

#[async_trait]
impl<F, Op> Domain<F, api::VerifyRequest> for Op
where
    F: Clone + Send,
    Op: Send + Sync + Operation<F, api::VerifyRequest>,
    for<'a> &'a Op: Operation<F, api::VerifyRequest>,
{
    #[instrument(skip_all)]
    async fn get_or_create_customer_details<'a>(
        &'a self,
        db: &dyn StorageInterface,
        payment_data: &mut PaymentData<F>,
        request: Option<payments::CustomerDetails>,
        merchant_id: &str,
    ) -> CustomResult<
        (
            BoxedOperation<'a, F, api::VerifyRequest>,
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
    async fn make_pm_data<'a>(
        &'a self,
        state: &'a AppState,
        payment_data: &mut PaymentData<F>,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::VerifyRequest>,
        Option<api::PaymentMethod>,
    )> {
        helpers::make_pm_data(Box::new(self), state, payment_data).await
    }

    async fn get_connector<'a>(
        &'a self,
        _merchant_account: &storage::MerchantAccount,
        state: &AppState,
        _request: &api::VerifyRequest,
        previously_used_connector: Option<&String>,
    ) -> CustomResult<api::ConnectorCallType, errors::ApiErrorResponse> {
        helpers::get_connector_default(state, previously_used_connector).await
    }
}

impl PaymentMethodValidate {
    #[instrument(skip_all)]
    fn make_payment_attempt(
        payment_id: &str,
        merchant_id: &str,
        payment_method: Option<api_enums::PaymentMethodType>,
        _request: &api::VerifyRequest,
    ) -> storage::PaymentAttemptNew {
        let created_at @ modified_at @ last_synced = Some(date_time::now());
        let status = storage_enums::AttemptStatus::Pending;

        storage::PaymentAttemptNew {
            payment_id: payment_id.to_string(),
            merchant_id: merchant_id.to_string(),
            attempt_id: Uuid::new_v4().simple().to_string(),
            status,
            // Amount & Currency will be zero in this case
            amount: 0,
            currency: Default::default(),
            connector: None,
            payment_method: payment_method.map(ForeignInto::foreign_into),
            confirm: true,
            created_at,
            modified_at,
            last_synced,
            ..Default::default()
        }
    }

    fn make_payment_intent(
        payment_id: &str,
        merchant_id: &str,
        request: &api::VerifyRequest,
    ) -> storage::PaymentIntentNew {
        let created_at @ modified_at @ last_synced = Some(date_time::now());
        let status = helpers::payment_intent_status_fsm(&request.payment_method_data, Some(true));

        let client_secret =
            utils::generate_id(consts::ID_LENGTH, format!("{payment_id}_secret").as_str());
        storage::PaymentIntentNew {
            payment_id: payment_id.to_string(),
            merchant_id: merchant_id.to_string(),
            status,
            amount: 0,
            currency: Default::default(),
            connector_id: None,
            created_at,
            modified_at,
            last_synced,
            client_secret: Some(client_secret),
            setup_future_usage: request.setup_future_usage.map(ForeignInto::foreign_into),
            off_session: request.off_session,
            ..Default::default()
        }
    }
}
