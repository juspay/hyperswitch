use std::{fmt::Debug, sync::Arc};

use app::AppState;
use error_stack::{report, IntoReport, ResultExt};
use masking::ExposeInterface;
use rand::Rng;
use redis_interface::RedisConnectionPool;
use tokio::time as tokio;

use super::{errors, types};
use crate::{core::errors as api_errors, logger, routes::app, services::api};

pub async fn tokio_mock_sleep(delay: u64, tolerance: u64) {
    let mut rng = rand::thread_rng();
    let effective_delay = rng.gen_range((delay - tolerance)..(delay + tolerance));
    tokio::sleep(tokio::Duration::from_millis(effective_delay)).await
}

pub async fn payment(
    state: &AppState,
    req: types::DummyConnectorPaymentRequest,
) -> types::DummyConnectorResponse<types::DummyConnectorPaymentResponse> {
    tokio_mock_sleep(
        state.conf.dummy_connector.payment_duration,
        state.conf.dummy_connector.payment_tolerance,
    )
    .await;

    let payment_id = format!("dummy_pay_{}", uuid::Uuid::new_v4());
    match req.payment_method_data {
        types::DummyConnectorPaymentMethodData::Card(card) => {
            let card_number: String = card.number.expose();

            if card_number != "4111111111111111" && card_number != "4242424242424242" {
                return Err(report!(errors::DummyConnectorErrors::CardNotSupported)
                    .attach_printable("The card is not supported"))
            }

            let timestamp = common_utils::date_time::date_as_yyyymmddthhmmssmmmz()
                .map_err(|_| errors::DummyConnectorErrors::InternalServerError)?;

            let payment_data = types::DummyConnectorPaymentData::new(
                types::DummyConnectorStatus::Succeeded.to_string(),
                req.amount,
                req.amount,
                req.currency,
                timestamp.to_owned(),
                "card".to_string(),
            );
            let redis_conn = state.store.get_redis_conn();
            store_data_in_redis(
                redis_conn,
                payment_id.to_owned(),
                payment_data,
                state.conf.dummy_connector.payment_ttl,
            )
            .await?;
            Ok(api::ApplicationResponse::Json(
                types::DummyConnectorPaymentResponse::new(
                    types::DummyConnectorStatus::Succeeded.to_string(),
                    payment_id,
                    req.amount,
                    req.currency,
                    timestamp,
                    "card".to_string(),
                ),
            ))
        }
    }
}

pub async fn payment_data(
    state: &AppState,
    req: types::DummyConnectorPaymentRetrieveRequest,
) -> types::DummyConnectorResponse<types::DummyConnectorPaymentResponse> {
    let payment_id = req.payment_id;
    tokio_mock_sleep(
        state.conf.dummy_connector.payment_retrieve_duration,
        state.conf.dummy_connector.payment_retrieve_tolerance,
    )
    .await;

    let redis_conn = state.store.get_redis_conn();
    let payment_data = redis_conn
        .get_and_deserialize_key::<types::DummyConnectorPaymentData>(
            payment_id.as_str(),
            "DummyConnectorPaymentData",
        )
        .await
        .map_err(|error| {
            logger::error!(dummy_connector_payment_deserialize_error=?error);
            errors::DummyConnectorErrors::PaymentNotFound
        })?;

    Ok(api::ApplicationResponse::Json(
        types::DummyConnectorPaymentResponse::new(
            payment_data.status,
            payment_id,
            payment_data.amount,
            payment_data.currency,
            payment_data.created,
            payment_data.payment_method_type,
        ),
    ))
}

pub async fn refund_payment(
    state: &AppState,
    req: types::DummyConnectorRefundRequest,
) -> types::DummyConnectorResponse<types::DummyConnectorRefundResponse> {
    tokio_mock_sleep(
        state.conf.dummy_connector.refund_duration,
        state.conf.dummy_connector.refund_tolerance,
    )
    .await;

    let payment_id = req
        .payment_id
        .ok_or(errors::DummyConnectorErrors::MissingRequiredField {
            field_name: "payment_id",
        })?;

    let redis_conn = state.store.get_redis_conn();
    let mut payment_data = redis_conn
        .get_and_deserialize_key::<types::DummyConnectorPaymentData>(
            payment_id.as_str(),
            "DummyConnectorPaymentData",
        )
        .await
        .map_err(|error| {
            logger::error!(dummy_connector_payment_deserialize_error=?error);
            errors::DummyConnectorErrors::PaymentNotFound
        })?;

    if payment_data.eligible_amount < req.amount {
        return Err(
            report!(errors::DummyConnectorErrors::RefundAmountExceedsPaymentAmount)
                .attach_printable("Eligible amount is lesser than refund amount"),
        );
    }

    if payment_data.status != types::DummyConnectorStatus::Succeeded.to_string() {
        return Err(report!(errors::DummyConnectorErrors::PaymentNotSuccessful)
            .attach_printable("Payment is not successful to process the refund"));
    }

    let refund_id = format!("dummy_ref_{}", uuid::Uuid::new_v4());
    payment_data.eligible_amount -= req.amount;
    store_data_in_redis(
        redis_conn.to_owned(),
        payment_id,
        payment_data.to_owned(),
        state.conf.dummy_connector.payment_ttl,
    )
    .await?;

    let refund_data = types::DummyConnectorRefundResponse::new(
        types::DummyConnectorStatus::Succeeded.to_string(),
        refund_id.to_owned(),
        payment_data.currency,
        common_utils::date_time::date_as_yyyymmddthhmmssmmmz()
            .map_err(|_| errors::DummyConnectorErrors::InternalServerError)?,
        payment_data.amount,
        req.amount,
    );

    store_data_in_redis(
        redis_conn,
        refund_id,
        refund_data.to_owned(),
        state.conf.dummy_connector.refund_ttl,
    )
    .await?;
    Ok(api::ApplicationResponse::Json(refund_data))
}

pub async fn refund_data(
    state: &AppState,
    req: types::DummyConnectorRefundRetrieveRequest,
) -> types::DummyConnectorResponse<types::DummyConnectorRefundResponse> {
    let refund_id = req.refund_id;
    tokio_mock_sleep(
        state.conf.dummy_connector.refund_retrieve_duration,
        state.conf.dummy_connector.refund_retrieve_tolerance,
    )
    .await;

    let redis_conn = state.store.get_redis_conn();
    let refund_data = redis_conn
        .get_and_deserialize_key::<types::DummyConnectorRefundResponse>(
            refund_id.as_str(),
            "DummyConnectorRefundResponse",
        )
        .await
        .map_err(|error| {
            logger::error!(dummy_connector_payment_deserialize_error=?error);
            errors::DummyConnectorErrors::RefundNotFound
        })?;
    Ok(api::ApplicationResponse::Json(refund_data))
}

async fn store_data_in_redis(
    redis_conn: Arc<RedisConnectionPool>,
    key: String,
    data: impl serde::Serialize + Debug,
    ttl: i64,
) -> Result<(), error_stack::Report<errors::DummyConnectorErrors>> {
    redis_conn
        .serialize_and_set_key_with_expiry(&key, data, ttl)
        .await
        .map_err(|error| {
            logger::error!(dummy_connector_payment_storage_error=?error);
            api_errors::StorageError::KVError
        })
        .into_report()
        .change_context(errors::DummyConnectorErrors::PaymentStoringError)
        .attach_printable("Failed to add data in redis")?;
    Ok(())
}
