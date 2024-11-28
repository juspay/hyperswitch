use std::marker::PhantomData;

use api_models::{enums::FrmSuggestion, payments::PaymentsUpdateIntentRequest};
use async_trait::async_trait;
use common_utils::{
    errors::CustomResult,
    ext_traits::{AsyncExt, ValueExt},
    types::MinorUnit,
};
use diesel_models::{types::FeatureMetadata, PaymentIntentNew};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payments::payment_intent::{PaymentIntentUpdate, PaymentIntentUpdateFields},
    ApiModelToDieselModelConvertor,
};
use masking::Secret;
use router_env::{instrument, tracing};

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        errors::{self, RouterResult},
        payment_methods::cards::create_encrypted_data,
        payments::{self, operations},
    },
    db::errors::StorageErrorExt,
    routes::{app::ReqState, SessionState},
    types::{
        api, domain,
        storage::{self, enums},
    },
};

#[derive(Debug, Clone, Copy)]
pub struct PaymentUpdateIntent;

impl<F: Send + Clone> Operation<F, PaymentsUpdateIntentRequest> for &PaymentUpdateIntent {
    type Data = payments::PaymentIntentData<F>;
    fn to_validate_request(
        &self,
    ) -> RouterResult<
        &(dyn ValidateRequest<F, PaymentsUpdateIntentRequest, Self::Data> + Send + Sync),
    > {
        Ok(*self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<F, Self::Data, PaymentsUpdateIntentRequest> + Send + Sync)>
    {
        Ok(*self)
    }
    fn to_domain(&self) -> RouterResult<&(dyn Domain<F, PaymentsUpdateIntentRequest, Self::Data>)> {
        Ok(*self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, Self::Data, PaymentsUpdateIntentRequest> + Send + Sync)>
    {
        Ok(*self)
    }
}

impl<F: Send + Clone> Operation<F, PaymentsUpdateIntentRequest> for PaymentUpdateIntent {
    type Data = payments::PaymentIntentData<F>;
    fn to_validate_request(
        &self,
    ) -> RouterResult<
        &(dyn ValidateRequest<F, PaymentsUpdateIntentRequest, Self::Data> + Send + Sync),
    > {
        Ok(self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<F, Self::Data, PaymentsUpdateIntentRequest> + Send + Sync)>
    {
        Ok(self)
    }
    fn to_domain(&self) -> RouterResult<&dyn Domain<F, PaymentsUpdateIntentRequest, Self::Data>> {
        Ok(self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, Self::Data, PaymentsUpdateIntentRequest> + Send + Sync)>
    {
        Ok(self)
    }
}

type PaymentsUpdateIntentOperation<'b, F> =
    BoxedOperation<'b, F, PaymentsUpdateIntentRequest, payments::PaymentIntentData<F>>;

#[async_trait]
impl<F: Send + Clone> GetTracker<F, payments::PaymentIntentData<F>, PaymentsUpdateIntentRequest>
    for PaymentUpdateIntent
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a SessionState,
        payment_id: &common_utils::id_type::GlobalPaymentId,
        request: &PaymentsUpdateIntentRequest,
        merchant_account: &domain::MerchantAccount,
        profile: &domain::Profile,
        key_store: &domain::MerchantKeyStore,
        _header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<operations::GetTrackerResponse<payments::PaymentIntentData<F>>> {
        let db = &*state.store;
        let key_manager_state = &state.into();
        let storage_scheme = merchant_account.storage_scheme;
        let payment_intent = db
            .find_payment_intent_by_id(key_manager_state, payment_id, key_store, storage_scheme)
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        let PaymentsUpdateIntentRequest {
            amount_details,
            routing_algorithm_id,
            capture_method,
            authentication_type,
            billing,
            shipping,
            customer_present,
            description,
            return_url,
            setup_future_usage,
            apply_mit_exemption,
            statement_descriptor,
            order_details,
            allowed_payment_method_types,
            metadata,
            connector_metadata,
            feature_metadata,
            enable_payment_link,
            payment_link_config,
            request_incremental_authorization,
            session_expiry,
            frm_metadata,
            request_external_three_ds_authentication,
        } = request.clone();

        // TODO: Use Batch Encryption
        let billing_address = billing
            .clone()
            .async_map(|billing_details| {
                create_encrypted_data(key_manager_state, key_store, billing_details)
            })
            .await
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to encrypt billing details")?
            .map(|encrypted_value| {
                encrypted_value.deserialize_inner_value(|value| value.parse_value("Address"))
            })
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to deserialize decrypted value to Address")?;

        let shipping_address = shipping
            .clone()
            .async_map(|shipping_details| {
                create_encrypted_data(key_manager_state, key_store, shipping_details)
            })
            .await
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to encrypt shipping details")?
            .map(|encrypted_value| {
                encrypted_value.deserialize_inner_value(|value| value.parse_value("Address"))
            })
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to deserialize decrypted value to Address")?;

        let order_details = order_details.clone().map(|order_details| {
            order_details
                .into_iter()
                .map(|order_detail| {
                    Secret::new(diesel_models::types::OrderDetailsWithAmount::convert_from(
                        order_detail,
                    ))
                })
                .collect()
        });

        let session_expiry = session_expiry.map(|expiry| {
            payment_intent
                .created_at
                .saturating_add(time::Duration::seconds(i64::from(expiry)))
        });

        let updated_amount_details = match amount_details {
            Some(details) => payment_intent.amount_details.update_from_request(&details),
            None => payment_intent.amount_details,
        };

        let payment_intent = hyperswitch_domain_models::payments::PaymentIntent {
            amount_details: updated_amount_details,
            description: description.or(payment_intent.description),
            return_url: return_url.or(payment_intent.return_url),
            metadata: metadata.or(payment_intent.metadata),
            statement_descriptor: statement_descriptor.or(payment_intent.statement_descriptor),
            modified_at: common_utils::date_time::now(),
            order_details,
            connector_metadata: connector_metadata.or(payment_intent.connector_metadata),
            feature_metadata: (feature_metadata
                .map(FeatureMetadata::convert_from)
                .or(payment_intent.feature_metadata)),
            updated_by: storage_scheme.to_string(),
            request_incremental_authorization: request_incremental_authorization
                .unwrap_or(payment_intent.request_incremental_authorization),
            session_expiry: session_expiry.unwrap_or(payment_intent.session_expiry),
            request_external_three_ds_authentication: request_external_three_ds_authentication
                .unwrap_or(payment_intent.request_external_three_ds_authentication),
            frm_metadata: frm_metadata.or(payment_intent.frm_metadata),
            billing_address,
            shipping_address,
            capture_method: capture_method.unwrap_or(payment_intent.capture_method),
            authentication_type: authentication_type.unwrap_or(payment_intent.authentication_type),
            enable_payment_link: enable_payment_link.unwrap_or(payment_intent.enable_payment_link),
            apply_mit_exemption: apply_mit_exemption.unwrap_or(payment_intent.apply_mit_exemption),
            customer_present: customer_present.unwrap_or(payment_intent.customer_present),
            routing_algorithm_id: routing_algorithm_id.or(payment_intent.routing_algorithm_id),
            allowed_payment_method_types: allowed_payment_method_types
                .or(payment_intent.allowed_payment_method_types),
            ..payment_intent
        };

        let payment_data = payments::PaymentIntentData {
            flow: PhantomData,
            payment_intent,
        };

        let get_trackers_response = operations::GetTrackerResponse { payment_data };

        Ok(get_trackers_response)
    }
}

#[async_trait]
impl<F: Clone> UpdateTracker<F, payments::PaymentIntentData<F>, PaymentsUpdateIntentRequest>
    for PaymentUpdateIntent
{
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        state: &'b SessionState,
        _req_state: ReqState,
        payment_data: payments::PaymentIntentData<F>,
        _customer: Option<domain::Customer>,
        storage_scheme: enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        key_store: &domain::MerchantKeyStore,
        _frm_suggestion: Option<FrmSuggestion>,
        _header_payload: hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<(
        PaymentsUpdateIntentOperation<'b, F>,
        payments::PaymentIntentData<F>,
    )>
    where
        F: 'b + Send,
    {
        let db = &*state.store;
        let key_manager_state = &state.into();

        let intent = payment_data.payment_intent.clone();

        let payment_intent_update =
            PaymentIntentUpdate::UpdateIntent(Box::new(PaymentIntentUpdateFields {
                amount: Some(intent.amount_details.order_amount),
                currency: Some(intent.amount_details.currency),
                shipping_cost: intent.amount_details.shipping_cost,
                skip_external_tax_calculation: Some(
                    intent.amount_details.skip_external_tax_calculation,
                ),
                skip_surcharge_calculation: Some(intent.amount_details.skip_surcharge_calculation),
                surcharge_amount: intent.amount_details.surcharge_amount,
                tax_on_surcharge: intent.amount_details.tax_on_surcharge,
                routing_algorithm_id: intent.routing_algorithm_id,
                capture_method: Some(intent.capture_method),
                authentication_type: Some(intent.authentication_type),
                billing_address: intent.billing_address,
                shipping_address: intent.shipping_address,
                customer_present: Some(intent.customer_present),
                description: intent.description,
                return_url: intent.return_url,
                setup_future_usage: Some(intent.setup_future_usage),
                apply_mit_exemption: Some(intent.apply_mit_exemption),
                statement_descriptor: intent.statement_descriptor,
                order_details: intent.order_details,
                allowed_payment_method_types: intent.allowed_payment_method_types,
                metadata: intent.metadata,
                connector_metadata: intent.connector_metadata,
                feature_metadata: intent.feature_metadata,
                enable_payment_link: Some(intent.enable_payment_link),
                request_incremental_authorization: Some(intent.request_incremental_authorization),
                session_expiry: Some(intent.session_expiry),
                frm_metadata: intent.frm_metadata,
                request_external_three_ds_authentication: Some(
                    intent.request_external_three_ds_authentication,
                ),
                updated_by: intent.updated_by,
            }));

        let new_payment_intent = db
            .update_payment_intent(
                key_manager_state,
                payment_data.payment_intent,
                payment_intent_update,
                key_store,
                storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Could not update Intent")?;

        let payment_data = payments::PaymentIntentData {
            flow: PhantomData,
            payment_intent: new_payment_intent,
        };

        Ok((Box::new(self), payment_data))
    }
}

impl<F: Send + Clone>
    ValidateRequest<F, PaymentsUpdateIntentRequest, payments::PaymentIntentData<F>>
    for PaymentUpdateIntent
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        _request: &PaymentsUpdateIntentRequest,
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
impl<F: Clone + Send> Domain<F, PaymentsUpdateIntentRequest, payments::PaymentIntentData<F>>
    for PaymentUpdateIntent
{
    #[instrument(skip_all)]
    async fn get_customer_details<'a>(
        &'a self,
        _state: &SessionState,
        _payment_data: &mut payments::PaymentIntentData<F>,
        _merchant_key_store: &domain::MerchantKeyStore,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<
        (
            BoxedOperation<'a, F, PaymentsUpdateIntentRequest, payments::PaymentIntentData<F>>,
            Option<domain::Customer>,
        ),
        errors::StorageError,
    > {
        Ok((Box::new(self), None))
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
        PaymentsUpdateIntentOperation<'a, F>,
        Option<domain::PaymentMethodData>,
        Option<String>,
    )> {
        Ok((Box::new(self), None, None))
    }

    #[instrument(skip_all)]
    async fn perform_routing<'a>(
        &'a self,
        _merchant_account: &domain::MerchantAccount,
        _business_profile: &domain::Profile,
        _state: &SessionState,
        _payment_data: &mut payments::PaymentIntentData<F>,
        _mechant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<api::ConnectorCallType, errors::ApiErrorResponse> {
        Ok(api::ConnectorCallType::Skip)
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
