use std::collections::HashMap;

use async_trait::async_trait;
use common_enums::AuthorizationStatus;
use common_utils::ext_traits::Encode;
use data_models::payments::payment_attempt::PaymentAttempt;
use error_stack::{report, IntoReport, ResultExt};
use futures::FutureExt;
use router_derive;
use router_env::{instrument, tracing};
use storage_impl::DataModelExt;
use tracing_futures::Instrument;

use super::{Operation, PostUpdateTracker};
use crate::{
    connector::utils::PaymentResponseRouterData,
    core::{
        errors::{self, RouterResult, StorageErrorExt},
        mandate,
        payment_methods::PaymentMethodRetrieve,
        payments::{helpers as payments_helpers, types::MultipleCaptureData, PaymentData},
        utils as core_utils,
    },
    routes::{metrics, AppState},
    types::{
        self, api,
        storage::{self, enums},
        transformers::{ForeignFrom, ForeignTryFrom},
        CaptureSyncResponse,
    },
    utils,
};

#[derive(Debug, Clone, Copy, router_derive::PaymentOperation)]
#[operation(
    operations = "post_update_tracker",
    flow = "sync_data, authorize_data, cancel_data, capture_data, complete_authorize_data, approve_data, reject_data, setup_mandate_data, session_data,incremental_authorization_data"
)]
pub struct PaymentResponse;

#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentData<F>, types::PaymentsAuthorizeData>
    for PaymentResponse
{
    async fn update_tracker<'b>(
        &'b self,
        db: &'b AppState,
        payment_id: &api::PaymentIdType,
        mut payment_data: PaymentData<F>,
        router_data: types::RouterData<
            F,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<PaymentData<F>>
    where
        F: 'b + Send,
    {
        payment_data.mandate_id = payment_data
            .mandate_id
            .or_else(|| router_data.request.mandate_id.clone());

        payment_data = Box::pin(payment_response_update_tracker(
            db,
            payment_id,
            payment_data,
            router_data,
            storage_scheme,
        ))
        .await?;

        Ok(payment_data)
    }
}

#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentData<F>, types::PaymentsIncrementalAuthorizationData>
    for PaymentResponse
{
    async fn update_tracker<'b>(
        &'b self,
        db: &'b AppState,
        _payment_id: &api::PaymentIdType,
        mut payment_data: PaymentData<F>,
        router_data: types::RouterData<
            F,
            types::PaymentsIncrementalAuthorizationData,
            types::PaymentsResponseData,
        >,
        storage_scheme: enums::MerchantStorageScheme,
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
        let (option_payment_attempt_update, option_payment_intent_update) =
            match router_data.response.clone() {
                Err(_) => (None, None),
                Ok(types::PaymentsResponseData::IncrementalAuthorizationResponse {
                    status,
                    ..
                }) => {
                    if status == AuthorizationStatus::Success {
                        (Some(
                        storage::PaymentAttemptUpdate::IncrementalAuthorizationAmountUpdate {
                            amount: incremental_authorization_details.total_amount,
                            amount_capturable: incremental_authorization_details.total_amount,
                        },
                    ), Some(
                        storage::PaymentIntentUpdate::IncrementalAuthorizationAmountUpdate {
                            amount: incremental_authorization_details.total_amount,
                        },
                    ))
                    } else {
                        (None, None)
                    }
                }
                _ => Err(errors::ApiErrorResponse::InternalServerError)
                    .into_report()
                    .attach_printable("unexpected response in incremental_authorization flow")?,
            };
        //payment_attempt update
        if let Some(payment_attempt_update) = option_payment_attempt_update {
            payment_data.payment_attempt = db
                .store
                .update_payment_attempt_with_attempt_id(
                    payment_data.payment_attempt.clone(),
                    payment_attempt_update,
                    storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
        }
        // payment_intent update
        if let Some(payment_intent_update) = option_payment_intent_update {
            payment_data.payment_intent = db
                .store
                .update_payment_intent(
                    payment_data.payment_intent.clone(),
                    payment_intent_update,
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
                .into_report()
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
        db.store
            .update_authorization_by_merchant_id_authorization_id(
                router_data.merchant_id.clone(),
                authorization_id,
                authorization_update,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("failed while updating authorization")?;
        //Fetch all the authorizations of the payment and send in incremental authorization response
        let authorizations = db
            .store
            .find_all_authorizations_by_merchant_id_payment_id(
                &router_data.merchant_id,
                &payment_data.payment_intent.payment_id,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("failed while retrieving authorizations")?;
        payment_data.authorizations = authorizations;
        Ok(payment_data)
    }
}

#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentData<F>, types::PaymentsSyncData> for PaymentResponse {
    async fn update_tracker<'b>(
        &'b self,
        db: &'b AppState,
        payment_id: &api::PaymentIdType,
        payment_data: PaymentData<F>,
        router_data: types::RouterData<F, types::PaymentsSyncData, types::PaymentsResponseData>,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<PaymentData<F>>
    where
        F: 'b + Send,
    {
        Box::pin(payment_response_update_tracker(
            db,
            payment_id,
            payment_data,
            router_data,
            storage_scheme,
        ))
        .await
    }
}

#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentData<F>, types::PaymentsSessionData>
    for PaymentResponse
{
    async fn update_tracker<'b>(
        &'b self,
        db: &'b AppState,
        payment_id: &api::PaymentIdType,
        mut payment_data: PaymentData<F>,
        router_data: types::RouterData<F, types::PaymentsSessionData, types::PaymentsResponseData>,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<PaymentData<F>>
    where
        F: 'b + Send,
    {
        payment_data = Box::pin(payment_response_update_tracker(
            db,
            payment_id,
            payment_data,
            router_data,
            storage_scheme,
        ))
        .await?;

        Ok(payment_data)
    }
}

#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentData<F>, types::PaymentsCaptureData>
    for PaymentResponse
{
    async fn update_tracker<'b>(
        &'b self,
        db: &'b AppState,
        payment_id: &api::PaymentIdType,
        mut payment_data: PaymentData<F>,
        router_data: types::RouterData<F, types::PaymentsCaptureData, types::PaymentsResponseData>,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<PaymentData<F>>
    where
        F: 'b + Send,
    {
        payment_data = Box::pin(payment_response_update_tracker(
            db,
            payment_id,
            payment_data,
            router_data,
            storage_scheme,
        ))
        .await?;

        Ok(payment_data)
    }
}

#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentData<F>, types::PaymentsCancelData> for PaymentResponse {
    async fn update_tracker<'b>(
        &'b self,
        db: &'b AppState,
        payment_id: &api::PaymentIdType,
        mut payment_data: PaymentData<F>,
        router_data: types::RouterData<F, types::PaymentsCancelData, types::PaymentsResponseData>,

        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<PaymentData<F>>
    where
        F: 'b + Send,
    {
        payment_data = Box::pin(payment_response_update_tracker(
            db,
            payment_id,
            payment_data,
            router_data,
            storage_scheme,
        ))
        .await?;

        Ok(payment_data)
    }
}

#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentData<F>, types::PaymentsApproveData>
    for PaymentResponse
{
    async fn update_tracker<'b>(
        &'b self,
        db: &'b AppState,
        payment_id: &api::PaymentIdType,
        mut payment_data: PaymentData<F>,
        router_data: types::RouterData<F, types::PaymentsApproveData, types::PaymentsResponseData>,

        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<PaymentData<F>>
    where
        F: 'b + Send,
    {
        payment_data = Box::pin(payment_response_update_tracker(
            db,
            payment_id,
            payment_data,
            router_data,
            storage_scheme,
        ))
        .await?;

        Ok(payment_data)
    }
}

#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentData<F>, types::PaymentsRejectData> for PaymentResponse {
    async fn update_tracker<'b>(
        &'b self,
        db: &'b AppState,
        payment_id: &api::PaymentIdType,
        mut payment_data: PaymentData<F>,
        router_data: types::RouterData<F, types::PaymentsRejectData, types::PaymentsResponseData>,

        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<PaymentData<F>>
    where
        F: 'b + Send,
    {
        payment_data = Box::pin(payment_response_update_tracker(
            db,
            payment_id,
            payment_data,
            router_data,
            storage_scheme,
        ))
        .await?;

        Ok(payment_data)
    }
}

#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentData<F>, types::SetupMandateRequestData>
    for PaymentResponse
{
    async fn update_tracker<'b>(
        &'b self,
        db: &'b AppState,
        payment_id: &api::PaymentIdType,
        mut payment_data: PaymentData<F>,
        router_data: types::RouterData<
            F,
            types::SetupMandateRequestData,
            types::PaymentsResponseData,
        >,

        storage_scheme: enums::MerchantStorageScheme,
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
            storage_scheme,
        ))
        .await?;

        Ok(payment_data)
    }
}

#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentData<F>, types::CompleteAuthorizeData>
    for PaymentResponse
{
    async fn update_tracker<'b>(
        &'b self,
        db: &'b AppState,
        payment_id: &api::PaymentIdType,
        payment_data: PaymentData<F>,
        response: types::RouterData<F, types::CompleteAuthorizeData, types::PaymentsResponseData>,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<PaymentData<F>>
    where
        F: 'b + Send,
    {
        Box::pin(payment_response_update_tracker(
            db,
            payment_id,
            payment_data,
            response,
            storage_scheme,
        ))
        .await
    }
}

#[instrument(skip_all)]
async fn payment_response_update_tracker<F: Clone, T: types::Capturable>(
    state: &AppState,
    _payment_id: &api::PaymentIdType,
    mut payment_data: PaymentData<F>,
    router_data: types::RouterData<F, T, types::PaymentsResponseData>,
    storage_scheme: enums::MerchantStorageScheme,
) -> RouterResult<PaymentData<F>> {
    router_data.payment_method_status.and_then(|status| payment_data.payment_method_info.as_mut().map(|info| info.status = status));
    let (capture_update, mut payment_attempt_update) = match router_data.response.clone() {
        Err(err) => {
            let (capture_update, attempt_update) = match payment_data.multiple_capture_data {
                Some(multiple_capture_data) => {
                    let capture_update = storage::CaptureUpdate::ErrorUpdate {
                        status: match err.status_code {
                            500..=511 => storage::enums::CaptureStatus::Pending,
                            _ => storage::enums::CaptureStatus::Failed,
                        },
                        error_code: Some(err.code),
                        error_message: Some(err.message),
                        error_reason: err.reason,
                    };
                    let capture_update_list = vec![(
                        multiple_capture_data.get_latest_capture().clone(),
                        capture_update,
                    )];
                    (Some((multiple_capture_data, capture_update_list)), None)
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
                    let status = match err.attempt_status {
                        // Use the status sent by connector in error_response if it's present
                        Some(status) => status,
                        None =>
                        // mark previous attempt status for technical failures in PSync flow
                        {
                            if flow_name == "PSync" {
                                match err.status_code {
                                    // marking failure for 2xx because this is genuine payment failure
                                    200..=299 => storage::enums::AttemptStatus::Failure,
                                    _ => router_data.status,
                                }
                            } else if flow_name == "Capture" {
                                match err.status_code {
                                    500..=511 => storage::enums::AttemptStatus::Pending,
                                    // don't update the status for 429 error status
                                    429 => router_data.status,
                                    _ => storage::enums::AttemptStatus::Failure,
                                }
                            } else {
                                match err.status_code {
                                    500..=511 => storage::enums::AttemptStatus::Pending,
                                    _ => storage::enums::AttemptStatus::Failure,
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
                                .get_amount_capturable(&payment_data, status),
                            updated_by: storage_scheme.to_string(),
                            unified_code: option_gsm.clone().map(|gsm| gsm.unified_code),
                            unified_message: option_gsm.map(|gsm| gsm.unified_message),
                            connector_transaction_id: err.connector_transaction_id,
                        }),
                    )
                }
            };
            (capture_update, attempt_update)
        }
        Ok(payments_response) => {
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
                    enums::FraudCheckStatus::Fraud | enums::FraudCheckStatus::ManualReview => {
                        attempt_status
                    }
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
                        types::PreprocessingResponseId::PreProcessingId(pre_processing_id) => {
                            Some(pre_processing_id)
                        }
                        types::PreprocessingResponseId::ConnectorTransactionId(_) => None,
                    };
                    let payment_attempt_update =
                        storage::PaymentAttemptUpdate::PreprocessingUpdate {
                            status: updated_attempt_status,
                            payment_method_id: Some(router_data.payment_method_id),
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
                        types::ResponseId::ConnectorTransactionId(id)
                        | types::ResponseId::EncodedData(id) => Some(id),
                    };

                    let encoded_data = payment_data.payment_attempt.encoded_data.clone();

                    let authentication_data = redirection_data
                        .as_ref()
                        .map(Encode::encode_to_value)
                        .transpose()
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Could not parse the connector response")?;

                    // incase of success, update error code and error message
                    let error_status = if router_data.status == enums::AttemptStatus::Charged {
                        Some(None)
                    } else {
                        None
                    };

                    if router_data.status == enums::AttemptStatus::Charged {
                        payment_data.payment_intent.fingerprint_id =
                            payment_data.payment_attempt.fingerprint_id.clone();
                        metrics::SUCCESSFUL_PAYMENT.add(&metrics::CONTEXT, 1, &[]);
                    }

                    let payment_method_id = router_data.payment_method_id.clone();

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
                            let capture_update = storage::CaptureUpdate::ResponseUpdate {
                                status: enums::CaptureStatus::foreign_try_from(router_data.status)?,
                                connector_capture_id: connector_transaction_id.clone(),
                                connector_response_reference_id,
                            };
                            let capture_update_list = vec![(
                                multiple_capture_data.get_latest_capture().clone(),
                                capture_update,
                            )];
                            (Some((multiple_capture_data, capture_update_list)), None)
                        }
                        None => (
                            None,
                            Some(storage::PaymentAttemptUpdate::ResponseUpdate {
                                status: updated_attempt_status,
                                connector: None,
                                connector_transaction_id: connector_transaction_id.clone(),
                                authentication_type: None,
                                amount_capturable: router_data
                                    .request
                                    .get_amount_capturable(&payment_data, updated_attempt_status),
                                payment_method_id: Some(payment_method_id),
                                mandate_id: payment_data
                                    .mandate_id
                                    .clone()
                                    .and_then(|mandate| mandate.mandate_id),
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
                            payment_method_id: Some(router_data.payment_method_id),
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
                types::PaymentsResponseData::ConnectorCustomerResponse { .. } => (None, None),
                types::PaymentsResponseData::ThreeDSEnrollmentResponse { .. } => (None, None),
                types::PaymentsResponseData::IncrementalAuthorizationResponse { .. } => {
                    (None, None)
                }
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

    let amount_captured = get_total_amount_captured(
        router_data.request,
        router_data.amount_captured,
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

    update_payment_method_status(state, &mut payment_data, router_data.status).await?;
    let m_db = state.clone().store;
    let m_payment_data_payment_intent = payment_data.payment_intent.clone();
    let m_payment_intent_update = payment_intent_update.clone();
    let payment_intent_fut = tokio::spawn(
        async move {
            m_db.update_payment_intent(
                m_payment_data_payment_intent,
                m_payment_intent_update,
                storage_scheme,
            )
            .map(|x| x.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound))
            .await
        }
        .in_current_span(),
    );

    // When connector requires redirection for mandate creation it can update the connector mandate_id during Psync and CompleteAuthorize
    let m_db = state.clone().store;
    let m_payment_method_id = payment_data.payment_attempt.payment_method_id.clone();
    let m_router_data_merchant_id = router_data.merchant_id.clone();
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
                m_router_data_merchant_id.clone(),
                m_payment_data_mandate_id,
                m_payment_method_id,
                m_router_data_response,
            )
            .await
        }
        .in_current_span(),
    );

    let (payment_intent, _, _) = futures::try_join!(
        utils::flatten_join_error(payment_intent_fut),
        utils::flatten_join_error(mandate_update_fut),
        utils::flatten_join_error(payment_attempt_fut)
    )?;

    payment_data.payment_intent = payment_intent;
    // if let Some(status) = router_data.payment_method_status {
    //     payment_data.payment_method_info.as_mut().map(|info| info.status = status);
    // }
    router_data.payment_method_status.and_then(|status| payment_data.payment_method_info.as_mut().map(|info| info.status = status));
    Ok(payment_data)
}

async fn update_payment_method_status<F: Clone>(
    state: &AppState,
    payment_data: &mut PaymentData<F>,
    attempt_status: common_enums::AttemptStatus,
) -> RouterResult<()> {
    if let Some(id) = &payment_data.payment_attempt.payment_method_id {
        let pm = state
            .store
            .find_payment_method(id)
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;

        if pm.status != attempt_status.into() {
            let updated_pm_status = common_enums::PaymentMethodStatus::from(attempt_status);

            payment_data.payment_method_info.as_mut().map(|info| info.status = updated_pm_status);
            let pm_update = storage::PaymentMethodUpdate::StatusUpdate {
                status: Some(updated_pm_status),
            };
            state
                .store
                .update_payment_method(pm, pm_update)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to update payment method in db")?;
        }
    };
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
            multiple_capture_data.get_capture_by_connector_capture_id(connector_capture_id);
        if let Some(capture) = capture {
            capture_update_list.push((capture.clone(), capture_sync_response.try_into()?))
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
                storage::CaptureUpdate::try_from(capture_sync_response)?,
            ))
        }
    }
    Ok(result)
}

fn get_total_amount_captured<F: Clone, T: types::Capturable>(
    request: T,
    amount_captured: Option<i64>,
    router_data_status: enums::AttemptStatus,
    payment_data: &PaymentData<F>,
) -> Option<i64> {
    match &payment_data.multiple_capture_data {
        Some(multiple_capture_data) => {
            //multiple capture
            Some(multiple_capture_data.get_total_blocked_amount())
        }
        None => {
            //Non multiple capture
            let amount = request.get_captured_amount(payment_data);
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
