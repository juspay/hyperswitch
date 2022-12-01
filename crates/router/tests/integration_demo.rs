use http::StatusCode;
use reqwest::Client;
use router::{configs::settings::Settings, services::Store, types::storage::AddressNew};
use serde_json::Value;

mod utils;
struct TestApp {
    store: Store,
}

impl TestApp {
    pub async fn init() -> (Client, TestApp) {
        utils::setup().await;

        let client = Client::new();
        let conf = Settings::new().unwrap();
        let store = Store::new(&conf).await;
        let app = TestApp { store };

        (client, app)
    }
}

fn mk_merchant_account() -> serde_json::Value {
    let timestamp = common_utils::date_time::now();

    serde_json::json!({
      "merchant_id": format!("merchant_{timestamp}"),
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
      "routing_algorithm": "custom",
      "custom_routing_rules": [
        {
          "payment_methods_incl": [
            "card",
            "card"
          ],
          "payment_methods_excl": [
            "card",
            "card"
          ],
          "payment_method_types_incl": [
            "credit",
            "credit"
          ],
          "payment_method_types_excl": [
            "credit",
            "credit"
          ],
          "payment_method_issuers_incl": [
            "Citibank",
            "JPMorgan"
          ],
          "payment_method_issuers_excl": [
            "Citibank",
            "JPMorgan"
          ],
          "countries_incl": [
            "US",
            "UK",
            "IN"
          ],
          "countries_excl": [
            "US",
            "UK",
            "IN"
          ],
          "currencies_incl": [
            "USD",
            "EUR"
          ],
          "currencies_excl": [
            "AED",
            "SGD"
          ],
          "metadata_filters_keys": [
            "payments.udf1",
            "payments.udf2"
          ],
          "metadata_filters_values": [
            "android",
            "Category_Electronics"
          ],
          "connectors_pecking_order": [
            "stripe",
            "adyen",
            "brain_tree"
          ],
          "connectors_traffic_weightage_key": [
            "stripe",
            "adyen",
            "brain_tree"
          ],
          "connectors_traffic_weightage_value": [
            50,
            30,
            20
          ]
        },
        {
          "payment_methods_incl": [
            "card",
            "card"
          ],
          "payment_methods_excl": [
            "card",
            "card"
          ],
          "payment_method_types_incl": [
            "credit",
            "credit"
          ],
          "payment_method_types_excl": [
            "credit",
            "credit"
          ],
          "payment_method_issuers_incl": [
            "Citibank",
            "JPMorgan"
          ],
          "payment_method_issuers_excl": [
            "Citibank",
            "JPMorgan"
          ],
          "countries_incl": [
            "US",
            "UK",
            "IN"
          ],
          "countries_excl": [
            "US",
            "UK",
            "IN"
          ],
          "currencies_incl": [
            "USD",
            "EUR"
          ],
          "currencies_excl": [
            "AED",
            "SGD"
          ],
          "metadata_filters_keys": [
            "payments.udf1",
            "payments.udf2"
          ],
          "metadata_filters_values": [
            "android",
            "Category_Electronics"
          ],
          "connectors_pecking_order": [
            "stripe",
            "adyen",
            "brain_tree"
          ],
          "connectors_traffic_weightage_key": [
            "stripe",
            "adyen",
            "brain_tree"
          ],
          "connectors_traffic_weightage_value": [
            50,
            30,
            20
          ]
        }
      ],
      "sub_merchants_enabled": false,
      "metadata": {
        "city": "NY",
        "unit": "245"
      }
    })
}

fn mk_payment() -> serde_json::Value {
    serde_json::json!({
      "amount": 6540,
      "currency": "USD",
      "confirm": true,
      "capture_method": "automatic",
      "capture_on": "2022-09-10T10:11:12Z",
      "amount_to_capture": 6540,
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
      "statement_descriptor_name": "Juspay",
      "statement_descriptor_suffix": "Router",
      "metadata": {
        "udf1": "value1",
        "new_customer": "true",
        "login_date": "2019-09-10T10:11:12Z"
      }
    })
}

fn mk_connector() -> serde_json::Value {
    serde_json::json!({
      "merchant_id": "y3oqhf46pyzuxjbcn2giaqnb44",
      "connector_type": "payment_processor",
      "connector_name": "stripe",
      "connector_account_details": {
        "api_key": "Basic MyVerySecretApiKey"
      },
      "test_mode": false,
      "disabled": false,
      "payment_methods_enabled": [
        {
          "payment_method": "card",
          "payment_method_types": [
            "credit_card"
          ],
          "payment_method_issuers": [
            [
              "HDFC"
            ]
          ],
          "payment_schemes": [
            "VISA"
          ],
          "accepted_currencies": [
            "USD"
          ],
          "accepted_countries": [
            "US"
          ],
          "minimum_amount": 1,
          "maximum_amount": null,
          "recurring_enabled": true,
          "installment_payment_enabled": true,
          "payment_experience": [
            "redirect_to_url"
          ]
        }
      ],
      "metadata": {
        "city": "NY",
        "unit": "245"
      }
    })
}

#[actix_web::test]
async fn create_payment() {
    let (client, _) = TestApp::init().await;

    let merchant_account = client
        .post("http://localhost:8080/accounts")
        .header("api-key", "test_admin")
        .json(&mk_merchant_account())
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();

    println!("{:?}", merchant_account);

    let merchant_id = merchant_account
        .get("merchant_id")
        .and_then(Value::as_str)
        .unwrap();

    let api_key = merchant_account
        .get("api_key")
        .and_then(Value::as_str)
        .unwrap();

    let _connector = client
        .post(format!(
            "http://localhost:8080/account/{merchant_id}/connectors"
        ))
        .json(&mk_connector())
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>();

    let payment = client
        .post("http://localhost:8080/payments")
        .header("api-key", api_key)
        .json(&mk_payment())
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();

    println!("{payment}");
}

#[actix_web::test]
async fn address() {
    use router::db::address::IAddress;

    let (_, app) = TestApp::init().await;
    let store = app.store;

    let address = store
        .insert_address(AddressNew {
            city: "City!".to_owned().into(),
            ..AddressNew::default()
        })
        .await
        .unwrap();

    let address = store.find_address(&address.address_id).await.unwrap();

    assert_eq!(address.city, Some("City!".to_owned()));
}

#[actix_web::test]
async fn health() {
    let (client, _) = TestApp::init().await;

    let n = client
        .get("http://localhost:8080/health")
        .send()
        .await
        .unwrap();

    assert_eq!(n.status(), StatusCode::OK);
}
