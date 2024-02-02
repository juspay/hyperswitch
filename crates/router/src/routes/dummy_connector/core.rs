use app::AppState;
use common_utils::generate_id_with_default_len;
use error_stack::ResultExt;

use super::{errors, types, utils};
use crate::{
    routes::{app, dummy_connector::consts},
    services::api,
    utils::OptionExt,
};

/// Asynchronously processes a payment request using the provided state and payment request, and returns a response containing the payment data.
pub async fn payment(
    state: AppState,
    req: types::DummyConnectorPaymentRequest,
) -> types::DummyConnectorResponse<types::DummyConnectorPaymentResponse> {
    utils::tokio_mock_sleep(
        state.conf.dummy_connector.payment_duration,
        state.conf.dummy_connector.payment_tolerance,
    )
    .await;

    let payment_attempt: types::DummyConnectorPaymentAttempt = req.into();
    let payment_data =
        types::DummyConnectorPaymentData::process_payment_attempt(&state, payment_attempt)?;

    utils::store_data_in_redis(
        &state,
        payment_data.attempt_id.clone(),
        payment_data.payment_id.clone(),
        state.conf.dummy_connector.authorize_ttl,
    )
    .await?;
    utils::store_data_in_redis(
        &state,
        payment_data.payment_id.clone(),
        payment_data.clone(),
        state.conf.dummy_connector.payment_ttl,
    )
    .await?;
    Ok(api::ApplicationResponse::Json(payment_data.into()))
}

/// This method retrieves payment data from the dummy connector service, with an asynchronous functionality. It uses the provided application state and request to retrieve payment data, and then returns a response containing the payment data.
pub async fn payment_data(
    state: AppState,
    req: types::DummyConnectorPaymentRetrieveRequest,
) -> types::DummyConnectorResponse<types::DummyConnectorPaymentResponse> {
    utils::tokio_mock_sleep(
        state.conf.dummy_connector.payment_retrieve_duration,
        state.conf.dummy_connector.payment_retrieve_tolerance,
    )
    .await;

    let payment_data = utils::get_payment_data_from_payment_id(&state, req.payment_id).await?;
    Ok(api::ApplicationResponse::Json(payment_data.into()))
}

/// Asynchronously authorizes a payment using the provided state and request data. It retrieves payment data by attempt id from the state, and then checks if the payment data exists. If it does, it generates a return URL, gets the authorize page content using the payment data, return URL, and dummy connector configuration, and then returns the authorize page content as a file data response. If the payment data does not exist, it returns an expired page content as a file data response.
pub async fn payment_authorize(
    state: AppState,
    req: types::DummyConnectorPaymentConfirmRequest,
) -> types::DummyConnectorResponse<String> {
    let payment_data = utils::get_payment_data_by_attempt_id(&state, req.attempt_id.clone()).await;
    let dummy_connector_conf = &state.conf.dummy_connector;

    if let Ok(payment_data_inner) = payment_data {
        let return_url = format!(
            "{}/dummy-connector/complete/{}",
            state.conf.server.base_url, req.attempt_id
        );
        Ok(api::ApplicationResponse::FileData((
            utils::get_authorize_page(payment_data_inner, return_url, dummy_connector_conf)
                .as_bytes()
                .to_vec(),
            mime::TEXT_HTML,
        )))
    } else {
        Ok(api::ApplicationResponse::FileData((
            utils::get_expired_page(dummy_connector_conf)
                .as_bytes()
                .to_vec(),
            mime::TEXT_HTML,
        )))
    }
}

/// Asynchronously completes a payment process, updates the payment status in Redis, and returns a redirection response.
pub async fn payment_complete(
    state: AppState,
    req: types::DummyConnectorPaymentCompleteRequest,
) -> types::DummyConnectorResponse<()> {
    utils::tokio_mock_sleep(
        state.conf.dummy_connector.payment_duration,
        state.conf.dummy_connector.payment_tolerance,
    )
    .await;

    let payment_data = utils::get_payment_data_by_attempt_id(&state, req.attempt_id.clone()).await;

    let payment_status = if req.confirm {
        types::DummyConnectorStatus::Succeeded
    } else {
        types::DummyConnectorStatus::Failed
    };

    let redis_conn = state
        .store
        .get_redis_conn()
        .change_context(errors::DummyConnectorErrors::InternalServerError)
        .attach_printable("Failed to get redis connection")?;

    let _ = redis_conn.delete_key(req.attempt_id.as_str()).await;

    if let Ok(payment_data) = payment_data {
        let updated_payment_data = types::DummyConnectorPaymentData {
            status: payment_status,
            next_action: None,
            ..payment_data
        };
        utils::store_data_in_redis(
            &state,
            updated_payment_data.payment_id.clone(),
            updated_payment_data.clone(),
            state.conf.dummy_connector.payment_ttl,
        )
        .await?;
        return Ok(api::ApplicationResponse::JsonForRedirection(
            api_models::payments::RedirectionResponse {
                return_url: String::new(),
                params: vec![],
                return_url_with_query_params: updated_payment_data
                    .return_url
                    .unwrap_or(state.conf.dummy_connector.default_return_url.clone()),
                http_method: "GET".to_string(),
                headers: vec![],
            },
        ));
    }
    Ok(api::ApplicationResponse::JsonForRedirection(
        api_models::payments::RedirectionResponse {
            return_url: String::new(),
            params: vec![],
            return_url_with_query_params: state.conf.dummy_connector.default_return_url.clone(),
            http_method: "GET".to_string(),
            headers: vec![],
        },
    ))
}

/// Asynchronously refunds a payment by waiting for a specified duration, then deducts the refund amount from the payment data, stores the updated payment data in Redis, generates a refund ID, creates a refund response with the refund details, and stores the refund data in Redis. Returns a JSON response with the refund data.
pub async fn refund_payment(
    state: AppState,
    req: types::DummyConnectorRefundRequest,
) -> types::DummyConnectorResponse<types::DummyConnectorRefundResponse> {
    utils::tokio_mock_sleep(
        state.conf.dummy_connector.refund_duration,
        state.conf.dummy_connector.refund_tolerance,
    )
    .await;

    let payment_id = req
        .payment_id
        .get_required_value("payment_id")
        .change_context(errors::DummyConnectorErrors::MissingRequiredField {
            field_name: "payment_id",
        })?;

    let mut payment_data =
        utils::get_payment_data_from_payment_id(&state, payment_id.clone()).await?;

    payment_data.is_eligible_for_refund(req.amount)?;

    let refund_id = generate_id_with_default_len(consts::REFUND_ID_PREFIX);
    payment_data.eligible_amount -= req.amount;

    utils::store_data_in_redis(
        &state,
        payment_id,
        payment_data.to_owned(),
        state.conf.dummy_connector.payment_ttl,
    )
    .await?;

    let refund_data = types::DummyConnectorRefundResponse::new(
        types::DummyConnectorStatus::Succeeded,
        refund_id.to_owned(),
        payment_data.currency,
        common_utils::date_time::now(),
        payment_data.amount,
        req.amount,
    );

    utils::store_data_in_redis(
        &state,
        refund_id,
        refund_data.to_owned(),
        state.conf.dummy_connector.refund_ttl,
    )
    .await?;
    Ok(api::ApplicationResponse::Json(refund_data))
}

/// This method is responsible for retrieving refund data from a Redis database, based on the given refund ID.
/// It first sleeps for a specified duration and tolerance, then retrieves the Redis connection from the application state, and finally fetches and deserializes the refund data based on the refund ID. If the refund data is found, it returns a JSON response containing the refund data.
pub async fn refund_data(
    state: AppState,
    req: types::DummyConnectorRefundRetrieveRequest,
) -> types::DummyConnectorResponse<types::DummyConnectorRefundResponse> {
    let refund_id = req.refund_id;
    utils::tokio_mock_sleep(
        state.conf.dummy_connector.refund_retrieve_duration,
        state.conf.dummy_connector.refund_retrieve_tolerance,
    )
    .await;

    let redis_conn = state
        .store
        .get_redis_conn()
        .change_context(errors::DummyConnectorErrors::InternalServerError)
        .attach_printable("Failed to get redis connection")?;
    let refund_data = redis_conn
        .get_and_deserialize_key::<types::DummyConnectorRefundResponse>(
            refund_id.as_str(),
            "DummyConnectorRefundResponse",
        )
        .await
        .change_context(errors::DummyConnectorErrors::RefundNotFound)?;
    Ok(api::ApplicationResponse::Json(refund_data))
}
