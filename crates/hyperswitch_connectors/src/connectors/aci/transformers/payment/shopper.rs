//! The shopper: who they are, where they are billed, and the browser they are paying from.

use hyperswitch_domain_models::router_request_types::BrowserInformation;
use hyperswitch_masking::Secret;

use super::PaymentItem;
use crate::{
    connectors::aci::transformers::request::{
        AciBillingAddress, AciCustomerBrowserInfo, AciCustomerData,
        ACI_CHALLENGE_WINDOW_FULL_SCREEN,
    },
    utils::{PaymentsAuthorizeRequestData, RouterData as _},
};

impl AciCustomerData {
    pub(super) fn of(item: PaymentItem<'_>) -> Self {
        let router_data = item.router_data;
        Self {
            ip: router_data.request.get_ip_address_as_optional(),
            email: router_data.get_optional_billing_email(),
            given_name: router_data.get_optional_billing_first_name(),
            surname: router_data.get_optional_billing_last_name(),
            merchant_customer_id: router_data.get_optional_customer_id().map(Secret::new),
        }
    }
}

impl AciBillingAddress {
    pub(super) fn of(item: PaymentItem<'_>) -> Self {
        let router_data = item.router_data;
        Self {
            street1: router_data.get_optional_billing_line1(),
            city: router_data.get_optional_billing_city(),
            state: router_data.get_optional_billing_state(),
            country: router_data.get_optional_billing_country(),
            postcode: router_data.get_optional_billing_zip(),
        }
    }
}

impl AciCustomerBrowserInfo {
    /// API `browser_info` as OPPWA `customer.browser.*`.
    ///
    /// With `mpgs_defaults` (see [`super::authentication::ConnectorThreeDs::needs_browser`]),
    /// `javascriptEnabled` and `javaEnabled` are filled when the merchant left them unset, since
    /// MPGS requires them; explicit `false` values are preserved.
    pub(super) fn of(browser: &BrowserInformation, mpgs_defaults: bool) -> Self {
        let (javascript_enabled, java_enabled) = if mpgs_defaults {
            (
                browser.java_script_enabled.or(Some(true)),
                browser.java_enabled.or(Some(false)),
            )
        } else {
            (browser.java_script_enabled, browser.java_enabled)
        };

        Self {
            accept_header: browser.accept_header.clone(),
            language: browser.language.clone(),
            screen_height: browser.screen_height.map(|h| h.to_string()),
            screen_width: browser.screen_width.map(|w| w.to_string()),
            timezone: browser.time_zone.map(|t| t.to_string()),
            user_agent: browser.user_agent.clone(),
            javascript_enabled,
            java_enabled,
            color_depth: browser.color_depth.map(|c| c.to_string()),
            // Tied to the rest of the browser block rather than sent unconditionally: it only
            // means anything to an ACS, and a browserless MIT/MOTO request sends no
            // `customer.browser.*` keys at all.
            challenge_window: Some(ACI_CHALLENGE_WINDOW_FULL_SCREEN.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectors::aci::{
        test_fixtures::*,
        transformers::request::{AciCustomerData, PaymentDetails},
    };

    /// [PT-4158] When `browser_info` is present but omits `java_enabled`, MPGS still needs
    /// `customer.browser.javaEnabled` on the wire alongside `3DS2_enrolled=true`.
    #[test]
    fn mpgs_browser_defaults_fill_only_unset_fields() {
        let partial = BrowserInformation {
            user_agent: Some("Mozilla/5.0".to_string()),
            ..Default::default()
        };
        let aci = AciCustomerBrowserInfo::of(&partial, true);
        assert_eq!(aci.java_enabled, Some(false));
        assert_eq!(aci.javascript_enabled, Some(true));
        assert_eq!(
            aci.challenge_window.as_deref(),
            Some(ACI_CHALLENGE_WINDOW_FULL_SCREEN)
        );

        let aci = AciCustomerBrowserInfo::of(&partial, false);
        assert_eq!(aci.java_enabled, None);
        assert_eq!(aci.javascript_enabled, None);

        let explicit = BrowserInformation {
            java_script_enabled: Some(false),
            java_enabled: Some(true),
            ..Default::default()
        };
        let aci = AciCustomerBrowserInfo::of(&explicit, true);
        assert_eq!(aci.java_enabled, Some(true));
        assert_eq!(aci.javascript_enabled, Some(false));
    }

    /// [PT-4158] MPGS defaults must appear on the flat authorize body alongside `3DS2_enrolled`.
    #[test]
    fn mpgs_browser_defaults_serialize_on_the_authorize_body() {
        let partial = BrowserInformation {
            user_agent: Some("Mozilla/5.0".to_string()),
            ..Default::default()
        };
        let request = customer_request(
            AciCustomerBrowserInfo::of(&partial, true),
            AciCustomerData::default(),
            PaymentDetails::Mandate,
            Some(true),
        );
        let body = serde_urlencoded::to_string(&request).expect("request serializes cleanly");

        assert!(
            body.contains("customParameters%5B3DS2_enrolled%5D=true"),
            "{body}"
        );
        assert!(
            body.contains("customer.browser.javaEnabled=false"),
            "{body}"
        );
        assert!(
            body.contains("customer.browser.javascriptEnabled=true"),
            "{body}"
        );
    }
}
