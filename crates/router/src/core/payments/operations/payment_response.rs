use async_trait::async_trait;
use error_stack::{report, ResultExt};
use router_derive;

use super::{Operation, PostUpdateTracker};
use crate::{
    core::{
        errors::{self, RouterResult, StorageErrorExt},
        payments::PaymentData,
    },
    db::Db,
    services::RedirectForm,
    types::{
        self, api,
        storage::{self, enums},
    },
    utils::{self, OptionExt},
};

#[derive(Debug, Clone, Copy, router_derive::PaymentOperation)]
#[operation(
    ops = "post_tracker",
    flow = "syncdata,authorizedata,canceldata,capturedata"
)]
pub struct PaymentResponse;

#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentData<F>, types::PaymentsRequestData>
    for PaymentResponse
{
    async fn update_tracker<'b>(
        &'b self,
        db: &dyn Db,
        payment_id: &api::PaymentIdType,
        mut payment_data: PaymentData<F>,
        response: Option<
            types::RouterData<F, types::PaymentsRequestData, types::PaymentsResponseData>,
        >,
    ) -> RouterResult<PaymentData<F>>
    where
        F: 'b + Send,
    {
        let router_data = response.ok_or(report!(errors::ApiErrorResponse::InternalServerError))?;
        payment_data.mandate_id = payment_data
            .mandate_id
            .or_else(|| router_data.request.mandate_id.clone());
        Ok(payment_response_ut(db, payment_id, payment_data, Some(router_data)).await?)
    }
}

#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentData<F>, types::PaymentsRequestSyncData>
    for PaymentResponse
{
    async fn update_tracker<'b>(
        &'b self,
        db: &dyn Db,
        payment_id: &api::PaymentIdType,
        payment_data: PaymentData<F>,
        response: Option<
            types::RouterData<F, types::PaymentsRequestSyncData, types::PaymentsResponseData>,
        >,
    ) -> RouterResult<PaymentData<F>>
    where
        F: 'b + Send,
    {
        Ok(payment_response_ut(db, payment_id, payment_data, response).await?)
    }
}

#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentData<F>, types::PaymentsRequestCaptureData>
    for PaymentResponse
{
    async fn update_tracker<'b>(
        &'b self,
        db: &dyn Db,
        payment_id: &api::PaymentIdType,
        payment_data: PaymentData<F>,
        response: Option<
            types::RouterData<F, types::PaymentsRequestCaptureData, types::PaymentsResponseData>,
        >,
    ) -> RouterResult<PaymentData<F>>
    where
        F: 'b + Send,
    {
        Ok(payment_response_ut(db, payment_id, payment_data, response).await?)
    }
}

#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentData<F>, types::PaymentRequestCancelData>
    for PaymentResponse
{
    async fn update_tracker<'b>(
        &'b self,
        db: &dyn Db,
        payment_id: &api::PaymentIdType,
        payment_data: PaymentData<F>,
        response: Option<
            types::RouterData<F, types::PaymentRequestCancelData, types::PaymentsResponseData>,
        >,
    ) -> RouterResult<PaymentData<F>>
    where
        F: 'b + Send,
    {
        Ok(payment_response_ut(db, payment_id, payment_data, response).await?)
    }
}

async fn payment_response_ut<F: Clone, T>(
    db: &dyn Db,
    _payment_id: &api::PaymentIdType,
    mut payment_data: PaymentData<F>,
    response: Option<types::RouterData<F, T, types::PaymentsResponseData>>,
) -> RouterResult<PaymentData<F>> {
    let router_data = response.ok_or(report!(errors::ApiErrorResponse::InternalServerError))?;
    let mut connector_response_data = None;

    let payment_attempt_update = match router_data.error_response.as_ref() {
        Some(err) => storage::PaymentAttemptUpdate::ErrorUpdate {
            status: storage::enums::AttemptStatus::Failure,
            error_message: Some(err.message.to_owned()),
        },
        None => {
            let response = router_data
                .response
                .get_required_value("router_data.response")?;

            connector_response_data = Some(response.clone());

            storage::PaymentAttemptUpdate::ResponseUpdate {
                status: router_data.status,
                connector_transaction_id: Some(response.connector_transaction_id),
                authentication_type: None,
                payment_method_id: Some(router_data.payment_method_id),
                redirect: Some(response.redirect),
                mandate_id: payment_data.mandate_id.clone(),
            }
        }
    };

    payment_data.payment_attempt = db
        .update_payment_attempt(payment_data.payment_attempt, payment_attempt_update)
        .await
        .map_err(|error| error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound))?;

    payment_data.connector_response = match connector_response_data {
        Some(connector_response) => {
            let authentication_data = connector_response
                .redirection_data
                .map(|data| utils::Encode::<RedirectForm>::encode_to_value(&data))
                .transpose()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Could not parse the connector response")?;

            let connector_response_update = storage::ConnectorResponseUpdate::ResponseUpdate {
                connector_transaction_id: Some(connector_response.connector_transaction_id.clone()),
                authentication_data,
                encoded_data: payment_data.connector_response.encoded_data.clone(),
            };

            db.update_connector_response(payment_data.connector_response, connector_response_update)
                .await
                .map_err(|error| {
                    error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
                })?
        }
        None => payment_data.connector_response,
    };

    let payment_intent_update = match router_data.error_response {
        Some(_) => storage::PaymentIntentUpdate::PGStatusUpdate {
            status: enums::IntentStatus::Failed,
        },
        None => storage::PaymentIntentUpdate::ResponseUpdate {
            status: router_data.status.into(),
            return_url: router_data.return_url,
            amount_captured: None,
        },
    };

    payment_data.payment_intent = db
        .update_payment_intent(payment_data.payment_intent, payment_intent_update)
        .await
        .map_err(|error| error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound))?;

    Ok(payment_data)
}
