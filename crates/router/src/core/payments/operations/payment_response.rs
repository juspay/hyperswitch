use std::cmp::Ordering;

use async_trait::async_trait;
use common_utils::fp_utils;
use error_stack::ResultExt;
use router_derive;

use super::{Operation, PostUpdateTracker};
use crate::{
    core::{
        errors::{self, RouterResult, StorageErrorExt},
        mandate,
        payments::PaymentData,
    },
    db::StorageInterface,
    routes::metrics,
    services::RedirectForm,
    types::{
        self, api,
        storage::{self, enums},
        transformers::{ForeignFrom, ForeignInto},
        Capturable,
    },
    utils::{self, OptionExt},
};

#[derive(Debug, Clone, Copy, router_derive::PaymentOperation)]
#[operation(
    ops = "post_tracker",
    flow = "syncdata,authorizedata,canceldata,capturedata,completeauthorizedata,verifydata,sessiondata"
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

        let router_response = router_data.response.clone();
        let connector = router_data.connector.clone();

        payment_data = payment_response_update_tracker(
            db,
            payment_id,
            payment_data,
            router_data,
            storage_scheme,
        )
        .await?;

        router_response.map(|_| ()).or_else(|error_response| {
            fp_utils::when(
                !(200..300).contains(&error_response.status_code)
                    && !(500..=511).contains(&error_response.status_code),
                || {
                    Err(errors::ApiErrorResponse::ExternalConnectorError {
                        code: error_response.code,
                        message: error_response.message,
                        connector,
                        status_code: error_response.status_code,
                        reason: error_response.reason,
                    })
                },
            )
        })?;

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
        let router_response = router_data.response.clone();
        let connector = router_data.connector.clone();

        payment_data = payment_response_update_tracker(
            db,
            payment_id,
            payment_data,
            router_data,
            storage_scheme,
        )
        .await?;

        router_response.map_err(|error_response| {
            errors::ApiErrorResponse::ExternalConnectorError {
                message: error_response.message,
                code: error_response.code,
                status_code: error_response.status_code,
                reason: error_response.reason,
                connector,
            }
        })?;

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
        let router_response = router_data.response.clone();
        let connector = router_data.connector.clone();

        payment_data = capture_payment_response_update_tracker(
            db,
            payment_id,
            payment_data,
            router_data,
            storage_scheme,
        )
        .await?;

        router_response.map_err(|error_response| {
            errors::ApiErrorResponse::ExternalConnectorError {
                message: error_response.message,
                code: error_response.code,
                status_code: error_response.status_code,
                reason: error_response.reason,
                connector,
            }
        })?;

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
        let router_response = router_data.response.clone();
        let connector = router_data.connector.clone();

        payment_data = payment_response_update_tracker(
            db,
            payment_id,
            payment_data,
            router_data,
            storage_scheme,
        )
        .await?;

        router_response.map_err(|error_response| {
            errors::ApiErrorResponse::ExternalConnectorError {
                message: error_response.message,
                code: error_response.code,
                status_code: error_response.status_code,
                reason: error_response.reason,
                connector,
            }
        })?;

        Ok(payment_data)
    }
}

#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentData<F>, types::VerifyRequestData> for PaymentResponse {
    async fn update_tracker<'b>(
        &'b self,
        db: &dyn StorageInterface,
        payment_id: &api::PaymentIdType,
        mut payment_data: PaymentData<F>,
        router_data: types::RouterData<F, types::VerifyRequestData, types::PaymentsResponseData>,

        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<PaymentData<F>>
    where
        F: 'b + Send,
    {
        payment_data.mandate_id = payment_data.mandate_id.or_else(|| {
            router_data.request.mandate_id.clone()
            // .map(api_models::payments::MandateIds::new)
        });

        let router_response = router_data.response.clone();
        let connector = router_data.connector.clone();

        payment_data = payment_response_update_tracker(
            db,
            payment_id,
            payment_data,
            router_data,
            storage_scheme,
        )
        .await?;

        router_response.map_err(|error_response| {
            errors::ApiErrorResponse::ExternalConnectorError {
                message: error_response.message,
                code: error_response.code,
                status_code: error_response.status_code,
                reason: error_response.reason,
                connector,
            }
        })?;

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

fn is_multiple_capture_request(
    multiple_capture_status: Option<enums::CaptureStatus>,
    capture: Option<storage::Capture>,
    response: Result<types::PaymentsResponseData, types::ErrorResponse>,
) -> Option<(enums::CaptureStatus, storage::Capture)> {
    match response {
        Ok(_) => multiple_capture_status.zip(capture),
        Err(_) => capture.map(|capture| (enums::CaptureStatus::Pending, capture)),
    }
}

async fn capture_payment_response_update_tracker<F: Clone>(
    db: &dyn StorageInterface,
    payment_id: &api::PaymentIdType,
    mut payment_data: PaymentData<F>,
    router_data: types::RouterData<F, types::PaymentsCaptureData, types::PaymentsResponseData>,
    storage_scheme: enums::MerchantStorageScheme,
) -> RouterResult<PaymentData<F>> {
    if let Some((capture_status, capture)) = is_multiple_capture_request(
        router_data.multiple_capture_status,
        payment_data.capture.clone(),
        router_data.response.clone(),
    ) {
        let (payment_attempt_update, capture_update, connector_response_update) = match router_data
            .response
            .clone()
        {
            Err(err) => (
                Some(
                    storage::PaymentAttemptUpdate::MultipleCaptureResponseUpdate {
                        status: Some(payment_data.payment_attempt.status),
                        multiple_capture_count: Some(
                            payment_data
                                .payment_attempt
                                .multiple_capture_count
                                .unwrap_or(0)
                                + 1,
                        ),
                        succeeded_capture_count: None,
                    },
                ),
                Some(storage::CaptureUpdate::ErrorUpdate {
                    status: match err.status_code {
                        500..=511 => Some(storage::enums::CaptureStatus::Pending),
                        _ => Some(storage::enums::CaptureStatus::Failure),
                    },
                    error_message: Some(err.message),
                    error_code: Some(err.code),
                    error_reason: err.reason,
                }),
                Some(storage::ConnectorResponseUpdate::ErrorUpdate {
                    connector_name: Some(router_data.connector.clone()),
                }),
            ),
            Ok(payments_response) => match payments_response {
                types::PaymentsResponseData::TransactionResponse {
                    resource_id,
                    redirection_data,
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

                    if router_data.status == enums::AttemptStatus::Charged {
                        metrics::SUCCESSFUL_PAYMENT.add(&metrics::CONTEXT, 1, &[]);
                    }

                    let capture_update = storage::CaptureUpdate::ResponseUpdate {
                        status: Some(capture_status),
                        connector_transaction_id: connector_transaction_id.clone(),
                        error_code: None,
                        error_message: None,
                        error_reason: None,
                    };
                    let amount_captured = router_data
                        .amount_captured
                        .get_required_value("capture_amount")?;
                    let previously_captured_amount =
                        payment_data.payment_intent.amount_captured.unwrap_or(0);
                    let authorized_amount = payment_data.payment_attempt.amount;
                    let total_amount_captured = previously_captured_amount + amount_captured;

                    let attempt_status = match capture_status {
                        enums::CaptureStatus::Charged => {
                            match authorized_amount.cmp(&total_amount_captured) {
                                Ordering::Greater => enums::AttemptStatus::PartialCharged,
                                Ordering::Equal => enums::AttemptStatus::Charged,
                                Ordering::Less => {
                                    Err(errors::ApiErrorResponse::InvalidDataValue {
                                        field_name: "capture_amount", //ideally should not come till here
                                    })?
                                }
                            }
                        }
                        enums::CaptureStatus::Pending => enums::AttemptStatus::CaptureInitiated,
                        _ => payment_data.payment_attempt.status,
                    };

                    let payment_attempt_update =
                        storage::PaymentAttemptUpdate::MultipleCaptureResponseUpdate {
                            status: Some(attempt_status),
                            multiple_capture_count: Some(capture.capture_sequence),
                            succeeded_capture_count: if capture_status
                                == enums::CaptureStatus::Charged
                            {
                                payment_data
                                    .payment_attempt
                                    .succeeded_capture_count
                                    .or(Some(0))
                                    .map(|previous_count| previous_count + 1)
                            } else {
                                None
                            },
                        };

                    let connector_response_update =
                        storage::ConnectorResponseUpdate::ResponseUpdate {
                            connector_transaction_id,
                            authentication_data,
                            encoded_data,
                            connector_name: Some(connector_name),
                        };

                    (
                        Some(payment_attempt_update),
                        Some(capture_update),
                        Some(connector_response_update),
                    )
                }
                types::PaymentsResponseData::TransactionUnresolvedResponse { .. } => {
                    (None, None, None)
                }
                types::PaymentsResponseData::SessionResponse { .. } => (None, None, None),
                types::PaymentsResponseData::SessionTokenResponse { .. } => (None, None, None),
                types::PaymentsResponseData::TokenizationResponse { .. } => (None, None, None),
                types::PaymentsResponseData::ConnectorCustomerResponse { .. } => (None, None, None),
                types::PaymentsResponseData::ThreeDSEnrollmentResponse { .. } => (None, None, None),
                types::PaymentsResponseData::PreProcessingResponse { .. } => (None, None, None),
            },
        };

        payment_data.capture = match capture_update.zip(payment_data.capture) {
            Some((capture_update, capture)) => Some(
                db.update_capture_with_capture_id(capture, capture_update, storage_scheme)
                    .await
                    .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?,
            ),
            None => None,
        };
        payment_data.payment_attempt = match payment_attempt_update {
            Some(payment_attempt_update) => db
                .update_payment_attempt_with_attempt_id(
                    payment_data.payment_attempt,
                    payment_attempt_update,
                    storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?,
            None => payment_data.payment_attempt,
        };
        payment_data.connector_response = match connector_response_update {
            Some(connector_response_update) => db
                .update_connector_response(
                    payment_data.connector_response,
                    connector_response_update,
                    storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?,
            None => payment_data.connector_response,
        };
        let new_captured_amount = router_data
            .amount_captured
            .get_required_value("amount_captured")?;
        let updated_amount_captured = match capture_status {
            enums::CaptureStatus::Charged => {
                Some(payment_data.payment_intent.amount_captured.unwrap_or(0) + new_captured_amount)
            }
            enums::CaptureStatus::Started
            | enums::CaptureStatus::Pending
            | enums::CaptureStatus::Failure => None,
        };
        let payment_intent_update = match &router_data.response {
            Err(_) => None,
            Ok(_) => Some(storage::PaymentIntentUpdate::ResponseUpdate {
                status: enums::IntentStatus::foreign_from(payment_data.payment_attempt.status),
                return_url: router_data.return_url.clone(),
                amount_captured: updated_amount_captured,
            }),
        };

        payment_data.payment_intent = match payment_intent_update {
            Some(payment_intent_update) => db
                .update_payment_intent(
                    payment_data.payment_intent,
                    payment_intent_update,
                    storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?,
            None => payment_data.payment_intent,
        };

        Ok(payment_data)
    } else {
        payment_response_update_tracker(db, payment_id, payment_data, router_data, storage_scheme)
            .await
    }
}

async fn payment_response_update_tracker<F: Clone, T: Capturable>(
    db: &dyn StorageInterface,
    _payment_id: &api::PaymentIdType,
    mut payment_data: PaymentData<F>,
    router_data: types::RouterData<F, T, types::PaymentsResponseData>,
    storage_scheme: enums::MerchantStorageScheme,
) -> RouterResult<PaymentData<F>> {
    let (payment_attempt_update, connector_response_update) = match router_data.response.clone() {
        Err(err) => (
            Some(storage::PaymentAttemptUpdate::ErrorUpdate {
                connector: None,
                status: match err.status_code {
                    500..=511 => storage::enums::AttemptStatus::Pending,
                    _ => storage::enums::AttemptStatus::Failure,
                },
                error_message: Some(Some(err.message)),
                error_code: Some(Some(err.code)),
                error_reason: Some(err.reason),
            }),
            Some(storage::ConnectorResponseUpdate::ErrorUpdate {
                connector_name: Some(router_data.connector.clone()),
            }),
        ),
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
                };

                (Some(payment_attempt_update), None)
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

                let payment_attempt_update = storage::PaymentAttemptUpdate::ResponseUpdate {
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
                };

                let connector_response_update = storage::ConnectorResponseUpdate::ResponseUpdate {
                    connector_transaction_id,
                    authentication_data,
                    encoded_data,
                    connector_name: Some(connector_name),
                };

                (
                    Some(payment_attempt_update),
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
                    Some(storage::PaymentAttemptUpdate::UnresolvedResponseUpdate {
                        status: router_data.status,
                        connector: None,
                        connector_transaction_id,
                        payment_method_id: Some(router_data.payment_method_id),
                        error_code: Some(reason.clone().map(|cd| cd.code)),
                        error_message: Some(reason.clone().map(|cd| cd.message)),
                        error_reason: Some(reason.map(|cd| cd.message)),
                        connector_response_reference_id,
                    }),
                    None,
                )
            }
            types::PaymentsResponseData::SessionResponse { .. } => (None, None),
            types::PaymentsResponseData::SessionTokenResponse { .. } => (None, None),
            types::PaymentsResponseData::TokenizationResponse { .. } => (None, None),
            types::PaymentsResponseData::ConnectorCustomerResponse { .. } => (None, None),
            types::PaymentsResponseData::ThreeDSEnrollmentResponse { .. } => (None, None),
        },
    };

    payment_data.payment_attempt = match payment_attempt_update {
        Some(payment_attempt_update) => db
            .update_payment_attempt_with_attempt_id(
                payment_data.payment_attempt,
                payment_attempt_update,
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?,
        None => payment_data.payment_attempt,
    };

    payment_data.connector_response = match connector_response_update {
        Some(connector_response_update) => db
            .update_connector_response(
                payment_data.connector_response,
                connector_response_update,
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?,
        None => payment_data.connector_response,
    };
    let amount = router_data.request.get_capture_amount();

    let amount_captured = router_data.amount_captured.or_else(|| {
        if router_data.status == enums::AttemptStatus::Charged {
            amount
        } else {
            None
        }
    });
    let payment_intent_update = match &router_data.response {
        Err(err) => storage::PaymentIntentUpdate::PGStatusUpdate {
            status: match err.status_code {
                500..=511 => enums::IntentStatus::Processing,
                _ => enums::IntentStatus::Failed,
            },
        },
        Ok(_) => storage::PaymentIntentUpdate::ResponseUpdate {
            status: router_data.status.foreign_into(),
            return_url: router_data.return_url.clone(),
            amount_captured,
        },
    };

    payment_data.payment_intent = db
        .update_payment_intent(
            payment_data.payment_intent,
            payment_intent_update,
            storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    // When connector requires redirection for mandate creation it can update the connector mandate_id during Psync
    mandate::update_connector_mandate_id(
        db,
        router_data.merchant_id,
        payment_data.mandate_id.clone(),
        router_data.response.clone(),
    )
    .await?;

    Ok(payment_data)
}
