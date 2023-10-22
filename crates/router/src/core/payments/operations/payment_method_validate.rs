use std::marker::PhantomData;

use api_models::enums::FrmSuggestion;
use async_trait::async_trait;
use common_utils::{date_time, errors::CustomResult, ext_traits::AsyncExt};
use error_stack::ResultExt;
use router_derive::PaymentOperation;
use router_env::{instrument, tracing};

use super::{BoxedOperation, Domain, GetTracker, PaymentCreate, UpdateTracker, ValidateRequest};
use crate::{
    consts,
    core::{
        errors::{self, RouterResult, StorageErrorExt},
        payment_methods::PaymentMethodRetrieve,
        payments::{self, helpers, operations, Operation, PaymentData},
        utils as core_utils,
    },
    db::StorageInterface,
    routes::AppState,
    services,
    types::{
        self,
        api::{self, enums as api_enums, PaymentIdTypeExt},
        domain,
        storage::{self, enums as storage_enums},
    },
    utils,
};

#[derive(Debug, Clone, Copy, PaymentOperation)]
#[operation(ops = "all", flow = "verify")]
pub struct PaymentMethodValidate;

impl<F: Send + Clone, Ctx: PaymentMethodRetrieve> ValidateRequest<F, api::VerifyRequest, Ctx>
    for PaymentMethodValidate
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        request: &api::VerifyRequest,
        merchant_account: &'a domain::MerchantAccount,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::VerifyRequest, Ctx>,
        operations::ValidateResult<'a>,
    )> {
        let request_merchant_id = request.merchant_id.as_deref();
        helpers::validate_merchant_id(&merchant_account.merchant_id, request_merchant_id)
            .change_context(errors::ApiErrorResponse::MerchantAccountNotFound)?;

        let mandate_type =
            helpers::validate_mandate(request, payments::is_operation_confirm(self))?;
        let validation_id = core_utils::get_or_generate_id("validation_id", &None, "val")?;

        Ok((
            Box::new(self),
            operations::ValidateResult {
                merchant_id: &merchant_account.merchant_id,
                payment_id: api::PaymentIdType::PaymentIntentId(validation_id),
                mandate_type,
                storage_scheme: merchant_account.storage_scheme,
                requeue: false,
            },
        ))
    }
}

#[async_trait]
impl<F: Send + Clone, Ctx: PaymentMethodRetrieve>
    GetTracker<F, PaymentData<F>, api::VerifyRequest, Ctx> for PaymentMethodValidate
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a AppState,
        payment_id: &api::PaymentIdType,
        request: &api::VerifyRequest,
        _mandate_type: Option<api::MandateTransactionType>,
        merchant_account: &domain::MerchantAccount,
        _mechant_key_store: &domain::MerchantKeyStore,
        _auth_flow: services::AuthFlow,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::VerifyRequest, Ctx>,
        PaymentData<F>,
        Option<payments::CustomerDetails>,
    )> {
        let db = &*state.store;

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
                    state,
                    merchant_account.storage_scheme,
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
                Self::make_payment_intent(
                    &payment_id,
                    merchant_id,
                    request,
                    payment_attempt.attempt_id.clone(),
                    merchant_account.storage_scheme,
                ),
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
                /// currency and amount are irrelevant in this scenario
                currency: storage_enums::Currency::default(),
                amount: api::Amount::Zero,
                email: None,
                mandate_id: None,
                mandate_connector: None,
                setup_mandate: request.mandate_data.clone().map(Into::into),
                token: request.payment_token.clone(),
                connector_response,
                payment_method_data: request.payment_method_data.clone(),
                confirm: Some(true),
                address: types::PaymentAddress::default(),
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
impl<F: Clone, Ctx: PaymentMethodRetrieve> UpdateTracker<F, PaymentData<F>, api::VerifyRequest, Ctx>
    for PaymentMethodValidate
{
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        db: &dyn StorageInterface,
        mut payment_data: PaymentData<F>,
        _customer: Option<domain::Customer>,
        storage_scheme: storage_enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        _mechant_key_store: &domain::MerchantKeyStore,
        _frm_suggestion: Option<FrmSuggestion>,
        _header_payload: api::HeaderPayload,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::VerifyRequest, Ctx>,
        PaymentData<F>,
    )>
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
                    updated_by: storage_scheme.to_string(),
                },
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::VerificationFailed { data: None })?;

        Ok((Box::new(self), payment_data))
    }
}

#[async_trait]
impl<F, Op, Ctx: PaymentMethodRetrieve> Domain<F, api::VerifyRequest, Ctx> for Op
where
    F: Clone + Send,
    Op: Send + Sync + Operation<F, api::VerifyRequest, Ctx>,
    for<'a> &'a Op: Operation<F, api::VerifyRequest, Ctx>,
{
    #[instrument(skip_all)]
    async fn get_or_create_customer_details<'a>(
        &'a self,
        db: &dyn StorageInterface,
        payment_data: &mut PaymentData<F>,
        request: Option<payments::CustomerDetails>,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<
        (
            BoxedOperation<'a, F, api::VerifyRequest, Ctx>,
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
    async fn make_pm_data<'a>(
        &'a self,
        state: &'a AppState,
        payment_data: &mut PaymentData<F>,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::VerifyRequest, Ctx>,
        Option<api::PaymentMethodData>,
    )> {
        helpers::make_pm_data(Box::new(self), state, payment_data).await
    }

    async fn get_connector<'a>(
        &'a self,
        _merchant_account: &domain::MerchantAccount,
        state: &AppState,
        _request: &api::VerifyRequest,
        _payment_intent: &storage::PaymentIntent,
        _mechant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<api::ConnectorChoice, errors::ApiErrorResponse> {
        helpers::get_connector_default(state, None).await
    }
}

impl PaymentMethodValidate {
    #[instrument(skip_all)]
    fn make_payment_attempt(
        payment_id: &str,
        merchant_id: &str,
        payment_method: Option<api_enums::PaymentMethod>,
        _request: &api::VerifyRequest,
        state: &AppState,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> storage::PaymentAttemptNew {
        let created_at @ modified_at @ last_synced = Some(date_time::now());
        let status = storage_enums::AttemptStatus::Pending;
        let attempt_id = if core_utils::is_merchant_enabled_for_payment_id_as_connector_request_id(
            &state.conf,
            merchant_id,
        ) {
            payment_id.to_string()
        } else {
            utils::get_payment_attempt_id(payment_id, 1)
        };

        storage::PaymentAttemptNew {
            payment_id: payment_id.to_string(),
            merchant_id: merchant_id.to_string(),
            attempt_id,
            status,
            // Amount & Currency will be zero in this case
            amount: 0,
            currency: Default::default(),
            connector: None,
            payment_method,
            confirm: true,
            created_at,
            modified_at,
            last_synced,
            updated_by: storage_scheme.to_string(),
            ..Default::default()
        }
    }

    fn make_payment_intent(
        payment_id: &str,
        merchant_id: &str,
        request: &api::VerifyRequest,
        active_attempt_id: String,
        storage_scheme: storage_enums::MerchantStorageScheme,
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
            setup_future_usage: request.setup_future_usage,
            off_session: request.off_session,
            active_attempt: data_models::RemoteStorageObject::ForeignID(active_attempt_id),
            attempt_count: 1,
            amount_captured: Default::default(),
            customer_id: Default::default(),
            description: Default::default(),
            return_url: Default::default(),
            metadata: Default::default(),
            shipping_address_id: Default::default(),
            billing_address_id: Default::default(),
            statement_descriptor_name: Default::default(),
            statement_descriptor_suffix: Default::default(),
            business_country: Default::default(),
            business_label: Default::default(),
            order_details: Default::default(),
            allowed_payment_method_types: Default::default(),
            connector_metadata: Default::default(),
            feature_metadata: Default::default(),
            profile_id: Default::default(),
            merchant_decision: Default::default(),
            payment_confirm_source: Default::default(),
            surcharge_applicable: Default::default(),
            payment_link_id: Default::default(),
            updated_by: storage_scheme.to_string(),
        }
    }
}
