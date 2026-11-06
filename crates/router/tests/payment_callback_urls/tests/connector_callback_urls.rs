use common_utils::id_type::PaymentId;
use hyperswitch_domain_models::router_response_types::RedirectCallback;
use hyperswitch_interfaces::api::ConnectorRedirectResponse;
use hyperswitch_masking::PeekInterface;
use router::{
    core::{
        configs::payment_callback::PaymentCallbackMode,
        payments::callback_urls::ConnectorCallbackUrls,
    },
    routes::payments::{
        callback_routes::ConnectorCallbackPath, callback_tokens::PaymentCallbackUrl,
    },
    types::api::ConnectorData,
};

use super::callback_path_and_token;

#[test]
fn browser_and_notification_callbacks_have_independent_tokens() {
    let payment_id = PaymentId::wrap("pay_1".to_string()).expect("payment ID");
    let merchant_id =
        common_utils::id_type::MerchantId::wrap("merchant_1".to_string()).expect("merchant ID");
    for mode in [
        PaymentCallbackMode::Legacy,
        PaymentCallbackMode::Migration,
        PaymentCallbackMode::Enforced,
    ] {
        for creds in [None, Some("creds_1")] {
            for (connector, action) in [("hipay", "response"), ("paybox", "complete")] {
                let selected_callback = ConnectorData::convert_connector(connector)
                    .expect("connector")
                    .get_notification_callback();
                let expected_callback = if action == "response" {
                    RedirectCallback::Response
                } else {
                    RedirectCallback::Complete
                };
                assert_eq!(selected_callback, Some(expected_callback));
                let path = ConnectorCallbackPath {
                    payment_id: payment_id.clone(),
                    merchant_id: merchant_id.clone(),
                    connector: connector.to_string(),
                };
                let mut callbacks = ConnectorCallbackUrls::default();
                let notification = callbacks
                    .notification_url(selected_callback, "https://router.test", mode, &path, creds)
                    .expect("notification callback");
                let browser = match expected_callback {
                    RedirectCallback::Response => callbacks.redirect_response_url(
                        "https://router.test",
                        mode,
                        &payment_id,
                        &merchant_id,
                        connector,
                        creds,
                    ),
                    RedirectCallback::Complete => callbacks.redirect_complete_url(
                        "https://router.test",
                        mode,
                        &payment_id,
                        &merchant_id,
                        connector,
                        creds,
                    ),
                }
                .clone();
                let creds_path = creds.map(|value| format!("/{value}")).unwrap_or_default();
                let expected = format!(
                        "https://router.test/payments/pay_1/merchant_1/redirect/{action}/{connector}{creds_path}"
                    );
                if mode.issues_tokens() {
                    let (browser_path, browser_token) = callback_path_and_token(browser.peek());
                    let (notification_path, notification_token) =
                        callback_path_and_token(notification.peek());
                    let expected = format!(
                            "https://router.test/payments/pay_1/merchant_1/redirect/token/{{callback_token}}/{action}/{connector}{creds_path}"
                        );
                    assert_eq!(browser_path, expected);
                    assert_eq!(notification_path, expected);
                    assert_ne!(browser_token, notification_token);
                } else {
                    assert_eq!(browser.peek(), &expected);
                    assert_eq!(notification.peek(), &expected);
                }
                assert_eq!(
                    callbacks.notification.as_ref().map(PaymentCallbackUrl::url),
                    Some(&notification)
                );
                assert_eq!(
                    callbacks.notification_url(
                        selected_callback,
                        "https://router.test",
                        mode,
                        &path,
                        creds,
                    ),
                    Some(notification)
                );
            }
        }
    }
}

#[test]
fn connectors_without_notifications_do_not_allocate_callback_urls() {
    for connector in ["stripe", "recurly"] {
        let selected_callback = ConnectorData::convert_connector(connector)
            .expect("connector")
            .get_notification_callback();
        assert_eq!(selected_callback, None);
        let mut callbacks = ConnectorCallbackUrls::default();
        assert!(callbacks
            .notification_url(
                selected_callback,
                "https://router.test",
                PaymentCallbackMode::Enforced,
                &ConnectorCallbackPath {
                    payment_id: PaymentId::wrap("pay_1".to_string()).expect("payment ID"),
                    merchant_id: common_utils::id_type::MerchantId::wrap("merchant_1".to_string())
                        .expect("merchant ID"),
                    connector: connector.to_string(),
                },
                None,
            )
            .is_none());
        assert!(callbacks.notification.is_none());
        assert!(callbacks.redirect_response.is_none());
        assert!(callbacks.redirect_complete.is_none());
    }
}

#[test]
fn notification_endpoint_selection_does_not_depend_on_connector_name() {
    let path = ConnectorCallbackPath {
        payment_id: PaymentId::wrap("pay_1".to_string()).expect("payment ID"),
        merchant_id: common_utils::id_type::MerchantId::wrap("merchant_1".to_string())
            .expect("merchant ID"),
        connector: "custom_processor".to_string(),
    };
    for (callback, action) in [
        (RedirectCallback::Response, "response"),
        (RedirectCallback::Complete, "complete"),
    ] {
        let mut callbacks = ConnectorCallbackUrls::default();
        let url = callbacks
            .notification_url(
                Some(callback),
                "https://router.test",
                PaymentCallbackMode::Enforced,
                &path,
                None,
            )
            .expect("notification callback");
        assert_eq!(callback_path_and_token(url.peek()).0, format!(
                "https://router.test/payments/pay_1/merchant_1/redirect/token/{{callback_token}}/{action}/custom_processor"
            ));
    }
}

#[test]
fn connector_urls_are_lazy_cached_and_preserve_route_variants() {
    let payment_id = PaymentId::wrap("pay_1".to_string()).expect("payment ID");
    let merchant_id =
        common_utils::id_type::MerchantId::wrap("merchant_1".to_string()).expect("merchant ID");
    for mode in [
        PaymentCallbackMode::Legacy,
        PaymentCallbackMode::Migration,
        PaymentCallbackMode::Enforced,
    ] {
        for creds in [None, Some("creds_1")] {
            let mut callbacks = ConnectorCallbackUrls::default();
            assert!(callbacks.redirect_response.is_none());
            assert!(callbacks.redirect_complete.is_none());
            let response = callbacks
                .redirect_response_url(
                    "https://router.test",
                    mode,
                    &payment_id,
                    &merchant_id,
                    "stripe",
                    creds,
                )
                .clone();
            assert!(callbacks.redirect_response.is_some());
            assert!(callbacks.redirect_complete.is_none());
            assert_eq!(
                callbacks.redirect_response_url(
                    "https://other-router.test",
                    mode,
                    &payment_id,
                    &merchant_id,
                    "adyen",
                    None,
                ),
                &response
            );
            let complete = callbacks
                .redirect_complete_url(
                    "https://router.test",
                    mode,
                    &payment_id,
                    &merchant_id,
                    "stripe",
                    creds,
                )
                .clone();
            assert_eq!(
                callbacks.redirect_complete_url(
                    "https://other-router.test",
                    mode,
                    &payment_id,
                    &merchant_id,
                    "adyen",
                    None,
                ),
                &complete
            );
            assert_eq!(
                callbacks
                    .redirect_response
                    .as_ref()
                    .map(PaymentCallbackUrl::url),
                Some(&response)
            );
            for (action, callback) in [("response", response), ("complete", complete)] {
                let url = url::Url::parse(callback.peek()).expect("callback URL");
                let creds_path = creds.map(|value| format!("/{value}")).unwrap_or_default();
                let expected =
                    format!("/payments/pay_1/merchant_1/redirect/{action}/stripe{creds_path}");
                if mode == PaymentCallbackMode::Legacy {
                    assert_eq!(url.path(), expected);
                } else {
                    let (path, token) = callback_path_and_token(url.path());
                    assert_eq!(path, format!(
                            "/payments/pay_1/merchant_1/redirect/token/{{callback_token}}/{action}/stripe{creds_path}"
                        ));
                    assert_eq!(
                        uuid::Uuid::parse_str(token)
                            .expect("UUID")
                            .get_version_num(),
                        4
                    );
                }
                assert!(url.query().is_none());
            }
        }
    }
}
