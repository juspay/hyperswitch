#![cfg(feature = "v1")]

use api_models::payments::{
    NextActionData, PollConfigResponse, RedirectionResponse, ThreeDsData, ThreeDsMethodData,
};
use hyperswitch_masking::Secret;

const CALLBACK_TOKEN: &str = "d9dc53cc-d1a1-4f57-9077-d20c2d614daa";
const CALLBACK_URL: &str = "https://router.test/payments/pay/merchant/authorize/stripe/callback/d9dc53cc-d1a1-4f57-9077-d20c2d614daa";

fn assert_secret_serialization<T: serde::Serialize + std::fmt::Debug>(
    response: &T,
    url_fields: &[&str],
) {
    let wire = serde_json::to_value(response).expect("wire response");
    for field in url_fields {
        assert_eq!(
            wire.pointer(field).and_then(serde_json::Value::as_str),
            Some(CALLBACK_URL)
        );
    }
    let logged = hyperswitch_masking::masked_serialize(response).expect("masked response");
    assert!(!logged.to_string().contains(CALLBACK_TOKEN));
    assert!(!format!("{response:?}").contains(CALLBACK_TOKEN));
}

#[test]
fn callback_urls_remain_strings_on_the_wire_and_are_masked_in_logs() {
    let url = || Secret::new(CALLBACK_URL.to_string());
    assert_secret_serialization(
        &NextActionData::RedirectToUrl {
            redirect_to_url: url(),
        },
        &["/redirect_to_url"],
    );
    assert_secret_serialization(
        &NextActionData::RedirectInsidePopup {
            popup_url: url(),
            redirect_response_url: url(),
        },
        &["/popup_url", "/redirect_response_url"],
    );
    assert_secret_serialization(
        &NextActionData::ThreeDsInvoke {
            three_ds_data: ThreeDsData {
                three_ds_authentication_url: url(),
                three_ds_authorize_url: url(),
                three_ds_method_details: ThreeDsMethodData::AcsThreeDsMethodData {
                    three_ds_method_data_submission: false,
                    three_ds_method_data: None,
                    three_ds_method_url: None,
                    three_ds_method_key: None,
                    consume_post_message_for_three_ds_method_completion: false,
                },
                poll_config: PollConfigResponse {
                    poll_id: "poll_test".to_string(),
                    delay_in_secs: 1,
                    frequency: 1,
                },
                message_version: None,
                directory_server_id: None,
                card_network: None,
                three_ds_connector: None,
            },
        },
        &[
            "/three_ds_data/three_ds_authentication_url",
            "/three_ds_data/three_ds_authorize_url",
        ],
    );
    assert_secret_serialization(
        &RedirectionResponse {
            return_url: String::new(),
            params: vec![("return_url".to_string(), url())],
            return_url_with_query_params: url(),
            http_method: "GET".to_string(),
            headers: vec![],
        },
        &["/return_url_with_query_params", "/params/0/1"],
    );
}

#[test]
fn authentication_return_url_is_masked_and_preserves_its_wire_value() {
    let request: api_models::authentication::AuthenticationCreateRequest =
        serde_json::from_value(serde_json::json!({
            "amount": 100,
            "currency": "USD",
            "return_url": CALLBACK_URL,
        }))
        .expect("authentication create request");
    assert_secret_serialization(&request, &["/return_url"]);
}
