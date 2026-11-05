#![cfg(feature = "v1")]

#[cfg(test)]
mod sdk_callback_url_tests {
    use common_enums::Currency;
    use common_utils::id_type::PaymentId;
    use hyperswitch_domain_models::router_response_types::{RedirectCallback, RedirectForm};
    use hyperswitch_interfaces::api::{ConnectorCommon, ConnectorRedirectResponse};
    use hyperswitch_masking::PeekInterface;
    use router::{
        configs::settings::PaymentCallbackMode,
        core::payments::transformers::{next_action_invoke_ddc_iframe, SdkCallbackUrls},
        routes::payments::callback_routes::{
            ConnectorCallbackPath, PaymentCallbackRoute, StartCallbackPath,
        },
        types::api::ConnectorData,
    };

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
            assert!(ddc
                .iframe_url
                .path()
                .starts_with("/payments/redirect/pay/merchant/attempt"));
            assert_eq!(
                ddc.iframe_url.path().contains("/callback/"),
                mode.issues_tokens()
            );
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
        assert!(url::Url::parse(redirect.peek())
            .expect("redirect URL")
            .path()
            .starts_with("/payments/pay/merchant/redirect/response/stripe/creds/callback/"));
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
    fn authentication_url_is_cached_without_token_registration() {
        assert_eq!(
            PaymentCallbackRoute::Authentication.path(),
            PaymentCallbackRoute::Authentication.legacy_path()
        );
        let mut callbacks = SdkCallbackUrls::default();
        let authentication = callbacks
            .authentication_url("https://router.test", "pay")
            .clone();
        assert_eq!(
            authentication.peek(),
            "https://router.test/payments/pay/3ds/authentication"
        );
        assert_eq!(
            callbacks.authentication_url("https://router.test", "other_pay"),
            &authentication
        );
        assert!(callbacks.start.is_none());
        assert!(callbacks.authorize.is_none());
        assert!(callbacks.redirect_response.is_none());
        assert!(callbacks.redirect_complete.is_none());
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
            let connector_instance =
                ConnectorData::convert_connector(connector).expect("connector");
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
            assert!(url.path().starts_with(&format!(
                "/payments/pay/merchant/redirect/{action}/{connector}/callback/"
            )));
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
}

mod connector_callback_url_tests {
    use common_utils::id_type::PaymentId;
    use hyperswitch_masking::PeekInterface;
    use router::{
        configs::settings::PaymentCallbackMode,
        core::payments::transformers::ConnectorCallbackUrls,
        routes::payments::callback_tokens::PaymentCallbackUrl,
    };

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
                        let (path, token) =
                            url.path().rsplit_once("/callback/").expect("token path");
                        assert_eq!(path, expected);
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
}
