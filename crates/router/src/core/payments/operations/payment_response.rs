use async_trait::async_trait;
use error_stack::ResultExt;

use super::{EndOperation, PostUpdateTracker};
use crate::{
    core::{
        errors::{self, RouterResult, StorageErrorExt},
        payments::PaymentData,
    },
    db::StorageInterface,
    services::RedirectForm,
    types::{
        self, api,
        storage::{self, enums},
        transformers::ForeignInto,
    },
    utils,
};

#[derive(Debug, Clone, Copy)]
pub struct PaymentResponse;

impl<F: Send + Clone> EndOperation<F, types::PaymentsSyncData> for &PaymentResponse {
    fn to_post_update_tracker(
        &self,
    ) -> RouterResult<&(dyn PostUpdateTracker<F, PaymentData, types::PaymentsSyncData> + Send + Sync)>
    {
        Ok(*self)
    }
}
#[automatically_derived]
impl<F: Send + Clone> EndOperation<F, types::PaymentsAuthorizeData> for &PaymentResponse {
    fn to_post_update_tracker(
        &self,
    ) -> RouterResult<
        &(dyn PostUpdateTracker<F, PaymentData, types::PaymentsAuthorizeData> + Send + Sync),
    > {
        Ok(*self)
    }
}
#[automatically_derived]
impl<F: Send + Clone> EndOperation<F, types::PaymentsCancelData> for &PaymentResponse {
    fn to_post_update_tracker(
        &self,
    ) -> RouterResult<
        &(dyn PostUpdateTracker<F, PaymentData, types::PaymentsCancelData> + Send + Sync),
    > {
        Ok(*self)
    }
}
#[automatically_derived]
impl<F: Send + Clone> EndOperation<F, types::PaymentsCaptureData> for &PaymentResponse {
    fn to_post_update_tracker(
        &self,
    ) -> RouterResult<
        &(dyn PostUpdateTracker<F, PaymentData, types::PaymentsCaptureData> + Send + Sync),
    > {
        Ok(*self)
    }
}
#[automatically_derived]
impl<F: Send + Clone> EndOperation<F, types::VerifyRequestData> for &PaymentResponse {
    fn to_post_update_tracker(
        &self,
    ) -> RouterResult<
        &(dyn PostUpdateTracker<F, PaymentData, types::VerifyRequestData> + Send + Sync),
    > {
        Ok(*self)
    }
}
#[automatically_derived]
impl<F: Send + Clone> EndOperation<F, types::PaymentsSessionData> for &PaymentResponse {
    fn to_post_update_tracker(
        &self,
    ) -> RouterResult<
        &(dyn PostUpdateTracker<F, PaymentData, types::PaymentsSessionData> + Send + Sync),
    > {
        Ok(*self)
    }
}
#[automatically_derived]
impl<F: Send + Clone> EndOperation<F, types::PaymentsSyncData> for PaymentResponse {
    fn to_post_update_tracker(
        &self,
    ) -> RouterResult<&(dyn PostUpdateTracker<F, PaymentData, types::PaymentsSyncData> + Send + Sync)>
    {
        Ok(self)
    }
}
#[automatically_derived]
impl<F: Send + Clone> EndOperation<F, types::PaymentsAuthorizeData> for PaymentResponse {
    fn to_post_update_tracker(
        &self,
    ) -> RouterResult<
        &(dyn PostUpdateTracker<F, PaymentData, types::PaymentsAuthorizeData> + Send + Sync),
    > {
        Ok(self)
    }
}
#[automatically_derived]
impl<F: Send + Clone> EndOperation<F, types::PaymentsCancelData> for PaymentResponse {
    fn to_post_update_tracker(
        &self,
    ) -> RouterResult<
        &(dyn PostUpdateTracker<F, PaymentData, types::PaymentsCancelData> + Send + Sync),
    > {
        Ok(self)
    }
}
#[automatically_derived]
impl<F: Send + Clone> EndOperation<F, types::PaymentsCaptureData> for PaymentResponse {
    fn to_post_update_tracker(
        &self,
    ) -> RouterResult<
        &(dyn PostUpdateTracker<F, PaymentData, types::PaymentsCaptureData> + Send + Sync),
    > {
        Ok(self)
    }
}
#[automatically_derived]
impl<F: Send + Clone> EndOperation<F, types::VerifyRequestData> for PaymentResponse {
    fn to_post_update_tracker(
        &self,
    ) -> RouterResult<
        &(dyn PostUpdateTracker<F, PaymentData, types::VerifyRequestData> + Send + Sync),
    > {
        Ok(self)
    }
}
#[automatically_derived]
impl<F: Send + Clone> EndOperation<F, types::PaymentsSessionData> for PaymentResponse {
    fn to_post_update_tracker(
        &self,
    ) -> RouterResult<
        &(dyn PostUpdateTracker<F, PaymentData, types::PaymentsSessionData> + Send + Sync),
    > {
        Ok(self)
    }
}

#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentData, types::PaymentsAuthorizeData> for PaymentResponse {
    async fn update_tracker<'b>(
        &'b self,
        db: &dyn StorageInterface,
        payment_id: &api::PaymentIdType,
        mut payment_data: PaymentData,
        router_data: types::RouterData<
            F,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<PaymentData>
    where
        F: 'b + Send,
    {
        payment_data.mandate_id = payment_data
            .mandate_id
            .or_else(|| router_data.request.mandate_id.clone());

        payment_response_update_tracker(db, payment_id, payment_data, router_data, storage_scheme)
            .await
    }
}

#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentData, types::PaymentsSyncData> for PaymentResponse {
    async fn update_tracker<'b>(
        &'b self,
        db: &dyn StorageInterface,
        payment_id: &api::PaymentIdType,
        payment_data: PaymentData,
        response: types::RouterData<F, types::PaymentsSyncData, types::PaymentsResponseData>,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<PaymentData>
    where
        F: 'b + Send,
    {
        payment_response_update_tracker(db, payment_id, payment_data, response, storage_scheme)
            .await
    }
}

#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentData, types::PaymentsSessionData> for PaymentResponse {
    async fn update_tracker<'b>(
        &'b self,
        db: &dyn StorageInterface,
        payment_id: &api::PaymentIdType,
        payment_data: PaymentData,
        response: types::RouterData<F, types::PaymentsSessionData, types::PaymentsResponseData>,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<PaymentData>
    where
        F: 'b + Send,
    {
        payment_response_update_tracker(db, payment_id, payment_data, response, storage_scheme)
            .await
    }
}

#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentData, types::PaymentsCaptureData> for PaymentResponse {
    async fn update_tracker<'b>(
        &'b self,
        db: &dyn StorageInterface,
        payment_id: &api::PaymentIdType,
        payment_data: PaymentData,
        response: types::RouterData<F, types::PaymentsCaptureData, types::PaymentsResponseData>,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<PaymentData>
    where
        F: 'b + Send,
    {
        payment_response_update_tracker(db, payment_id, payment_data, response, storage_scheme)
            .await
    }
}

#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentData, types::PaymentsCancelData> for PaymentResponse {
    async fn update_tracker<'b>(
        &'b self,
        db: &dyn StorageInterface,
        payment_id: &api::PaymentIdType,
        payment_data: PaymentData,
        response: types::RouterData<F, types::PaymentsCancelData, types::PaymentsResponseData>,

        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<PaymentData>
    where
        F: 'b + Send,
    {
        payment_response_update_tracker(db, payment_id, payment_data, response, storage_scheme)
            .await
    }
}

#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentData, types::VerifyRequestData> for PaymentResponse {
    async fn update_tracker<'b>(
        &'b self,
        db: &dyn StorageInterface,
        payment_id: &api::PaymentIdType,
        mut payment_data: PaymentData,
        router_data: types::RouterData<F, types::VerifyRequestData, types::PaymentsResponseData>,

        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<PaymentData>
    where
        F: 'b + Send,
    {
        payment_data.mandate_id = payment_data.mandate_id.or_else(|| {
            router_data.request.mandate_id.clone()
            // .map(api_models::payments::MandateIds::new)
        });

        payment_response_update_tracker(db, payment_id, payment_data, router_data, storage_scheme)
            .await
    }
}

async fn payment_response_update_tracker<F: Clone, T>(
    db: &dyn StorageInterface,
    _payment_id: &api::PaymentIdType,
    mut payment_data: PaymentData,
    router_data: types::RouterData<F, T, types::PaymentsResponseData>,
    storage_scheme: enums::MerchantStorageScheme,
) -> RouterResult<PaymentData> {
    let (payment_attempt_update, connector_response_update) = match router_data.response.clone() {
        Err(err) => (
            Some(storage::PaymentAttemptUpdate::ErrorUpdate {
                connector: Some(router_data.connector.clone()),
                status: storage::enums::AttemptStatus::Failure,
                error_message: Some(err.message),
                error_code: Some(err.code),
            }),
            Some(storage::ConnectorResponseUpdate::ErrorUpdate {
                connector_name: Some(router_data.connector.clone()),
            }),
        ),
        Ok(payments_response) => match payments_response {
            types::PaymentsResponseData::TransactionResponse {
                resource_id,
                redirection_data,
                redirect,
                connector_metadata,
                ..
            } => {
                let connector_transaction_id = match resource_id {
                    types::ResponseId::NoResponseId => None,
                    types::ResponseId::ConnectorTransactionId(id)
                    | types::ResponseId::EncodedData(id) => Some(id),
                };

                let encoded_data = payment_data.connector_response.encoded_data.clone();
                let connector_name = payment_data.payment_attempt.connector.clone();

                let authentication_data = redirection_data
                    .map(|data| utils::Encode::<RedirectForm>::encode_to_value(&data))
                    .transpose()
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Could not parse the connector response")?;

                let payment_attempt_update = storage::PaymentAttemptUpdate::ResponseUpdate {
                    status: router_data.status,
                    connector: Some(router_data.connector),
                    connector_transaction_id: connector_transaction_id.clone(),
                    authentication_type: None,
                    payment_method_id: Some(router_data.payment_method_id),
                    redirect: Some(redirect),
                    mandate_id: payment_data
                        .mandate_id
                        .clone()
                        .map(|mandate| mandate.mandate_id),
                    connector_metadata,
                };

                let connector_response_update = storage::ConnectorResponseUpdate::ResponseUpdate {
                    connector_transaction_id,
                    authentication_data,
                    encoded_data,
                    connector_name,
                };

                (
                    Some(payment_attempt_update),
                    Some(connector_response_update),
                )
            }

            types::PaymentsResponseData::SessionResponse { .. } => (None, None),
        },
    };

    payment_data.payment_attempt = match payment_attempt_update {
        Some(payment_attempt_update) => db
            .update_payment_attempt(
                payment_data.payment_attempt,
                payment_attempt_update,
                storage_scheme,
            )
            .await
            .map_err(|error| {
                error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
            })?,
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
            .map_err(|error| {
                error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
            })?,
        None => payment_data.connector_response,
    };

    let payment_intent_update = match router_data.response {
        Err(_) => storage::PaymentIntentUpdate::PGStatusUpdate {
            status: enums::IntentStatus::Failed,
        },
        Ok(_) => storage::PaymentIntentUpdate::ResponseUpdate {
            status: router_data.status.foreign_into(),
            return_url: router_data.return_url,
            amount_captured: router_data.amount_captured,
        },
    };

    payment_data.payment_intent = db
        .update_payment_intent(
            payment_data.payment_intent,
            payment_intent_update,
            storage_scheme,
        )
        .await
        .map_err(|error| error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound))?;

    Ok(payment_data)
}
