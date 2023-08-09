use api_models::webhooks;
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
        webhooks::types::OutgoingWebhookTrigger,
    },
    routes::{metrics, AppState},
    services::RedirectForm,
    types::{
        self, api,
        storage::{self, enums},
        transformers::ForeignInto,
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
        state: &AppState,
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
            state,
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
        state: &AppState,
        payment_id: &api::PaymentIdType,
        payment_data: PaymentData<F>,
        router_data: types::RouterData<F, types::PaymentsSyncData, types::PaymentsResponseData>,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<PaymentData<F>>
    where
        F: 'b + Send,
    {
        let output = payment_response_update_tracker(
            state,
            payment_id,
            payment_data,
            router_data,
            storage_scheme,
        )
        .await?;

        // #[cfg(feature = "db_webhooks")]
        // {
        //     match actix_rt::Arbiter::try_current() {
        //         Some(arbiter) => {
        //             let state = state.clone();
        //             let payment_intent = output.payment_intent.clone();
        //             arbiter.spawn(async move {
        //                 match payment_intent.trigger_outgoing_webhook::<webhooks::OutgoingWebhook>(&state).await {
        //                     Ok(_) => crate::logger::info!("webhook successfully triggered"),
        //                     Err(err) => crate::logger::error!(error =? err, "Error while triggering state change webhook"),
        //                 }
        //             });
        //         }
        //         None => crate::logger::error!("Unable to fetch the arbiter"),
        //     }
        // }

        Ok(output)
    }
}

#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentData<F>, types::PaymentsSessionData>
    for PaymentResponse
{
    async fn update_tracker<'b>(
        &'b self,
        state: &AppState,
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
            state,
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
        state: &AppState,
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
            state,
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
        state: &AppState,
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
            state,
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
        state: &AppState,
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
            state,
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
        state: &AppState,
        payment_id: &api::PaymentIdType,
        payment_data: PaymentData<F>,
        response: types::RouterData<F, types::CompleteAuthorizeData, types::PaymentsResponseData>,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<PaymentData<F>>
    where
        F: 'b + Send,
    {
        payment_response_update_tracker(state, payment_id, payment_data, response, storage_scheme)
            .await
    }
}

async fn payment_response_update_tracker<F: Clone, T: types::Capturable>(
    state: &AppState,
    _payment_id: &api::PaymentIdType,
    mut payment_data: PaymentData<F>,
    router_data: types::RouterData<F, T, types::PaymentsResponseData>,
    storage_scheme: enums::MerchantStorageScheme,
) -> RouterResult<PaymentData<F>> {
    let db = &*state.store;
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
