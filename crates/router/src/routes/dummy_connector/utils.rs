use std::{fmt::Debug, sync::Arc};

use error_stack::{report, ResultExt};
use redis_interface::RedisConnectionPool;

use super::{errors, types};

pub async fn store_data_in_redis(
    redis_conn: Arc<RedisConnectionPool>,
    key: String,
    data: impl serde::Serialize + Debug,
    ttl: i64,
) -> types::DummyConnectorResult<()> {
    redis_conn
        .serialize_and_set_key_with_expiry(&key, data, ttl)
        .await
        .change_context(errors::DummyConnectorErrors::PaymentStoringError)
        .attach_printable("Failed to add data in redis")?;
    Ok(())
}

pub fn get_flow_from_card_number(
    card_number: &str,
) -> types::DummyConnectorResult<types::DummyConnectorFlow> {
    match card_number {
        "4111111111111111" | "4242424242424242" => Ok(types::DummyConnectorFlow::NoThreeDS(
            types::DummyConnectorStatus::Succeeded,
            None,
        )),
        "4000003800000446" => Ok(types::DummyConnectorFlow::ThreeDS(
            types::DummyConnectorStatus::Succeeded,
            None,
        )),
        _ => Err(report!(errors::DummyConnectorErrors::CardNotSupported)
            .attach_printable("The card is not supported")),
    }
}
