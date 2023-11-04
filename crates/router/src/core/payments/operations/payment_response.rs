use std::collections::HashMap;

use async_trait::async_trait;
use error_stack::ResultExt;
use futures::FutureExt;
use router_derive;
use router_env::{instrument, tracing};

use super::{Operation, PostUpdateTracker};
use crate::{
    core::{
        errors::{self, RouterResult, StorageErrorExt},
        mandate,
        payment_methods::PaymentMethodRetrieve,
        payments::{types::MultipleCaptureData, PaymentData},
        utils as core_utils,
    },
    db::StorageInterface,
    routes::metrics,
    services::RedirectForm,
    types::{
        self, api,
        storage::{
            self, enums,
            payment_attempt::{AttemptStatusExt, PaymentAttemptExt},
        },
        transformers::ForeignTryFrom,
        CaptureSyncResponse,
    },
    utils,
};

#[derive(Debug, Clone, Copy, router_derive::PaymentOperation)]
#[operation(
    ops = "post_tracker",
    flow = "syncdata,authorizedata,canceldata,capturedata,completeauthorizedata,approvedata,rejectdata,setupmandatedata,sessiondata"
)]
pub struct PaymentResponse;

#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentData<F>, types::PaymentsAuthorizeData>
    for PaymentResponse
{
    async fn update_tracker<'b>(
        &'b self,
        db: &dyn StorageInterface,
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

        payment_data = payment_response_update_tracker(
            db,
            payment_id,
            payment_data,
            router_data,
            storage_scheme,
        )
        .await?;

        Ok(payment_data)
    }
}

#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentData<F>, types::PaymentsSyncData> for PaymentResponse {
    async fn update_tracker<'b>(
        &'b self,
        db: &dyn StorageInterface,
        payment_id: &api::PaymentIdType,
        payment_data: PaymentData<F>,
        router_data: types::RouterData<F, types::PaymentsSyncData, types::PaymentsResponseData>,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<PaymentData<F>>
    where
        F: 'b + Send,
    {
        payment_response_update_tracker(db, payment_id, payment_data, router_data, storage_scheme)
            .await
    }
}

#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentData<F>, types::PaymentsSessionData>
    for PaymentResponse
{
    async fn update_tracker<'b>(
        &'b self,
        db: &dyn StorageInterface,
        payment_id: &api::PaymentIdType,
        mut payment_data: PaymentData<F>,
        router_data: types::RouterData<F, types::PaymentsSessionData, types::PaymentsResponseData>,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<PaymentData<F>>
    where
        F: 'b + Send,
    {
        payment_data = payment_response_update_tracker(
            db,
            payment_id,
            payment_data,
            router_data,
            storage_scheme,
        )
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
        db: &dyn StorageInterface,
        payment_id: &api::PaymentIdType,
        mut payment_data: PaymentData<F>,
        router_data: types::RouterData<F, types::PaymentsCaptureData, types::PaymentsResponseData>,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<PaymentData<F>>
    where
        F: 'b + Send,
    {
        payment_data = payment_response_update_tracker(
            db,
            payment_id,
            payment_data,
            router_data,
            storage_scheme,
        )
        .await?;

        Ok(payment_data)
    }
}

#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentData<F>, types::PaymentsCancelData> for PaymentResponse {
    async fn update_tracker<'b>(
        &'b self,
        db: &dyn StorageInterface,
        payment_id: &api::PaymentIdType,
        mut payment_data: PaymentData<F>,
        router_data: types::RouterData<F, types::PaymentsCancelData, types::PaymentsResponseData>,

        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<PaymentData<F>>
    where
        F: 'b + Send,
    {
        payment_data = payment_response_update_tracker(
            db,
            payment_id,
            payment_data,
            router_data,
            storage_scheme,
        )
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
        db: &dyn StorageInterface,
        payment_id: &api::PaymentIdType,
        mut payment_data: PaymentData<F>,
        router_data: types::RouterData<F, types::PaymentsApproveData, types::PaymentsResponseData>,

        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<PaymentData<F>>
    where
        F: 'b + Send,
    {
        payment_data = payment_response_update_tracker(
            db,
            payment_id,
            payment_data,
            router_data,
            storage_scheme,
        )
        .await?;

        Ok(payment_data)
    }
}

#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentData<F>, types::PaymentsRejectData> for PaymentResponse {
    async fn update_tracker<'b>(
        &'b self,
        db: &dyn StorageInterface,
        payment_id: &api::PaymentIdType,
        mut payment_data: PaymentData<F>,
        router_data: types::RouterData<F, types::PaymentsRejectData, types::PaymentsResponseData>,

        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<PaymentData<F>>
    where
        F: 'b + Send,
    {
        payment_data = payment_response_update_tracker(
            db,
            payment_id,
            payment_data,
            router_data,
            storage_scheme,
        )
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
        db: &dyn StorageInterface,
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

        payment_data = payment_response_update_tracker(
            db,
            payment_id,
            payment_data,
            router_data,
            storage_scheme,
        )
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
        db: &dyn StorageInterface,
        payment_id: &api::PaymentIdType,
        payment_data: PaymentData<F>,
        response: types::RouterData<F, types::CompleteAuthorizeData, types::PaymentsResponseData>,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<PaymentData<F>>
    where
        F: 'b + Send,
    {
        payment_response_update_tracker(db, payment_id, payment_data, response, storage_scheme)
            .await
    }
}

#[instrument(skip_all)]
async fn payment_response_update_tracker<F: Clone, T: types::Capturable>(
    db: &dyn StorageInterface,
    _payment_id: &api::PaymentIdType,
    mut payment_data: PaymentData<F>,
    router_data: types::RouterData<F, T, types::PaymentsResponseData>,
    storage_scheme: enums::MerchantStorageScheme,
) -> RouterResult<PaymentData<F>> {
    let (capture_update, mut payment_attempt_update, connector_response_update) = match router_data
        .response
        .clone()
    {
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
                    let flow_name = core_utils::get_flow_name::<F>()?;
                    let status =
                        // mark previous attempt status for technical failures in PSync flow
                        if flow_name == "PSync" {
                            match err.status_code {
                                // marking failure for 2xx because this is genuine payment failure
                                200..=299 => storage::enums::AttemptStatus::Failure,
                                _ => router_data.status,
                            }
                        } else {
                            match err.status_code {
                                500..=511 => storage::enums::AttemptStatus::Pending,
                                _ => storage::enums::AttemptStatus::Failure,
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
                            amount_capturable: if status.is_terminal_status()
                                || router_data
                                    .status
                                    .maps_to_intent_status(enums::IntentStatus::Processing)
                            {
                                Some(0)
                            } else {
                                None
                            },
                            updated_by: storage_scheme.to_string(),
                        }),
                    )
                }
            };
            (
                capture_update,
                attempt_update,
                Some(storage::ConnectorResponseUpdate::ErrorUpdate {
                    connector_name: Some(router_data.connector.clone()),
                    updated_by: storage_scheme.to_string(),
                }),
            )
        }
        Ok(payments_response) => match payments_response {
            types::PaymentsResponseData::PreProcessingResponse {
                pre_processing_id,
                connector_metadata,
                connector_response_reference_id,
                ..
            } => {
                let connector_transaction_id = match pre_processing_id.to_owned() {
                    types::PreprocessingResponseId::PreProcessingId(_) => None,
                    types::PreprocessingResponseId::ConnectorTransactionId(connector_txn_id) => {
                        Some(connector_txn_id)
                    }
                };
                let preprocessing_step_id = match pre_processing_id {
                    types::PreprocessingResponseId::PreProcessingId(pre_processing_id) => {
                        Some(pre_processing_id)
                    }
                    types::PreprocessingResponseId::ConnectorTransactionId(_) => None,
                };
                let payment_attempt_update = storage::PaymentAttemptUpdate::PreprocessingUpdate {
                    status: router_data.status,
                    payment_method_id: Some(router_data.payment_method_id),
                    connector_metadata,
                    preprocessing_step_id,
                    connector_transaction_id,
                    connector_response_reference_id,
                    updated_by: storage_scheme.to_string(),
                };

                (None, Some(payment_attempt_update), None)
            }
            types::PaymentsResponseData::TransactionResponse {
                resource_id,
                redirection_data,
                connector_metadata,
                connector_response_reference_id,
                ..
            } => {
                let connector_transaction_id = match resource_id {
                    types::ResponseId::NoResponseId => None,
                    types::ResponseId::ConnectorTransactionId(id)
                    | types::ResponseId::EncodedData(id) => Some(id),
                };

                let encoded_data = payment_data.connector_response.encoded_data.clone();
                let connector_name = router_data.connector.clone();

                let authentication_data = redirection_data
                    .map(|data| utils::Encode::<RedirectForm>::encode_to_value(&data))
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
                    metrics::SUCCESSFUL_PAYMENT.add(&metrics::CONTEXT, 1, &[]);
                }

                utils::add_apple_pay_payment_status_metrics(
                    router_data.status,
                    router_data.apple_pay_flow,
                    payment_data.payment_attempt.connector.clone(),
                    payment_data.payment_attempt.merchant_id.clone(),
                );

                let (capture_updates, payment_attempt_update) =
                    match payment_data.multiple_capture_data {
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
                                status: router_data.status,
                                connector: None,
                                connector_transaction_id: connector_transaction_id.clone(),
                                authentication_type: None,
                                payment_method_id: Some(router_data.payment_method_id),
                                mandate_id: payment_data
                                    .mandate_id
                                    .clone()
                                    .map(|mandate| mandate.mandate_id),
                                connector_metadata,
                                payment_token: None,
                                error_code: error_status.clone(),
                                error_message: error_status.clone(),
                                error_reason: error_status,
                                connector_response_reference_id,
                                amount_capturable: if router_data.status.is_terminal_status()
                                    || router_data
                                        .status
                                        .maps_to_intent_status(enums::IntentStatus::Processing)
                                {
                                    Some(0)
                                } else {
                                    None
                                },
                                updated_by: storage_scheme.to_string(),
                            }),
                        ),
                    };

                let connector_response_update = storage::ConnectorResponseUpdate::ResponseUpdate {
                    connector_transaction_id,
                    authentication_data,
                    encoded_data,
                    connector_name: Some(connector_name),
                    updated_by: storage_scheme.to_string(),
                };

                (
                    capture_updates,
                    payment_attempt_update,
                    Some(connector_response_update),
                )
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
                        status: router_data.status,
                        connector: None,
                        connector_transaction_id,
                        payment_method_id: Some(router_data.payment_method_id),
                        error_code: Some(reason.clone().map(|cd| cd.code)),
                        error_message: Some(reason.clone().map(|cd| cd.message)),
                        error_reason: Some(reason.map(|cd| cd.message)),
                        connector_response_reference_id,
                        updated_by: storage_scheme.to_string(),
                    }),
                    None,
                )
            }
            types::PaymentsResponseData::SessionResponse { .. } => (None, None, None),
            types::PaymentsResponseData::SessionTokenResponse { .. } => (None, None, None),
            types::PaymentsResponseData::TokenizationResponse { .. } => (None, None, None),
            types::PaymentsResponseData::ConnectorCustomerResponse { .. } => (None, None, None),
            types::PaymentsResponseData::ThreeDSEnrollmentResponse { .. } => (None, None, None),
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
        },
    };
    payment_data.multiple_capture_data = match capture_update {
        Some((mut multiple_capture_data, capture_updates)) => {
            for (capture, capture_update) in capture_updates {
                let updated_capture = db
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
    let connector_response = payment_data.connector_response.clone();

    let payment_attempt_fut = Box::pin(async move {
        Ok::<_, error_stack::Report<errors::ApiErrorResponse>>(match payment_attempt_update {
            Some(payment_attempt_update) => db
                .update_payment_attempt_with_attempt_id(
                    payment_attempt,
                    payment_attempt_update,
                    storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?,
            None => payment_attempt,
        })
    });

    let connector_response_fut = Box::pin(async move {
        Ok::<_, error_stack::Report<errors::ApiErrorResponse>>(match connector_response_update {
            Some(connector_response_update) => db
                .update_connector_response(
                    connector_response,
                    connector_response_update,
                    storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?,
            None => connector_response,
        })
    });

    let (payment_attempt, connector_response) =
        futures::try_join!(payment_attempt_fut, connector_response_fut)?;
    payment_data.payment_attempt = payment_attempt;
    payment_data.connector_response = connector_response;

    let amount_captured = get_total_amount_captured(
        router_data.request,
        router_data.amount_captured,
        router_data.status,
        &payment_data,
    );
    let payment_intent_update = match &router_data.response {
        Err(_) => storage::PaymentIntentUpdate::PGStatusUpdate {
            status: payment_data
                .payment_attempt
                .get_intent_status(payment_data.payment_intent.amount_captured),
            updated_by: storage_scheme.to_string(),
        },
        Ok(_) => storage::PaymentIntentUpdate::ResponseUpdate {
            status: payment_data
                .payment_attempt
                .get_intent_status(payment_data.payment_intent.amount_captured),
            return_url: router_data.return_url.clone(),
            amount_captured,
            updated_by: storage_scheme.to_string(),
        },
    };

    let payment_intent_fut = db
        .update_payment_intent(
            payment_data.payment_intent.clone(),
            payment_intent_update,
            storage_scheme,
        )
        .map(|x| x.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound));

    // When connector requires redirection for mandate creation it can update the connector mandate_id during Psync
    let mandate_update_fut = mandate::update_connector_mandate_id(
        db,
        router_data.merchant_id,
        payment_data.mandate_id.clone(),
        router_data.response.clone(),
    );

    let (payment_intent, _) = futures::try_join!(payment_intent_fut, mandate_update_fut)?;
    payment_data.payment_intent = payment_intent;

    Ok(payment_data)
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
            let amount = request.get_capture_amount();
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
