use std::{fmt::Debug, sync::Arc};

use app::AppState;
use common_utils::generate_id;
use error_stack::{report, ResultExt};
use masking::PeekInterface;
use rand::Rng;
use redis_interface::RedisConnectionPool;
use tokio::time as tokio;

use super::{errors, types};
use crate::{routes::app, services::api, utils::OptionExt};

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

    let payment_id = generate_id(20, "dummy_pay");
    match req.payment_method_data {
        types::DummyConnectorPaymentMethodData::Card(card) => {
            let card_number = card.number.peek();

            match card_number.as_str() {
                "4111111111111111" | "4242424242424242" => {
                    let timestamp = common_utils::date_time::now();
                    let payment_data = types::DummyConnectorPaymentData::new(
                        types::DummyConnectorStatus::Succeeded,
                        req.amount,
                        req.amount,
                        req.currency,
                        timestamp.to_owned(),
                        types::PaymentMethodType::Card,
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
                            types::DummyConnectorStatus::Succeeded,
                            payment_id,
                            req.amount,
                            req.currency,
                            timestamp,
                            types::PaymentMethodType::Card,
                        ),
                    ))
                }
                _ => Err(report!(errors::DummyConnectorErrors::CardNotSupported)
                    .attach_printable("The card is not supported")),
            }
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
        .change_context(errors::DummyConnectorErrors::PaymentNotFound)?;

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
        .get_required_value("payment_id")
        .change_context(errors::DummyConnectorErrors::MissingRequiredField {
            field_name: "payment_id",
        })?;

    let redis_conn = state.store.get_redis_conn();
    let mut payment_data = redis_conn
        .get_and_deserialize_key::<types::DummyConnectorPaymentData>(
            payment_id.as_str(),
            "DummyConnectorPaymentData",
        )
        .await
        .change_context(errors::DummyConnectorErrors::PaymentNotFound)?;

    if payment_data.eligible_amount < req.amount {
        return Err(
            report!(errors::DummyConnectorErrors::RefundAmountExceedsPaymentAmount)
                .attach_printable("Eligible amount is lesser than refund amount"),
        );
    }

    if payment_data.status != types::DummyConnectorStatus::Succeeded {
        return Err(report!(errors::DummyConnectorErrors::PaymentNotSuccessful)
            .attach_printable("Payment is not successful to process the refund"));
    }

    let refund_id = generate_id(20, "dummy_ref");
    payment_data.eligible_amount -= req.amount;
    store_data_in_redis(
        redis_conn.to_owned(),
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
        .change_context(errors::DummyConnectorErrors::RefundNotFound)?;
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
        .change_context(errors::DummyConnectorErrors::PaymentStoringError)
        .attach_printable("Failed to add data in redis")?;
    Ok(())
}
