// use std::sync;

// use router::{configs::settings, connection, core::webhooks, types::api};
// mod utils;

// fn get_config() -> settings::Settings {
//     settings::Settings::new().expect("Settings")
// }

// struct TestApp {
//     redis_conn: connection::RedisPool,
// }

// impl TestApp {
//     async fn init() -> Self {
//         utils::setup().await;
//         let conf = get_config();

//         Self {
//             redis_conn: sync::Arc::new(connection::redis_connection(&conf.redis).await),
//         }
//     }
// }

// #[actix_web::test]
// async fn test_webhook_config_lookup() {
//     let app = TestApp::init().await;
//     let timestamp = router::utils::date_time::now();

//     let merchant_id = format!("merchant_{timestamp}");
//     let connector_id = "stripe";
//     let config = serde_json::json!(["payment_intent_success"]);

//     let lookup_res = webhooks::utils::lookup_webhook_event(
//         connector_id,
//         &merchant_id,
//         &api::IncomingWebhookEvent::PaymentIntentSuccess,
//         sync::Arc::clone(&app.redis_conn),
//     )
//     .await;

//     assert!(lookup_res);

//     app.redis_conn
//         .serialize_and_set_key(&format!("whconf_{merchant_id}_{connector_id}"), &config)
//         .await
//         .expect("Save merchant webhook config");

//     let lookup_res = webhooks::utils::lookup_webhook_event(
//         connector_id,
//         &merchant_id,
//         &api::IncomingWebhookEvent::PaymentIntentSuccess,
//         sync::Arc::clone(&app.redis_conn),
//     )
//     .await;

//     assert!(lookup_res);
// }
