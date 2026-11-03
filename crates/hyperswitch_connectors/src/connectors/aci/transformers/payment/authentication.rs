//! Shopper authentication on a payment: ACI's own 3DS2, 3DS evidence from authentication
//! performed elsewhere, and when neither may be sent.

use hyperswitch_domain_models::router_request_types::AuthenticationData;

use super::{channel::Channel, PaymentItem};
use crate::{
    connectors::aci::transformers::{
        request::{AciExternalThreeDsData, PaymentDetails},
        three_ds::map_authentication_status,
    },
    utils::RouterData as _,
};

/// Whether the shopper is in session.
///
/// The spec is explicit that a merchant-initiated REPEATED charge "may not contain shopper
/// authentication data like the CVV code or 3D parameters — the shopper is not present anymore".
/// The authentication captured on the CIT must not be replayed onto a later charge: the acquirer
/// authenticates the MIT through the stored-credential linkage instead.
///
/// Decided from the request rather than from the instruction that goes out, because a MOTO
/// channel strips that instruction: a MOTO MIT declares no agreement, but the shopper is still
/// absent and the guards must still fire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::connectors::aci::transformers) enum ShopperPresence {
    Present,
    Absent,
}

/// ACI's own 3DS2, flagged as `customParameters[3DS2_enrolled]` on a 3DS payment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ConnectorThreeDs {
    enrolled: bool,
}

impl ConnectorThreeDs {
    /// ACI's 3DS2 on a 3DS payment with the shopper present. The enrolment flag is shopper
    /// authentication data too, so it is suppressed on a merchant-initiated charge alongside the
    /// rest of the `threeDSecure.*` block.
    pub(super) fn of(item: PaymentItem<'_>, shopper: ShopperPresence) -> Option<Self> {
        (shopper == ShopperPresence::Present && item.router_data.is_three_ds()).then_some(Self {
            enrolled: item.router_data.request.enrolled_for_3ds,
        })
    }

    pub(super) fn enrolled(self) -> bool {
        self.enrolled
    }

    /// [PT-4158] MPGS-backed entities reject `3DS2_enrolled=true` without a populated
    /// `customer.browser.*` block, `javaEnabled` included. A MOTO order has no browser.
    pub(super) fn needs_browser(self, channel: Channel) -> bool {
        self.enrolled && !channel.is_moto()
    }
}

/// The evidence of 3DS performed outside the payment call, as `threeDSecure.*`.
///
/// Dropped entirely with the shopper absent. Its ECI is also dropped when the instrument is a
/// decrypted network token carrying its own: both serialize `threeDSecure.eci`, and the key must
/// not appear twice in the form body.
pub(super) fn external_three_ds(
    authentication: Option<&AuthenticationData>,
    shopper: ShopperPresence,
    instrument: &PaymentDetails,
) -> AciExternalThreeDsData {
    let Some(authentication) = authentication.filter(|_| shopper == ShopperPresence::Present)
    else {
        return AciExternalThreeDsData::default();
    };
    let evidence = AciExternalThreeDsData::from(authentication);
    match instrument {
        PaymentDetails::AciNetworkToken(token) if token.eci.is_some() => AciExternalThreeDsData {
            eci: None,
            ..evidence
        },
        _ => evidence,
    }
}

impl From<&AuthenticationData> for AciExternalThreeDsData {
    fn from(authentication: &AuthenticationData) -> Self {
        Self {
            eci: authentication.eci.clone(),
            verification_id: Some(authentication.cavv.clone()),
            ds_transaction_id: authentication.ds_trans_id.clone(),
            acs_transaction_id: authentication.acs_trans_id.clone(),
            authentication_status: authentication
                .transaction_status
                .as_ref()
                .and_then(map_authentication_status)
                .map(|status| status.to_string()),
            // `major.minor` rather than `SemanticVersion::to_string`, which would render "2.2.0";
            // OPPWA documents the field as e.g. "2.2".
            version: authentication
                .message_version
                .as_ref()
                .map(|v| format!("{}.{}", v.get_major(), v.get_minor())),
        }
    }
}

#[cfg(test)]
mod tests {
    use common_enums::enums;
    use common_utils::types::{SemanticVersion, StringMajorUnit};
    use hyperswitch_domain_models::types::PaymentsAuthorizeRouterData;
    use hyperswitch_masking::Secret;

    use super::*;
    use crate::connectors::aci::{
        test_fixtures::*,
        transformers::{
            metadata::AciMetadata,
            request::{
                AciNetworkTokenData, AciTokenAccountType, AciTransactionCategory, PaymentBrand,
            },
            AciRouterData,
        },
    };

    fn authentication() -> AuthenticationData {
        AuthenticationData {
            eci: Some("05".to_string()),
            cavv: Secret::new("cavv".to_string()),
            threeds_server_transaction_id: None,
            message_version: Some(SemanticVersion::new(2, 2, 0)),
            ds_trans_id: Some("ds".to_string()),
            created_at: common_utils::date_time::now(),
            challenge_code: None,
            challenge_cancel: None,
            challenge_code_reason: None,
            message_extension: None,
            acs_trans_id: Some("acs".to_string()),
            authentication_type: None,
            transaction_status: Some(enums::TransactionStatus::Success),
            cb_network_params: None,
            exemption_indicator: None,
        }
    }

    fn network_token(eci: Option<&str>) -> PaymentDetails {
        PaymentDetails::AciNetworkToken(Box::new(AciNetworkTokenData {
            token_type: AciTokenAccountType::Network,
            token_number: "4895370012003478".parse().expect("valid token"),
            token_expiry_month: Secret::new("12".to_string()),
            token_expiry_year: Secret::new("2029".to_string()),
            token_cryptogram: None,
            eci: eci.map(String::from),
            payment_brand: PaymentBrand::Visa,
        }))
    }

    /// A merchant-initiated REPEATED charge "may not contain shopper authentication data like the
    /// CVV code or 3D parameters". A cardholder-present leg — initial or one-click — keeps it:
    /// stripping there would forfeit SCA compliance and liability shift.
    #[test]
    fn shopper_authentication_is_stripped_only_with_the_shopper_absent() {
        let stripped = external_three_ds(
            Some(&authentication()),
            ShopperPresence::Absent,
            &PaymentDetails::Mandate,
        );
        let encoded = serde_urlencoded::to_string(&stripped).expect("should encode");
        assert!(
            encoded.is_empty(),
            "no threeDSecure.* key may survive on a merchant-initiated charge: {encoded}"
        );

        let kept = external_three_ds(
            Some(&authentication()),
            ShopperPresence::Present,
            &PaymentDetails::Mandate,
        );
        assert_eq!(kept.eci.as_deref(), Some("05"));
        assert_eq!(kept.authentication_status.as_deref(), Some("Y"));
        assert_eq!(kept.version.as_deref(), Some("2.2"));
    }

    #[test]
    fn a_network_token_eci_is_not_sent_twice() {
        let deduplicated = external_three_ds(
            Some(&authentication()),
            ShopperPresence::Present,
            &network_token(Some("07")),
        );
        assert_eq!(deduplicated.eci, None);
        assert_eq!(deduplicated.authentication_status.as_deref(), Some("Y"));

        let kept = external_three_ds(
            Some(&authentication()),
            ShopperPresence::Present,
            &network_token(None),
        );
        assert_eq!(kept.eci.as_deref(), Some("05"));
    }

    /// [PT-4158] The MPGS browser rule applies to an enrolled 3DS payment with the shopper
    /// present, on any channel but MOTO (point of sale included).
    #[test]
    fn mpgs_browser_rule_covers_enrolled_attended_non_moto_3ds() {
        let cases = [
            (
                true,
                true,
                ShopperPresence::Present,
                Some(AciTransactionCategory::Ecommerce),
                true,
            ),
            (true, true, ShopperPresence::Present, None, true),
            (
                true,
                false,
                ShopperPresence::Present,
                Some(AciTransactionCategory::Ecommerce),
                false,
            ),
            (
                false,
                true,
                ShopperPresence::Present,
                Some(AciTransactionCategory::Ecommerce),
                false,
            ),
            (
                true,
                true,
                ShopperPresence::Absent,
                Some(AciTransactionCategory::Ecommerce),
                false,
            ),
            (
                true,
                true,
                ShopperPresence::Present,
                Some(AciTransactionCategory::MailOrder),
                false,
            ),
            (
                true,
                true,
                ShopperPresence::Present,
                Some(AciTransactionCategory::TelephoneOrder),
                false,
            ),
            (
                true,
                true,
                ShopperPresence::Present,
                Some(AciTransactionCategory::MailOrTelephoneOrder),
                false,
            ),
            (
                true,
                true,
                ShopperPresence::Present,
                Some(AciTransactionCategory::PointOfSale),
                true,
            ),
        ];
        for (is_three_ds, enrolled, shopper, category, expected) in cases {
            let mut router_data: PaymentsAuthorizeRouterData =
                cof_router_data(cof_authorize_data());
            router_data.auth_type = if is_three_ds {
                enums::AuthenticationType::ThreeDs
            } else {
                enums::AuthenticationType::NoThreeDs
            };
            router_data.request.enrolled_for_3ds = enrolled;
            let item = AciRouterData::from((StringMajorUnit::zero(), &router_data));
            let channel = Channel::resolve(
                AciMetadata {
                    transaction_category: category,
                    ..AciMetadata::default()
                },
                None,
            );

            assert_eq!(
                ConnectorThreeDs::of(&item, shopper).is_some_and(|tds| tds.needs_browser(channel)),
                expected,
                "is_three_ds={is_three_ds}, enrolled={enrolled}, shopper={shopper:?}, category={category:?}"
            );
        }
    }

    /// The enrolment flag goes out on every attended 3DS payment, enrolled or not.
    #[test]
    fn enrolment_is_flagged_on_attended_three_ds_only() {
        let mut router_data: PaymentsAuthorizeRouterData = cof_router_data(cof_authorize_data());
        router_data.auth_type = enums::AuthenticationType::ThreeDs;
        let item = AciRouterData::from((StringMajorUnit::zero(), &router_data));

        assert_eq!(
            ConnectorThreeDs::of(&item, ShopperPresence::Present).map(ConnectorThreeDs::enrolled),
            Some(false)
        );
        assert_eq!(ConnectorThreeDs::of(&item, ShopperPresence::Absent), None);
    }
}
