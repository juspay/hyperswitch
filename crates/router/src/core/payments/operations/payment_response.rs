use std::collections::HashMap;

use api_models::payments::{ConnectorMandateReferenceId, MandateReferenceId};
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use api_models::routing::RoutableConnectorChoice;
use async_trait::async_trait;
use common_enums::{AuthorizationStatus, SessionUpdateStatus};
use common_utils::{
    ext_traits::{AsyncExt, Encode, ValueExt},
    types::{keymanager::KeyManagerState, ConnectorTransactionId, MinorUnit},
};
use error_stack::{report, ResultExt};
use futures::FutureExt;
use hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt;
use router_derive;
use router_env::{instrument, logger, metrics::add_attributes, tracing};
use storage_impl::DataModelExt;
use tracing_futures::Instrument;

use super::{Operation, OperationSessionSetters, PostUpdateTracker};
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use crate::core::routing::helpers::push_metrics_for_success_based_routing;
use crate::{
    connector::utils::PaymentResponseRouterData,
    consts,
    core::{
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        mandate, payment_methods,
        payment_methods::cards::create_encrypted_data,
        payments::{
            helpers::{
                self as payments_helpers,
                update_additional_payment_data_with_connector_response_pm_data,
            },
            tokenization,
            types::MultipleCaptureData,
            PaymentData, PaymentMethodChecker,
        },
        utils as core_utils,
    },
    routes::{metrics, SessionState},
    types::{
        self, api, domain,
        storage::{self, enums},
        transformers::{ForeignFrom, ForeignTryFrom},
        CaptureSyncResponse, ErrorResponse,
    },
    utils,
};

#[cfg(feature = "v1")]
#[derive(Debug, Clone, Copy, router_derive::PaymentOperation)]
#[operation(
    operations = "post_update_tracker",
    flow = "sync_data, cancel_data, authorize_data, capture_data, complete_authorize_data, approve_data, reject_data, setup_mandate_data, session_data,incremental_authorization_data, sdk_session_update_data, post_session_tokens_data"
)]
pub struct PaymentResponse;

#[cfg(feature = "v2")]
#[derive(Debug, Clone, Copy)]
pub struct PaymentResponse;

#[cfg(feature = "v1")]
#[async_trait]
impl<F: Send + Clone> PostUpdateTracker<F, PaymentData<F>, types::PaymentsAuthorizeData>
    for PaymentResponse
{
    async fn update_tracker<'b>(
        &'b self,
        db: &'b SessionState,
        payment_id: &api::PaymentIdType,
        mut payment_data: PaymentData<F>,
        router_data: types::RouterData<
            F,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: enums::MerchantStorageScheme,
        locale: &Option<String>,
        #[cfg(all(feature = "v1", feature = "dynamic_routing"))] routable_connector: Vec<
            RoutableConnectorChoice,
        >,
        #[cfg(all(feature = "v1", feature = "dynamic_routing"))] business_profile: &domain::Profile,
    ) -> RouterResult<PaymentData<F>>
    where
        F: 'b,
    {
        payment_data.mandate_id = payment_data
            .mandate_id
            .or_else(|| router_data.request.mandate_id.clone());

        payment_data = Box::pin(payment_response_update_tracker(
            db,
            payment_id,
            payment_data,
            router_data,
            key_store,
            storage_scheme,
            locale,
            #[cfg(all(feature = "v1", feature = "dynamic_routing"))]
            routable_connector,
            #[cfg(all(feature = "v1", feature = "dynamic_routing"))]
            business_profile,
        ))
        .await?;

        Ok(payment_data)
    }

    #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
    async fn save_pm_and_mandate<'b>(
        &self,
        state: &SessionState,
        resp: &types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        payment_data: &mut PaymentData<F>,
        business_profile: &domain::Profile,
    ) -> CustomResult<(), errors::ApiErrorResponse>
    where
        F: 'b + Clone + Send + Sync,
    {
        todo!()
    }

    #[cfg(all(
        any(feature = "v2", feature = "v1"),
        not(feature = "payment_methods_v2")
    ))]
    async fn save_pm_and_mandate<'b>(
        &self,
        state: &SessionState,
        resp: &types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        payment_data: &mut PaymentData<F>,
        business_profile: &domain::Profile,
    ) -> CustomResult<(), errors::ApiErrorResponse>
    where
        F: 'b + Clone + Send + Sync,
    {
        let customer_id = payment_data.payment_intent.customer_id.clone();
        let save_payment_data = tokenization::SavePaymentMethodData::from(resp);
        let payment_method_billing_address = payment_data.address.get_payment_method_billing();

        let connector_name = payment_data
            .payment_attempt
            .connector
            .clone()
            .ok_or_else(|| {
                logger::error!("Missing required Param connector_name");
                errors::ApiErrorResponse::MissingRequiredField {
                    field_name: "connector_name",
                }
            })?;
        let merchant_connector_id = payment_data.payment_attempt.merchant_connector_id.clone();
        let billing_name = resp
            .address
            .get_payment_method_billing()
            .and_then(|billing_details| billing_details.address.as_ref())
            .and_then(|address| address.get_optional_full_name());
        let mut should_avoid_saving = false;

        if let Some(payment_method_info) = &payment_data.payment_method_info {
            if payment_data.payment_intent.off_session.is_none() && resp.response.is_ok() {
                should_avoid_saving = resp.request.payment_method_type
                    == Some(enums::PaymentMethodType::ApplePay)
                    || resp.request.payment_method_type
                        == Some(enums::PaymentMethodType::GooglePay);
                payment_methods::cards::update_last_used_at(
                    payment_method_info,
                    state,
                    merchant_account.storage_scheme,
                    key_store,
                )
                .await
                .map_err(|e| {
                    logger::error!("Failed to update last used at: {:?}", e);
                })
                .ok();
            }
        };

        let save_payment_call_future = Box::pin(tokenization::save_payment_method(
            state,
            connector_name.clone(),
            save_payment_data,
            customer_id.clone(),
            merchant_account,
            resp.request.payment_method_type,
            key_store,
            billing_name.clone(),
            payment_method_billing_address,
            business_profile,
        ));

        let is_connector_mandate = resp.request.customer_acceptance.is_some()
            && matches!(
                resp.request.setup_future_usage,
                Some(enums::FutureUsage::OffSession)
            );

        let is_legacy_mandate = resp.request.setup_mandate_details.is_some()
            && matches!(
                resp.request.setup_future_usage,
                Some(enums::FutureUsage::OffSession)
            );
        let storage_scheme = merchant_account.storage_scheme;
        if is_legacy_mandate {
            // Mandate is created on the application side and at the connector.
            let tokenization::SavePaymentMethodDataResponse {
                payment_method_id, ..
            } = save_payment_call_future.await?;

            let mandate_id = mandate::mandate_procedure(
                state,
                resp,
                &customer_id.clone(),
                payment_method_id.clone(),
                merchant_connector_id.clone(),
                merchant_account.storage_scheme,
                payment_data.payment_intent.get_id(),
            )
            .await?;
            payment_data.payment_attempt.payment_method_id = payment_method_id;
            payment_data.payment_attempt.mandate_id = mandate_id;

            Ok(())
        } else if is_connector_mandate {
            // The mandate is created on connector's end.
            let tokenization::SavePaymentMethodDataResponse {
                payment_method_id,
                connector_mandate_reference_id,
                ..
            } = save_payment_call_future.await?;
            payment_data.payment_method_info = if let Some(payment_method_id) = &payment_method_id {
                match state
                    .store
                    .find_payment_method(
                        &(state.into()),
                        key_store,
                        payment_method_id,
                        storage_scheme,
                    )
                    .await
                {
                    Ok(payment_method) => Some(payment_method),
                    Err(error) => {
                        if error.current_context().is_db_not_found() {
                            logger::info!("Payment Method not found in db {:?}", error);
                            None
                        } else {
                            Err(error)
                                .change_context(errors::ApiErrorResponse::InternalServerError)
                                .attach_printable("Error retrieving payment method from db")
                                .map_err(|err| logger::error!(payment_method_retrieve=?err))
                                .ok()
                        }
                    }
                }
            } else {
                None
            };
            payment_data.payment_attempt.payment_method_id = payment_method_id;
            payment_data.payment_attempt.connector_mandate_detail = connector_mandate_reference_id
                .clone()
                .map(ForeignFrom::foreign_from);
            payment_data.set_mandate_id(api_models::payments::MandateIds {
                mandate_id: None,
                mandate_reference_id: connector_mandate_reference_id.map(|connector_mandate_id| {
                    MandateReferenceId::ConnectorMandateId(connector_mandate_id)
                }),
            });
            Ok(())
        } else if should_avoid_saving {
            if let Some(pm_info) = &payment_data.payment_method_info {
                payment_data.payment_attempt.payment_method_id = Some(pm_info.get_id().clone());
            };
            Ok(())
        } else {
            // Save card flow
            let save_payment_data = tokenization::SavePaymentMethodData::from(resp);
            let merchant_account = merchant_account.clone();
            let key_store = key_store.clone();
            let state = state.clone();
            let customer_id = payment_data.payment_intent.customer_id.clone();
            let payment_attempt = payment_data.payment_attempt.clone();

            let business_profile = business_profile.clone();
            let payment_method_type = resp.request.payment_method_type;
            let payment_method_billing_address = payment_method_billing_address.cloned();

            logger::info!("Call to save_payment_method in locker");
            let _task_handle = tokio::spawn(
                async move {
                    logger::info!("Starting async call to save_payment_method in locker");

                    let result = Box::pin(tokenization::save_payment_method(
                        &state,
                        connector_name,
                        save_payment_data,
                        customer_id,
                        &merchant_account,
                        payment_method_type,
                        &key_store,
                        billing_name,
                        payment_method_billing_address.as_ref(),
                        &business_profile,
                    ))
                    .await;

                    if let Err(err) = result {
                        logger::error!("Asynchronously saving card in locker failed : {:?}", err);
                    } else if let Ok(tokenization::SavePaymentMethodDataResponse {
                        payment_method_id,
                        ..
                    }) = result
                    {
                        let payment_attempt_update =
                            storage::PaymentAttemptUpdate::PaymentMethodDetailsUpdate {
                                payment_method_id,
                                updated_by: storage_scheme.clone().to_string(),
                            };

                        #[cfg(feature = "v1")]
                        let respond = state
                            .store
                            .update_payment_attempt_with_attempt_id(
                                payment_attempt,
                                payment_attempt_update,
                                storage_scheme,
                            )
                            .await;

                        #[cfg(feature = "v2")]
                        let respond = state
                            .store
                            .update_payment_attempt_with_attempt_id(
                                &(&state).into(),
                                &key_store,
                                payment_attempt,
                                payment_attempt_update,
                                storage_scheme,
                            )
                            .await;

                        if let Err(err) = respond {
                            logger::error!("Error updating payment attempt: {:?}", err);
                        };
                    }
                }
                .in_current_span(),
            );
            Ok(())
        }
    }
}

#[cfg(feature = "v1")]
#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentData<F>, types::PaymentsIncrementalAuthorizationData>
    for PaymentResponse
{
    async fn update_tracker<'b>(
        &'b self,
        state: &'b SessionState,
        _payment_id: &api::PaymentIdType,
        mut payment_data: PaymentData<F>,
        router_data: types::RouterData<
            F,
            types::PaymentsIncrementalAuthorizationData,
            types::PaymentsResponseData,
        >,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: enums::MerchantStorageScheme,
        _locale: &Option<String>,
        #[cfg(all(feature = "v1", feature = "dynamic_routing"))] _routable_connector: Vec<
            RoutableConnectorChoice,
        >,
        #[cfg(all(feature = "v1", feature = "dynamic_routing"))]
        _business_profile: &domain::Profile,
    ) -> RouterResult<PaymentData<F>>
    where
        F: 'b + Send,
    {
        let incremental_authorization_details = payment_data
            .incremental_authorization_details
            .clone()
            .ok_or_else(|| {
                report!(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("missing incremental_authorization_details in payment_data")
            })?;
        // Update payment_intent and payment_attempt 'amount' if incremental_authorization is successful
        let (option_payment_attempt_update, option_payment_intent_update) = match router_data
            .response
            .clone()
        {
            Err(_) => (None, None),
            Ok(types::PaymentsResponseData::IncrementalAuthorizationResponse {
                status, ..
            }) => {
                if status == AuthorizationStatus::Success {
                    (
                        Some(
                            storage::PaymentAttemptUpdate::IncrementalAuthorizationAmountUpdate {
                                net_amount: hyperswitch_domain_models::payments::payment_attempt::NetAmount::new(
                                    incremental_authorization_details.total_amount,
                                    None,
                                    None,
                                    None,
                                    None,
                                ),
                                amount_capturable: incremental_authorization_details.total_amount,
                            },
                        ),
                        Some(
                            storage::PaymentIntentUpdate::IncrementalAuthorizationAmountUpdate {
                                amount: incremental_authorization_details.total_amount,
                            },
                        ),
                    )
                } else {
                    (None, None)
                }
            }
            _ => Err(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("unexpected response in incremental_authorization flow")?,
        };
        //payment_attempt update
        if let Some(payment_attempt_update) = option_payment_attempt_update {
            #[cfg(feature = "v1")]
            {
                payment_data.payment_attempt = state
                    .store
                    .update_payment_attempt_with_attempt_id(
                        payment_data.payment_attempt.clone(),
                        payment_attempt_update,
                        storage_scheme,
                    )
                    .await
                    .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
            }

            #[cfg(feature = "v2")]
            {
                payment_data.payment_attempt = state
                    .store
                    .update_payment_attempt_with_attempt_id(
                        &state.into(),
                        key_store,
                        payment_data.payment_attempt.clone(),
                        payment_attempt_update,
                        storage_scheme,
                    )
                    .await
                    .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
            }
        }
        // payment_intent update
        if let Some(payment_intent_update) = option_payment_intent_update {
            payment_data.payment_intent = state
                .store
                .update_payment_intent(
                    &state.into(),
                    payment_data.payment_intent.clone(),
                    payment_intent_update,
                    key_store,
                    storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
        }
        // Update the status of authorization record
        let authorization_update = match &router_data.response {
            Err(res) => Ok(storage::AuthorizationUpdate::StatusUpdate {
                status: AuthorizationStatus::Failure,
                error_code: Some(res.code.clone()),
                error_message: Some(res.message.clone()),
                connector_authorization_id: None,
            }),
            Ok(types::PaymentsResponseData::IncrementalAuthorizationResponse {
                status,
                error_code,
                error_message,
                connector_authorization_id,
            }) => Ok(storage::AuthorizationUpdate::StatusUpdate {
                status: status.clone(),
                error_code: error_code.clone(),
                error_message: error_message.clone(),
                connector_authorization_id: connector_authorization_id.clone(),
            }),
            Ok(_) => Err(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("unexpected response in incremental_authorization flow"),
        }?;
        let authorization_id = incremental_authorization_details
            .authorization_id
            .clone()
            .ok_or(
                report!(errors::ApiErrorResponse::InternalServerError).attach_printable(
                    "missing authorization_id in incremental_authorization_details in payment_data",
                ),
            )?;
        state
            .store
            .update_authorization_by_merchant_id_authorization_id(
                router_data.merchant_id.clone(),
                authorization_id,
                authorization_update,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("failed while updating authorization")?;
        //Fetch all the authorizations of the payment and send in incremental authorization response
        let authorizations = state
            .store
            .find_all_authorizations_by_merchant_id_payment_id(
                &router_data.merchant_id,
                payment_data.payment_intent.get_id(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("failed while retrieving authorizations")?;
        payment_data.authorizations = authorizations;
        Ok(payment_data)
    }
}

#[cfg(feature = "v1")]
#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentData<F>, types::PaymentsSyncData> for PaymentResponse {
    async fn update_tracker<'b>(
        &'b self,
        db: &'b SessionState,
        payment_id: &api::PaymentIdType,
        payment_data: PaymentData<F>,
        router_data: types::RouterData<F, types::PaymentsSyncData, types::PaymentsResponseData>,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: enums::MerchantStorageScheme,
        locale: &Option<String>,
        #[cfg(all(feature = "v1", feature = "dynamic_routing"))] routable_connector: Vec<
            RoutableConnectorChoice,
        >,
        #[cfg(all(feature = "v1", feature = "dynamic_routing"))] business_profile: &domain::Profile,
    ) -> RouterResult<PaymentData<F>>
    where
        F: 'b + Send,
    {
        Box::pin(payment_response_update_tracker(
            db,
            payment_id,
            payment_data,
            router_data,
            key_store,
            storage_scheme,
            locale,
            #[cfg(all(feature = "v1", feature = "dynamic_routing"))]
            routable_connector,
            #[cfg(all(feature = "v1", feature = "dynamic_routing"))]
            business_profile,
        ))
        .await
    }

    async fn save_pm_and_mandate<'b>(
        &self,
        state: &SessionState,
        resp: &types::RouterData<F, types::PaymentsSyncData, types::PaymentsResponseData>,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        payment_data: &mut PaymentData<F>,
        business_profile: &domain::Profile,
    ) -> CustomResult<(), errors::ApiErrorResponse>
    where
        F: 'b + Clone + Send + Sync,
    {
        let (connector_mandate_id, mandate_metadata) = resp
            .response
            .clone()
            .ok()
            .and_then(|resp| {
                if let types::PaymentsResponseData::TransactionResponse {
                    mandate_reference, ..
                } = resp
                {
                    mandate_reference.map(|mandate_ref| {
                        (
                            mandate_ref.connector_mandate_id.clone(),
                            mandate_ref.mandate_metadata.clone(),
                        )
                    })
                } else {
                    None
                }
            })
            .unwrap_or((None, None));

        update_connector_mandate_details_for_the_flow(
            connector_mandate_id,
            mandate_metadata,
            payment_data,
        )?;

        update_payment_method_status_and_ntid(
            state,
            key_store,
            payment_data,
            resp.status,
            resp.response.clone(),
            merchant_account.storage_scheme,
            business_profile.is_connector_agnostic_mit_enabled,
        )
        .await?;
        Ok(())
    }
}

#[cfg(feature = "v1")]
#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentData<F>, types::PaymentsSessionData>
    for PaymentResponse
{
    async fn update_tracker<'b>(
        &'b self,
        db: &'b SessionState,
        payment_id: &api::PaymentIdType,
        mut payment_data: PaymentData<F>,
        router_data: types::RouterData<F, types::PaymentsSessionData, types::PaymentsResponseData>,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: enums::MerchantStorageScheme,
        locale: &Option<String>,
        #[cfg(all(feature = "v1", feature = "dynamic_routing"))] routable_connector: Vec<
            RoutableConnectorChoice,
        >,
        #[cfg(all(feature = "v1", feature = "dynamic_routing"))] business_profile: &domain::Profile,
    ) -> RouterResult<PaymentData<F>>
    where
        F: 'b + Send,
    {
        payment_data = Box::pin(payment_response_update_tracker(
            db,
            payment_id,
            payment_data,
            router_data,
            key_store,
            storage_scheme,
            locale,
            #[cfg(all(feature = "v1", feature = "dynamic_routing"))]
            routable_connector,
            #[cfg(all(feature = "v1", feature = "dynamic_routing"))]
            business_profile,
        ))
        .await?;

        Ok(payment_data)
    }
}

#[cfg(feature = "v1")]
#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentData<F>, types::SdkPaymentsSessionUpdateData>
    for PaymentResponse
{
    async fn update_tracker<'b>(
        &'b self,
        db: &'b SessionState,
        _payment_id: &api::PaymentIdType,
        mut payment_data: PaymentData<F>,
        router_data: types::RouterData<
            F,
            types::SdkPaymentsSessionUpdateData,
            types::PaymentsResponseData,
        >,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: enums::MerchantStorageScheme,
        _locale: &Option<String>,
        #[cfg(feature = "dynamic_routing")] _routable_connector: Vec<RoutableConnectorChoice>,
        #[cfg(feature = "dynamic_routing")] _business_profile: &domain::Profile,
    ) -> RouterResult<PaymentData<F>>
    where
        F: 'b + Send,
    {
        let connector = payment_data
            .payment_attempt
            .connector
            .clone()
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("connector not found")?;

        let key_manager_state = db.into();

        // For PayPal, if we call TaxJar for tax calculation, we need to call the connector again to update the order amount so that we can confirm the updated amount and order details. Therefore, we will store the required changes in the database during the post_update_tracker call.
        if payment_data.should_update_in_post_update_tracker() {
            match router_data.response.clone() {
                Ok(types::PaymentsResponseData::SessionUpdateResponse { status }) => {
                    if status == SessionUpdateStatus::Success {
                        let shipping_address = payment_data
                            .tax_data
                            .clone()
                            .map(|tax_data| tax_data.shipping_details);

                        let shipping_details = shipping_address
                            .clone()
                            .async_map(|shipping_details| {
                                create_encrypted_data(
                                    &key_manager_state,
                                    key_store,
                                    shipping_details,
                                )
                            })
                            .await
                            .transpose()
                            .change_context(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("Unable to encrypt shipping details")?;

                        let shipping_address =
                            payments_helpers::create_or_update_address_for_payment_by_request(
                                db,
                                shipping_address.as_ref(),
                                payment_data.payment_intent.shipping_address_id.as_deref(),
                                &payment_data.payment_intent.merchant_id,
                                payment_data.payment_intent.customer_id.as_ref(),
                                key_store,
                                &payment_data.payment_intent.payment_id,
                                storage_scheme,
                            )
                            .await?;

                        let payment_intent_update = hyperswitch_domain_models::payments::payment_intent::PaymentIntentUpdate::SessionResponseUpdate {
                    tax_details: payment_data.payment_intent.tax_details.clone().ok_or(errors::ApiErrorResponse::InternalServerError).attach_printable("payment_intent.tax_details not found")?,
                    shipping_address_id: shipping_address.map(|address| address.address_id),
                    updated_by: payment_data.payment_intent.updated_by.clone(),
                    shipping_details,
        };

                        let m_db = db.clone().store;
                        let payment_intent = payment_data.payment_intent.clone();
                        let key_manager_state: KeyManagerState = db.into();

                        let updated_payment_intent = m_db
                            .update_payment_intent(
                                &key_manager_state,
                                payment_intent,
                                payment_intent_update,
                                key_store,
                                storage_scheme,
                            )
                            .await
                            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

                        payment_data.payment_intent = updated_payment_intent;
                    } else {
                        router_data.response.map_err(|err| {
                            errors::ApiErrorResponse::ExternalConnectorError {
                                code: err.code,
                                message: err.message,
                                connector,
                                status_code: err.status_code,
                                reason: err.reason,
                            }
                        })?;
                    }
                }
                Err(err) => {
                    Err(errors::ApiErrorResponse::ExternalConnectorError {
                        code: err.code,
                        message: err.message,
                        connector,
                        status_code: err.status_code,
                        reason: err.reason,
                    })?;
                }
                _ => {
                    Err(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Unexpected response in session_update flow")?;
                }
            }
        }

        Ok(payment_data)
    }
}

#[cfg(feature = "v1")]
#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentData<F>, types::PaymentsPostSessionTokensData>
    for PaymentResponse
{
    async fn update_tracker<'b>(
        &'b self,
        db: &'b SessionState,
        _payment_id: &api::PaymentIdType,
        mut payment_data: PaymentData<F>,
        router_data: types::RouterData<
            F,
            types::PaymentsPostSessionTokensData,
            types::PaymentsResponseData,
        >,
        _key_store: &domain::MerchantKeyStore,
        storage_scheme: enums::MerchantStorageScheme,
        _locale: &Option<String>,
        #[cfg(all(feature = "v1", feature = "dynamic_routing"))] _routable_connector: Vec<
            RoutableConnectorChoice,
        >,
        #[cfg(all(feature = "v1", feature = "dynamic_routing"))]
        _business_profile: &domain::Profile,
    ) -> RouterResult<PaymentData<F>>
    where
        F: 'b + Send,
    {
        match router_data.response.clone() {
            Ok(types::PaymentsResponseData::TransactionResponse {
                connector_metadata, ..
            }) => {
                let m_db = db.clone().store;
                let payment_attempt_update =
                    storage::PaymentAttemptUpdate::PostSessionTokensUpdate {
                        updated_by: storage_scheme.clone().to_string(),
                        connector_metadata,
                    };
                let updated_payment_attempt = m_db
                    .update_payment_attempt_with_attempt_id(
                        payment_data.payment_attempt.clone(),
                        payment_attempt_update,
                        storage_scheme,
                    )
                    .await
                    .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
                payment_data.payment_attempt = updated_payment_attempt;
            }
            Err(err) => {
                logger::error!("Invalid request sent to connector: {:?}", err);
                Err(errors::ApiErrorResponse::InvalidRequestData {
                    message: "Invalid request sent to connector".to_string(),
                })?;
            }
            _ => {
                Err(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Unexpected response in PostSessionTokens flow")?;
            }
        }
        Ok(payment_data)
    }
}

#[cfg(feature = "v1")]
#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentData<F>, types::PaymentsCaptureData>
    for PaymentResponse
{
    async fn update_tracker<'b>(
        &'b self,
        db: &'b SessionState,
        payment_id: &api::PaymentIdType,
        mut payment_data: PaymentData<F>,
        router_data: types::RouterData<F, types::PaymentsCaptureData, types::PaymentsResponseData>,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: enums::MerchantStorageScheme,
        locale: &Option<String>,
        #[cfg(all(feature = "v1", feature = "dynamic_routing"))] routable_connector: Vec<
            RoutableConnectorChoice,
        >,
        #[cfg(all(feature = "v1", feature = "dynamic_routing"))] business_profile: &domain::Profile,
    ) -> RouterResult<PaymentData<F>>
    where
        F: 'b + Send,
    {
        payment_data = Box::pin(payment_response_update_tracker(
            db,
            payment_id,
            payment_data,
            router_data,
            key_store,
            storage_scheme,
            locale,
            #[cfg(all(feature = "v1", feature = "dynamic_routing"))]
            routable_connector,
            #[cfg(all(feature = "v1", feature = "dynamic_routing"))]
            business_profile,
        ))
        .await?;

        Ok(payment_data)
    }
}

#[cfg(feature = "v1")]
#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentData<F>, types::PaymentsCancelData> for PaymentResponse {
    async fn update_tracker<'b>(
        &'b self,
        db: &'b SessionState,
        payment_id: &api::PaymentIdType,
        mut payment_data: PaymentData<F>,
        router_data: types::RouterData<F, types::PaymentsCancelData, types::PaymentsResponseData>,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: enums::MerchantStorageScheme,
        locale: &Option<String>,
        #[cfg(all(feature = "v1", feature = "dynamic_routing"))] routable_connector: Vec<
            RoutableConnectorChoice,
        >,
        #[cfg(all(feature = "v1", feature = "dynamic_routing"))] business_profile: &domain::Profile,
    ) -> RouterResult<PaymentData<F>>
    where
        F: 'b + Send,
    {
        payment_data = Box::pin(payment_response_update_tracker(
            db,
            payment_id,
            payment_data,
            router_data,
            key_store,
            storage_scheme,
            locale,
            #[cfg(all(feature = "v1", feature = "dynamic_routing"))]
            routable_connector,
            #[cfg(all(feature = "v1", feature = "dynamic_routing"))]
            business_profile,
        ))
        .await?;

        Ok(payment_data)
    }
}

#[cfg(feature = "v1")]
#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentData<F>, types::PaymentsApproveData>
    for PaymentResponse
{
    async fn update_tracker<'b>(
        &'b self,
        db: &'b SessionState,
        payment_id: &api::PaymentIdType,
        mut payment_data: PaymentData<F>,
        router_data: types::RouterData<F, types::PaymentsApproveData, types::PaymentsResponseData>,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: enums::MerchantStorageScheme,
        locale: &Option<String>,
        #[cfg(all(feature = "v1", feature = "dynamic_routing"))] routable_connector: Vec<
            RoutableConnectorChoice,
        >,
        #[cfg(all(feature = "v1", feature = "dynamic_routing"))] business_profile: &domain::Profile,
    ) -> RouterResult<PaymentData<F>>
    where
        F: 'b + Send,
    {
        payment_data = Box::pin(payment_response_update_tracker(
            db,
            payment_id,
            payment_data,
            router_data,
            key_store,
            storage_scheme,
            locale,
            #[cfg(all(feature = "v1", feature = "dynamic_routing"))]
            routable_connector,
            #[cfg(all(feature = "v1", feature = "dynamic_routing"))]
            business_profile,
        ))
        .await?;

        Ok(payment_data)
    }
}

#[cfg(feature = "v1")]
#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentData<F>, types::PaymentsRejectData> for PaymentResponse {
    async fn update_tracker<'b>(
        &'b self,
        db: &'b SessionState,
        payment_id: &api::PaymentIdType,
        mut payment_data: PaymentData<F>,
        router_data: types::RouterData<F, types::PaymentsRejectData, types::PaymentsResponseData>,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: enums::MerchantStorageScheme,
        locale: &Option<String>,
        #[cfg(all(feature = "v1", feature = "dynamic_routing"))] routable_connector: Vec<
            RoutableConnectorChoice,
        >,
        #[cfg(all(feature = "v1", feature = "dynamic_routing"))] business_profile: &domain::Profile,
    ) -> RouterResult<PaymentData<F>>
    where
        F: 'b + Send,
    {
        payment_data = Box::pin(payment_response_update_tracker(
            db,
            payment_id,
            payment_data,
            router_data,
            key_store,
            storage_scheme,
            locale,
            #[cfg(all(feature = "v1", feature = "dynamic_routing"))]
            routable_connector,
            #[cfg(all(feature = "v1", feature = "dynamic_routing"))]
            business_profile,
        ))
        .await?;

        Ok(payment_data)
    }
}

#[cfg(feature = "v1")]
#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentData<F>, types::SetupMandateRequestData>
    for PaymentResponse
{
    async fn update_tracker<'b>(
        &'b self,
        db: &'b SessionState,
        payment_id: &api::PaymentIdType,
        mut payment_data: PaymentData<F>,
        router_data: types::RouterData<
            F,
            types::SetupMandateRequestData,
            types::PaymentsResponseData,
        >,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: enums::MerchantStorageScheme,
        locale: &Option<String>,
        #[cfg(all(feature = "v1", feature = "dynamic_routing"))] routable_connector: Vec<
            RoutableConnectorChoice,
        >,
        #[cfg(all(feature = "v1", feature = "dynamic_routing"))] business_profile: &domain::Profile,
    ) -> RouterResult<PaymentData<F>>
    where
        F: 'b + Send,
    {
        payment_data.mandate_id = payment_data.mandate_id.or_else(|| {
            router_data.request.mandate_id.clone()
            // .map(api_models::payments::MandateIds::new)
        });

        payment_data = Box::pin(payment_response_update_tracker(
            db,
            payment_id,
            payment_data,
            router_data,
            key_store,
            storage_scheme,
            locale,
            #[cfg(all(feature = "v1", feature = "dynamic_routing"))]
            routable_connector,
            #[cfg(all(feature = "v1", feature = "dynamic_routing"))]
            business_profile,
        ))
        .await?;

        Ok(payment_data)
    }

    async fn save_pm_and_mandate<'b>(
        &self,
        state: &SessionState,
        resp: &types::RouterData<F, types::SetupMandateRequestData, types::PaymentsResponseData>,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        payment_data: &mut PaymentData<F>,
        business_profile: &domain::Profile,
    ) -> CustomResult<(), errors::ApiErrorResponse>
    where
        F: 'b + Clone + Send + Sync,
    {
        let billing_name = resp
            .address
            .get_payment_method_billing()
            .and_then(|billing_details| billing_details.address.as_ref())
            .and_then(|address| address.get_optional_full_name());

        let save_payment_data = tokenization::SavePaymentMethodData::from(resp);
        let customer_id = payment_data.payment_intent.customer_id.clone();
        let connector_name = payment_data
            .payment_attempt
            .connector
            .clone()
            .ok_or_else(|| {
                logger::error!("Missing required Param connector_name");
                errors::ApiErrorResponse::MissingRequiredField {
                    field_name: "connector_name",
                }
            })?;
        let merchant_connector_id = payment_data.payment_attempt.merchant_connector_id.clone();
        let tokenization::SavePaymentMethodDataResponse {
            payment_method_id,
            connector_mandate_reference_id,
            ..
        } = Box::pin(tokenization::save_payment_method(
            state,
            connector_name,
            save_payment_data,
            customer_id.clone(),
            merchant_account,
            resp.request.payment_method_type,
            key_store,
            billing_name,
            None,
            business_profile,
        ))
        .await?;

        let mandate_id = mandate::mandate_procedure(
            state,
            resp,
            &customer_id,
            payment_method_id.clone(),
            merchant_connector_id.clone(),
            merchant_account.storage_scheme,
            payment_data.payment_intent.get_id(),
        )
        .await?;
        payment_data.payment_attempt.payment_method_id = payment_method_id;
        payment_data.payment_attempt.mandate_id = mandate_id;
        payment_data.payment_attempt.connector_mandate_detail = connector_mandate_reference_id
            .clone()
            .map(ForeignFrom::foreign_from);
        payment_data.set_mandate_id(api_models::payments::MandateIds {
            mandate_id: None,
            mandate_reference_id: connector_mandate_reference_id.map(|connector_mandate_id| {
                MandateReferenceId::ConnectorMandateId(connector_mandate_id)
            }),
        });
        Ok(())
    }
}

#[cfg(feature = "v1")]
#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentData<F>, types::CompleteAuthorizeData>
    for PaymentResponse
{
    async fn update_tracker<'b>(
        &'b self,
        db: &'b SessionState,
        payment_id: &api::PaymentIdType,
        payment_data: PaymentData<F>,
        response: types::RouterData<F, types::CompleteAuthorizeData, types::PaymentsResponseData>,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: enums::MerchantStorageScheme,
        locale: &Option<String>,
        #[cfg(all(feature = "v1", feature = "dynamic_routing"))] routable_connector: Vec<
            RoutableConnectorChoice,
        >,
        #[cfg(all(feature = "v1", feature = "dynamic_routing"))] business_profile: &domain::Profile,
    ) -> RouterResult<PaymentData<F>>
    where
        F: 'b + Send,
    {
        Box::pin(payment_response_update_tracker(
            db,
            payment_id,
            payment_data,
            response,
            key_store,
            storage_scheme,
            locale,
            #[cfg(all(feature = "v1", feature = "dynamic_routing"))]
            routable_connector,
            #[cfg(all(feature = "v1", feature = "dynamic_routing"))]
            business_profile,
        ))
        .await
    }

    async fn save_pm_and_mandate<'b>(
        &self,
        state: &SessionState,
        resp: &types::RouterData<F, types::CompleteAuthorizeData, types::PaymentsResponseData>,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        payment_data: &mut PaymentData<F>,
        business_profile: &domain::Profile,
    ) -> CustomResult<(), errors::ApiErrorResponse>
    where
        F: 'b + Clone + Send + Sync,
    {
        let (connector_mandate_id, mandate_metadata) = resp
            .response
            .clone()
            .ok()
            .and_then(|resp| {
                if let types::PaymentsResponseData::TransactionResponse {
                    mandate_reference, ..
                } = resp
                {
                    mandate_reference.map(|mandate_ref| {
                        (
                            mandate_ref.connector_mandate_id.clone(),
                            mandate_ref.mandate_metadata.clone(),
                        )
                    })
                } else {
                    None
                }
            })
            .unwrap_or((None, None));
        update_connector_mandate_details_for_the_flow(
            connector_mandate_id,
            mandate_metadata,
            payment_data,
        )?;

        update_payment_method_status_and_ntid(
            state,
            key_store,
            payment_data,
            resp.status,
            resp.response.clone(),
            merchant_account.storage_scheme,
            business_profile.is_connector_agnostic_mit_enabled,
        )
        .await?;
        Ok(())
    }
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
async fn payment_response_update_tracker<F: Clone, T: types::Capturable>(
    state: &SessionState,
    _payment_id: &api::PaymentIdType,
    mut payment_data: PaymentData<F>,
    router_data: types::RouterData<F, T, types::PaymentsResponseData>,
    key_store: &domain::MerchantKeyStore,
    storage_scheme: enums::MerchantStorageScheme,
    locale: &Option<String>,
) -> RouterResult<PaymentData<F>> {
    todo!()
}

#[cfg(feature = "v1")]
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
async fn payment_response_update_tracker<F: Clone, T: types::Capturable>(
    state: &SessionState,
    _payment_id: &api::PaymentIdType,
    mut payment_data: PaymentData<F>,
    router_data: types::RouterData<F, T, types::PaymentsResponseData>,
    key_store: &domain::MerchantKeyStore,
    storage_scheme: enums::MerchantStorageScheme,
    locale: &Option<String>,
    #[cfg(all(feature = "v1", feature = "dynamic_routing"))] routable_connectors: Vec<
        RoutableConnectorChoice,
    >,
    #[cfg(all(feature = "v1", feature = "dynamic_routing"))] business_profile: &domain::Profile,
) -> RouterResult<PaymentData<F>> {
    // Update additional payment data with the payment method response that we received from connector

    let additional_payment_method_data = match payment_data.payment_method_data.clone() {
        Some(payment_method_data) => match payment_method_data {
            hyperswitch_domain_models::payment_method_data::PaymentMethodData::Card(_)
            | hyperswitch_domain_models::payment_method_data::PaymentMethodData::CardRedirect(_)
            | hyperswitch_domain_models::payment_method_data::PaymentMethodData::Wallet(_)
            | hyperswitch_domain_models::payment_method_data::PaymentMethodData::PayLater(_)
            | hyperswitch_domain_models::payment_method_data::PaymentMethodData::BankRedirect(_)
            | hyperswitch_domain_models::payment_method_data::PaymentMethodData::BankDebit(_)
            | hyperswitch_domain_models::payment_method_data::PaymentMethodData::BankTransfer(_)
            | hyperswitch_domain_models::payment_method_data::PaymentMethodData::Crypto(_)
            | hyperswitch_domain_models::payment_method_data::PaymentMethodData::MandatePayment
            | hyperswitch_domain_models::payment_method_data::PaymentMethodData::Reward
            | hyperswitch_domain_models::payment_method_data::PaymentMethodData::RealTimePayment(
                _,
            )
            | hyperswitch_domain_models::payment_method_data::PaymentMethodData::Upi(_)
            | hyperswitch_domain_models::payment_method_data::PaymentMethodData::Voucher(_)
            | hyperswitch_domain_models::payment_method_data::PaymentMethodData::GiftCard(_)
            | hyperswitch_domain_models::payment_method_data::PaymentMethodData::CardToken(_)
            | hyperswitch_domain_models::payment_method_data::PaymentMethodData::OpenBanking(_) => {
                update_additional_payment_data_with_connector_response_pm_data(
                    payment_data.payment_attempt.payment_method_data.clone(),
                    router_data
                        .connector_response
                        .as_ref()
                        .and_then(|connector_response| {
                            connector_response.additional_payment_method_data.clone()
                        }),
                )?
            }
            hyperswitch_domain_models::payment_method_data::PaymentMethodData::NetworkToken(_) => {
                payment_data.payment_attempt.payment_method_data.clone()
            }
            hyperswitch_domain_models::payment_method_data::PaymentMethodData::CardDetailsForNetworkTransactionId(_) => {
                payment_data.payment_attempt.payment_method_data.clone()
            }
        },
        None => None,
    };

    router_data.payment_method_status.and_then(|status| {
        payment_data
            .payment_method_info
            .as_mut()
            .map(|info| info.status = status)
    });
    let (capture_update, mut payment_attempt_update) = match router_data.response.clone() {
        Err(err) => {
            let auth_update = if Some(router_data.auth_type)
                != payment_data.payment_attempt.authentication_type
            {
                Some(router_data.auth_type)
            } else {
                None
            };
            let (capture_update, attempt_update) = match payment_data.multiple_capture_data {
                Some(multiple_capture_data) => {
                    let capture_update = storage::CaptureUpdate::ErrorUpdate {
                        status: match err.status_code {
                            500..=511 => enums::CaptureStatus::Pending,
                            _ => enums::CaptureStatus::Failed,
                        },
                        error_code: Some(err.code),
                        error_message: Some(err.message),
                        error_reason: err.reason,
                    };
                    let capture_update_list = vec![(
                        multiple_capture_data.get_latest_capture().clone(),
                        capture_update,
                    )];
                    (
                        Some((multiple_capture_data, capture_update_list)),
                        auth_update.map(|auth_type| {
                            storage::PaymentAttemptUpdate::AuthenticationTypeUpdate {
                                authentication_type: auth_type,
                                updated_by: storage_scheme.to_string(),
                            }
                        }),
                    )
                }
                None => {
                    let connector_name = router_data.connector.to_string();
                    let flow_name = core_utils::get_flow_name::<F>()?;
                    let option_gsm = payments_helpers::get_gsm_record(
                        state,
                        Some(err.code.clone()),
                        Some(err.message.clone()),
                        connector_name,
                        flow_name.clone(),
                    )
                    .await;

                    let gsm_unified_code =
                        option_gsm.as_ref().and_then(|gsm| gsm.unified_code.clone());
                    let gsm_unified_message = option_gsm.and_then(|gsm| gsm.unified_message);

                    let (unified_code, unified_message) = if let Some((code, message)) =
                        gsm_unified_code.as_ref().zip(gsm_unified_message.as_ref())
                    {
                        (code.to_owned(), message.to_owned())
                    } else {
                        (
                            consts::DEFAULT_UNIFIED_ERROR_CODE.to_owned(),
                            consts::DEFAULT_UNIFIED_ERROR_MESSAGE.to_owned(),
                        )
                    };
                    let unified_translated_message = locale
                        .as_ref()
                        .async_and_then(|locale_str| async {
                            payments_helpers::get_unified_translation(
                                state,
                                unified_code.to_owned(),
                                unified_message.to_owned(),
                                locale_str.to_owned(),
                            )
                            .await
                        })
                        .await
                        .or(Some(unified_message));

                    let status = match err.attempt_status {
                        // Use the status sent by connector in error_response if it's present
                        Some(status) => status,
                        None =>
                        // mark previous attempt status for technical failures in PSync flow
                        {
                            if flow_name == "PSync" {
                                match err.status_code {
                                    // marking failure for 2xx because this is genuine payment failure
                                    200..=299 => enums::AttemptStatus::Failure,
                                    _ => router_data.status,
                                }
                            } else if flow_name == "Capture" {
                                match err.status_code {
                                    500..=511 => enums::AttemptStatus::Pending,
                                    // don't update the status for 429 error status
                                    429 => router_data.status,
                                    _ => enums::AttemptStatus::Failure,
                                }
                            } else {
                                match err.status_code {
                                    500..=511 => enums::AttemptStatus::Pending,
                                    _ => enums::AttemptStatus::Failure,
                                }
                            }
                        }
                    };
                    (
                        None,
                        Some(storage::PaymentAttemptUpdate::ErrorUpdate {
                            connector: None,
                            status,
                            error_message: Some(Some(err.message)),
                            error_code: Some(Some(err.code)),
                            error_reason: Some(err.reason),
                            amount_capturable: router_data
                                .request
                                .get_amount_capturable(&payment_data, status)
                                .map(MinorUnit::new),
                            updated_by: storage_scheme.to_string(),
                            unified_code: Some(Some(unified_code)),
                            unified_message: Some(unified_translated_message),
                            connector_transaction_id: err.connector_transaction_id,
                            payment_method_data: additional_payment_method_data,
                            authentication_type: auth_update,
                        }),
                    )
                }
            };
            (capture_update, attempt_update)
        }

        Ok(payments_response) => {
            // match on connector integrity check
            match router_data.integrity_check.clone() {
                Err(err) => {
                    let auth_update = if Some(router_data.auth_type)
                        != payment_data.payment_attempt.authentication_type
                    {
                        Some(router_data.auth_type)
                    } else {
                        None
                    };
                    let field_name = err.field_names;
                    let connector_transaction_id = err.connector_transaction_id;
                    (
                        None,
                        Some(storage::PaymentAttemptUpdate::ErrorUpdate {
                            connector: None,
                            status: enums::AttemptStatus::Pending,
                            error_message: Some(Some("Integrity Check Failed!".to_string())),
                            error_code: Some(Some("IE".to_string())),
                            error_reason: Some(Some(format!(
                                "Integrity Check Failed! Value mismatched for fields {field_name}"
                            ))),
                            amount_capturable: None,
                            updated_by: storage_scheme.to_string(),
                            unified_code: None,
                            unified_message: None,
                            connector_transaction_id,
                            payment_method_data: None,
                            authentication_type: auth_update,
                        }),
                    )
                }
                Ok(()) => {
                    let attempt_status = payment_data.payment_attempt.status.to_owned();
                    let connector_status = router_data.status.to_owned();
                    let updated_attempt_status = match (
                        connector_status,
                        attempt_status,
                        payment_data.frm_message.to_owned(),
                    ) {
                        (
                            enums::AttemptStatus::Authorized,
                            enums::AttemptStatus::Unresolved,
                            Some(frm_message),
                        ) => match frm_message.frm_status {
                            enums::FraudCheckStatus::Fraud
                            | enums::FraudCheckStatus::ManualReview => attempt_status,
                            _ => router_data.get_attempt_status_for_db_update(&payment_data),
                        },
                        _ => router_data.get_attempt_status_for_db_update(&payment_data),
                    };
                    match payments_response {
                        types::PaymentsResponseData::PreProcessingResponse {
                            pre_processing_id,
                            connector_metadata,
                            connector_response_reference_id,
                            ..
                        } => {
                            let connector_transaction_id = match pre_processing_id.to_owned() {
                                types::PreprocessingResponseId::PreProcessingId(_) => None,

                                types::PreprocessingResponseId::ConnectorTransactionId(
                                    connector_txn_id,
                                ) => Some(connector_txn_id),
                            };
                            let preprocessing_step_id = match pre_processing_id {
                                types::PreprocessingResponseId::PreProcessingId(
                                    pre_processing_id,
                                ) => Some(pre_processing_id),
                                types::PreprocessingResponseId::ConnectorTransactionId(_) => None,
                            };
                            let payment_attempt_update =
                                storage::PaymentAttemptUpdate::PreprocessingUpdate {
                                    status: updated_attempt_status,
                                    payment_method_id: payment_data
                                        .payment_attempt
                                        .payment_method_id
                                        .clone(),
                                    connector_metadata,
                                    preprocessing_step_id,
                                    connector_transaction_id,
                                    connector_response_reference_id,
                                    updated_by: storage_scheme.to_string(),
                                };

                            (None, Some(payment_attempt_update))
                        }
                        types::PaymentsResponseData::TransactionResponse {
                            resource_id,
                            redirection_data,
                            connector_metadata,
                            connector_response_reference_id,
                            incremental_authorization_allowed,
                            charge_id,
                            ..
                        } => {
                            payment_data
                                .payment_intent
                                .incremental_authorization_allowed =
                                core_utils::get_incremental_authorization_allowed_value(
                                    incremental_authorization_allowed,
                                    payment_data
                                        .payment_intent
                                        .request_incremental_authorization,
                                );
                            let connector_transaction_id = match resource_id {
                                types::ResponseId::NoResponseId => None,
                                types::ResponseId::ConnectorTransactionId(ref id)
                                | types::ResponseId::EncodedData(ref id) => Some(id),
                            };

                            let encoded_data = payment_data.payment_attempt.encoded_data.clone();

                            let authentication_data = redirection_data
                                .as_ref()
                                .map(Encode::encode_to_value)
                                .transpose()
                                .change_context(errors::ApiErrorResponse::InternalServerError)
                                .attach_printable("Could not parse the connector response")?;

                            let auth_update = if Some(router_data.auth_type)
                                != payment_data.payment_attempt.authentication_type
                            {
                                Some(router_data.auth_type)
                            } else {
                                None
                            };

                            // incase of success, update error code and error message
                            let error_status =
                                if router_data.status == enums::AttemptStatus::Charged {
                                    Some(None)
                                } else {
                                    None
                                };
                            // update connector_mandate_details in case of Authorized/Charged Payment Status
                            if matches!(
                                router_data.status,
                                enums::AttemptStatus::Charged | enums::AttemptStatus::Authorized
                            ) {
                                payment_data
                                    .payment_intent
                                    .fingerprint_id
                                    .clone_from(&payment_data.payment_attempt.fingerprint_id);

                                if let Some(payment_method) =
                                    payment_data.payment_method_info.clone()
                                {
                                    // Parse value to check for mandates' existence
                                    let mandate_details = payment_method
                                        .connector_mandate_details
                                        .clone()
                                        .map(|val| {
                                            val.parse_value::<storage::PaymentsMandateReference>(
                                                "PaymentsMandateReference",
                                            )
                                        })
                                        .transpose()
                                        .change_context(
                                            errors::ApiErrorResponse::InternalServerError,
                                        )
                                        .attach_printable(
                                            "Failed to deserialize to Payment Mandate Reference ",
                                        )?;

                                    if let Some(mca_id) =
                                        payment_data.payment_attempt.merchant_connector_id.clone()
                                    {
                                        // check if the mandate has not already been set to active
                                        if !mandate_details
                                        .as_ref()
                                        .map(|payment_mandate_reference| {

                                                payment_mandate_reference.0.get(&mca_id)
                                                    .map(|payment_mandate_reference_record| payment_mandate_reference_record.connector_mandate_status == Some(common_enums::ConnectorMandateStatus::Active))
                                                    .unwrap_or(false)
                                        })
                                        .unwrap_or(false)
                                    {
                                        let (connector_mandate_id, mandate_metadata) = payment_data.payment_attempt.connector_mandate_detail.clone()
                                        .map(|cmr| (cmr.connector_mandate_id, cmr.mandate_metadata))
                                        .unwrap_or((None, None));

                                        // Update the connector mandate details with the payment attempt connector mandate id
                                        let connector_mandate_details =
                                                    tokenization::update_connector_mandate_details(
                                                        mandate_details,
                                                        payment_data.payment_attempt.payment_method_type,
                                                        Some(
                                                            payment_data
                                                                .payment_attempt
                                                                .net_amount
                                                                .get_total_amount()
                                                                .get_amount_as_i64(),
                                                        ),
                                                        payment_data.payment_attempt.currency,
                                                        payment_data.payment_attempt.merchant_connector_id.clone(),
                                                        connector_mandate_id,
                                                        mandate_metadata,
                                                    )?;
                                        // Update the payment method table with the active mandate record
                                        payment_methods::cards::update_payment_method_connector_mandate_details(
                                                        state,
                                                        key_store,
                                                        &*state.store,
                                                        payment_method,
                                                        connector_mandate_details,
                                                        storage_scheme,
                                                    )
                                                    .await
                                                    .change_context(errors::ApiErrorResponse::InternalServerError)
                                                    .attach_printable("Failed to update payment method in db")?;
                                    }
                                    }
                                }

                                metrics::SUCCESSFUL_PAYMENT.add(&metrics::CONTEXT, 1, &[]);
                            }

                            let payment_method_id =
                                payment_data.payment_attempt.payment_method_id.clone();

                            utils::add_apple_pay_payment_status_metrics(
                                router_data.status,
                                router_data.apple_pay_flow.clone(),
                                payment_data.payment_attempt.connector.clone(),
                                payment_data.payment_attempt.merchant_id.clone(),
                            );
                            let (capture_updates, payment_attempt_update) = match payment_data
                                .multiple_capture_data
                            {
                                Some(multiple_capture_data) => {
                                    let (connector_capture_id, connector_capture_data) =
                                        match resource_id {
                                            types::ResponseId::NoResponseId => (None, None),
                                            types::ResponseId::ConnectorTransactionId(id)
                                            | types::ResponseId::EncodedData(id) => {
                                                let (txn_id, txn_data) =
                                                    ConnectorTransactionId::form_id_and_data(id);
                                                (Some(txn_id), txn_data)
                                            }
                                        };
                                    let capture_update = storage::CaptureUpdate::ResponseUpdate {
                                        status: enums::CaptureStatus::foreign_try_from(
                                            router_data.status,
                                        )?,
                                        connector_capture_id: connector_capture_id.clone(),
                                        connector_response_reference_id,
                                        connector_capture_data: connector_capture_data.clone(),
                                    };
                                    let capture_update_list = vec![(
                                        multiple_capture_data.get_latest_capture().clone(),
                                        capture_update,
                                    )];
                                    (Some((multiple_capture_data, capture_update_list)), auth_update.map(|auth_type| {
                                        storage::PaymentAttemptUpdate::AuthenticationTypeUpdate {
                                            authentication_type: auth_type,
                                            updated_by: storage_scheme.to_string(),
                                        }
                                    }))
                                }
                                None => (
                                    None,
                                    Some(storage::PaymentAttemptUpdate::ResponseUpdate {
                                        status: updated_attempt_status,
                                        connector: None,
                                        connector_transaction_id: connector_transaction_id.cloned(),
                                        authentication_type: auth_update,
                                        amount_capturable: router_data
                                            .request
                                            .get_amount_capturable(
                                                &payment_data,
                                                updated_attempt_status,
                                            )
                                            .map(MinorUnit::new),
                                        payment_method_id,
                                        mandate_id: payment_data.payment_attempt.mandate_id.clone(),
                                        connector_metadata,
                                        payment_token: None,
                                        error_code: error_status.clone(),
                                        error_message: error_status.clone(),
                                        error_reason: error_status.clone(),
                                        unified_code: error_status.clone(),
                                        unified_message: error_status,
                                        connector_response_reference_id,
                                        updated_by: storage_scheme.to_string(),
                                        authentication_data,
                                        encoded_data,
                                        payment_method_data: additional_payment_method_data,
                                        charge_id,
                                        connector_mandate_detail: payment_data
                                            .payment_attempt
                                            .connector_mandate_detail
                                            .clone(),
                                    }),
                                ),
                            };

                            (capture_updates, payment_attempt_update)
                        }
                        types::PaymentsResponseData::TransactionUnresolvedResponse {
                            resource_id,
                            reason,
                            connector_response_reference_id,
                        } => {
                            let connector_transaction_id = match resource_id {
                                types::ResponseId::NoResponseId => None,
                                types::ResponseId::ConnectorTransactionId(id)
                                | types::ResponseId::EncodedData(id) => Some(id),
                            };
                            (
                                None,
                                Some(storage::PaymentAttemptUpdate::UnresolvedResponseUpdate {
                                    status: updated_attempt_status,
                                    connector: None,
                                    connector_transaction_id,
                                    payment_method_id: payment_data
                                        .payment_attempt
                                        .payment_method_id
                                        .clone(),
                                    error_code: Some(reason.clone().map(|cd| cd.code)),
                                    error_message: Some(reason.clone().map(|cd| cd.message)),
                                    error_reason: Some(reason.map(|cd| cd.message)),
                                    connector_response_reference_id,
                                    updated_by: storage_scheme.to_string(),
                                }),
                            )
                        }
                        types::PaymentsResponseData::SessionResponse { .. } => (None, None),
                        types::PaymentsResponseData::SessionTokenResponse { .. } => (None, None),
                        types::PaymentsResponseData::TokenizationResponse { .. } => (None, None),
                        types::PaymentsResponseData::ConnectorCustomerResponse { .. } => {
                            (None, None)
                        }
                        types::PaymentsResponseData::ThreeDSEnrollmentResponse { .. } => {
                            (None, None)
                        }
                        types::PaymentsResponseData::PostProcessingResponse { .. } => (None, None),
                        types::PaymentsResponseData::IncrementalAuthorizationResponse {
                            ..
                        } => (None, None),
                        types::PaymentsResponseData::SessionUpdateResponse { .. } => (None, None),
                        types::PaymentsResponseData::MultipleCaptureResponse {
                            capture_sync_response_list,
                        } => match payment_data.multiple_capture_data {
                            Some(multiple_capture_data) => {
                                let capture_update_list = response_to_capture_update(
                                    &multiple_capture_data,
                                    capture_sync_response_list,
                                )?;
                                (Some((multiple_capture_data, capture_update_list)), None)
                            }
                            None => (None, None),
                        },
                    }
                }
            }
        }
    };
    payment_data.multiple_capture_data = match capture_update {
        Some((mut multiple_capture_data, capture_updates)) => {
            for (capture, capture_update) in capture_updates {
                let updated_capture = state
                    .store
                    .update_capture_with_capture_id(capture, capture_update, storage_scheme)
                    .await
                    .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
                multiple_capture_data.update_capture(updated_capture);
            }

            let authorized_amount = payment_data.payment_attempt.get_total_amount();

            payment_attempt_update = Some(storage::PaymentAttemptUpdate::AmountToCaptureUpdate {
                status: multiple_capture_data.get_attempt_status(authorized_amount),
                amount_capturable: authorized_amount
                    - multiple_capture_data.get_total_blocked_amount(),
                updated_by: storage_scheme.to_string(),
            });
            Some(multiple_capture_data)
        }
        None => None,
    };

    // Stage 1

    let payment_attempt = payment_data.payment_attempt.clone();

    let m_db = state.clone().store;
    let m_payment_attempt_update = payment_attempt_update.clone();
    let m_payment_attempt = payment_attempt.clone();

    let payment_attempt = payment_attempt_update
        .map(|payment_attempt_update| {
            PaymentAttempt::from_storage_model(
                payment_attempt_update
                    .to_storage_model()
                    .apply_changeset(payment_attempt.clone().to_storage_model()),
            )
        })
        .unwrap_or_else(|| payment_attempt);

    let payment_attempt_fut = tokio::spawn(
        async move {
            Box::pin(async move {
                Ok::<_, error_stack::Report<errors::ApiErrorResponse>>(
                    match m_payment_attempt_update {
                        Some(payment_attempt_update) => m_db
                            .update_payment_attempt_with_attempt_id(
                                m_payment_attempt,
                                payment_attempt_update,
                                storage_scheme,
                            )
                            .await
                            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?,
                        None => m_payment_attempt,
                    },
                )
            })
            .await
        }
        .in_current_span(),
    );

    payment_data.payment_attempt = payment_attempt;

    payment_data.authentication = match payment_data.authentication {
        Some(authentication) => {
            let authentication_update = storage::AuthenticationUpdate::PostAuthorizationUpdate {
                authentication_lifecycle_status: enums::AuthenticationLifecycleStatus::Used,
            };
            let updated_authentication = state
                .store
                .update_authentication_by_merchant_id_authentication_id(
                    authentication,
                    authentication_update,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
            Some(updated_authentication)
        }
        None => None,
    };

    let amount_captured = get_total_amount_captured(
        &router_data.request,
        router_data.amount_captured.map(MinorUnit::new),
        router_data.status,
        &payment_data,
    );

    let payment_intent_update = match &router_data.response {
        Err(_) => storage::PaymentIntentUpdate::PGStatusUpdate {
            status: api_models::enums::IntentStatus::foreign_from(
                payment_data.payment_attempt.status,
            ),
            updated_by: storage_scheme.to_string(),
            // make this false only if initial payment fails, if incremental authorization call fails don't make it false
            incremental_authorization_allowed: Some(false),
        },
        Ok(_) => storage::PaymentIntentUpdate::ResponseUpdate {
            status: api_models::enums::IntentStatus::foreign_from(
                payment_data.payment_attempt.status,
            ),
            return_url: router_data.return_url.clone(),
            amount_captured,
            updated_by: storage_scheme.to_string(),
            fingerprint_id: payment_data.payment_attempt.fingerprint_id.clone(),
            incremental_authorization_allowed: payment_data
                .payment_intent
                .incremental_authorization_allowed,
        },
    };

    let m_db = state.clone().store;
    let m_key_store = key_store.clone();
    let m_payment_data_payment_intent = payment_data.payment_intent.clone();
    let m_payment_intent_update = payment_intent_update.clone();
    let key_manager_state: KeyManagerState = state.into();
    let payment_intent_fut = tokio::spawn(
        async move {
            m_db.update_payment_intent(
                &key_manager_state,
                m_payment_data_payment_intent,
                m_payment_intent_update,
                &m_key_store,
                storage_scheme,
            )
            .map(|x| x.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound))
            .await
        }
        .in_current_span(),
    );

    // When connector requires redirection for mandate creation it can update the connector mandate_id during Psync and CompleteAuthorize
    let m_db = state.clone().store;
    let m_router_data_merchant_id = router_data.merchant_id.clone();
    let m_payment_method_id = payment_data.payment_attempt.payment_method_id.clone();
    let m_payment_data_mandate_id =
        payment_data
            .payment_attempt
            .mandate_id
            .clone()
            .or(payment_data
                .mandate_id
                .clone()
                .and_then(|mandate_ids| mandate_ids.mandate_id));
    let m_router_data_response = router_data.response.clone();
    let mandate_update_fut = tokio::spawn(
        async move {
            mandate::update_connector_mandate_id(
                m_db.as_ref(),
                &m_router_data_merchant_id,
                m_payment_data_mandate_id,
                m_payment_method_id,
                m_router_data_response,
                storage_scheme,
            )
            .await
        }
        .in_current_span(),
    );

    let (payment_intent, _, payment_attempt) = futures::try_join!(
        utils::flatten_join_error(payment_intent_fut),
        utils::flatten_join_error(mandate_update_fut),
        utils::flatten_join_error(payment_attempt_fut)
    )?;

    #[cfg(all(feature = "v1", feature = "dynamic_routing"))]
    {
        if let Some(dynamic_routing_algorithm) = business_profile.dynamic_routing_algorithm.clone()
        {
            let state = state.clone();
            let business_profile = business_profile.clone();
            let payment_attempt = payment_attempt.clone();
            tokio::spawn(
                async move {
                    push_metrics_for_success_based_routing(
                        &state,
                        &payment_attempt,
                        routable_connectors,
                        &business_profile,
                        dynamic_routing_algorithm,
                    )
                    .await
                    .map_err(|e| logger::error!(dynamic_routing_metrics_error=?e))
                    .ok();
                }
                .in_current_span(),
            );
        }
    }
    payment_data.payment_intent = payment_intent;
    payment_data.payment_attempt = payment_attempt;
    router_data.payment_method_status.and_then(|status| {
        payment_data
            .payment_method_info
            .as_mut()
            .map(|info| info.status = status)
    });

    match router_data.integrity_check {
        Ok(()) => Ok(payment_data),
        Err(err) => {
            metrics::INTEGRITY_CHECK_FAILED.add(
                &metrics::CONTEXT,
                1,
                &add_attributes([
                    (
                        "connector",
                        payment_data
                            .payment_attempt
                            .connector
                            .clone()
                            .unwrap_or_default(),
                    ),
                    (
                        "merchant_id",
                        payment_data
                            .payment_attempt
                            .merchant_id
                            .get_string_repr()
                            .to_owned(),
                    ),
                ]),
            );
            Err(error_stack::Report::new(
                errors::ApiErrorResponse::IntegrityCheckFailed {
                    connector_transaction_id: payment_data
                        .payment_attempt
                        .get_connector_payment_id()
                        .map(ToString::to_string),
                    reason: payment_data
                        .payment_attempt
                        .error_message
                        .unwrap_or_default(),
                    field_names: err.field_names,
                },
            ))
        }
    }
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
async fn update_payment_method_status_and_ntid<F: Clone>(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    payment_data: &mut PaymentData<F>,
    attempt_status: common_enums::AttemptStatus,
    payment_response: Result<types::PaymentsResponseData, ErrorResponse>,
    storage_scheme: enums::MerchantStorageScheme,
    is_connector_agnostic_mit_enabled: Option<bool>,
) -> RouterResult<()> {
    todo!()
}

#[cfg(all(
    any(feature = "v2", feature = "v1"),
    not(feature = "payment_methods_v2")
))]
async fn update_payment_method_status_and_ntid<F: Clone>(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    payment_data: &mut PaymentData<F>,
    attempt_status: common_enums::AttemptStatus,
    payment_response: Result<types::PaymentsResponseData, ErrorResponse>,
    storage_scheme: enums::MerchantStorageScheme,
    is_connector_agnostic_mit_enabled: Option<bool>,
) -> RouterResult<()> {
    // If the payment_method is deleted then ignore the error related to retrieving payment method
    // This should be handled when the payment method is soft deleted
    if let Some(id) = &payment_data.payment_attempt.payment_method_id {
        let payment_method = match state
            .store
            .find_payment_method(&(state.into()), key_store, id, storage_scheme)
            .await
        {
            Ok(payment_method) => payment_method,
            Err(error) => {
                if error.current_context().is_db_not_found() {
                    logger::info!(
                        "Payment Method not found in db and skipping payment method update {:?}",
                        error
                    );
                    return Ok(());
                } else {
                    Err(error)
                            .change_context(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("Error retrieving payment method from db in update_payment_method_status_and_ntid")?
                }
            }
        };

        let pm_resp_network_transaction_id = payment_response
            .map(|resp| if let types::PaymentsResponseData::TransactionResponse { network_txn_id: network_transaction_id, .. } = resp {
                network_transaction_id
    } else {None})
    .map_err(|err| {
        logger::error!(error=?err, "Failed to obtain the network_transaction_id from payment response");
    })
    .ok()
    .flatten();
        let network_transaction_id =
            if let Some(network_transaction_id) = pm_resp_network_transaction_id {
                if is_connector_agnostic_mit_enabled == Some(true)
                    && payment_data.payment_intent.setup_future_usage
                        == Some(diesel_models::enums::FutureUsage::OffSession)
                {
                    Some(network_transaction_id)
                } else {
                    logger::info!("Skip storing network transaction id");
                    None
                }
            } else {
                None
            };

        let pm_update = if payment_method.status != common_enums::PaymentMethodStatus::Active
            && payment_method.status != attempt_status.into()
        {
            let updated_pm_status = common_enums::PaymentMethodStatus::from(attempt_status);
            payment_data
                .payment_method_info
                .as_mut()
                .map(|info| info.status = updated_pm_status);
            storage::PaymentMethodUpdate::NetworkTransactionIdAndStatusUpdate {
                network_transaction_id,
                status: Some(updated_pm_status),
            }
        } else {
            storage::PaymentMethodUpdate::NetworkTransactionIdAndStatusUpdate {
                network_transaction_id,
                status: None,
            }
        };

        state
            .store
            .update_payment_method(
                &(state.into()),
                key_store,
                payment_method,
                pm_update,
                storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to update payment method in db")?;
    };
    Ok(())
}

fn update_connector_mandate_details_for_the_flow<F: Clone>(
    connector_mandate_id: Option<String>,
    mandate_metadata: Option<serde_json::Value>,
    payment_data: &mut PaymentData<F>,
) -> RouterResult<()> {
    let connector_mandate_reference_id = if connector_mandate_id.is_some() {
        Some(ConnectorMandateReferenceId {
            connector_mandate_id: connector_mandate_id.clone(),
            payment_method_id: None,
            update_history: None,
            mandate_metadata: mandate_metadata.clone(),
        })
    } else {
        None
    };

    payment_data.payment_attempt.connector_mandate_detail = connector_mandate_reference_id
        .clone()
        .map(ForeignFrom::foreign_from);

    payment_data.set_mandate_id(api_models::payments::MandateIds {
        mandate_id: None,
        mandate_reference_id: connector_mandate_reference_id.map(|connector_mandate_id| {
            MandateReferenceId::ConnectorMandateId(connector_mandate_id)
        }),
    });
    Ok(())
}

fn response_to_capture_update(
    multiple_capture_data: &MultipleCaptureData,
    response_list: HashMap<String, CaptureSyncResponse>,
) -> RouterResult<Vec<(storage::Capture, storage::CaptureUpdate)>> {
    let mut capture_update_list = vec![];
    let mut unmapped_captures = vec![];
    for (connector_capture_id, capture_sync_response) in response_list {
        let capture =
            multiple_capture_data.get_capture_by_connector_capture_id(&connector_capture_id);
        if let Some(capture) = capture {
            capture_update_list.push((
                capture.clone(),
                storage::CaptureUpdate::foreign_try_from(capture_sync_response)?,
            ))
        } else {
            // connector_capture_id may not be populated in the captures table in some case
            // if so, we try to map the unmapped capture response and captures in DB.
            unmapped_captures.push(capture_sync_response)
        }
    }
    capture_update_list.extend(get_capture_update_for_unmapped_capture_responses(
        unmapped_captures,
        multiple_capture_data,
    )?);

    Ok(capture_update_list)
}

fn get_capture_update_for_unmapped_capture_responses(
    unmapped_capture_sync_response_list: Vec<CaptureSyncResponse>,
    multiple_capture_data: &MultipleCaptureData,
) -> RouterResult<Vec<(storage::Capture, storage::CaptureUpdate)>> {
    let mut result = Vec::new();
    let captures_without_connector_capture_id: Vec<_> = multiple_capture_data
        .get_pending_captures_without_connector_capture_id()
        .into_iter()
        .cloned()
        .collect();
    for capture_sync_response in unmapped_capture_sync_response_list {
        if let Some(capture) = captures_without_connector_capture_id
            .iter()
            .find(|capture| {
                capture_sync_response.get_connector_response_reference_id()
                    == Some(capture.capture_id.clone())
                    || capture_sync_response.get_amount_captured() == Some(capture.amount)
            })
        {
            result.push((
                capture.clone(),
                storage::CaptureUpdate::foreign_try_from(capture_sync_response)?,
            ))
        }
    }
    Ok(result)
}

fn get_total_amount_captured<F: Clone, T: types::Capturable>(
    request: &T,
    amount_captured: Option<MinorUnit>,
    router_data_status: enums::AttemptStatus,
    payment_data: &PaymentData<F>,
) -> Option<MinorUnit> {
    match &payment_data.multiple_capture_data {
        Some(multiple_capture_data) => {
            //multiple capture
            Some(multiple_capture_data.get_total_blocked_amount())
        }
        None => {
            //Non multiple capture
            let amount = request
                .get_captured_amount(payment_data)
                .map(MinorUnit::new);
            amount_captured.or_else(|| {
                if router_data_status == enums::AttemptStatus::Charged {
                    amount
                } else {
                    None
                }
            })
        }
    }
}
