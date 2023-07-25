use std::fmt::Debug;

use common_utils::ext_traits::AsyncExt;
use error_stack::{report, IntoReport, ResultExt};
use masking::PeekInterface;
use maud::html;
use rand::Rng;
use tokio::time as tokio;

use super::{consts, errors, types};
use crate::routes::AppState;

pub async fn tokio_mock_sleep(delay: u64, tolerance: u64) {
    let mut rng = rand::thread_rng();
    let effective_delay = rng.gen_range((delay - tolerance)..(delay + tolerance));
    tokio::sleep(tokio::Duration::from_millis(effective_delay)).await
}

pub async fn store_data_in_redis(
    state: &AppState,
    key: String,
    data: impl serde::Serialize + Debug,
    ttl: i64,
) -> types::DummyConnectorResult<()> {
    let redis_conn = state.store.get_redis_conn();

    redis_conn
        .serialize_and_set_key_with_expiry(&key, data, ttl)
        .await
        .change_context(errors::DummyConnectorErrors::PaymentStoringError)
        .attach_printable("Failed to add data in redis")?;
    Ok(())
}

pub async fn get_payment_data_from_payment_id(
    state: &AppState,
    payment_id: String,
) -> types::DummyConnectorResult<types::DummyConnectorPaymentData> {
    let redis_conn = state.store.get_redis_conn();
    redis_conn
        .get_and_deserialize_key::<types::DummyConnectorPaymentData>(
            payment_id.as_str(),
            "types DummyConnectorPaymentData",
        )
        .await
        .change_context(errors::DummyConnectorErrors::PaymentNotFound)
}

pub async fn get_payment_data_by_attempt_id(
    state: &AppState,
    attempt_id: String,
) -> types::DummyConnectorResult<types::DummyConnectorPaymentData> {
    let redis_conn = state.store.get_redis_conn();
    redis_conn
        .get_and_deserialize_key::<String>(attempt_id.as_str(), "String")
        .await
        .async_and_then(|payment_id| async move {
            redis_conn
                .get_and_deserialize_key::<types::DummyConnectorPaymentData>(
                    payment_id.as_str(),
                    "DummyConnectorPaymentData",
                )
                .await
        })
        .await
        .change_context(errors::DummyConnectorErrors::PaymentNotFound)
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

pub trait ProcessPaymentAttempt {
    fn build_payment_data_from_payment_attempt(
        self,
        payment_attempt: types::DummyConnectorPaymentAttempt,
        redirect_url: String,
    ) -> types::DummyConnectorResult<types::DummyConnectorPaymentData>;
}

impl ProcessPaymentAttempt for types::DummyConnectorCard {
    fn build_payment_data_from_payment_attempt(
        self,
        payment_attempt: types::DummyConnectorPaymentAttempt,
        redirect_url: String,
    ) -> types::DummyConnectorResult<types::DummyConnectorPaymentData> {
        match self.get_flow_from_card_number()? {
            types::DummyConnectorFlow::NoThreeDS(status, error) => {
                if let Some(error) = error {
                    Err(error).into_report()?;
                }
                Ok(payment_attempt.build_payment_data(status, None, None))
            }
            types::DummyConnectorFlow::ThreeDS(_, _) => Ok(payment_attempt.build_payment_data(
                types::DummyConnectorStatus::Processing,
                Some(types::DummyConnectorNextAction::RedirectToUrl(redirect_url)),
                payment_attempt.payment_request.return_url,
            )),
        }
    }
}

impl types::DummyConnectorCard {
    pub fn get_flow_from_card_number(
        self,
    ) -> types::DummyConnectorResult<types::DummyConnectorFlow> {
        let card_number = self.number.peek();
        match card_number.as_str() {
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
}

impl ProcessPaymentAttempt for types::DummyConnectorWallet {
    fn build_payment_data_from_payment_attempt(
        self,
        payment_attempt: types::DummyConnectorPaymentAttempt,
        redirect_url: String,
    ) -> types::DummyConnectorResult<types::DummyConnectorPaymentData> {
        Ok(payment_attempt.build_payment_data(
            types::DummyConnectorStatus::Processing,
            Some(types::DummyConnectorNextAction::RedirectToUrl(redirect_url)),
            payment_attempt.payment_request.return_url,
        ))
    }
}

impl ProcessPaymentAttempt for types::DummyConnectorPayLater {
    fn build_payment_data_from_payment_attempt(
        self,
        payment_attempt: types::DummyConnectorPaymentAttempt,
        redirect_url: String,
    ) -> types::DummyConnectorResult<types::DummyConnectorPaymentData> {
        Ok(payment_attempt.build_payment_data(
            types::DummyConnectorStatus::Processing,
            Some(types::DummyConnectorNextAction::RedirectToUrl(redirect_url)),
            payment_attempt.payment_request.return_url,
        ))
    }
}

impl ProcessPaymentAttempt for types::PaymentMethodData {
    fn build_payment_data_from_payment_attempt(
        self,
        payment_attempt: types::DummyConnectorPaymentAttempt,
        redirect_url: String,
    ) -> types::DummyConnectorResult<types::DummyConnectorPaymentData> {
        match self {
            types::PaymentMethodData::Card(card) => {
                card.build_payment_data_from_payment_attempt(payment_attempt, redirect_url)
            }
            types::PaymentMethodData::Wallet(wallet) => {
                wallet.build_payment_data_from_payment_attempt(payment_attempt, redirect_url)
            }
            types::PaymentMethodData::PayLater(pay_later) => {
                pay_later.build_payment_data_from_payment_attempt(payment_attempt, redirect_url)
            }
        }
    }
}

impl types::DummyConnectorPaymentAttempt {
    pub fn process_payment_attempt(
        self,
        state: &AppState,
    ) -> types::DummyConnectorResult<types::DummyConnectorPaymentData> {
        let redirect_url = format!(
            "{}/dummy-connector/authorize/{}",
            state.conf.server.base_url, self.attempt_id
        );
        self.payment_request
            .payment_method_data
            .build_payment_data_from_payment_attempt(self, redirect_url)
    }
}
