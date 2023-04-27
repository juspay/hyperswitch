use app::AppState;
use error_stack::{IntoReport, ResultExt};
use masking::ExposeInterface;
use rand::Rng;
use redis_interface::RedisConnectionPool;
use tokio::time as tokio;

use super::{errors, types};
use crate::{connection, core::errors as api_errors, logger, routes::app, services::api};

pub async fn tokio_mock_sleep(delay: u64, tolerance: u64) {
    let mut rng = rand::thread_rng();
    let effective_delay = rng.gen_range((delay - tolerance)..(delay + tolerance));
    tokio::sleep(tokio::Duration::from_millis(effective_delay)).await
}

pub async fn payment(
    state: &AppState,
    req: types::DummyConnectorPaymentRequest,
) -> types::DummyConnectorResponse<types::DummyConnectorPaymentResponse> {
    let payment_id = format!("dummy_{}", uuid::Uuid::new_v4());
    match req.payment_method_data {
        types::DummyConnectorPaymentMethodData::Card(card) => {
            let card_number: String = card.number.expose();
            tokio_mock_sleep(
                state.conf.dummy_connector.payment_duration,
                state.conf.dummy_connector.payment_tolerance,
            )
            .await;

            if card_number == "4111111111111111" || card_number == "4242424242424242" {
                let key_for_dummy_payment = format!("p_{}", payment_id);

                let redis_conn = connection::redis_connection(&state.conf).await;
                store_payment_data(
                    &redis_conn,
                    key_for_dummy_payment,
                    types::DummyConnectorPaymentData::new(
                        types::DummyConnectorTransactionStatus::Success.to_string(),
                        req.amount,
                        req.amount,
                        "card".to_string(),
                    ),
                    state.conf.dummy_connector.payment_ttl,
                )
                .await?;

                Ok(api::ApplicationResponse::Json(
                    types::DummyConnectorPaymentResponse::new(
                        types::DummyConnectorTransactionStatus::Success.to_string(),
                        payment_id,
                        req.amount,
                        "card".to_string(),
                    ),
                ))
            } else {
                Ok(api::ApplicationResponse::Json(
                    types::DummyConnectorPaymentResponse::new(
                        types::DummyConnectorTransactionStatus::Fail.to_string(),
                        payment_id,
                        req.amount,
                        "card".to_string(),
                    ),
                ))
            }
        }
    }
}

async fn store_payment_data(
    redis_conn: &RedisConnectionPool,
    key: String,
    payment_data: types::DummyConnectorPaymentData,
    ttl: i64,
) -> Result<(), error_stack::Report<errors::DummyConnectorErrors>> {
    redis_conn
        .serialize_and_set_key_with_expiry(&key, payment_data, ttl)
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
