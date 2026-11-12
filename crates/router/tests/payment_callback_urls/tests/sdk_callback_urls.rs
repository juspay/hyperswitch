use common_enums::Currency;
use common_utils::id_type::PaymentId;
use hyperswitch_domain_models::router_response_types::{RedirectCallback, RedirectForm};
use hyperswitch_interfaces::api::{ConnectorCommon, ConnectorRedirectResponse};
use hyperswitch_masking::PeekInterface;
use router::{
    core::{
        configs::payment_callback::PaymentCallbackMode,
        payments::{callback_urls::SdkCallbackUrls, transformers::next_action_invoke_ddc_iframe},
    },
    routes::payments::callback_routes::{ConnectorCallbackPath, StartCallbackPath},
    types::api::ConnectorData,
};

use super::callback_path_and_token;

fn payment_id(value: &str) -> PaymentId {
    PaymentId::wrap(value.to_string()).expect("payment ID")
}
fn merchant_id(value: &str) -> common_utils::id_type::MerchantId {
    common_utils::id_type::MerchantId::wrap(value.to_string()).expect("merchant ID")
}

#[test]
fn ddc_next_action_uses_a_collected_start_url_in_every_mode() {
    let path = StartCallbackPath {
        payment_id: payment_id("pay"),
        merchant_id: merchant_id("merchant"),
        attempt_id: "attempt".into(),
    };
    let metadata = serde_json::json!({
        "iframe_url": "/payments/redirect/old_pay/old_merchant/old_attempt",
        "timeout_ms": 1000,
    });
    for mode in [
        PaymentCallbackMode::Legacy,
        PaymentCallbackMode::Migration,
        PaymentCallbackMode::Enforced,
    ] {
        let mut callbacks = SdkCallbackUrls::default();
        let ddc = next_action_invoke_ddc_iframe(
            Some(&metadata),
            "https://router.test",
            mode,
            &path,
            &mut callbacks,
        )
        .expect("next action")
        .expect("DDC");
        assert_eq!(ddc.timeout_ms, Some(1000));
        assert_eq!(
            ddc.iframe_url.as_str(),
            callbacks
                .start
                .as_ref()
                .expect("pending start URL")
                .url()
                .peek()
        );
        if mode.issues_tokens() {
            assert_eq!(
                callback_path_and_token(ddc.iframe_url.path()).0,
                "/payments/pay/merchant/redirect/token/{callback_token}/start/attempt"
            );
        } else {
            assert_eq!(
                ddc.iframe_url.path(),
                "/payments/redirect/pay/merchant/attempt"
            );
        }
    }
    for metadata in [None, Some(serde_json::json!({"unrelated": true}))] {
        let mut callbacks = SdkCallbackUrls::default();
        assert!(next_action_invoke_ddc_iframe(
            metadata.as_ref(),
            "https://router.test",
            PaymentCallbackMode::Enforced,
            &path,
            &mut callbacks
        )
        .expect("unrelated metadata")
        .is_none());
        assert!(callbacks.start.is_none());
    }
}

#[test]
fn sdk_url_methods_reuse_cached_urls_and_populate_only_their_field() {
    let mut callbacks = SdkCallbackUrls::default();
    let start = callbacks
        .start_url(
            "https://router.test",
            PaymentCallbackMode::Migration,
            &payment_id("pay"),
            &merchant_id("merchant"),
            "attempt",
        )
        .clone();
    assert!(callbacks.authorize.is_none());
    assert!(callbacks.redirect_response.is_none());
    assert_eq!(
        callbacks.start_url(
            "https://router.test",
            PaymentCallbackMode::Migration,
            &payment_id("pay"),
            &merchant_id("merchant"),
            "attempt"
        ),
        &start
    );
    let authorize = callbacks
        .authorize_url(
            "https://router.test",
            PaymentCallbackMode::Migration,
            &payment_id("pay"),
            &merchant_id("merchant"),
            "stripe",
        )
        .clone();
    assert!(callbacks.redirect_response.is_none());
    assert_eq!(
        callbacks.authorize_url(
            "https://router.test",
            PaymentCallbackMode::Migration,
            &payment_id("pay"),
            &merchant_id("merchant"),
            "stripe"
        ),
        &authorize
    );
    let redirect = callbacks
        .redirect_response_url(
            "https://router.test",
            PaymentCallbackMode::Migration,
            &payment_id("pay"),
            &merchant_id("merchant"),
            "stripe",
            Some("creds"),
        )
        .clone();
    assert_eq!(
        callback_path_and_token(
            url::Url::parse(redirect.peek())
                .expect("redirect URL")
                .path()
        )
        .0,
        "/payments/pay/merchant/redirect/token/{callback_token}/response/stripe/creds"
    );
    assert_eq!(
        callbacks.redirect_response_url(
            "https://router.test",
            PaymentCallbackMode::Migration,
            &payment_id("pay"),
            &merchant_id("merchant"),
            "stripe",
            None
        ),
        &redirect
    );
}

#[test]
fn browser_forms_construct_only_the_required_callback_endpoint() {
    let forms = [
        (RedirectForm::Payme, "payme", false),
        (
            RedirectForm::BlueSnap {
                payment_fields_token: "fields".into(),
            },
            "bluesnap",
            false,
        ),
        (
            RedirectForm::BarclaycardAuthSetup {
                access_token: "token".into(),
                ddc_url: "https://connector.test".into(),
                reference_id: "reference".into(),
            },
            "barclaycard",
            false,
        ),
        (
            RedirectForm::CybersourceAuthSetup {
                access_token: "token".into(),
                ddc_url: "https://connector.test".into(),
                reference_id: "reference".into(),
            },
            "cybersource",
            false,
        ),
        (
            RedirectForm::Braintree {
                client_token: "client".into(),
                card_token: "card".into(),
                bin: "123456".into(),
                acs_url: "https://connector.test".to_string().into(),
            },
            "braintree",
            true,
        ),
        (
            RedirectForm::Mifinity {
                initialization_token: "initialization".into(),
            },
            "mifinity",
            true,
        ),
        (
            RedirectForm::Nmi {
                amount: "100".into(),
                currency: Currency::USD,
                public_key: "public".to_string().into(),
                customer_vault_id: "vault".into(),
                order_id: "order".into(),
            },
            "nmi",
            false,
        ),
        (
            RedirectForm::WorldpayDDCForm {
                endpoint: url::Url::parse("https://connector.test").expect("connector URL"),
                method: common_utils::request::Method::Post,
                form_fields: Default::default(),
                collection_id: None,
            },
            "worldpay",
            false,
        ),
        (
            RedirectForm::WorldpayxmlDDCForm {
                bin: "123456".into(),
                jwt: "jwt".into(),
            },
            "worldpayxml",
            false,
        ),
    ];
    for (form, connector, is_response) in forms {
        let connector_instance = ConnectorData::convert_connector(connector).expect("connector");
        let selected_callback = connector_instance.get_redirect_callback(&form);
        assert_eq!(
            selected_callback,
            Some(if is_response {
                RedirectCallback::Response
            } else {
                RedirectCallback::Complete
            })
        );
        let path = ConnectorCallbackPath {
            payment_id: payment_id("pay"),
            merchant_id: merchant_id("merchant"),
            connector: connector_instance.id().to_string(),
        };
        let mut callbacks = SdkCallbackUrls::default();
        let callback = callbacks
            .redirection_form_callback_url(
                selected_callback,
                "https://router.test",
                PaymentCallbackMode::Migration,
                &path,
                None,
            )
            .expect("browser callback")
            .clone();
        let url = url::Url::parse(callback.peek()).expect("URL");
        let action = if is_response { "response" } else { "complete" };
        assert_eq!(
            callback_path_and_token(url.path()).0,
            format!(
                "/payments/pay/merchant/redirect/token/{{callback_token}}/{action}/{connector}"
            )
        );
        assert!(url.query().is_none());
        assert!(callbacks.start.is_none());
        assert!(callbacks.authorize.is_none());
        assert_eq!(callbacks.redirect_response.is_some(), is_response);
        assert_eq!(callbacks.redirect_complete.is_some(), !is_response);
        assert_eq!(
            callbacks.redirection_form_callback_url(
                selected_callback,
                "https://router.test",
                PaymentCallbackMode::Migration,
                &ConnectorCallbackPath {
                    payment_id: payment_id("other_pay"),
                    merchant_id: merchant_id("merchant"),
                    connector: connector.to_string(),
                },
                None
            ),
            Some(&callback)
        );
        let pending = if is_response {
            callbacks.redirect_response.as_ref()
        } else {
            callbacks.redirect_complete.as_ref()
        }
        .expect("prepared browser callback");
        assert_eq!(pending.url(), &callback);
    }
}

#[test]
fn html_forms_do_not_generate_unused_callback_urls() {
    let connector = ConnectorData::convert_connector("stripe").expect("connector");
    let callback = connector.get_redirect_callback(&RedirectForm::Html {
        html_data: "<html></html>".to_string().into(),
    });
    assert_eq!(callback, None);
    let mut callbacks = SdkCallbackUrls::default();
    assert!(callbacks
        .redirection_form_callback_url(
            callback,
            "https://router.test",
            PaymentCallbackMode::Migration,
            &ConnectorCallbackPath {
                payment_id: payment_id("pay"),
                merchant_id: merchant_id("merchant"),
                connector: connector.id().to_string(),
            },
            None
        )
        .is_none());
    assert!(callbacks.start.is_none());
    assert!(callbacks.authorize.is_none());
    assert!(callbacks.redirect_response.is_none());
    assert!(callbacks.redirect_complete.is_none());
}
