use app::AppState;
use common_utils::ext_traits::Encode;
use error_stack::{IntoReport, ResultExt};
use masking::ExposeInterface;
use rand::Rng;
use redis_interface::RedisConnectionPool;
use router_env::logger;
use tokio::time as tokio;

use super::{
    errors::DummyConnectorErrors,
    types::{
        DummyConnectorPaymentData, DummyConnectorPaymentMethodData, DummyConnectorPaymentsRequest,
        DummyConnectorPaymentsResponse, DummyConnectorResponse, DummyConnectorTransactionStatus,
    },
};
use crate::{connection, core::errors, routes::app, services::api};

pub async fn tokio_mock_sleep(delay: u64, tolerance: u64) {
    let mut rng = rand::thread_rng();
    let add: bool = rng.gen();
    let effective_delay = if add {
        delay + rng.gen_range(0..tolerance)
    } else {
        delay - rng.gen_range(0..tolerance)
    };
    tokio::sleep(tokio::Duration::from_millis(effective_delay)).await
}

pub async fn payment(
    state: &AppState,
    req: DummyConnectorPaymentsRequest,
) -> DummyConnectorResponse<DummyConnectorPaymentsResponse> {
    let payment_id = format!("dummy_{}", uuid::Uuid::new_v4());
    match req.payment_method_data {
        DummyConnectorPaymentMethodData::Card(card) => {
            let card_number: String = card.number.expose();
            tokio_mock_sleep(
                state.conf.dummy_connector.payment_duration,
                state.conf.dummy_connector.payment_tolerance,
            )
            .await;

            if card_number == "4111111111111111" || card_number == "4242424242424242" {
                let key_for_dummy_payment = format!("p_{}", payment_id);

                let mut redis_conn = connection::redis_connection(&state.conf).await;
                store_payment_data(
                    &redis_conn,
                    key_for_dummy_payment,
                    DummyConnectorPaymentData::new(
                        DummyConnectorTransactionStatus::Success.to_string(),
                        req.amount,
                        req.amount,
                        "card".to_string(),
                    ),
                    state.conf.dummy_connector.payment_ttl,
                )
                .await?;
                redis_conn.close_connections().await;

                Ok(api::ApplicationResponse::Json(
                    DummyConnectorPaymentsResponse::new(
                        DummyConnectorTransactionStatus::Success.to_string(),
                        payment_id,
                        req.amount,
                        String::from("card"),
                    ),
                ))
            } else {
                Ok(api::ApplicationResponse::Json(
                    DummyConnectorPaymentsResponse::new(
                        DummyConnectorTransactionStatus::Fail.to_string(),
                        payment_id,
                        req.amount,
                        String::from("card"),
                    ),
                ))
            }
        }
    }
}

async fn store_payment_data(
    redis_conn: &RedisConnectionPool,
    key: String,
    payment_data: DummyConnectorPaymentData,
    ttl: i64,
) -> Result<(), error_stack::Report<DummyConnectorErrors>> {
    let payment_data = Encode::<DummyConnectorPaymentData>::encode_to_string_of_json(&payment_data)
        .map_err(|error| {
            logger::error!(dummy_connector_payment_serialize_error=?error);
            DummyConnectorErrors::InternalServerError
        })?;

    redis_conn
        .set_key_with_expiry(&key, payment_data, ttl)
        .await
        .map_err(|error| {
            logger::error!(dummy_connector_payment_storage_error=?error);
            errors::StorageError::KVError
        })
        .into_report()
        .change_context(DummyConnectorErrors::PaymentStoringError)
        .attach_printable("Failed to add data in redis")?;
    Ok(())
}
