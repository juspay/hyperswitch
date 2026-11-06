//! Cards, with or without a CVC. The only method ACI runs its own 3DS2 for.

use hyperswitch_domain_models::{
    merchant_connector_account::should_collect_cvv_from_mca_metadata,
    payment_method_data::{self, CardWithOptionalCVC},
};
use hyperswitch_interfaces::errors;
use hyperswitch_masking::{PeekInterface, Secret};

use super::{AciPayment, PaymentMethodAdapter};
use crate::{
    connectors::aci::transformers::{
        payment::authentication::ShopperPresence,
        request::{AciPaymentsRequest, CardDetails, PaymentBrand, PaymentDetails},
        Error,
    },
    utils::{CardData, RouterData as _},
};

/// Four-digit expiry year for a CVV-optional card, the same widening `utils::CardData` does for
/// `Card`: a two-digit year is assumed to be in the 2000s.
fn optional_cvc_expiry_year_4_digit(card: &CardWithOptionalCVC) -> Secret<String> {
    let mut year = card.card_exp_year.peek().clone();
    if year.len() == 2 {
        year = format!("20{year}");
    }
    Secret::new(year)
}

pub(in crate::connectors::aci::transformers::payment) enum Card<'a> {
    WithCvc(&'a payment_method_data::Card),
    /// A card confirmed without a CVC, which the routed account must permit.
    OptionalCvc(&'a CardWithOptionalCVC),
}

impl PaymentMethodAdapter for Card<'_> {
    const CARD_ON_FILE: bool = true;
    const CONNECTOR_THREE_DS: bool = true;

    /// A card is the one method whose instrument is validated first (CVV-less permission,
    /// holder, expiry), so a request faulty in both reports the card's error.
    fn request(self, payment: AciPayment<'_>) -> Result<AciPaymentsRequest, Error> {
        let instrument = self.instrument(&payment)?;
        let txn_details = payment.transaction_details()?;
        payment.into_request::<Self>(txn_details, instrument)
    }

    fn instrument(self, payment: &AciPayment<'_>) -> Result<PaymentDetails, Error> {
        let router_data = payment.item().router_data;
        if let Self::OptionalCvc(card) = self {
            ensure_cvv_less_card_is_permitted(router_data.connector_meta_data.as_ref(), card)?;
        }
        // `card.holder` is required on every card request. The required-fields config collects
        // it through `billing.address.first_name`/`last_name`, which is why the billing name is
        // preferred — but a client may equally send it as
        // `payment_method_data.card.card_holder_name` and nothing else, so that is honoured too.
        let holder = router_data
            .get_optional_billing_full_name()
            .or_else(|| self.holder_name());
        let details = match self {
            Self::WithCvc(card) => CardFields {
                number: &card.card_number,
                expiry_month: &card.card_exp_month,
                expiry_year: card.get_expiry_year_4_digit(),
                cvc: Some(&card.card_cvc),
                network: card.card_network.as_ref(),
            },
            Self::OptionalCvc(card) => CardFields {
                number: &card.card_number,
                expiry_month: &card.card_exp_month,
                expiry_year: optional_cvc_expiry_year_4_digit(card),
                cvc: card.card_cvc.as_ref(),
                network: card.card_network.as_ref(),
            },
        }
        .into_details(holder, payment.shopper())?;
        Ok(PaymentDetails::AciCard(Box::new(details)))
    }
}

impl Card<'_> {
    fn holder_name(&self) -> Option<Secret<String>> {
        match self {
            Self::WithCvc(card) => card.card_holder_name.clone(),
            Self::OptionalCvc(card) => card.card_holder_name.clone(),
        }
    }
}

/// The card fields ACI takes, whichever card type they come from.
struct CardFields<'a> {
    number: &'a cards::CardNumber,
    expiry_month: &'a Secret<String>,
    expiry_year: Secret<String>,
    cvc: Option<&'a Secret<String>>,
    network: Option<&'a common_enums::CardNetwork>,
}

impl CardFields<'_> {
    /// The CVV is shopper authentication data: it may not accompany a charge the shopper is
    /// absent from. That only happens when a MIT arrives with raw card data rather than
    /// `PaymentMethodData::MandatePayment`. An unmapped network sends no brand.
    fn into_details(
        self,
        holder: Option<Secret<String>>,
        shopper: ShopperPresence,
    ) -> Result<CardDetails, Error> {
        Ok(CardDetails {
            card_number: self.number.clone(),
            card_holder: holder.ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "card_holder_name".into(),
            })?,
            card_expiry_month: self.expiry_month.clone(),
            card_expiry_year: self.expiry_year,
            card_cvv: match shopper {
                ShopperPresence::Present => self.cvc.cloned(),
                ShopperPresence::Absent => None,
            },
            payment_brand: PaymentBrand::for_card_network(self.network).ok(),
        })
    }
}

/// The card fields of a standalone 3DS authentication, where the shopper is always present and
/// the holder must come with the card.
pub(in crate::connectors::aci::transformers) fn card_details(
    card: &payment_method_data::Card,
) -> Result<CardDetails, Error> {
    CardFields {
        number: &card.card_number,
        expiry_month: &card.card_exp_month,
        expiry_year: card.get_expiry_year_4_digit(),
        cvc: Some(&card.card_cvc),
        network: card.card_network.as_ref(),
    }
    .into_details(card.card_holder_name.clone(), ShopperPresence::Present)
}

/// A card without a CVV is only accepted when the routed MCA has opted in via
/// `"collect_cvv": false` in its metadata. Without this, the list-time flag would be advisory
/// only and any API client could bypass a merchant's CVV posture by confirming with
/// `card_with_no_cvc`.
fn ensure_cvv_less_card_is_permitted(
    connector_meta_data: Option<&Secret<serde_json::Value>>,
    card: &CardWithOptionalCVC,
) -> Result<(), Error> {
    if card.card_cvc.is_none() && should_collect_cvv_from_mca_metadata(connector_meta_data) {
        Err(errors::ConnectorError::MissingRequiredField {
            field_name: "payment_method_data.card.card_cvc".into(),
        })?
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    fn card_with_optional_cvc(card_cvc: Option<Secret<String>>) -> CardWithOptionalCVC {
        CardWithOptionalCVC {
            card_number: cards::CardNumber::from_str("4111111111111111")
                .expect("valid visa test PAN"),
            card_exp_month: Secret::new("03".to_string()),
            card_exp_year: Secret::new("30".to_string()),
            card_cvc,
            card_network: Some(common_enums::CardNetwork::Visa),
            ..Default::default()
        }
    }

    fn fields(card: &CardWithOptionalCVC) -> CardFields<'_> {
        CardFields {
            number: &card.card_number,
            expiry_month: &card.card_exp_month,
            expiry_year: optional_cvc_expiry_year_4_digit(card),
            cvc: card.card_cvc.as_ref(),
            network: card.card_network.as_ref(),
        }
    }

    fn holder() -> Option<Secret<String>> {
        Some(Secret::new("Jane Cardholder".to_string()))
    }

    /// A CVV-optional card without a CVC must serialize with no `card.cvv` key at all (rather
    /// than an empty one) and still expand a 2-digit expiry year.
    #[test]
    fn optional_cvc_card_omits_cvv_when_absent() {
        let card = card_with_optional_cvc(None);
        let details = fields(&card)
            .into_details(holder(), ShopperPresence::Present)
            .expect("card details build without a cvc");

        let body = serde_urlencoded::to_string(&details).expect("card details serialize");
        assert!(!body.contains("card.cvv"), "{body}");
        assert!(body.contains("card.expiryYear=2030"), "{body}");
        assert!(body.contains("paymentBrand=VISA"), "{body}");
    }

    #[test]
    fn optional_cvc_card_sends_cvv_when_present() {
        let card = card_with_optional_cvc(Some(Secret::new("737".to_string())));
        let details = fields(&card)
            .into_details(holder(), ShopperPresence::Present)
            .expect("card details build with a cvc");

        let body = serde_urlencoded::to_string(&details).expect("card details serialize");
        assert!(body.contains("card.cvv=737"), "{body}");
    }

    /// The CVV never accompanies a charge the shopper is absent from.
    #[test]
    fn cvv_is_withheld_with_the_shopper_absent() {
        let card = card_with_optional_cvc(Some(Secret::new("737".to_string())));
        let details = fields(&card)
            .into_details(holder(), ShopperPresence::Absent)
            .expect("card details build");
        assert!(details.card_cvv.is_none());
    }

    #[test]
    fn card_holder_name_is_required() {
        let card = card_with_optional_cvc(None);
        let error = fields(&card)
            .into_details(None, ShopperPresence::Present)
            .expect_err("missing card holder name must be rejected");
        assert!(matches!(
            error.current_context(),
            errors::ConnectorError::MissingRequiredField { field_name }
                if field_name == "card_holder_name"
        ));
    }

    /// A CVV-less card is only permitted when the routed MCA opted in via
    /// `"collect_cvv": false`; otherwise the confirm-time guard rejects it so the
    /// list-time flag is enforceable, not advisory.
    #[test]
    fn cvv_less_card_requires_mca_opt_in() {
        let card_without_cvc = card_with_optional_cvc(None);
        let card_with_cvc = card_with_optional_cvc(Some(Secret::new("737".to_string())));
        let opted_in = Secret::new(serde_json::json!({ "collect_cvv": false }));
        let opted_out = Secret::new(serde_json::json!({ "other_key": true }));

        assert!(ensure_cvv_less_card_is_permitted(Some(&opted_in), &card_without_cvc).is_ok());
        assert!(ensure_cvv_less_card_is_permitted(Some(&opted_out), &card_with_cvc).is_ok());

        for metadata in [None, Some(&opted_out)] {
            let error = ensure_cvv_less_card_is_permitted(metadata, &card_without_cvc)
                .expect_err("CVV-less card without opt-in must be rejected");
            assert!(matches!(
                error.current_context(),
                errors::ConnectorError::MissingRequiredField { field_name }
                    if field_name == "payment_method_data.card.card_cvc"
            ));
        }
    }
}
