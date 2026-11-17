use api_models::payments::{AdditionalCardInfo, AdditionalPaymentData};
use hyperswitch_domain_models::payment_method_data::{Card, PaymentMethodData};

use super::super::matching_additional_card_data;

#[test]
fn reuses_only_card_details_derived_from_the_selected_payment_method() {
    let card = PaymentMethodData::Card(Card::default());
    let different_card = PaymentMethodData::Card(Card {
        card_issuer: Some("another issuer".to_owned()),
        ..Card::default()
    });
    let details = AdditionalPaymentData::Card(Box::new(AdditionalCardInfo {
        card_issuer: Some("issuer".to_owned()),
        ..AdditionalCardInfo::default()
    }));
    assert_eq!(
        matching_additional_card_data(Some(&card), Some(&card), Some(&details)),
        Some(details.clone())
    );
    assert_eq!(
        matching_additional_card_data(Some(&card), Some(&different_card), Some(&details)),
        None
    );
    assert_eq!(
        matching_additional_card_data(None, None, Some(&details)),
        None
    );
    assert_eq!(
        matching_additional_card_data(Some(&card), Some(&card), None),
        None
    );
    let non_card = PaymentMethodData::MandatePayment;
    assert_eq!(
        matching_additional_card_data(Some(&non_card), Some(&non_card), Some(&details)),
        None
    );
    let unresolved = AdditionalPaymentData::Card(Box::new(AdditionalCardInfo::default()));
    assert_eq!(
        matching_additional_card_data(Some(&card), Some(&card), Some(&unresolved)),
        None
    );
}
