use std::fmt::Debug;

use common_utils::ext_traits::AsyncExt;
use error_stack::{report, IntoReport, ResultExt};
use masking::PeekInterface;
use maud::html;
use rand::{distributions::Uniform, prelude::Distribution};
use tokio::time as tokio;

use super::{
    consts, errors,
    types::{self, GetPaymentMethodDetails},
};
use crate::{configs::settings, routes::AppState};

/// Asynchronously sleeps for a random duration within the specified tolerance
/// around the given delay, using the tokio runtime.
///
/// # Arguments
///
/// * `delay` - The desired delay in milliseconds
/// * `tolerance` - The tolerance around the delay in milliseconds
///
pub async fn tokio_mock_sleep(delay: u64, tolerance: u64) {
    let mut rng = rand::thread_rng();
    // TODO: change this to `Uniform::try_from`
    // this would require changing the fn signature
    // to return a Result
    let effective_delay = Uniform::from((delay - tolerance)..(delay + tolerance));
    tokio::sleep(tokio::Duration::from_millis(
        effective_delay.sample(&mut rng),
    ))
    .await
}

/// Asynchronously stores the provided data in Redis with the specified key and time-to-live (TTL).
///
/// # Arguments
///
/// * `state` - The application state containing the Redis connection
/// * `key` - The key under which the data will be stored in Redis
/// * `data` - The data to be serialized and stored in Redis
/// * `ttl` - The time-to-live (TTL) for the stored data in seconds
///
/// # Returns
///
/// This method returns a `DummyConnectorResult` indicating the success or failure of storing the data in Redis.
///
pub async fn store_data_in_redis(
    state: &AppState,
    key: String,
    data: impl serde::Serialize + Debug,
    ttl: i64,
) -> types::DummyConnectorResult<()> {
    let redis_conn = state
        .store
        .get_redis_conn()
        .change_context(errors::DummyConnectorErrors::InternalServerError)
        .attach_printable("Failed to get redis connection")?;

    redis_conn
        .serialize_and_set_key_with_expiry(&key, data, ttl)
        .await
        .change_context(errors::DummyConnectorErrors::PaymentStoringError)
        .attach_printable("Failed to add data in redis")?;
    Ok(())
}

/// Retrieves payment data from Redis using the provided payment ID.
///
/// # Arguments
///
/// * `state` - The application state containing the Redis connection.
/// * `payment_id` - The ID of the payment data to retrieve from Redis.
///
/// # Returns
///
/// A result containing the payment data if found, or an error if the payment data is not found or if there is an internal server error.
///
pub async fn get_payment_data_from_payment_id(
    state: &AppState,
    payment_id: String,
) -> types::DummyConnectorResult<types::DummyConnectorPaymentData> {
    let redis_conn = state
        .store
        .get_redis_conn()
        .change_context(errors::DummyConnectorErrors::InternalServerError)
        .attach_printable("Failed to get redis connection")?;

    redis_conn
        .get_and_deserialize_key::<types::DummyConnectorPaymentData>(
            payment_id.as_str(),
            "types DummyConnectorPaymentData",
        )
        .await
        .change_context(errors::DummyConnectorErrors::PaymentNotFound)
}
/// Retrieves payment data by attempt ID from the Redis store asynchronously. It first obtains a Redis connection from the application state, then uses it to retrieve a payment ID associated with the given attempt ID. Finally, it uses the payment ID to fetch the payment data from the Redis store. If successful, it returns the payment data; otherwise, it returns an error indicating that the payment was not found.
pub async fn get_payment_data_by_attempt_id(
    state: &AppState,
    attempt_id: String,
) -> types::DummyConnectorResult<types::DummyConnectorPaymentData> {
    let redis_conn = state
        .store
        .get_redis_conn()
        .change_context(errors::DummyConnectorErrors::InternalServerError)
        .attach_printable("Failed to get redis connection")?;

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

/// Generates an HTML page for authorizing a payment based on the provided payment data, return URL, and dummy connector configuration.
pub fn get_authorize_page(
    payment_data: types::DummyConnectorPaymentData,
    return_url: String,
    dummy_connector_conf: &settings::DummyConnector,
) -> String {
    let mode = payment_data.payment_method_type.get_name();
    let image = payment_data
        .payment_method_type
        .get_image_link(dummy_connector_conf.assets_base_url.as_str());
    let connector_image = payment_data
        .connector
        .get_connector_image_link(dummy_connector_conf.assets_base_url.as_str());
    let currency = payment_data.currency.to_string();

    html! {
        head {
            title { "Authorize Payment" }
            style { (consts::THREE_DS_CSS) }
            link rel="icon" href=(connector_image) {}
        }
        body {
            div.heading {
                img.logo src="https://app.hyperswitch.io/assets/Dark/hyperswitchLogoIconWithText.svg" alt="Hyperswitch Logo" {}
                h1 { "Test Payment Page" }
            }
            div.container {
                div.payment_details {
                    img src=(image) {}
                    div.border_horizontal {}
                    img src=(connector_image) {}
                }
                (maud::PreEscaped(
                    format!(r#"
                        <p class="disclaimer">
                            This is a test payment of <span id="amount"></span> {} using {}
                            <script>
                                document.getElementById("amount").innerHTML = ({} / 100).toFixed(2);
                            </script>
                        </p>
                        "#, currency, mode, payment_data.amount)
                    )
                )
                p { b { "Real money will not be debited for the payment." } " \
                        You can choose to simulate successful or failed payment while testing this payment." }
                div.user_action {
                    button.authorize onclick=(format!("window.location.href='{}?confirm=true'", return_url))
                        { "Complete Payment" }
                    button.reject onclick=(format!("window.location.href='{}?confirm=false'", return_url))
                        { "Reject Payment" }
                }
            }
            div.container {
                p.disclaimer { "What is this page?" }
                p { "This page is just a simulation for integration and testing purpose. \
                    In live mode, this page will not be displayed and the user will be taken to \
                    the Bank page (or) Google Pay cards popup (or) original payment method's page. \
                    Contact us for any queries."
                }
                div.contact {
                    div.contact_item.hover_cursor onclick=(dummy_connector_conf.slack_invite_url) {
                        img src="https://hyperswitch.io/logos/logo_slack.svg" alt="Slack Logo" {}
                    }
                    div.contact_item.hover_cursor onclick=(dummy_connector_conf.discord_invite_url) {
                        img src="https://hyperswitch.io/logos/logo_discord.svg" alt="Discord Logo" {}
                    }
                    div.border_vertical {}
                    div.contact_item.email {
                        p { "Or email us at" }
                        a href="mailto:hyperswitch@juspay.in" { "hyperswitch@juspay.in" }
                    }
                }
            }
        }
    }
    .into_string()
}

/// Retrieves an expired payment page with a disclaimer and contact information for integration and testing purposes. 
pub fn get_expired_page(dummy_connector_conf: &settings::DummyConnector) -> String {
    html! {
        head {
            title { "Authorize Payment" }
            style { (consts::THREE_DS_CSS) }
            link rel="icon" href="https://app.hyperswitch.io/HyperswitchFavicon.png" {}
        }
        body {
            div.heading {
                img.logo src="https://app.hyperswitch.io/assets/Dark/hyperswitchLogoIconWithText.svg" alt="Hyperswitch Logo" {}
                h1 { "Test Payment Page" }
            }
            div.container {
                p.disclaimer { "This link is not valid or it is expired" }
            }
            div.container {
                p.disclaimer { "What is this page?" }
                p { "This page is just a simulation for integration and testing purpose.\
                    In live mode, this is not visible. Contact us for any queries."
                }
                div.contact {
                    div.contact_item.hover_cursor onclick=(dummy_connector_conf.slack_invite_url) {
                        img src="https://hyperswitch.io/logos/logo_slack.svg" alt="Slack Logo" {}
                    }
                    div.contact_item.hover_cursor onclick=(dummy_connector_conf.discord_invite_url) {
                        img src="https://hyperswitch.io/logos/logo_discord.svg" alt="Discord Logo" {}
                    }
                    div.border_vertical {}
                    div.contact_item.email {
                        p { "Or email us at" }
                        a href="mailto:hyperswitch@juspay.in" { "hyperswitch@juspay.in" }
                    }
                }
            }
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
        /// Builds payment data from a given payment attempt and redirect URL. 
    fn build_payment_data_from_payment_attempt(
        self,
        payment_attempt: types::DummyConnectorPaymentAttempt,
        redirect_url: String,
    ) -> types::DummyConnectorResult<types::DummyConnectorPaymentData> {
        match self.get_flow_from_card_number()? {
            types::DummyConnectorCardFlow::NoThreeDS(status, error) => {
                if let Some(error) = error {
                    Err(error).into_report()?;
                }
                Ok(payment_attempt.build_payment_data(status, None, None))
            }
            types::DummyConnectorCardFlow::ThreeDS(_, _) => {
                Ok(payment_attempt.clone().build_payment_data(
                    types::DummyConnectorStatus::Processing,
                    Some(types::DummyConnectorNextAction::RedirectToUrl(redirect_url)),
                    payment_attempt.payment_request.return_url,
                ))
            }
        }
    }
}

impl types::DummyConnectorCard {
        /// Retrieves the card flow based on the card number. Returns a result containing the card flow or an error if the card is not supported.
    pub fn get_flow_from_card_number(
        self,
    ) -> types::DummyConnectorResult<types::DummyConnectorCardFlow> {
        let card_number = self.number.peek();
        match card_number.as_str() {
            "4111111111111111" | "4242424242424242" | "5555555555554444" | "38000000000006"
            | "378282246310005" | "6011111111111117" => {
                Ok(types::DummyConnectorCardFlow::NoThreeDS(
                    types::DummyConnectorStatus::Succeeded,
                    None,
                ))
            }
            "5105105105105100" | "4000000000000002" => {
                Ok(types::DummyConnectorCardFlow::NoThreeDS(
                    types::DummyConnectorStatus::Failed,
                    Some(errors::DummyConnectorErrors::PaymentDeclined {
                        message: "Card declined",
                    }),
                ))
            }
            "4000000000009995" => Ok(types::DummyConnectorCardFlow::NoThreeDS(
                types::DummyConnectorStatus::Failed,
                Some(errors::DummyConnectorErrors::PaymentDeclined {
                    message: "Insufficient funds",
                }),
            )),
            "4000000000009987" => Ok(types::DummyConnectorCardFlow::NoThreeDS(
                types::DummyConnectorStatus::Failed,
                Some(errors::DummyConnectorErrors::PaymentDeclined {
                    message: "Lost card",
                }),
            )),
            "4000000000009979" => Ok(types::DummyConnectorCardFlow::NoThreeDS(
                types::DummyConnectorStatus::Failed,
                Some(errors::DummyConnectorErrors::PaymentDeclined {
                    message: "Stolen card",
                }),
            )),
            "4000003800000446" => Ok(types::DummyConnectorCardFlow::ThreeDS(
                types::DummyConnectorStatus::Succeeded,
                None,
            )),
            _ => Err(report!(errors::DummyConnectorErrors::CardNotSupported)
                .attach_printable("The card is not supported")),
        }
    }
}

impl ProcessPaymentAttempt for types::DummyConnectorWallet {
        /// Builds a payment data from a payment attempt with the provided redirect URL.
        ///
        /// It takes a payment attempt and a redirect URL as input and constructs a payment data with the status set to Processing and a redirect action to the provided URL. It then returns a Result containing the constructed payment data.
        fn build_payment_data_from_payment_attempt(
            self,
            payment_attempt: types::DummyConnectorPaymentAttempt,
            redirect_url: String,
        ) -> types::DummyConnectorResult<types::DummyConnectorPaymentData> {
            Ok(payment_attempt.clone().build_payment_data(
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
        Ok(payment_attempt.clone().build_payment_data(
            types::DummyConnectorStatus::Processing,
            Some(types::DummyConnectorNextAction::RedirectToUrl(redirect_url)),
            payment_attempt.payment_request.return_url,
        ))
    }
}

impl ProcessPaymentAttempt for types::DummyConnectorPaymentMethodData {
        /// Builds payment data from the given payment attempt and redirect URL, by delegating the task to the appropriate payment method (card, wallet, or pay later) based on the current instance of DummyConnector.
    fn build_payment_data_from_payment_attempt(
        self,
        payment_attempt: types::DummyConnectorPaymentAttempt,
        redirect_url: String,
    ) -> types::DummyConnectorResult<types::DummyConnectorPaymentData> {
        match self {
            Self::Card(card) => {
                card.build_payment_data_from_payment_attempt(payment_attempt, redirect_url)
            }
            Self::Wallet(wallet) => {
                wallet.build_payment_data_from_payment_attempt(payment_attempt, redirect_url)
            }
            Self::PayLater(pay_later) => {
                pay_later.build_payment_data_from_payment_attempt(payment_attempt, redirect_url)
            }
        }
    }
}

impl types::DummyConnectorPaymentData {
        /// Process a payment attempt by building a redirect URL and constructing payment data from the payment attempt.
    pub fn process_payment_attempt(
        state: &AppState,
        payment_attempt: types::DummyConnectorPaymentAttempt,
    ) -> types::DummyConnectorResult<Self> {
        let redirect_url = format!(
            "{}/dummy-connector/authorize/{}",
            state.conf.server.base_url, payment_attempt.attempt_id
        );
        payment_attempt
            .clone()
            .payment_request
            .payment_method_data
            .build_payment_data_from_payment_attempt(payment_attempt, redirect_url)
    }
}
