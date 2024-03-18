#![allow(
    dead_code,
    clippy::expect_used,
    clippy::missing_panics_doc,
    clippy::unwrap_used
)]

use actix_http::{body::MessageBody, Request};
use actix_web::{
    dev::{Service, ServiceResponse},
    test::{call_and_read_body_json, TestRequest},
};
use derive_deref::Deref;
use router::{configs::settings::Settings, routes::AppState, services};
use router_env::tracing::Instrument;
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::{json, Value};
use tokio::sync::{oneshot, OnceCell};

static SERVER: OnceCell<bool> = OnceCell::const_new();

async fn spawn_server() -> bool {
    let conf = Settings::new().expect("invalid settings");
    let server = Box::pin(router::start_server(conf))
        .await
        .expect("failed to create server");

    let _server = tokio::spawn(server).in_current_span();
    true
}

pub async fn setup() {
    Box::pin(SERVER.get_or_init(spawn_server)).await;
}

const STRIPE_MOCK: &str = "http://localhost:12111/";

async fn stripemock() -> Option<String> {
    // not working: https://github.com/stripe/stripe-mock/issues/231
    None
}

pub async fn mk_service(
) -> impl Service<Request, Response = ServiceResponse<impl MessageBody>, Error = actix_web::Error> {
    let mut conf = Settings::new().unwrap();
    let request_body_limit = conf.server.request_body_limit;

    if let Some(url) = stripemock().await {
        conf.connectors.stripe.base_url = url;
    }
    let tx: oneshot::Sender<()> = oneshot::channel().0;

    let app_state = Box::pin(AppState::with_storage(
        conf,
        router::db::StorageImpl::Mock,
        tx,
        Box::new(services::MockApiClient),
    ))
    .await;
    actix_web::test::init_service(router::mk_app(app_state, request_body_limit)).await
}

pub struct Guest;

pub struct Admin {
    authkey: String,
}

pub struct User {
    authkey: String,
}

#[allow(dead_code)]
pub struct AppClient<T> {
    state: T,
}

impl AppClient<Guest> {
    pub fn guest() -> Self {
        Self { state: Guest }
    }
}

impl AppClient<Admin> {
    pub async fn create_merchant_account<T: DeserializeOwned, S, B>(
        &self,
        app: &S,
        merchant_id: impl Into<Option<String>>,
    ) -> T
    where
        S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
        B: MessageBody,
    {
        let request = TestRequest::post()
            .uri("/accounts")
            .append_header(("api-key".to_owned(), self.state.authkey.clone()))
            .set_json(mk_merchant_account(merchant_id.into()))
            .to_request();

        call_and_read_body_json(app, request).await
    }

    pub async fn create_connector<T: DeserializeOwned, S, B>(
        &self,
        app: &S,
        merchant_id: &str,
        connector_name: &str,
        api_key: &str,
    ) -> T
    where
        S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
        B: MessageBody,
    {
        let request = TestRequest::post()
            .uri(&format!("/account/{merchant_id}/connectors"))
            .append_header(("api-key".to_owned(), self.state.authkey.clone()))
            .set_json(mk_connector(connector_name, api_key))
            .to_request();

        call_and_read_body_json(app, request).await
    }
}

impl AppClient<User> {
    pub async fn create_payment<T: DeserializeOwned, S, B>(
        &self,
        app: &S,
        amount: i64,
        amount_to_capture: i32,
    ) -> T
    where
        S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
        B: MessageBody,
    {
        let request = TestRequest::post()
            .uri("/payments")
            .append_header(("api-key".to_owned(), self.state.authkey.clone()))
            .set_json(mk_payment(amount, amount_to_capture))
            .to_request();
        call_and_read_body_json(app, request).await
    }

    pub async fn create_refund<T: DeserializeOwned, S, B>(
        &self,
        app: &S,
        payment_id: &str,
        amount: usize,
    ) -> T
    where
        S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
        B: MessageBody,
    {
        let request = TestRequest::post()
            .uri("/refunds")
            .append_header(("api-key".to_owned(), self.state.authkey.clone()))
            .set_json(mk_refund(payment_id, amount))
            .to_request();
        call_and_read_body_json(app, request).await
    }
}

impl<T> AppClient<T> {
    pub fn admin(&self, authkey: &str) -> AppClient<Admin> {
        AppClient {
            state: Admin {
                authkey: authkey.to_string(),
            },
        }
    }

    pub fn user(&self, authkey: &str) -> AppClient<User> {
        AppClient {
            state: User {
                authkey: authkey.to_string(),
            },
        }
    }

    pub async fn health<S, B>(&self, app: &S) -> String
    where
        S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
        B: MessageBody,
    {
        let request = TestRequest::get().uri("/health").to_request();
        let bytes = actix_web::test::call_and_read_body(app, request).await;
        String::from_utf8(bytes.to_vec()).unwrap()
    }
}

fn mk_merchant_account(merchant_id: Option<String>) -> Value {
    let merchant_id = merchant_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    json!({
      "merchant_id": merchant_id,
      "merchant_name": "NewAge Retailer",
      "merchant_details": {
        "primary_contact_person": "John Test",
        "primary_email": "JohnTest@test.com",
        "primary_phone": "sunt laborum",
        "secondary_contact_person": "John Test2",
        "secondary_email": "JohnTest2@test.com",
        "secondary_phone": "cillum do dolor id",
        "website": "www.example.com",
        "about_business": "Online Retail with a wide selection of organic products for North America",
        "address": {
          "line1": "Juspay Router",
          "line2": "Koramangala",
          "line3": "Stallion",
          "city": "Bangalore",
          "state": "Karnataka",
          "zip": "560095",
          "country": "IN"
        }
      },
      "return_url": "www.example.com/success",
      "webhook_details": {
        "webhook_version": "1.0.1",
        "webhook_username": "ekart_retail",
        "webhook_password": "password_ekart@123",
        "payment_created_enabled": true,
        "payment_succeeded_enabled": true,
        "payment_failed_enabled": true
      },
      "routing_algorithm": {
        "type": "single",
        "data": "stripe"
      },
      "sub_merchants_enabled": false,
      "metadata": {
        "city": "NY",
        "unit": "245"
      }
    })
}

fn mk_payment(amount: i64, amount_to_capture: i32) -> Value {
    json!({
      "amount": amount,
      "currency": "USD",
      "confirm": true,
      "capture_method": "automatic",
      "capture_on": "2022-10-10T10:11:12Z",
      "amount_to_capture": amount_to_capture,
      "customer_id": "cus_udst2tfldj6upmye2reztkmm4i",
      "email": "guest@example.com",
      "name": "John Doe",
      "phone": "999999999",
      "phone_country_code": "+65",
      "description": "Its my first payment request",
      "authentication_type": "no_three_ds",
      "payment_method": "card",
      "payment_method_data": {
        "card": {
          "card_number": "4242424242424242",
          "card_exp_month": "10",
          "card_exp_year": "35",
          "card_holder_name": "John Doe",
          "card_cvc": "123"
        }
      },
      "statement_descriptor_name": "Hyperswitch",
      "statement_descriptor_suffix": "Hyperswitch",
      "metadata": {
        "udf1": "value1",
        "new_customer": "true",
        "login_date": "2019-09-10T10:11:12Z"
      }
    })
}

fn mk_connector(connector_name: &str, api_key: &str) -> Value {
    json!({
      "connector_type": "fiz_operations",
      "connector_name": connector_name,
      "connector_account_details": {
        "auth_type": "HeaderKey",
        "api_key": api_key,
      },
      "test_mode": false,
      "disabled": false,
      "payment_methods_enabled": [
        {
          "payment_method": "wallet",
          "payment_method_types": [
            "upi_collect",
            "upi_intent"
          ],
          "payment_method_issuers": [
            "labore magna ipsum",
            "aute"
          ],
          "payment_schemes": [
            "Discover",
            "Discover"
          ],
          "accepted_currencies": [
            "AED",
            "AED"
          ],
          "accepted_countries": [
            "in",
            "us"
          ],
          "minimum_amount": 1,
          "maximum_amount": 68607706,
          "recurring_enabled": true,
          "installment_payment_enabled": true
        }
      ],
      "metadata": {
        "city": "NY",
        "unit": "245"
      }
    })
}

fn _mk_payment_confirm() -> Value {
    json!({
      "return_url": "http://example.com/payments",
      "setup_future_usage": "on_session",
      "authentication_type": "no_three_ds",
      "payment_method": "card",
      "payment_method_data": {
        "card": {
          "card_number": "4242424242424242",
          "card_exp_month": "10",
          "card_exp_year": "35",
          "card_holder_name": "John Doe",
          "card_cvc": "123"
        }
      },
      "shipping": {},
      "billing": {}
    })
}

fn mk_refund(payment_id: &str, amount: usize) -> Value {
    let timestamp = common_utils::date_time::now().to_string();

    json!({
      "payment_id": payment_id,
      "refund_id": timestamp.get(23..),
      "amount": amount,
      "reason": "Customer returned product",
      "metadata": {
        "udf1": "value1",
        "new_customer": "true",
        "login_date": "2019-09-10T10:11:12Z"
      }
    })
}

pub struct HNil;

impl<'de> Deserialize<'de> for HNil {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(serde::de::IgnoredAny)?;
        Ok(Self)
    }
}

#[derive(Deserialize)]
pub struct HCons<H, T> {
    #[serde(flatten)]
    pub head: H,
    #[serde(flatten)]
    pub tail: T,
}

#[macro_export]
macro_rules! HList {
    () => { $crate::utils::HNil };
    ($head:ty $(, $rest:ty)* $(,)?) => { $crate::utils::HCons<$head, HList![$($rest),*]> };
}

#[macro_export]
macro_rules! hlist_pat {
    () => { $crate::utils::HNil };
    ($head:pat $(, $rest:pat)* $(,)?) => { $crate::utils::HCons { head: $head, tail: hlist_pat!($($rest),*) } };
}

#[derive(Deserialize, Deref)]
pub struct MerchantId {
    merchant_id: String,
}

#[derive(Deserialize, Deref)]
pub struct ApiKey {
    api_key: String,
}

#[derive(Deserialize)]
pub struct Error {
    pub message: Message,
}

#[derive(Deserialize, Deref)]
pub struct Message {
    message: String,
}

#[derive(Deserialize, Deref)]
pub struct PaymentId {
    payment_id: String,
}

#[derive(Deserialize, Deref)]
pub struct Status {
    status: String,
}
