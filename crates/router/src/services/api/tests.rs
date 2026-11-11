use hyperswitch_masking::Secret;

fn callback_forms() -> [super::RedirectForm; 9] {
    [
        super::RedirectForm::BarclaycardAuthSetup {
            access_token: "token".to_string(),
            ddc_url: "https://connector.test/ddc".to_string(),
            reference_id: "reference".to_string(),
        },
        super::RedirectForm::BlueSnap {
            payment_fields_token: "fields".to_string(),
        },
        super::RedirectForm::CybersourceAuthSetup {
            access_token: "token".to_string(),
            ddc_url: "https://connector.test/ddc".to_string(),
            reference_id: "reference".to_string(),
        },
        super::RedirectForm::Payme,
        super::RedirectForm::Braintree {
            client_token: "client".to_string(),
            card_token: "card".to_string(),
            bin: "123456".to_string(),
            acs_url: Secret::new("https://connector.test/acs".to_string()),
        },
        super::RedirectForm::Nmi {
            amount: "100".to_string(),
            currency: common_enums::Currency::USD,
            public_key: Secret::new("public".to_string()),
            customer_vault_id: "vault".to_string(),
            order_id: "order".to_string(),
        },
        super::RedirectForm::Mifinity {
            initialization_token: "initialization".to_string(),
        },
        super::RedirectForm::WorldpayDDCForm {
            endpoint: url::Url::parse("https://connector.test/ddc").expect("connector URL"),
            method: common_utils::request::Method::Post,
            form_fields: Default::default(),
            collection_id: None,
        },
        super::RedirectForm::WorldpayxmlDDCForm {
            bin: "123456".to_string(),
            jwt: "jwt".to_string(),
        },
    ]
}

fn render_form(form: &super::RedirectForm, callback: &str) -> String {
    super::build_redirection_form(
        form,
        Some(&Secret::new(callback.to_string())),
        None,
        "100".to_string(),
        "USD".to_string(),
        super::Settings::default(),
    )
    .into_string()
}

#[test]
fn callback_url_attribute_escapes_html_without_inserting_the_url_into_scripts() {
    let callback = "https://router.test/payments/</script>/callback/token?value=\"quoted\"&next=\u{2028}\u{2029}";
    let attribute = "data-callback-url=\"https://router.test/payments/&lt;/script&gt;/callback/token?value=&quot;quoted&quot;&amp;next=\u{2028}\u{2029}\"";
    for form in callback_forms() {
        let html = render_form(&form, callback);
        assert_eq!(html.matches(attribute).count(), 1);
        assert!(!html.contains(callback));
        assert!(html.contains("document.body.getAttribute('data-callback-url')"));
    }
}

#[test]
fn braintree_exposes_distinct_success_and_error_callbacks() {
    let complete = "https://router.test/redirect/complete/braintree?value=\"quoted\"&next=1";
    let response = "https://router.test/redirect/response/braintree";
    let form = super::RedirectForm::Braintree {
        client_token: "client".to_string(),
        card_token: "card".to_string(),
        bin: "123456".to_string(),
        acs_url: Secret::new(complete.to_string()),
    };
    let html = render_form(&form, response);
    assert!(html.contains(&format!("data-callback-url=\"{response}\"")));
    assert!(html.contains(
        "data-complete-callback-url=\"https://router.test/redirect/complete/braintree?value=&quot;quoted&quot;&amp;next=1\""
    ));
    assert!(!html.contains(complete));
    assert!(html.contains("document.body.getAttribute('data-complete-callback-url')"));
}

#[test]
fn test_mime_essence() {
    assert_eq!(mime::APPLICATION_JSON.essence_str(), "application/json");
}
