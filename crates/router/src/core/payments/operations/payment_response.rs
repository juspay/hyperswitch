use std::{collections::HashMap, ops::Deref};

#[cfg(feature = "v1")]
use ::payment_methods::client::{
    CardDetailUpdate, PaymentMethodUpdateData, UpdatePaymentMethodV1Payload,
};
use api_models::payments::{ConnectorMandateReferenceId, MandateReferenceId};
#[cfg(feature = "dynamic_routing")]
use api_models::routing::RoutableConnectorChoice;
use async_trait::async_trait;
use common_enums::AuthorizationStatus;
#[cfg(feature = "v1")]
use common_enums::{ConnectorTokenStatus, TokenizationType};
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use common_utils::ext_traits::ValueExt;
use common_utils::{
    ext_traits::{AsyncExt, Encode},
    types::{keymanager::KeyManagerState, ConnectorTransactionId, MinorUnit},
};
use error_stack::{report, ResultExt};
use futures::FutureExt;
#[cfg(feature = "v2")]
use hyperswitch_domain_models::payments::{
    PaymentConfirmData, PaymentIntentData, PaymentStatusData,
};
use hyperswitch_domain_models::{behaviour::Conversion, payments::payment_attempt::PaymentAttempt};
#[cfg(feature = "v2")]
use masking::{ExposeInterface, PeekInterface};
use router_derive;
use router_env::{instrument, logger, tracing};
#[cfg(feature = "v1")]
use tracing_futures::Instrument;

use super::{Operation, OperationSessionSetters, PostUpdateTracker};
#[cfg(feature = "v1")]
use crate::core::payment_methods::transformers::call_modular_payment_method_update;
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use crate::core::routing::helpers as routing_helpers;
#[cfg(feature = "v2")]
use crate::utils::OptionExt;
use crate::{
    connector::utils::PaymentResponseRouterData,
    consts,
    core::{
        card_testing_guard::utils as card_testing_guard_utils,
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        mandate,
        payment_methods::{self, cards::create_encrypted_data},
        payments::{
            helpers::{
                self as payments_helpers,
                update_additional_payment_data_with_connector_response_pm_data,
            },
            tokenization,
            types::MultipleCaptureData,
            OperationSessionGetters, PaymentData, PaymentMethodChecker,
        },
        utils as core_utils,
    },
    routes::{metrics, SessionState},
    types::{
        self, domain,
        storage::{self, enums},
        transformers::{ForeignFrom, ForeignTryFrom},
        CaptureSyncResponse, ErrorResponse,
    },
    utils,
};

/// This implementation executes the flow only when
/// 1. Payment was created with supported payment methods
/// 2. Payment attempt's status was not a terminal failure
#[cfg(feature = "v1")]
async fn update_modular_pm_and_mandate_impl<F, T>(
    state: &SessionState,
    resp: &types::RouterData<F, T, types::PaymentsResponseData>,
    request_payment_method_data: Option<&domain::PaymentMethodData>,
    payment_data: &mut PaymentData<F>,
) -> CustomResult<(), ::payment_methods::errors::ModularPaymentMethodError>
where
    F: Clone + Send + Sync,
{
    if matches!(
        payment_data.payment_attempt.payment_method,
        Some(enums::PaymentMethod::Card)
    ) && resp.status.should_update_payment_method()
    {
        //#1 - Check if Payment method id is present in the payment data
        match payment_data
            .payment_method_info
            .as_ref()
            .map(|pm_info| pm_info.get_id().clone())
        {
            Some(payment_method_id) => {
                logger::info!("Payment method is card and eligible for modular update");

                // #2 - Derive network transaction ID from the connector response.
                let (network_transaction_id, connector_token_details) = if matches!(
                    payment_data.payment_attempt.setup_future_usage_applied,
                    Some(common_enums::FutureUsage::OffSession)
                ) {
                    let network_transaction_id = resp
                    .response
                    .as_ref()
                    .map_err(|err| {
                        logger::debug!(error=?err, "Failed to obtain the network_transaction_id from payment response in modular payment method update call");
                    })
                    .ok()
                    .and_then(types::PaymentsResponseData::get_network_transaction_id);

                    let connector_token_details = match resp
                        .response
                        .as_ref()
                        .ok()
                        .and_then(types::PaymentsResponseData::get_mandate_reference)
                    {
                        Some(mandate_reference) => {
                            let connector_id = payment_data
                            .payment_attempt
                            .merchant_connector_id
                            .clone()
                            .ok_or_else(|| {
                                logger::error!("Missing required Param merchant_connector_id");
                                ::payment_methods::errors::ModularPaymentMethodError::RetrieveFailed
                            })?;
                            update_connector_mandate_details_for_the_flow(
                                mandate_reference.connector_mandate_id.clone(),
                                mandate_reference.mandate_metadata.clone(),
                                mandate_reference
                                    .connector_mandate_request_reference_id
                                    .clone(),
                                payment_data,
                            )
                            .change_context(
                                ::payment_methods::errors::ModularPaymentMethodError::UpdateFailed,
                            )?;
                            mandate_reference
                                .connector_mandate_id
                                .map(|connector_mandate_id| {
                                    ::payment_methods::types::ConnectorTokenDetails {
                                        connector_id,
                                        token_type: TokenizationType::MultiUse,
                                        status: ConnectorTokenStatus::Active,
                                        connector_token_request_reference_id: mandate_reference
                                            .connector_mandate_request_reference_id,
                                        original_payment_authorized_amount: Some(
                                            payment_data
                                                .payment_attempt
                                                .net_amount
                                                .get_total_amount(),
                                        ),
                                        original_payment_authorized_currency: payment_data
                                            .payment_attempt
                                            .currency,
                                        metadata: mandate_reference.mandate_metadata,
                                        token: masking::Secret::new(connector_mandate_id),
                                    }
                                })
                        }
                        None => None,
                    };

                    (network_transaction_id, connector_token_details)
                } else {
                    (None, None)
                };

                // #3 - Fill payment method data for cards (update card holder name, nick_name & cvc).
                // Use request payment method data for card_holder_name and nick_name
                let payment_method_data =
                    request_payment_method_data.and_then(|method_data| match method_data {
                        domain::PaymentMethodData::CardToken(card) => {
                            Some(PaymentMethodUpdateData::Card(CardDetailUpdate {
                                card_holder_name: card.card_holder_name.clone(),
                                nick_name: card.card_holder_name.clone(),
                                card_cvc: None,
                            }))
                        }
                        _ => None,
                    });
                let acknowledgement_status = if resp.status.should_update_payment_method() {
                    Some(common_enums::AcknowledgementStatus::Authenticated)
                } else {
                    None
                };

                let payload = UpdatePaymentMethodV1Payload {
                    payment_method_data,
                    connector_token_details,
                    network_transaction_id: network_transaction_id.map(masking::Secret::new),
                    acknowledgement_status,
                };

                // #5 - Execute the modular payment-method update call if there is something to be updated
                if payload.payment_method_data.is_some()
                    || payload.connector_token_details.is_some()
                    || payload.network_transaction_id.is_some()
                    || payload.acknowledgement_status.is_some()
                {
                    match call_modular_payment_method_update(
                        state,
                        &payment_data.payment_attempt.processor_merchant_id,
                        &payment_data.payment_attempt.profile_id,
                        &payment_method_id,
                        payload,
                    )
                    .await
                    {
                        Ok(_) => {
                            logger::info!("Successfully called modular payment method update");
                        }
                        Err(err) => {
                            logger::error!("Failed to call modular payment method update: {}", err);
                        }
                    };
                    payment_data.payment_attempt.payment_method_id =
                        Some(payment_method_id.clone());
                } else {
                    logger::info!("No updates found for modular payment method update call");
                }
            }
            _ => {
                logger::info!("Payment method is not eligible for modular update");
            }
        }
    }

    Ok(())
}

/// Helper function to update payment method connector mandate details.
/// This is called after a successful payment to activate/update the connector mandate.
#[cfg(feature = "v1")]
async fn update_pm_connector_mandate_details<F, Req>(
    state: &SessionState,
    provider: &domain::Provider,
    initiator: Option<&domain::Initiator>,
    payment_data: &PaymentData<F>,
    router_data: &types::RouterData<F, Req, types::PaymentsResponseData>,
) -> RouterResult<()>
where
    F: Clone + Send + Sync,
{
    let is_valid_response = matches!(
        router_data.response.as_ref(),
        Ok(types::PaymentsResponseData::TransactionResponse { .. })
    );
    let is_integrity_ok = router_data.integrity_check.is_ok();

    // Check payment status from payment_data (which has the final processed status)
    let payment_attempt = payment_data.get_payment_attempt();
    let is_payment_successful = matches!(
        payment_attempt.status,
        enums::AttemptStatus::Charged
            | enums::AttemptStatus::Authorized
            | enums::AttemptStatus::PartiallyAuthorized
    );

    let is_eligible_for_mandate_update =
        is_valid_response && is_integrity_ok && is_payment_successful;

    if let (true, Some(payment_method), Some(mca_id)) = (
        is_eligible_for_mandate_update,
        payment_data.get_payment_method_info().cloned(),
        payment_attempt.merchant_connector_id.clone(),
    ) {
        let payment_intent = payment_data.get_payment_intent();

        let mandate_details = payment_method
            .get_common_mandate_reference()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to deserialize to Payment Mandate Reference")?;

        let is_active_mandate = mandate_details
            .payments
            .as_ref()
            .and_then(|payments| payments.0.get(&mca_id))
            .is_some_and(|record| {
                record.connector_mandate_status
                    == Some(common_enums::ConnectorMandateStatus::Active)
            });

        let is_off_session = matches!(
            payment_intent.setup_future_usage,
            Some(common_enums::FutureUsage::OffSession)
        );

        // Combine business logic conditions: not active mandate AND off_session
        if !is_active_mandate && is_off_session {
            let (connector_mandate_id, mandate_metadata, connector_mandate_request_reference_id) =
                payment_attempt
                    .connector_mandate_detail
                    .clone()
                    .map(|cmr| {
                        (
                            cmr.connector_mandate_id,
                            cmr.mandate_metadata,
                            cmr.connector_mandate_request_reference_id,
                        )
                    })
                    .unwrap_or((None, None, None));

            let connector_mandate_details = tokenization::update_connector_mandate_details(
                Some(mandate_details),
                payment_attempt.payment_method_type,
                Some(
                    payment_attempt
                        .net_amount
                        .get_total_amount()
                        .get_amount_as_i64(),
                ),
                payment_attempt.currency,
                payment_attempt.merchant_connector_id.clone(),
                connector_mandate_id,
                mandate_metadata,
                connector_mandate_request_reference_id,
            )?;

            payment_methods::cards::update_payment_method_connector_mandate_details(
                provider.get_key_store(),
                &*state.store,
                payment_method,
                connector_mandate_details,
                provider.get_account().storage_scheme,
                initiator,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to update payment method in db")?;
        }
    }
    Ok(())
}

#[cfg(feature = "v1")]
#[derive(Debug, Clone, Copy, router_derive::PaymentOperation)]
#[operation(
    operations = "post_update_tracker",
    flow = "sync_data, cancel_data, authorize_data, capture_data, complete_authorize_data, approve_data, reject_data, setup_mandate_data, session_data,incremental_authorization_data, sdk_session_update_data, post_session_tokens_data, update_metadata_data, cancel_post_capture_data, extend_authorization_data"
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
        processor: &domain::Processor,
        mut payment_data: PaymentData<F>,
        router_data: types::RouterData<
            F,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
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

        // update setup_future_usage incase it is downgraded to on-session
        payment_data.payment_attempt.setup_future_usage_applied =
            router_data.request.setup_future_usage;

        payment_data = Box::pin(payment_response_update_tracker(
            db,
            payment_data,
            router_data,
            processor,
            locale,
            #[cfg(all(feature = "v1", feature = "dynamic_routing"))]
            routable_connector,
            #[cfg(all(feature = "v1", feature = "dynamic_routing"))]
            business_profile,
        ))
        .await?;

        Ok(payment_data)
    }

    #[cfg(feature = "v2")]
    async fn save_pm_and_mandate<'b>(
        &self,
        state: &SessionState,
        resp: &types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
        platform: &domain::Platform,
        payment_data: &mut PaymentData<F>,
        business_profile: &domain::Profile,
    ) -> CustomResult<(), errors::ApiErrorResponse>
    where
        F: 'b + Clone + Send + Sync,
    {
        todo!()
    }

    #[cfg(feature = "v1")]
    async fn save_pm_and_mandate<'b>(
        &self,
        state: &SessionState,
        resp: &types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
        platform: &domain::Platform,
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
        let vault_operation = payment_data.vault_operation.clone();
        let payment_method_info = payment_data.payment_method_info.clone();

        if let Some(payment_method_info) = &payment_data.payment_method_info {
            if payment_data.payment_intent.off_session.is_none() && resp.response.is_ok() {
                should_avoid_saving = resp.request.payment_method_type
                    == Some(enums::PaymentMethodType::ApplePay)
                    || resp.request.payment_method_type
                        == Some(enums::PaymentMethodType::GooglePay);
                payment_methods::cards::update_last_used_at(
                    payment_method_info,
                    state,
                    platform.get_provider().get_account().storage_scheme,
                    platform.get_provider().get_key_store(),
                )
                .await
                .map_err(|e| {
                    logger::error!("Failed to update last used at: {:?}", e);
                })
                .ok();
            }
        };
        let connector_mandate_reference_id = payment_data
            .payment_attempt
            .connector_mandate_detail
            .as_ref()
            .map(|detail| ConnectorMandateReferenceId::foreign_from(detail.clone()));
        let customer_details = payment_data
            .payment_intent
            .get_customer_document_details()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to extract customer document details from payment_intent")?;

        let save_payment_call_future = Box::pin(tokenization::save_payment_method(
            state,
            connector_name.clone(),
            save_payment_data,
            customer_id.clone(),
            platform,
            resp.request.payment_method_type,
            billing_name.clone(),
            payment_method_billing_address,
            business_profile,
            connector_mandate_reference_id.clone(),
            merchant_connector_id.clone(),
            vault_operation.clone(),
            payment_method_info.clone(),
            payment_data.payment_method_token.clone(),
            customer_details.clone(),
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

        // Skip payment method creation when on-session saving is not supported for the payment method
        should_avoid_saving = if resp
            .request
            .setup_future_usage
            .map(|usage| usage.is_on_session())
            .unwrap_or(false)
            && payment_data
                .payment_attempt
                .is_save_payment_method_not_supported_for_on_session(
                    &state
                        .conf
                        .save_payment_method_on_session
                        .unsupported_payment_methods,
                ) {
            true
        } else {
            should_avoid_saving
        };

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
                platform.get_processor().get_account().storage_scheme,
                payment_data.payment_intent.get_id(),
            )
            .await?;
            payment_data.payment_attempt.payment_method_id = payment_method_id;
            payment_data.payment_attempt.mandate_id = mandate_id;

            Ok(())
        } else if is_connector_mandate {
            // The mandate is created on connector's end.
            let save_payment_call_response = save_payment_call_future.await;
            match save_payment_call_response {
                Ok(tokenization::SavePaymentMethodDataResponse {
                    payment_method_id,
                    connector_mandate_reference_id,
                    ..
                }) => {
                    payment_data.payment_method_info = if let Some(payment_method_id) =
                        &payment_method_id
                    {
                        match state
                            .store
                            .find_payment_method(
                                platform.get_provider().get_key_store(),
                                payment_method_id,
                                platform.get_provider().get_account().storage_scheme,
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
                                        .change_context(
                                            errors::ApiErrorResponse::InternalServerError,
                                        )
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
                    payment_data.payment_attempt.connector_mandate_detail =
                        connector_mandate_reference_id
                            .clone()
                            .map(ForeignFrom::foreign_from);
                    payment_data.set_mandate_id(api_models::payments::MandateIds {
                        mandate_id: None,
                        mandate_reference_id: connector_mandate_reference_id.map(
                            |connector_mandate_id| {
                                MandateReferenceId::ConnectorMandateId(connector_mandate_id)
                            },
                        ),
                    })
                }
                Err(err) => {
                    logger::error!("Error while storing the payment method in locker {:?}", err);
                }
            }
            Ok(())
        } else if should_avoid_saving {
            if let Some(pm_info) = &payment_data.payment_method_info {
                payment_data.payment_attempt.payment_method_id = Some(pm_info.get_id().clone());
            };
            Ok(())
        } else {
            // Save card flow
            let save_payment_data = tokenization::SavePaymentMethodData::from(resp);
            let state = state.clone();
            let customer_id = payment_data.payment_intent.customer_id.clone();
            let payment_attempt = payment_data.payment_attempt.clone();

            let business_profile = business_profile.clone();
            let payment_method_type = resp.request.payment_method_type;
            let payment_method_billing_address = payment_method_billing_address.cloned();
            let payment_method_token = payment_data.payment_method_token.clone();

            let cloned_platform = platform.clone();
            logger::info!("Call to save_payment_method in locker");
            let _task_handle = tokio::spawn(
                async move {
                    logger::info!("Starting async call to save_payment_method in locker");

                    let result = Box::pin(tokenization::save_payment_method(
                        &state,
                        connector_name,
                        save_payment_data,
                        customer_id,
                        &cloned_platform,
                        payment_method_type,
                        billing_name,
                        payment_method_billing_address.as_ref(),
                        &business_profile,
                        connector_mandate_reference_id,
                        merchant_connector_id.clone(),
                        vault_operation.clone(),
                        payment_method_info.clone(),
                        payment_method_token.clone(),
                        customer_details.clone(),
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
                                updated_by: cloned_platform
                                    .get_processor()
                                    .get_account()
                                    .storage_scheme
                                    .clone()
                                    .to_string(),
                            };

                        #[cfg(feature = "v1")]
                        let respond = state
                            .store
                            .update_payment_attempt_with_attempt_id(
                                payment_attempt,
                                payment_attempt_update,
                                cloned_platform.get_processor().get_account().storage_scheme,
                                cloned_platform.get_processor().get_key_store(),
                            )
                            .await;

                        #[cfg(feature = "v2")]
                        let respond = state
                            .store
                            .update_payment_attempt_with_attempt_id(
                                &(&state).into(),
                                &key_store.clone(),
                                payment_attempt,
                                payment_attempt_update,
                                cloned_platform.get_processor().get_account().storage_scheme,
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

    #[cfg(feature = "v1")]
    async fn update_pm_and_mandate<'b>(
        &self,
        state: &SessionState,
        provider: &domain::Provider,
        initiator: Option<&domain::Initiator>,
        payment_data: &PaymentData<F>,
        router_data: &types::RouterData<
            F,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
        feature_set: &core_utils::FeatureConfig,
    ) -> RouterResult<()>
    where
        F: 'b + Clone + Send + Sync,
    {
        if !feature_set.is_payment_method_modular_allowed {
            update_pm_connector_mandate_details(
                state,
                provider,
                initiator,
                payment_data,
                router_data,
            )
            .await
        } else {
            Ok(())
        }
    }

    #[cfg(feature = "v1")]
    async fn update_modular_pm_and_mandate<'b>(
        &self,
        state: &SessionState,
        resp: &types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
        _platform: &domain::Platform,
        payment_data: &mut PaymentData<F>,
        _business_profile: &domain::Profile,
        request_payment_method_data: Option<&domain::PaymentMethodData>,
    ) -> CustomResult<(), errors::ApiErrorResponse>
    where
        F: 'b + Clone + Send + Sync,
    {
        update_modular_pm_and_mandate_impl(state, resp, request_payment_method_data, payment_data)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to update modular payment method and mandate")
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
        processor: &domain::Processor,
        mut payment_data: PaymentData<F>,
        router_data: types::RouterData<
            F,
            types::PaymentsIncrementalAuthorizationData,
            types::PaymentsResponseData,
        >,
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
                                    // Internally, `NetAmount` is computed as (order_amount + additional_amount), so we subtract here to avoid double-counting.
                                    incremental_authorization_details.total_amount - payment_data.payment_attempt.net_amount.get_additional_amount(),
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
                                amount: incremental_authorization_details.total_amount - payment_data.payment_attempt.net_amount.get_additional_amount(),
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
                        processor.get_account().storage_scheme,
                        processor.get_key_store(),
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
                        processor.get_key_store(),
                        payment_data.payment_attempt.clone(),
                        payment_attempt_update,
                        processor.get_account().storage_scheme,
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
                    payment_data.payment_intent.clone(),
                    payment_intent_update,
                    processor.get_key_store(),
                    processor.get_account().storage_scheme,
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
            .update_authorization_by_processor_merchant_id_authorization_id(
                payment_data.payment_intent.processor_merchant_id.clone(),
                authorization_id,
                authorization_update,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("failed while updating authorization")?;
        //Fetch all the authorizations of the payment and send in incremental authorization response
        let authorizations = state
            .store
            .find_all_authorizations_by_processor_merchant_id_payment_id(
                &payment_data.payment_intent.processor_merchant_id,
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
        processor: &domain::Processor,
        payment_data: PaymentData<F>,
        router_data: types::RouterData<F, types::PaymentsSyncData, types::PaymentsResponseData>,
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
            payment_data,
            router_data,
            processor,
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
        platform: &domain::Platform,
        payment_data: &mut PaymentData<F>,
        _business_profile: &domain::Profile,
    ) -> CustomResult<(), errors::ApiErrorResponse>
    where
        F: 'b + Clone + Send + Sync,
    {
        let (connector_mandate_id, mandate_metadata, connector_mandate_request_reference_id) = resp
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
                            mandate_ref.connector_mandate_request_reference_id.clone(),
                        )
                    })
                } else {
                    None
                }
            })
            .unwrap_or((None, None, None));

        update_connector_mandate_details_for_the_flow(
            connector_mandate_id,
            mandate_metadata,
            connector_mandate_request_reference_id,
            payment_data,
        )?;

        update_payment_method_status_and_ntid(
            state,
            platform.get_provider().get_key_store(),
            payment_data,
            resp.status,
            resp.response.clone(),
            platform.get_provider().get_account().storage_scheme,
            platform.get_initiator(),
        )
        .await?;
        Ok(())
    }

    #[cfg(feature = "v1")]
    async fn update_pm_and_mandate<'b>(
        &self,
        state: &SessionState,
        provider: &domain::Provider,
        initiator: Option<&domain::Initiator>,
        payment_data: &PaymentData<F>,
        router_data: &types::RouterData<F, types::PaymentsSyncData, types::PaymentsResponseData>,
        feature_set: &core_utils::FeatureConfig,
    ) -> RouterResult<()>
    where
        F: 'b + Clone + Send + Sync,
    {
        if !feature_set.is_payment_method_modular_allowed {
            update_pm_connector_mandate_details(
                state,
                provider,
                initiator,
                payment_data,
                router_data,
            )
            .await
        } else {
            Ok(())
        }
    }

    #[cfg(feature = "v1")]
    async fn update_modular_pm_and_mandate<'b>(
        &self,
        state: &SessionState,
        resp: &types::RouterData<F, types::PaymentsSyncData, types::PaymentsResponseData>,
        _platform: &domain::Platform,
        payment_data: &mut PaymentData<F>,
        _business_profile: &domain::Profile,
        request_payment_method_data: Option<&domain::PaymentMethodData>,
    ) -> CustomResult<(), errors::ApiErrorResponse>
    where
        F: 'b + Clone + Send + Sync,
    {
        update_modular_pm_and_mandate_impl(state, resp, request_payment_method_data, payment_data)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to update modular payment method and mandate")
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
        processor: &domain::Processor,
        mut payment_data: PaymentData<F>,
        router_data: types::RouterData<F, types::PaymentsSessionData, types::PaymentsResponseData>,
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
            payment_data,
            router_data,
            processor,
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
        processor: &domain::Processor,
        mut payment_data: PaymentData<F>,
        router_data: types::RouterData<
            F,
            types::SdkPaymentsSessionUpdateData,
            types::PaymentsResponseData,
        >,
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
                Ok(types::PaymentsResponseData::PaymentResourceUpdateResponse { status }) => {
                    if status.is_success() {
                        let shipping_address = payment_data
                            .tax_data
                            .clone()
                            .map(|tax_data| tax_data.shipping_details);

                        let shipping_details = shipping_address
                            .clone()
                            .async_map(|shipping_details| {
                                create_encrypted_data(
                                    &key_manager_state,
                                    processor.get_key_store(),
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
                                shipping_address.map(From::from).as_ref(),
                                payment_data.payment_intent.shipping_address_id.as_deref(),
                                &payment_data.payment_intent.merchant_id,
                                payment_data.payment_intent.customer_id.as_ref(),
                                processor.get_key_store(),
                                &payment_data.payment_intent.payment_id,
                                processor.get_account().storage_scheme,
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

                        let updated_payment_intent = m_db
                            .update_payment_intent(
                                payment_intent,
                                payment_intent_update,
                                processor.get_key_store(),
                                processor.get_account().storage_scheme,
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
        processor: &domain::Processor,
        mut payment_data: PaymentData<F>,
        router_data: types::RouterData<
            F,
            types::PaymentsPostSessionTokensData,
            types::PaymentsResponseData,
        >,
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
                        updated_by: processor.get_account().storage_scheme.clone().to_string(),
                        connector_metadata,
                    };
                let updated_payment_attempt = m_db
                    .update_payment_attempt_with_attempt_id(
                        payment_data.payment_attempt.clone(),
                        payment_attempt_update,
                        processor.get_account().storage_scheme,
                        processor.get_key_store(),
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
impl<F: Clone> PostUpdateTracker<F, PaymentData<F>, types::PaymentsUpdateMetadataData>
    for PaymentResponse
{
    async fn update_tracker<'b>(
        &'b self,
        db: &'b SessionState,
        processor: &domain::Processor,
        mut payment_data: PaymentData<F>,
        router_data: types::RouterData<
            F,
            types::PaymentsUpdateMetadataData,
            types::PaymentsResponseData,
        >,
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
        let connector = payment_data
            .payment_attempt
            .connector
            .clone()
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("connector not found in payment_attempt")?;

        match router_data.response.clone() {
            Ok(types::PaymentsResponseData::PaymentResourceUpdateResponse { status, .. }) => {
                if status.is_success() {
                    let m_db = db.clone().store;
                    let payment_intent = payment_data.payment_intent.clone();
                    let payment_intent_update =
                        hyperswitch_domain_models::payments::payment_intent::PaymentIntentUpdate::MetadataUpdate {
                            metadata: payment_data
                                .payment_intent
                                .metadata
                                .clone(),
                            feature_metadata: payment_intent.feature_metadata.clone().map(masking::Secret::new),
                            updated_by: payment_data.payment_intent.updated_by.clone(),
                        };

                    let updated_payment_intent = m_db
                        .update_payment_intent(
                            payment_intent,
                            payment_intent_update,
                            processor.get_key_store(),
                            processor.get_account().storage_scheme,
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
                    .attach_printable("Unexpected response in Update Metadata flow")?;
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
        processor: &domain::Processor,
        mut payment_data: PaymentData<F>,
        router_data: types::RouterData<F, types::PaymentsCaptureData, types::PaymentsResponseData>,
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
            payment_data,
            router_data,
            processor,
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
        processor: &domain::Processor,
        mut payment_data: PaymentData<F>,
        router_data: types::RouterData<F, types::PaymentsCancelData, types::PaymentsResponseData>,
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
            payment_data,
            router_data,
            processor,
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
impl<F: Clone> PostUpdateTracker<F, PaymentData<F>, types::PaymentsCancelPostCaptureData>
    for PaymentResponse
{
    async fn update_tracker<'b>(
        &'b self,
        db: &'b SessionState,
        processor: &domain::Processor,
        mut payment_data: PaymentData<F>,
        router_data: types::RouterData<
            F,
            types::PaymentsCancelPostCaptureData,
            types::PaymentsResponseData,
        >,
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
            payment_data,
            router_data,
            processor,
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
impl<F: Clone> PostUpdateTracker<F, PaymentData<F>, types::PaymentsExtendAuthorizationData>
    for PaymentResponse
{
    async fn update_tracker<'b>(
        &'b self,
        db: &'b SessionState,
        processor: &domain::Processor,
        mut payment_data: PaymentData<F>,
        router_data: types::RouterData<
            F,
            types::PaymentsExtendAuthorizationData,
            types::PaymentsResponseData,
        >,
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
            payment_data,
            router_data,
            processor,
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
        processor: &domain::Processor,
        mut payment_data: PaymentData<F>,
        router_data: types::RouterData<F, types::PaymentsApproveData, types::PaymentsResponseData>,
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
            payment_data,
            router_data,
            processor,
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
        processor: &domain::Processor,
        mut payment_data: PaymentData<F>,
        router_data: types::RouterData<F, types::PaymentsRejectData, types::PaymentsResponseData>,
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
            payment_data,
            router_data,
            processor,
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
        processor: &domain::Processor,
        mut payment_data: PaymentData<F>,
        router_data: types::RouterData<
            F,
            types::SetupMandateRequestData,
            types::PaymentsResponseData,
        >,
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
            payment_data,
            router_data,
            processor,
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
        platform: &domain::Platform,
        payment_data: &mut PaymentData<F>,
        business_profile: &domain::Profile,
    ) -> CustomResult<(), errors::ApiErrorResponse>
    where
        F: 'b + Clone + Send + Sync,
    {
        let payment_method_billing_address = payment_data.address.get_payment_method_billing();
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
        let connector_mandate_reference_id = payment_data
            .payment_attempt
            .connector_mandate_detail
            .as_ref()
            .map(|detail| ConnectorMandateReferenceId::foreign_from(detail.clone()));
        let vault_operation = payment_data.vault_operation.clone();
        let payment_method_info = payment_data.payment_method_info.clone();
        let merchant_connector_id = payment_data.payment_attempt.merchant_connector_id.clone();
        let customer_details = payment_data
            .payment_intent
            .get_customer_document_details()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to extract customer document details from payment_intent")?;
        let tokenization::SavePaymentMethodDataResponse {
            payment_method_id,
            connector_mandate_reference_id,
            ..
        } = Box::pin(tokenization::save_payment_method(
            state,
            connector_name,
            save_payment_data,
            customer_id.clone(),
            platform,
            resp.request.payment_method_type,
            billing_name,
            payment_method_billing_address,
            business_profile,
            connector_mandate_reference_id,
            merchant_connector_id.clone(),
            vault_operation,
            payment_method_info,
            payment_data.payment_method_token.clone(),
            customer_details,
        ))
        .await?;

        payment_data.payment_method_info = if let Some(payment_method_id) = &payment_method_id {
            match state
                .store
                .find_payment_method(
                    platform.get_provider().get_key_store(),
                    payment_method_id,
                    platform.get_provider().get_account().storage_scheme,
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
        let mandate_id = mandate::mandate_procedure(
            state,
            resp,
            &customer_id,
            payment_method_id.clone(),
            merchant_connector_id.clone(),
            platform.get_processor().get_account().storage_scheme,
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

    #[cfg(feature = "v1")]
    async fn update_pm_and_mandate<'b>(
        &self,
        state: &SessionState,
        provider: &domain::Provider,
        initiator: Option<&domain::Initiator>,
        payment_data: &PaymentData<F>,
        router_data: &types::RouterData<
            F,
            types::SetupMandateRequestData,
            types::PaymentsResponseData,
        >,
        _feature_set: &core_utils::FeatureConfig,
    ) -> RouterResult<()>
    where
        F: 'b + Clone + Send + Sync,
    {
        update_pm_connector_mandate_details(state, provider, initiator, payment_data, router_data)
            .await
    }
    #[cfg(feature = "v1")]
    async fn update_modular_pm_and_mandate<'b>(
        &self,
        state: &SessionState,
        resp: &types::RouterData<F, types::SetupMandateRequestData, types::PaymentsResponseData>,
        _platform: &domain::Platform,
        payment_data: &mut PaymentData<F>,
        _business_profile: &domain::Profile,
        request_payment_method_data: Option<&domain::PaymentMethodData>,
    ) -> CustomResult<(), errors::ApiErrorResponse>
    where
        F: 'b + Clone + Send + Sync,
    {
        update_modular_pm_and_mandate_impl(state, resp, request_payment_method_data, payment_data)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to update modular payment method and mandate")
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
        processor: &domain::Processor,
        payment_data: PaymentData<F>,
        response: types::RouterData<F, types::CompleteAuthorizeData, types::PaymentsResponseData>,
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
            payment_data,
            response,
            processor,
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
        platform: &domain::Platform,
        payment_data: &mut PaymentData<F>,
        _business_profile: &domain::Profile,
    ) -> CustomResult<(), errors::ApiErrorResponse>
    where
        F: 'b + Clone + Send + Sync,
    {
        let (connector_mandate_id, mandate_metadata, connector_mandate_request_reference_id) = resp
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
                            mandate_ref.connector_mandate_request_reference_id.clone(),
                        )
                    })
                } else {
                    None
                }
            })
            .unwrap_or((None, None, None));
        update_connector_mandate_details_for_the_flow(
            connector_mandate_id,
            mandate_metadata,
            connector_mandate_request_reference_id,
            payment_data,
        )?;

        update_payment_method_status_and_ntid(
            state,
            platform.get_provider().get_key_store(),
            payment_data,
            resp.status,
            resp.response.clone(),
            platform.get_provider().get_account().storage_scheme,
            platform.get_initiator(),
        )
        .await?;
        Ok(())
    }

    #[cfg(feature = "v1")]
    async fn update_pm_and_mandate<'b>(
        &self,
        state: &SessionState,
        provider: &domain::Provider,
        initiator: Option<&domain::Initiator>,
        payment_data: &PaymentData<F>,
        router_data: &types::RouterData<
            F,
            types::CompleteAuthorizeData,
            types::PaymentsResponseData,
        >,
        feature_set: &core_utils::FeatureConfig,
    ) -> RouterResult<()>
    where
        F: 'b + Clone + Send + Sync,
    {
        if !feature_set.is_payment_method_modular_allowed {
            update_pm_connector_mandate_details(
                state,
                provider,
                initiator,
                payment_data,
                router_data,
            )
            .await
        } else {
            Ok(())
        }
    }

    #[cfg(feature = "v1")]
    async fn update_modular_pm_and_mandate<'b>(
        &self,
        state: &SessionState,
        resp: &types::RouterData<F, types::CompleteAuthorizeData, types::PaymentsResponseData>,
        _platform: &domain::Platform,
        payment_data: &mut PaymentData<F>,
        _business_profile: &domain::Profile,
        request_payment_method_data: Option<&domain::PaymentMethodData>,
    ) -> CustomResult<(), errors::ApiErrorResponse>
    where
        F: 'b + Clone + Send + Sync,
    {
        update_modular_pm_and_mandate_impl(state, resp, request_payment_method_data, payment_data)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to update modular payment method and mandate")
    }
}

#[cfg(feature = "v1")]
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
async fn payment_response_update_tracker<F: Clone, T: types::Capturable>(
    state: &SessionState,
    mut payment_data: PaymentData<F>,
    router_data: types::RouterData<F, T, types::PaymentsResponseData>,
    processor: &domain::Processor,
    locale: &Option<String>,
    #[cfg(all(feature = "v1", feature = "dynamic_routing"))] _routable_connectors: Vec<
        RoutableConnectorChoice,
    >,
    #[cfg(all(feature = "v1", feature = "dynamic_routing"))] business_profile: &domain::Profile,
) -> RouterResult<PaymentData<F>> {
    let key_manager_state = &state.into();
    // Update additional payment data with the payment method response that we received from connector
    // This is for details like whether 3ds was upgraded and which version of 3ds was used
    // also some connectors might send card network details in the response, which is captured and stored

    let additional_payment_data = payment_data.payment_attempt.get_payment_method_data();

    let additional_payment_method_data_intermediate = match payment_data.payment_method_data.clone() {
        Some(hyperswitch_domain_models::payment_method_data::PaymentMethodData::NetworkToken(_))
        | Some(hyperswitch_domain_models::payment_method_data::PaymentMethodData::CardDetailsForNetworkTransactionId(_))
        | Some(hyperswitch_domain_models::payment_method_data::PaymentMethodData::DecryptedWalletTokenDetailsForNetworkTransactionId(_))
        | Some(hyperswitch_domain_models::payment_method_data::PaymentMethodData::NetworkTokenDetailsForNetworkTransactionId(_)) => {
            payment_data.payment_attempt.payment_method_data.clone()
        }
        _ => {
            additional_payment_data
                .map(|_| {
                    update_additional_payment_data_with_connector_response_pm_data(
                        payment_data.payment_attempt.payment_method_data.clone(),
                        router_data
                            .connector_response
                            .as_ref()
                            .and_then(|connector_response| {
                                connector_response.additional_payment_method_data.clone()
                            }),
                    )
                })
                .transpose()?
                .flatten()
        }
    };

    // If the additional PM data is sensitive, encrypt it and populate encrypted_payment_method_data; otherwise populate additional_payment_method_data
    let (additional_payment_method_data, encrypted_payment_method_data) =
        payments_helpers::get_payment_method_data_and_encrypted_payment_method_data(
            &payment_data.payment_attempt,
            key_manager_state,
            processor.get_key_store(),
            additional_payment_method_data_intermediate,
        )
        .await?;

    payment_data.whole_connector_response = router_data.raw_connector_response.clone();

    let payment_method_status = router_data.payment_method_status;

    // TODO: refactor of gsm_error_category with respective feature flag
    #[allow(unused_variables)]
    let (capture_update, mut payment_attempt_update, gsm_error_category) = match router_data
        .response
        .clone()
    {
        Err(err) => {
            let auth_update = if Some(router_data.auth_type)
                != payment_data.payment_attempt.authentication_type
            {
                Some(router_data.auth_type)
            } else {
                None
            };
            let (capture_update, attempt_update, gsm_error_category) =
                match payment_data.multiple_capture_data {
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
                                    updated_by: processor.get_account().storage_scheme.to_string(),
                                }
                            }),
                            None,
                        )
                    }
                    None => {
                        let sub_flow = core_utils::get_flow_name::<F>()?;

                        let card_network = payment_data.payment_attempt.extract_card_network();

                        // GSM lookup for error object construction
                        let option_gsm = payments_helpers::get_gsm_record(
                            state,
                            router_data.connector.to_string(),
                            consts::PAYMENT_FLOW_STR,
                            &sub_flow,
                            Some(err.code.clone()),
                            Some(err.message.clone()),
                            err.network_decline_code.clone(),
                            card_network.clone(),
                        )
                        .await;

                        let gsm_unified_code =
                            option_gsm.as_ref().and_then(|gsm| gsm.unified_code.clone());
                        let gsm_unified_message = option_gsm
                            .as_ref()
                            .and_then(|gsm| gsm.unified_message.clone());
                        let gsm_standardised_code =
                            option_gsm.as_ref().and_then(|gsm| gsm.standardised_code);
                        let gsm_description =
                            option_gsm.as_ref().and_then(|gsm| gsm.description.clone());
                        let gsm_user_guidance_message = option_gsm
                            .as_ref()
                            .and_then(|gsm| gsm.user_guidance_message.clone());

                        // For MIT transactions, lookup recommended action from merchant_advice_codes config
                        let recommended_action =
                            payments_helpers::get_merchant_advice_code_recommended_action(
                                &state.conf.merchant_advice_codes,
                                payment_data.payment_intent.off_session,
                                card_network.as_ref(),
                                err.network_advice_code.as_deref(),
                            );

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
                            // mark previous attempt status for technical failures in PSync and ExtendAuthorization flow
                            {
                                if sub_flow == "PSync" || sub_flow == "ExtendAuthorization" {
                                    match err.status_code {
                                        // marking failure for 2xx because this is genuine payment failure
                                        200..=299 => enums::AttemptStatus::Failure,
                                        _ => router_data.status,
                                    }
                                } else if sub_flow == "Capture" {
                                    match err.status_code {
                                        500..=511 => enums::AttemptStatus::Pending,
                                        // don't update the status for 429 error status
                                        429 => router_data.status,
                                        _ => enums::AttemptStatus::Failure,
                                    }
                                } else if sub_flow == "CancelPostCapture" {
                                    router_data.status
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
                                error_message: Some(Some(err.message.clone())),
                                error_code: Some(Some(err.code.clone())),
                                error_reason: Some(err.reason.clone()),
                                amount_capturable: router_data
                                    .request
                                    .get_amount_capturable(
                                        &payment_data,
                                        router_data
                                            .minor_amount_capturable
                                            .map(MinorUnit::get_amount_as_i64),
                                        status,
                                    )
                                    .map(MinorUnit::new),
                                updated_by: processor.get_account().storage_scheme.to_string(),
                                unified_code: Some(Some(unified_code)),
                                unified_message: Some(unified_translated_message),
                                standardised_code: Some(gsm_standardised_code),
                                description: Some(gsm_description),
                                user_guidance_message: Some(gsm_user_guidance_message),
                                connector_transaction_id: err.connector_transaction_id.clone(),
                                payment_method_data: additional_payment_method_data,
                                encrypted_payment_method_data,
                                authentication_type: auth_update,
                                issuer_error_code: Some(err.network_decline_code.clone()),
                                issuer_error_message: Some(err.network_error_message.clone()),
                                network_details: Some(Some(ForeignFrom::foreign_from(&err))),
                                network_error_message: Some(err.network_error_message.clone()),
                                connector_response_reference_id: err
                                    .connector_response_reference_id
                                    .clone(),
                                recommended_action: Some(recommended_action),
                                card_network: payment_data.payment_attempt.extract_card_network(),
                            }),
                            option_gsm.and_then(|option_gsm| option_gsm.error_category),
                        )
                    }
                };
            (capture_update, attempt_update, gsm_error_category)
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
                            status: enums::AttemptStatus::IntegrityFailure,
                            error_message: Some(Some("Integrity Check Failed!".to_string())),
                            error_code: Some(Some("IE".to_string())),
                            error_reason: Some(Some(format!(
                                "Integrity Check Failed! Value mismatched for fields {field_name}"
                            ))),
                            amount_capturable: None,
                            updated_by: processor.get_account().storage_scheme.to_string(),
                            unified_code: None,
                            unified_message: None,
                            standardised_code: None,
                            description: None,
                            user_guidance_message: None,
                            connector_transaction_id,
                            payment_method_data: None,
                            encrypted_payment_method_data: None,
                            authentication_type: auth_update,
                            issuer_error_code: None,
                            issuer_error_message: None,
                            network_details: None,
                            network_error_message: None,
                            connector_response_reference_id: None,
                            recommended_action: None,
                            card_network: payment_data.payment_attempt.extract_card_network(),
                        }),
                        None,
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
                            _ => router_data.get_attempt_status_for_db_update(
                                &payment_data,
                                router_data.amount_captured,
                                router_data
                                    .minor_amount_capturable
                                    .map(MinorUnit::get_amount_as_i64),
                            )?,
                        },
                        _ => router_data.get_attempt_status_for_db_update(
                            &payment_data,
                            router_data.amount_captured,
                            router_data
                                .minor_amount_capturable
                                .map(MinorUnit::get_amount_as_i64),
                        )?,
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
                                    updated_by: processor.get_account().storage_scheme.to_string(),
                                };

                            (None, Some(payment_attempt_update), None)
                        }
                        types::PaymentsResponseData::TransactionResponse {
                            resource_id,
                            redirection_data,
                            connector_metadata,
                            connector_response_reference_id,
                            incremental_authorization_allowed,
                            charges,
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
                            let resp_network_transaction_id = router_data.response.as_ref()
                                .map_err(|err| {
                                    logger::error!(error = ?err, "Failed to obtain the network_transaction_id from payment response");
                                })
                                .ok()
                                .and_then(|resp| resp.get_network_transaction_id());

                            let encoded_data = payment_data.payment_attempt.encoded_data.clone();

                            let authentication_data = (*redirection_data)
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
                                enums::AttemptStatus::Charged
                                    | enums::AttemptStatus::Authorized
                                    | enums::AttemptStatus::PartiallyAuthorized
                            ) {
                                payment_data
                                    .payment_intent
                                    .fingerprint_id
                                    .clone_from(&payment_data.payment_attempt.fingerprint_id);

                                metrics::SUCCESSFUL_PAYMENT.add(1, &[]);
                            }

                            let payment_method_id =
                                payment_data.payment_attempt.payment_method_id.clone();

                            let debit_routing_savings =
                                payment_data.payment_method_data.as_ref().and_then(|data| {
                                    payments_helpers::get_debit_routing_savings_amount(
                                        data,
                                        &payment_data.payment_attempt,
                                    )
                                });

                            utils::add_apple_pay_payment_status_metrics(
                                router_data.status,
                                router_data.apple_pay_flow.clone(),
                                payment_data.payment_attempt.connector.clone(),
                                payment_data.payment_attempt.merchant_id.clone(),
                            );
                            let is_overcapture_enabled = router_data
                                .connector_response
                                .as_ref()
                                .and_then(|connector_response| {
                                    connector_response.is_overcapture_enabled()
                                }).or_else(|| {
                                    payment_data.payment_intent
                                                    .enable_overcapture
                                                    .as_ref()
                                                    .map(|enable_overcapture| common_types::primitive_wrappers::OvercaptureEnabledBool::new(*enable_overcapture.deref()))
                                            });

                            let (
                                capture_before,
                                extended_authorization_applied,
                                extended_authorization_last_applied_at,
                            ) = router_data
                                .connector_response
                                .as_ref()
                                .and_then(|connector_response| {
                                    connector_response.get_extended_authorization_response_data()
                                })
                                .map(|extended_auth_resp| {
                                    (
                                        extended_auth_resp.capture_before,
                                        extended_auth_resp.extended_authentication_applied,
                                        extended_auth_resp.extended_authorization_last_applied_at,
                                    )
                                })
                                .unwrap_or((None, None, None));
                            let (capture_updates, payment_attempt_update) = match payment_data
                                .multiple_capture_data
                            {
                                Some(multiple_capture_data) => {
                                    let (connector_capture_id, processor_capture_data) =
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
                                        processor_capture_data: processor_capture_data.clone(),
                                    };
                                    let capture_update_list = vec![(
                                        multiple_capture_data.get_latest_capture().clone(),
                                        capture_update,
                                    )];
                                    (Some((multiple_capture_data, capture_update_list)), auth_update.map(|auth_type| {
                                        storage::PaymentAttemptUpdate::AuthenticationTypeUpdate {
                                            authentication_type: auth_type,
                                            updated_by: processor.get_account().storage_scheme.to_string(),
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
                                                router_data
                                                    .minor_amount_capturable
                                                    .map(MinorUnit::get_amount_as_i64),
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
                                        unified_message: error_status.clone(),
                                        standardised_code: error_status.clone().map(|_| None),
                                        description: error_status.clone(),
                                        user_guidance_message: error_status.clone(),
                                        connector_response_reference_id,
                                        updated_by: processor
                                            .get_account()
                                            .storage_scheme
                                            .to_string(),
                                        authentication_data,
                                        encoded_data,
                                        payment_method_data: additional_payment_method_data,
                                        encrypted_payment_method_data,
                                        capture_before,
                                        extended_authorization_applied,
                                        extended_authorization_last_applied_at,
                                        connector_mandate_detail: Box::new(
                                            payment_data
                                                .payment_attempt
                                                .connector_mandate_detail
                                                .clone(),
                                        ),
                                        charges,
                                        setup_future_usage_applied: payment_data
                                            .payment_attempt
                                            .setup_future_usage_applied,
                                        debit_routing_savings,
                                        network_transaction_id: resp_network_transaction_id,
                                        is_overcapture_enabled,
                                        authorized_amount: router_data.authorized_amount,
                                        tokenization: payment_data
                                            .payment_attempt
                                            .clone()
                                            .get_tokenization_strategy(),
                                        issuer_error_code: error_status.clone(),
                                        issuer_error_message: error_status.clone(),
                                        network_details: error_status.clone().map(|_| None),
                                        network_error_message: error_status.clone(),
                                        recommended_action: error_status.map(|_| None),
                                        card_network: payment_data
                                            .payment_attempt
                                            .extract_card_network(),
                                    }),
                                ),
                            };

                            (capture_updates, payment_attempt_update, None)
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
                                    updated_by: processor.get_account().storage_scheme.to_string(),
                                }),
                                None,
                            )
                        }
                        types::PaymentsResponseData::SessionResponse { .. } => (None, None, None),
                        types::PaymentsResponseData::SessionTokenResponse { .. } => {
                            (None, None, None)
                        }
                        types::PaymentsResponseData::TokenizationResponse { .. } => {
                            (None, None, None)
                        }
                        types::PaymentsResponseData::ConnectorCustomerResponse(..) => {
                            (None, None, None)
                        }
                        types::PaymentsResponseData::ThreeDSEnrollmentResponse { .. } => {
                            (None, None, None)
                        }
                        types::PaymentsResponseData::PostProcessingResponse { .. } => {
                            (None, None, None)
                        }
                        types::PaymentsResponseData::IncrementalAuthorizationResponse {
                            ..
                        } => (None, None, None),
                        types::PaymentsResponseData::PaymentResourceUpdateResponse { .. } => {
                            (None, None, None)
                        }
                        types::PaymentsResponseData::MultipleCaptureResponse {
                            capture_sync_response_list,
                        } => match payment_data.multiple_capture_data {
                            Some(multiple_capture_data) => {
                                let capture_update_list = response_to_capture_update(
                                    &multiple_capture_data,
                                    capture_sync_response_list,
                                )?;
                                (
                                    Some((multiple_capture_data, capture_update_list)),
                                    None,
                                    None,
                                )
                            }
                            None => (None, None, None),
                        },
                        types::PaymentsResponseData::PaymentsCreateOrderResponse { .. } => (
                            None,
                            Some(storage::PaymentAttemptUpdate::StatusUpdate {
                                status: updated_attempt_status,
                                updated_by: processor.get_account().storage_scheme.to_string(),
                            }),
                            None,
                        ),
                        types::PaymentsResponseData::PostCaptureVoidResponse { .. } => {
                            (None, None, None)
                        }
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
                    .update_capture_with_capture_id(
                        capture,
                        capture_update,
                        processor.get_account().storage_scheme,
                    )
                    .await
                    .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
                multiple_capture_data.update_capture(updated_capture);
            }

            let authorized_amount = payment_data
                .payment_attempt
                .authorized_amount
                .unwrap_or_else(|| payment_data.payment_attempt.get_total_amount());

            payment_attempt_update = Some(storage::PaymentAttemptUpdate::AmountToCaptureUpdate {
                status: multiple_capture_data.get_attempt_status(authorized_amount),
                amount_capturable: authorized_amount
                    - multiple_capture_data.get_total_blocked_amount(),
                updated_by: processor.get_account().storage_scheme.to_string(),
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
    let m_storage_scheme = processor.get_account().storage_scheme;
    let m_key_store = processor.get_key_store().clone();

    let diesel_payment_attempt = payment_attempt
        .clone()
        .convert()
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error while construcing diesel attempt model")?;

    let payment_attempt = payment_attempt_update
        .map(|payment_attempt_update| {
            payment_attempt_update
                .to_storage_model()
                .apply_changeset(diesel_payment_attempt)
        })
        .async_map(|diesel_payment_attempt| async {
            PaymentAttempt::convert_back(
                key_manager_state,
                diesel_payment_attempt,
                processor.get_key_store().key.get_inner(),
                processor.get_key_store().merchant_id.clone().into(),
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error while construcing domain attempt model")
        })
        .await
        .transpose()?
        .unwrap_or(payment_attempt);

    let payment_attempt_fut = tokio::spawn(
        async move {
            Box::pin(async move {
                Ok::<_, error_stack::Report<errors::ApiErrorResponse>>(
                    match m_payment_attempt_update {
                        Some(payment_attempt_update) => m_db
                            .update_payment_attempt_with_attempt_id(
                                m_payment_attempt,
                                payment_attempt_update,
                                m_storage_scheme,
                                &m_key_store,
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
    let key_manager_state: KeyManagerState = state.into();
    payment_data.authentication = match payment_data.authentication {
        Some(mut authentication_store) => {
            let authentication_update = hyperswitch_domain_models::authentication::AuthenticationUpdate::PostAuthorizationUpdate {
                authentication_lifecycle_status: enums::AuthenticationLifecycleStatus::Used,
            };
            let updated_authentication = state
                .store
                .update_authentication_by_merchant_id_authentication_id(
                    authentication_store.authentication,
                    authentication_update,
                    processor.get_key_store(),
                    &key_manager_state,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
            authentication_store.authentication = updated_authentication;
            Some(authentication_store)
        }
        None => None,
    };

    let amount_captured = get_total_amount_captured(
        &router_data.request,
        router_data.amount_captured.map(MinorUnit::new),
        router_data.status,
        &payment_data,
    );

    let payment_intent_update = get_payment_intent_update_data::<_, _>(
        payment_data.clone(),
        &router_data,
        processor,
        amount_captured,
    );

    let m_db = state.clone().store;
    let m_key_store = processor.get_key_store().clone();
    let m_storage_scheme = processor.get_account().storage_scheme;
    let m_payment_data_payment_intent = payment_data.payment_intent.clone();
    let m_payment_intent_update = payment_intent_update.clone();
    let payment_intent_fut = tokio::spawn(
        async move {
            m_db.update_payment_intent(
                m_payment_data_payment_intent,
                m_payment_intent_update,
                &m_key_store,
                m_storage_scheme,
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
    let m_storage_scheme = processor.get_account().storage_scheme;
    let mandate_update_fut = tokio::spawn(
        async move {
            mandate::update_connector_mandate_id(
                m_db.as_ref(),
                &m_router_data_merchant_id,
                m_payment_data_mandate_id,
                m_payment_method_id,
                m_router_data_response,
                m_storage_scheme,
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
        if payment_intent.status.is_in_terminal_state()
            && business_profile.dynamic_routing_algorithm.is_some()
        {
            let dynamic_routing_algo_ref: api_models::routing::DynamicRoutingAlgorithmRef =
                business_profile
                    .dynamic_routing_algorithm
                    .clone()
                    .map(|val| val.parse_value("DynamicRoutingAlgorithmRef"))
                    .transpose()
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("unable to deserialize DynamicRoutingAlgorithmRef from JSON")?
                    .ok_or(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("DynamicRoutingAlgorithmRef not found in profile")?;

            let state = state.clone();
            let profile_id = business_profile.get_id().to_owned();
            let payment_attempt = payment_attempt.clone();

            tokio::spawn(
                async move {
                    let should_route_to_open_router =
                        state.conf.open_router.dynamic_routing_enabled;
                    let is_success_rate_based = matches!(
                        payment_attempt.routing_approach,
                        Some(enums::RoutingApproach::SuccessRateExploitation)
                            | Some(enums::RoutingApproach::SuccessRateExploration)
                    );

                    if should_route_to_open_router && is_success_rate_based {
                        routing_helpers::update_gateway_score_helper_with_open_router(
                            &state,
                            &payment_attempt,
                            &profile_id,
                            dynamic_routing_algo_ref.clone(),
                        )
                        .await
                        .map_err(|e| logger::error!(open_router_update_gateway_score_err=?e))
                        .ok();
                    }
                }
                .in_current_span(),
            );
        }
    }

    payment_data.payment_intent = payment_intent;
    payment_data.payment_attempt = payment_attempt;
    payment_method_status.and_then(|status| {
        payment_data
            .payment_method_info
            .as_mut()
            .map(|info| info.status = status)
    });

    if payment_data.payment_attempt.status == enums::AttemptStatus::Failure {
        let _ = card_testing_guard_utils::increment_blocked_count_in_cache(
            state,
            payment_data.card_testing_guard_data.clone(),
        )
        .await;
    }

    match router_data.integrity_check {
        Ok(()) => Ok(payment_data),
        Err(err) => {
            metrics::INTEGRITY_CHECK_FAILED.add(
                1,
                router_env::metric_attributes!(
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
                        payment_data.payment_attempt.merchant_id.clone(),
                    )
                ),
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

#[cfg(feature = "v1")]
fn get_payment_intent_update_data<F: Clone, T: types::Capturable>(
    payment_data: PaymentData<F>,
    router_data: &types::RouterData<F, T, types::PaymentsResponseData>,
    processor: &domain::Processor,
    amount_captured: Option<MinorUnit>,
) -> storage::PaymentIntentUpdate {
    match &router_data.response {
        Err(_) => storage::PaymentIntentUpdate::PGStatusUpdate {
            status: api_models::enums::IntentStatus::foreign_from(
                payment_data.payment_attempt.status,
            ),
            updated_by: processor.get_account().storage_scheme.to_string(),
            incremental_authorization_allowed: Some(false),
            feature_metadata: payment_data
                .payment_intent
                .feature_metadata
                .clone()
                .map(masking::Secret::new),
        },
        Ok(types::PaymentsResponseData::PostCaptureVoidResponse {
            post_capture_void_status,
            connector_reference_id,
            description,
        }) => {
            let post_capture_void_response = common_types::domain::PostCaptureVoidData {
                status: *post_capture_void_status,
                connector_reference_id: connector_reference_id.clone(),
                description: description.clone(),
            };

            let current_state = payment_data
                .payment_intent
                .state_metadata
                .clone()
                .unwrap_or_default()
                .set_post_capture_void_data(post_capture_void_response);

            storage::PaymentIntentUpdate::StateMetadataUpdate {
                state_metadata: current_state.clone(),
                updated_by: processor.get_account().storage_scheme.to_string(),
            }
        }
        Ok(_) => storage::PaymentIntentUpdate::ResponseUpdate {
            status: api_models::enums::IntentStatus::foreign_from(
                payment_data.payment_attempt.status,
            ),
            amount_captured,
            updated_by: processor.get_account().storage_scheme.to_string(),
            fingerprint_id: payment_data.payment_attempt.fingerprint_id.clone(),
            incremental_authorization_allowed: payment_data
                .payment_intent
                .incremental_authorization_allowed,
            feature_metadata: payment_data
                .payment_intent
                .feature_metadata
                .clone()
                .map(masking::Secret::new),
        },
    }
}

#[cfg(feature = "v2")]
async fn update_payment_method_status_and_ntid<F: Clone>(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    payment_data: &mut PaymentData<F>,
    attempt_status: common_enums::AttemptStatus,
    payment_response: Result<types::PaymentsResponseData, ErrorResponse>,
    storage_scheme: enums::MerchantStorageScheme,
    _initiator: Option<&domain::Initiator>,
) -> RouterResult<()> {
    todo!()
}

#[cfg(feature = "v1")]
async fn update_payment_method_status_and_ntid<F: Clone>(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    payment_data: &mut PaymentData<F>,
    attempt_status: common_enums::AttemptStatus,
    payment_response: Result<types::PaymentsResponseData, ErrorResponse>,
    storage_scheme: enums::MerchantStorageScheme,
    initiator: Option<&domain::Initiator>,
) -> RouterResult<()> {
    // If the payment_method is deleted then ignore the error related to retrieving payment method
    // This should be handled when the payment method is soft deleted
    if let Some(id) = &payment_data.payment_attempt.payment_method_id {
        let payment_method = match state
            .store
            .find_payment_method(key_store, id, storage_scheme)
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
        let network_transaction_id = if payment_data.payment_intent.setup_future_usage
            == Some(diesel_models::enums::FutureUsage::OffSession)
        {
            if pm_resp_network_transaction_id.is_some() {
                pm_resp_network_transaction_id
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
                last_modified_by: initiator
                    .and_then(|initiator| initiator.to_created_by())
                    .map(|last_modified_by| last_modified_by.to_string()),
            }
        } else {
            storage::PaymentMethodUpdate::NetworkTransactionIdAndStatusUpdate {
                network_transaction_id,
                status: None,
                last_modified_by: initiator
                    .and_then(|initiator| initiator.to_created_by())
                    .map(|last_modified_by| last_modified_by.to_string()),
            }
        };

        state
            .store
            .update_payment_method(key_store, payment_method, pm_update, storage_scheme)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to update payment method in db")?;
    };
    Ok(())
}

#[cfg(feature = "v2")]
impl<F: Send + Clone> Operation<F, types::PaymentsAuthorizeData> for &PaymentResponse {
    type Data = PaymentConfirmData<F>;
    fn to_post_update_tracker(
        &self,
    ) -> RouterResult<
        &(dyn PostUpdateTracker<F, Self::Data, types::PaymentsAuthorizeData> + Send + Sync),
    > {
        Ok(*self)
    }
}

#[cfg(feature = "v2")]
impl<F: Send + Clone> Operation<F, types::PaymentsAuthorizeData> for PaymentResponse {
    type Data = PaymentConfirmData<F>;
    fn to_post_update_tracker(
        &self,
    ) -> RouterResult<
        &(dyn PostUpdateTracker<F, Self::Data, types::PaymentsAuthorizeData> + Send + Sync),
    > {
        Ok(self)
    }
}

#[cfg(feature = "v2")]
impl<F: Send + Clone> Operation<F, types::PaymentsCaptureData> for PaymentResponse {
    type Data = hyperswitch_domain_models::payments::PaymentCaptureData<F>;
    fn to_post_update_tracker(
        &self,
    ) -> RouterResult<
        &(dyn PostUpdateTracker<F, Self::Data, types::PaymentsCaptureData> + Send + Sync),
    > {
        Ok(self)
    }
}

#[cfg(feature = "v2")]
#[async_trait]
impl<F: Clone>
    PostUpdateTracker<
        F,
        hyperswitch_domain_models::payments::PaymentCaptureData<F>,
        types::PaymentsCaptureData,
    > for PaymentResponse
{
    async fn update_tracker<'b>(
        &'b self,
        state: &'b SessionState,
        processor: &domain::Processor,
        _initiator: Option<&domain::Initiator>,
        mut payment_data: hyperswitch_domain_models::payments::PaymentCaptureData<F>,
        response: types::RouterData<F, types::PaymentsCaptureData, types::PaymentsResponseData>,
    ) -> RouterResult<hyperswitch_domain_models::payments::PaymentCaptureData<F>>
    where
        F: 'b + Send + Sync,
        types::RouterData<F, types::PaymentsCaptureData, types::PaymentsResponseData>:
            hyperswitch_domain_models::router_data::TrackerPostUpdateObjects<
                F,
                types::PaymentsCaptureData,
                hyperswitch_domain_models::payments::PaymentCaptureData<F>,
            >,
    {
        use hyperswitch_domain_models::router_data::TrackerPostUpdateObjects;

        let db = &*state.store;

        let response_router_data = response;

        let payment_intent_update = response_router_data
            .get_payment_intent_update(&payment_data, processor.get_account().storage_scheme);

        let payment_attempt_update = response_router_data
            .get_payment_attempt_update(&payment_data, processor.get_account().storage_scheme);

        let updated_payment_intent = db
            .update_payment_intent(
                payment_data.payment_intent,
                payment_intent_update,
                processor.get_key_store(),
                processor.get_account().storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to update payment intent")?;

        let updated_payment_attempt = db
            .update_payment_attempt(
                processor.get_key_store(),
                payment_data.payment_attempt,
                payment_attempt_update,
                processor.get_account().storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to update payment attempt")?;

        payment_data.payment_intent = updated_payment_intent;
        payment_data.payment_attempt = updated_payment_attempt;

        Ok(payment_data)
    }
}

#[cfg(feature = "v2")]
#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentConfirmData<F>, types::PaymentsAuthorizeData>
    for PaymentResponse
{
    async fn update_tracker<'b>(
        &'b self,
        state: &'b SessionState,
        processor: &domain::Processor,
        initiator: Option<&domain::Initiator>,
        mut payment_data: PaymentConfirmData<F>,
        response: types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
    ) -> RouterResult<PaymentConfirmData<F>>
    where
        F: 'b + Send + Sync,
        types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>:
            hyperswitch_domain_models::router_data::TrackerPostUpdateObjects<
                F,
                types::PaymentsAuthorizeData,
                PaymentConfirmData<F>,
            >,
    {
        use hyperswitch_domain_models::router_data::TrackerPostUpdateObjects;

        let db = &*state.store;

        let response_router_data = response;

        let payment_intent_update = response_router_data
            .get_payment_intent_update(&payment_data, processor.get_account().storage_scheme);
        let payment_attempt_update = response_router_data
            .get_payment_attempt_update(&payment_data, processor.get_account().storage_scheme);

        let updated_payment_intent = db
            .update_payment_intent(
                payment_data.payment_intent,
                payment_intent_update,
                processor.get_key_store(),
                processor.get_account().storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to update payment intent")?;

        let updated_payment_attempt = db
            .update_payment_attempt(
                processor.get_key_store(),
                payment_data.payment_attempt,
                payment_attempt_update,
                processor.get_account().storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to update payment attempt")?;

        let attempt_status = updated_payment_attempt.status;

        let mandate_reference_id = response_router_data
            .connector_response
            .as_ref()
            .and_then(|data| data.mandate_reference.as_ref())
            .and_then(|mandate_ref| mandate_ref.connector_mandate_id.clone());

        let updated_metadata_details = response_router_data
            .connector_response
            .as_ref()
            .and_then(|data| data.mandate_reference.as_ref())
            .and_then(|mandate_ref| mandate_ref.mandate_metadata.clone());

        let updated_metadata_info = updated_metadata_details
            .map(|data| {
                serde_json::from_value::<api_models::payments::UpdatedMandateDetails>(
                    data.peek().clone(),
                )
            })
            .transpose()
            .inspect_err(|e| {
                logger::error!(
                    "Failed to deserialize UpdatedMandateDetails from mandate metadata: {:?}",
                    e
                );
            })
            .ok()
            .flatten();

        let mandate_data_updated = match updated_metadata_info {
            Some(data) => Some(api_models::payments::MandateIds {
                mandate_id: None,
                mandate_reference_id: Some(
                    api_models::payments::MandateReferenceId::ConnectorMandateId(
                        api_models::payments::ConnectorMandateReferenceId::new(
                            mandate_reference_id,
                            None,
                            None,
                            None,
                            None,
                            Some(data),
                        ),
                    ),
                ),
            }),
            None => payment_data.mandate_data,
        };

        payment_data.payment_intent = updated_payment_intent;
        payment_data.payment_attempt = updated_payment_attempt;
        payment_data.mandate_data = mandate_data_updated;

        if let Some(payment_method) = &payment_data.payment_method {
            match attempt_status {
                common_enums::AttemptStatus::AuthenticationFailed
                | common_enums::AttemptStatus::RouterDeclined
                | common_enums::AttemptStatus::AuthorizationFailed
                | common_enums::AttemptStatus::Voided
                | common_enums::AttemptStatus::VoidedPostCharge
                | common_enums::AttemptStatus::VoidInitiated
                | common_enums::AttemptStatus::CaptureFailed
                | common_enums::AttemptStatus::VoidFailed
                | common_enums::AttemptStatus::AutoRefunded
                | common_enums::AttemptStatus::Unresolved
                | common_enums::AttemptStatus::Pending
                | common_enums::AttemptStatus::Failure
                | common_enums::AttemptStatus::Expired => (),

                common_enums::AttemptStatus::Started
                | common_enums::AttemptStatus::AuthenticationPending
                | common_enums::AttemptStatus::AuthenticationSuccessful
                | common_enums::AttemptStatus::Authorized
                | common_enums::AttemptStatus::PartiallyAuthorized
                | common_enums::AttemptStatus::Charged
                | common_enums::AttemptStatus::Authorizing
                | common_enums::AttemptStatus::CodInitiated
                | common_enums::AttemptStatus::PartialCharged
                | common_enums::AttemptStatus::PartialChargedAndChargeable
                | common_enums::AttemptStatus::CaptureInitiated
                | common_enums::AttemptStatus::PaymentMethodAwaited
                | common_enums::AttemptStatus::ConfirmationAwaited
                | common_enums::AttemptStatus::DeviceDataCollectionPending
                | common_enums::AttemptStatus::IntegrityFailure => {
                    let pm_update_status = enums::PaymentMethodStatus::Active;

                    // payment_methods microservice call
                    payment_methods::update_payment_method_status_internal(
                        state,
                        processor.get_key_store(),
                        processor.get_account().storage_scheme,
                        pm_update_status,
                        payment_method.get_id(),
                        initiator,
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to update payment method status")?;
                }
            }
        }

        Ok(payment_data)
    }
}

#[cfg(feature = "v2")]
impl<F: Send + Clone> Operation<F, types::PaymentsSyncData> for PaymentResponse {
    type Data = PaymentStatusData<F>;
    fn to_post_update_tracker(
        &self,
    ) -> RouterResult<&(dyn PostUpdateTracker<F, Self::Data, types::PaymentsSyncData> + Send + Sync)>
    {
        Ok(self)
    }
}

#[cfg(feature = "v2")]
#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentStatusData<F>, types::PaymentsSyncData>
    for PaymentResponse
{
    async fn update_tracker<'b>(
        &'b self,
        state: &'b SessionState,
        processor: &domain::Processor,
        _initiator: Option<&domain::Initiator>,
        mut payment_data: PaymentStatusData<F>,
        response: types::RouterData<F, types::PaymentsSyncData, types::PaymentsResponseData>,
    ) -> RouterResult<PaymentStatusData<F>>
    where
        F: 'b + Send + Sync,
        types::RouterData<F, types::PaymentsSyncData, types::PaymentsResponseData>:
            hyperswitch_domain_models::router_data::TrackerPostUpdateObjects<
                F,
                types::PaymentsSyncData,
                PaymentStatusData<F>,
            >,
    {
        use hyperswitch_domain_models::router_data::TrackerPostUpdateObjects;

        let db = &*state.store;

        let response_router_data = response;

        // Get updated additional payment method data from connector response
        let updated_payment_method_data = payment_data
            .payment_attempt
            .payment_method_data
            .as_ref()
            .map(|existing_payment_method_data| {
                let additional_payment_data_value =
                    Some(existing_payment_method_data.clone().expose());
                update_additional_payment_data_with_connector_response_pm_data(
                    additional_payment_data_value,
                    response_router_data.connector_response.as_ref().and_then(
                        |connector_response| {
                            connector_response.additional_payment_method_data.clone()
                        },
                    ),
                )
            })
            .transpose()?
            .flatten()
            .map(common_utils::pii::SecretSerdeValue::new);

        let payment_intent_update = response_router_data
            .get_payment_intent_update(&payment_data, processor.get_account().storage_scheme);
        let payment_attempt_update = response_router_data
            .get_payment_attempt_update(&payment_data, processor.get_account().storage_scheme);

        let payment_attempt = payment_data.payment_attempt;

        let updated_payment_intent = db
            .update_payment_intent(
                payment_data.payment_intent,
                payment_intent_update,
                processor.get_key_store(),
                processor.get_account().storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to update payment intent")?;

        let updated_payment_attempt = db
            .update_payment_attempt(
                processor.get_key_store(),
                payment_attempt,
                payment_attempt_update,
                processor.get_account().storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to update payment attempt")?;

        payment_data.payment_intent = updated_payment_intent;
        payment_data.payment_attempt = updated_payment_attempt;

        Ok(payment_data)
    }
}

#[cfg(feature = "v2")]
impl<F: Send + Clone> Operation<F, types::SetupMandateRequestData> for &PaymentResponse {
    type Data = PaymentConfirmData<F>;
    fn to_post_update_tracker(
        &self,
    ) -> RouterResult<
        &(dyn PostUpdateTracker<F, Self::Data, types::SetupMandateRequestData> + Send + Sync),
    > {
        Ok(*self)
    }
}

#[cfg(feature = "v2")]
impl<F: Send + Clone> Operation<F, types::SetupMandateRequestData> for PaymentResponse {
    type Data = PaymentConfirmData<F>;
    fn to_post_update_tracker(
        &self,
    ) -> RouterResult<
        &(dyn PostUpdateTracker<F, Self::Data, types::SetupMandateRequestData> + Send + Sync),
    > {
        Ok(self)
    }
}

#[cfg(feature = "v2")]
impl
    Operation<
        hyperswitch_domain_models::router_flow_types::ExternalVaultProxy,
        hyperswitch_domain_models::router_request_types::ExternalVaultProxyPaymentsData,
    > for PaymentResponse
{
    type Data =
        PaymentConfirmData<hyperswitch_domain_models::router_flow_types::ExternalVaultProxy>;
    fn to_post_update_tracker(
        &self,
    ) -> RouterResult<
        &(dyn PostUpdateTracker<
            hyperswitch_domain_models::router_flow_types::ExternalVaultProxy,
            Self::Data,
            hyperswitch_domain_models::router_request_types::ExternalVaultProxyPaymentsData,
        > + Send
              + Sync),
    > {
        Ok(self)
    }
}

#[cfg(feature = "v2")]
impl
    Operation<
        hyperswitch_domain_models::router_flow_types::ExternalVaultProxy,
        hyperswitch_domain_models::router_request_types::ExternalVaultProxyPaymentsData,
    > for &PaymentResponse
{
    type Data =
        PaymentConfirmData<hyperswitch_domain_models::router_flow_types::ExternalVaultProxy>;
    fn to_post_update_tracker(
        &self,
    ) -> RouterResult<
        &(dyn PostUpdateTracker<
            hyperswitch_domain_models::router_flow_types::ExternalVaultProxy,
            Self::Data,
            hyperswitch_domain_models::router_request_types::ExternalVaultProxyPaymentsData,
        > + Send
              + Sync),
    > {
        Ok(*self)
    }
}

#[cfg(feature = "v2")]
#[async_trait]
impl
    PostUpdateTracker<
        hyperswitch_domain_models::router_flow_types::ExternalVaultProxy,
        PaymentConfirmData<hyperswitch_domain_models::router_flow_types::ExternalVaultProxy>,
        hyperswitch_domain_models::router_request_types::ExternalVaultProxyPaymentsData,
    > for PaymentResponse
{
    async fn update_tracker<'b>(
        &'b self,
        state: &'b SessionState,
        processor: &domain::Processor,
        _initiator: Option<&domain::Initiator>,
        mut payment_data: PaymentConfirmData<
            hyperswitch_domain_models::router_flow_types::ExternalVaultProxy,
        >,
        response: types::RouterData<
            hyperswitch_domain_models::router_flow_types::ExternalVaultProxy,
            hyperswitch_domain_models::router_request_types::ExternalVaultProxyPaymentsData,
            types::PaymentsResponseData,
        >,
    ) -> RouterResult<
        PaymentConfirmData<hyperswitch_domain_models::router_flow_types::ExternalVaultProxy>,
    >
    where
        types::RouterData<
            hyperswitch_domain_models::router_flow_types::ExternalVaultProxy,
            hyperswitch_domain_models::router_request_types::ExternalVaultProxyPaymentsData,
            types::PaymentsResponseData,
        >: hyperswitch_domain_models::router_data::TrackerPostUpdateObjects<
            hyperswitch_domain_models::router_flow_types::ExternalVaultProxy,
            hyperswitch_domain_models::router_request_types::ExternalVaultProxyPaymentsData,
            PaymentConfirmData<hyperswitch_domain_models::router_flow_types::ExternalVaultProxy>,
        >,
    {
        use hyperswitch_domain_models::router_data::TrackerPostUpdateObjects;
        let db = &*state.store;

        let response_router_data = response;

        let payment_intent_update = response_router_data
            .get_payment_intent_update(&payment_data, processor.get_account().storage_scheme);
        let payment_attempt_update = response_router_data
            .get_payment_attempt_update(&payment_data, processor.get_account().storage_scheme);

        let updated_payment_intent = db
            .update_payment_intent(
                payment_data.payment_intent,
                payment_intent_update,
                processor.get_key_store(),
                processor.get_account().storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to update payment intent")?;

        let updated_payment_attempt = db
            .update_payment_attempt(
                processor.get_key_store(),
                payment_data.payment_attempt,
                payment_attempt_update,
                processor.get_account().storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to update payment attempt")?;

        payment_data.payment_intent = updated_payment_intent;
        payment_data.payment_attempt = updated_payment_attempt;

        // TODO: Add external vault specific post-update logic if needed

        Ok(payment_data)
    }
}

#[cfg(feature = "v2")]
#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentConfirmData<F>, types::SetupMandateRequestData>
    for PaymentResponse
{
    async fn update_tracker<'b>(
        &'b self,
        state: &'b SessionState,
        processor: &domain::Processor,
        _initiator: Option<&domain::Initiator>,
        mut payment_data: PaymentConfirmData<F>,
        response: types::RouterData<F, types::SetupMandateRequestData, types::PaymentsResponseData>,
    ) -> RouterResult<PaymentConfirmData<F>>
    where
        F: 'b + Send + Sync,
        types::RouterData<F, types::SetupMandateRequestData, types::PaymentsResponseData>:
            hyperswitch_domain_models::router_data::TrackerPostUpdateObjects<
                F,
                types::SetupMandateRequestData,
                PaymentConfirmData<F>,
            >,
    {
        use hyperswitch_domain_models::router_data::TrackerPostUpdateObjects;

        let db = &*state.store;

        let response_router_data = response;

        let payment_intent_update = response_router_data
            .get_payment_intent_update(&payment_data, processor.get_account().storage_scheme);
        let payment_attempt_update = response_router_data
            .get_payment_attempt_update(&payment_data, processor.get_account().storage_scheme);

        let updated_payment_intent = db
            .update_payment_intent(
                payment_data.payment_intent,
                payment_intent_update,
                processor.get_key_store(),
                processor.get_account().storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to update payment intent")?;

        let updated_payment_attempt = db
            .update_payment_attempt(
                processor.get_key_store(),
                payment_data.payment_attempt,
                payment_attempt_update,
                processor.get_account().storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to update payment attempt")?;

        payment_data.payment_intent = updated_payment_intent;
        payment_data.payment_attempt = updated_payment_attempt;

        Ok(payment_data)
    }

    async fn save_pm_and_mandate<'b>(
        &self,
        state: &SessionState,
        router_data: &types::RouterData<
            F,
            types::SetupMandateRequestData,
            types::PaymentsResponseData,
        >,
        platform: &domain::Platform,
        payment_data: &mut PaymentConfirmData<F>,
        business_profile: &domain::Profile,
    ) -> CustomResult<(), errors::ApiErrorResponse>
    where
        F: 'b + Clone + Send + Sync,
    {
        // If we received a payment_method_id from connector in the router data response
        // Then we either update the payment method or create a new payment method
        // The case for updating the payment method is when the payment is created from the payment method service

        let Ok(payments_response) = &router_data.response else {
            // In case there was an error response from the connector
            // We do not take any action related to the payment method
            return Ok(());
        };

        let connector_request_reference_id = payment_data
            .payment_attempt
            .connector_token_details
            .as_ref()
            .and_then(|token_details| token_details.get_connector_token_request_reference_id());

        let connector_token =
            payments_response.get_updated_connector_token_details(connector_request_reference_id);

        let payment_method_id = payment_data.payment_attempt.payment_method_id.clone();

        // TODO: check what all conditions we will need to see if card need to be saved
        match (
            connector_token
                .as_ref()
                .and_then(|connector_token| connector_token.connector_mandate_id.clone()),
            payment_method_id,
        ) {
            (Some(token), Some(payment_method_id)) => {
                if !matches!(
                    router_data.status,
                    enums::AttemptStatus::Charged | enums::AttemptStatus::Authorized
                ) {
                    return Ok(());
                }
                let connector_id = payment_data
                    .payment_attempt
                    .merchant_connector_id
                    .clone()
                    .get_required_value("merchant_connector_id")
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("missing connector id")?;

                let net_amount = payment_data.payment_attempt.amount_details.get_net_amount();
                let currency = payment_data.payment_intent.amount_details.currency;

                let connector_token_details_for_payment_method_update =
                    api_models::payment_methods::ConnectorTokenDetails {
                        connector_id,
                        status: common_enums::ConnectorTokenStatus::Active,
                        connector_token_request_reference_id: connector_token
                            .and_then(|details| details.connector_token_request_reference_id),
                        original_payment_authorized_amount: Some(net_amount),
                        original_payment_authorized_currency: Some(currency),
                        metadata: None,
                        token: masking::Secret::new(token),
                        token_type: common_enums::TokenizationType::MultiUse,
                    };

                let payment_method_update_request =
                    api_models::payment_methods::PaymentMethodUpdate {
                        payment_method_data: None,
                        connector_token_details: Some(
                            connector_token_details_for_payment_method_update,
                        ),
                        network_transaction_id: None,
                        acknowledgement_status: None, //based on the response from the connector we can decide the acknowledgement status to be sent to payment method service
                    };

                let payment_method_update_request =
                    hyperswitch_domain_models::payment_methods::PaymentMethodUpdate::from(
                        payment_method_update_request,
                    );

                Box::pin(payment_methods::update_payment_method_core(
                    state,
                    platform,
                    business_profile,
                    payment_method_update_request,
                    &payment_method_id,
                    None,
                    None,
                ))
                .await
                .attach_printable("Failed to update payment method")?;
            }
            (Some(_), None) => {
                // TODO: create a new payment method
            }
            (None, Some(_)) | (None, None) => {}
        }

        Ok(())
    }
}

#[cfg(feature = "v1")]
fn update_connector_mandate_details_for_the_flow<F: Clone>(
    connector_mandate_id: Option<String>,
    mandate_metadata: Option<masking::Secret<serde_json::Value>>,
    connector_mandate_request_reference_id: Option<String>,
    payment_data: &mut PaymentData<F>,
) -> RouterResult<()> {
    let mut original_connector_mandate_reference_id = payment_data
        .payment_attempt
        .connector_mandate_detail
        .as_ref()
        .map(|detail| ConnectorMandateReferenceId::foreign_from(detail.clone()));
    let connector_mandate_reference_id = if connector_mandate_id.is_some() {
        if let Some(ref mut record) = original_connector_mandate_reference_id {
            record.update(
                connector_mandate_id,
                None,
                None,
                mandate_metadata,
                connector_mandate_request_reference_id,
            );
            Some(record.clone())
        } else {
            Some(ConnectorMandateReferenceId::new(
                connector_mandate_id,
                None,
                None,
                mandate_metadata,
                connector_mandate_request_reference_id,
                None,
            ))
        }
    } else {
        original_connector_mandate_reference_id
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
                .get_captured_amount(
                    amount_captured.map(MinorUnit::get_amount_as_i64),
                    payment_data,
                )
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

#[cfg(feature = "v2")]
impl<F: Send + Clone + Sync> Operation<F, types::PaymentsCancelData> for PaymentResponse {
    type Data = hyperswitch_domain_models::payments::PaymentCancelData<F>;
    fn to_post_update_tracker(
        &self,
    ) -> RouterResult<
        &(dyn PostUpdateTracker<F, Self::Data, types::PaymentsCancelData> + Send + Sync),
    > {
        Ok(self)
    }
}

#[cfg(feature = "v2")]
#[async_trait]
impl<F: Clone + Send + Sync>
    PostUpdateTracker<
        F,
        hyperswitch_domain_models::payments::PaymentCancelData<F>,
        types::PaymentsCancelData,
    > for PaymentResponse
{
    async fn update_tracker<'b>(
        &'b self,
        state: &'b SessionState,
        processor: &domain::Processor,
        _initiator: Option<&domain::Initiator>,
        mut payment_data: hyperswitch_domain_models::payments::PaymentCancelData<F>,
        router_data: types::RouterData<F, types::PaymentsCancelData, types::PaymentsResponseData>,
    ) -> RouterResult<hyperswitch_domain_models::payments::PaymentCancelData<F>>
    where
        F: 'b + Send + Sync,
        types::RouterData<F, types::PaymentsCancelData, types::PaymentsResponseData>:
            hyperswitch_domain_models::router_data::TrackerPostUpdateObjects<
                F,
                types::PaymentsCancelData,
                hyperswitch_domain_models::payments::PaymentCancelData<F>,
            >,
    {
        let db = &*state.store;

        use hyperswitch_domain_models::router_data::TrackerPostUpdateObjects;

        let payment_intent_update = router_data
            .get_payment_intent_update(&payment_data, processor.get_account().storage_scheme);

        let updated_payment_intent = db
            .update_payment_intent(
                payment_data.payment_intent.clone(),
                payment_intent_update,
                processor.get_key_store(),
                processor.get_account().storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
            .attach_printable("Error while updating the payment_intent")?;

        let payment_attempt_update = router_data
            .get_payment_attempt_update(&payment_data, processor.get_account().storage_scheme);

        let updated_payment_attempt = db
            .update_payment_attempt(
                processor.get_key_store(),
                payment_data.payment_attempt.clone(),
                payment_attempt_update,
                processor.get_account().storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
            .attach_printable("Error while updating the payment_attempt")?;

        payment_data.set_payment_intent(updated_payment_intent);
        payment_data.set_payment_attempt(updated_payment_attempt);

        Ok(payment_data)
    }
}
