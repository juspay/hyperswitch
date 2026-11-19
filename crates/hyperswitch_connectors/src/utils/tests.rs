use hyperswitch_masking::{PeekInterface, Secret};

use super::append_url_query_param;
use crate::utils;

#[test]
fn appended_status_preserves_callback_token_and_fragment() {
    let token = "d9dc53cc-d1a1-4f57-9077-d20c2d614daa";
    let base = Secret::new(format!(
            "https://router.test/payments/pay/merchant/redirect/token/{token}/response/connector?result=ok#result"
        ));
    let result = append_url_query_param(&base, "status", "success").expect("URL");
    let parsed = url::Url::parse(result.peek()).expect("URL");
    let pairs = parsed.query_pairs().collect::<Vec<_>>();
    assert!(pairs
        .iter()
        .any(|(key, value)| key == "result" && value == "ok"));
    assert!(pairs
        .iter()
        .any(|(key, value)| key == "status" && value == "success"));
    assert_eq!(
        parsed.path(),
        format!("/payments/pay/merchant/redirect/token/{token}/response/connector")
    );
    assert_eq!(parsed.fragment(), Some("result"));
    assert_eq!(
        append_url_query_param(
            &Secret::new("https://router.test/payments/path".to_string()),
            "status",
            "failure"
        )
        .expect("URL")
        .peek(),
        "https://router.test/payments/path?status=failure"
    );
}

#[test]
fn test_image_data_source_url() {
    let qr_image_data_source_url = utils::QrImage::new_from_data("Hyperswitch".to_string());
    assert!(qr_image_data_source_url.is_ok());
}
