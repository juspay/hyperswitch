use std::{fmt::Debug, sync::Arc};

use error_stack::{report, ResultExt};
use maud::html;
use redis_interface::RedisConnectionPool;

use super::{consts, errors, types};

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

pub fn get_authorize_page(amount: f64, return_url: String) -> String {
    html! {
        head {
            title { "Authorize Payment" }
            style { (consts::THREE_DS_CSS) }
        }
        body {
            img src="https://hyperswitch.io/logos/hyperswitch.svg" alt="Hyperswitch Logo" {}
            p { (format!("This is a test payment of ${} USD using 3D Secure", amount)) }
            p { "Complete a required action for this payment" }
            div{
                button.authorize  onclick=({println!("Hello");format!("window.location.href='{}?confirm=true'", return_url)})
                    { "Authorize Payment" }
                button.reject onclick=(format!("window.location.href='{}?confirm=false'", return_url))
                    { "Reject Payment" }
            }
        }
    }
    .into_string()
}

pub fn get_expired_page() -> String {
    html! {
        head {
            title { "Authorize Payment" }
            style { (consts::THREE_DS_CSS) }
        }
        body {
            img src="https://hyperswitch.io/logos/hyperswitch.svg" alt="Hyperswitch Logo" {}
            p { "This is link is not valid or expired" }
        }
    }
    .into_string()
}
