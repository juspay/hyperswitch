use std::{fmt::Debug, sync::Arc};

use error_stack::{report, IntoReport, ResultExt};
use masking::PeekInterface;
use maud::html;
use redis_interface::RedisConnectionPool;

use super::{consts, errors, types};
use crate::routes::AppState;

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

pub fn get_authorize_page(
    payment_data: types::DummyConnectorPaymentData,
    return_url: String,
) -> String {
    let (image, mode) = match payment_data.payment_method_type {
        types::PaymentMethodType::Card => {
            ("https://hyperswitch.io/logos/hyperswitch.svg", "3D Secure")
        }
        types::PaymentMethodType::Wallet(wallet) => match wallet {
            types::DummyConnectorWallet::GooglePay => (
                "https://pay.google.com/about/static_kcs/images/logos/footer-logo.svg",
                "Google Pay",
            ),
            types::DummyConnectorWallet::Paypal => (
                "https://www.paypalobjects.com/digitalassets/c/website/logo/full-text/pp_fc_hl.svg",
                "PayPal",
            ),
            types::DummyConnectorWallet::WeChatPay => (
                "https://gtimg.wechatpay.cn/pay_en/img/common/logo.png",
                "WeChat Pay",
            ),
            types::DummyConnectorWallet::AliPay => (
                "https://upload.wikimedia.org/wikipedia/en/c/c7/Alipay_logo_%282020%29.svg",
                "AliPay",
            ),
            types::DummyConnectorWallet::AliPayHK => (
                "https://upload.wikimedia.org/wikipedia/en/c/c7/Alipay_logo_%282020%29.svg",
                "AliPay HK",
            ),
            types::DummyConnectorWallet::MbWay => (
                "https://upload.wikimedia.org/wikipedia/commons/e/e3/Logo_MBWay.svg",
                "MbWay",
            ),
        },
        types::PaymentMethodType::PayLater(_) => {
            ("https://hyperswitch.io/logos/hyperswitch.svg", "Pay Later")
        }
    };
    let amount = (payment_data.amount / 100) as f32;
    let currency = payment_data.currency.to_string();
    html! {
        head {
            title { "Authorize Payment" }
            style { (consts::THREE_DS_CSS) }
        }
        body {
            img src=(image) alt="Hyperswitch Logo" {}
            p { (format!("This is a test payment of {} {} through {}", amount, currency, mode)) }
            p { "Complete a required action for this payment" }
            div{
                button.authorize  onclick=({println!("Hello");format!("window.location.href='{}?confirm=true'", return_url)})
                    { "Continue Payment" }
                button.reject onclick=(format!("window.location.href='{}?confirm=false'", return_url))
                    { "Cancel Payment" }
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

pub fn handle_cards(
    state: &AppState,
    payment_request: types::DummyConnectorPaymentRequest,
    card: types::DummyConnectorCard,
    timestamp: time::PrimitiveDateTime,
    payment_id: String,
    attempt_id: String,
) -> types::DummyConnectorResult<types::DummyConnectorPaymentData> {
    let card_number = card.number.peek();
    match get_flow_from_card_number(card_number)? {
        types::DummyConnectorFlow::NoThreeDS(status, error) => {
            if let Some(error) = error {
                Err(error).into_report()?;
            }
            Ok(types::DummyConnectorPaymentData::new(
                payment_id.clone(),
                status,
                payment_request.amount,
                payment_request.amount,
                payment_request.currency,
                timestamp.clone(),
                types::PaymentMethodType::Card,
                None,
                None,
            ))
        }
        types::DummyConnectorFlow::ThreeDS(_, _) => Ok(types::DummyConnectorPaymentData::new(
            payment_id.clone(),
            types::DummyConnectorStatus::Processing,
            payment_request.amount,
            payment_request.amount,
            payment_request.currency,
            timestamp,
            types::PaymentMethodType::Card,
            Some(types::DummyConnectorNextAction::RedirectToUrl(format!(
                "{}/dummy-connector/authorize/{}",
                state.conf.server.base_url, attempt_id
            ))),
            payment_request.return_url,
        )),
    }
}

pub fn handle_wallets(
    state: &AppState,
    payment_request: types::DummyConnectorPaymentRequest,
    wallet: types::DummyConnectorWallet,
    timestamp: time::PrimitiveDateTime,
    payment_id: String,
    attempt_id: String,
) -> types::DummyConnectorPaymentData {
    let payment_data = types::DummyConnectorPaymentData::new(
        payment_id.clone(),
        types::DummyConnectorStatus::Processing,
        payment_request.amount,
        payment_request.amount,
        payment_request.currency,
        timestamp,
        types::PaymentMethodType::Wallet(wallet),
        Some(types::DummyConnectorNextAction::RedirectToUrl(format!(
            "{}/dummy-connector/authorize/{}",
            state.conf.server.base_url, attempt_id
        ))),
        payment_request.return_url,
    );
    payment_data
}

pub fn handle_pay_later(
    state: &AppState,
    payment_request: types::DummyConnectorPaymentRequest,
    pay_later: types::DummyConnectorPayLater,
    timestamp: time::PrimitiveDateTime,
    payment_id: String,
    attempt_id: String,
) -> types::DummyConnectorPaymentData {
    let payment_data = types::DummyConnectorPaymentData::new(
        payment_id.clone(),
        types::DummyConnectorStatus::Processing,
        payment_request.amount,
        payment_request.amount,
        payment_request.currency,
        timestamp,
        types::PaymentMethodType::PayLater(pay_later),
        Some(types::DummyConnectorNextAction::RedirectToUrl(format!(
            "{}/dummy-connector/authorize/{}",
            state.conf.server.base_url, attempt_id
        ))),
        payment_request.return_url,
    );
    payment_data
}
