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
        transformers::{ForeignFrom, ForeignTryFrom},
        Capturable,
    },
    utils,
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

async fn payment_response_update_tracker<F: Clone, T: Capturable>(
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
                Some(_) => (
                    Some(storage::CaptureUpdate::ErrorUpdate {
                        status: match err.status_code {
                            500..=511 => storage::enums::CaptureStatus::Pending,
                            _ => storage::enums::CaptureStatus::Failed,
                        },
                        error_code: Some(err.code),
                        error_message: Some(err.message),
                        error_reason: err.reason,
                    }),
                    // attempt status will depend on collective capture status
                    None,
                ),
                None => (
                    None,
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
                ),
            };
            (
                capture_update,
                attempt_update,
                Some(storage::ConnectorResponseUpdate::ErrorUpdate {
                    connector_name: Some(router_data.connector.clone()),
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

                let (capture_update, payment_attempt_update) =
                    match payment_data.multiple_capture_data {
                        Some(_) => (
                            //if payment_data.multiple_capture_data will be Some only for multiple partial capture.
                            Some(storage::CaptureUpdate::ResponseUpdate {
                                status: enums::CaptureStatus::foreign_try_from(router_data.status)?,
                                connector_transaction_id: connector_transaction_id.clone(),
                            }),
                            // attempt status will depend on collective capture status
                            None,
                        ),
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
                            }),
                        ),
                    };

                let connector_response_update = storage::ConnectorResponseUpdate::ResponseUpdate {
                    connector_transaction_id,
                    authentication_data,
                    encoded_data,
                    connector_name: Some(connector_name),
                };

                (
                    capture_update,
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
                    }),
                    None,
                )
            }
            types::PaymentsResponseData::SessionResponse { .. } => (None, None, None),
            types::PaymentsResponseData::SessionTokenResponse { .. } => (None, None, None),
            types::PaymentsResponseData::TokenizationResponse { .. } => (None, None, None),
            types::PaymentsResponseData::ConnectorCustomerResponse { .. } => (None, None, None),
            types::PaymentsResponseData::ThreeDSEnrollmentResponse { .. } => (None, None, None),
        },
    };
    payment_data.multiple_capture_data = match capture_update
        .zip(payment_data.multiple_capture_data)
    {
        Some((capture_update, mut multiple_capture_data)) => {
            let updated_capture = db
                .update_capture_with_capture_id(
                    multiple_capture_data.current_capture,
                    capture_update,
                    storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

            multiple_capture_data.current_capture = updated_capture;

            let authorized_amount = payment_data.payment_attempt.amount;

            payment_attempt_update = Some(storage::PaymentAttemptUpdate::MultipleCaptureUpdate {
                status: Some(multiple_capture_data.get_attempt_status(authorized_amount)),
                multiple_capture_count: Some(multiple_capture_data.get_captures_count()?),
            });
            Some(multiple_capture_data)
        }
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
    let amount_captured = get_total_amount_captured(
        router_data.request,
        router_data.amount_captured,
        router_data.status,
        &payment_data,
    );
    let payment_intent_update = match &router_data.response {
        Err(_) => storage::PaymentIntentUpdate::PGStatusUpdate {
            status: enums::IntentStatus::foreign_from(payment_data.payment_attempt.status),
        },
        Ok(_) => storage::PaymentIntentUpdate::ResponseUpdate {
            status: enums::IntentStatus::foreign_from(payment_data.payment_attempt.status),
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

fn get_total_amount_captured<F: Clone, T: Capturable>(
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
