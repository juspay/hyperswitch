use hyperswitch_domain_models::payment_method_data::{
    CardToken, CardWithOptionalCVC, PaymentMethodData,
};
use hyperswitch_masking::{PeekInterface, Secret};

use super::payment_method_data_for_additional_info;

#[test]
fn saved_card_overrides_do_not_replace_fetched_card_metadata() {
    let fetched = PaymentMethodData::CardWithOptionalCVC(CardWithOptionalCVC {
        card_number: "5555555555554444".parse().expect("test card number"),
        card_cvc: Some(Secret::new("123".to_owned())),
        card_holder_name: Some(Secret::new("Updated name".to_owned())),
        ..Default::default()
    });
    let selected = payment_method_data_for_additional_info(
        Some(PaymentMethodData::CardToken(CardToken::default())),
        None,
        Some(&fetched),
    )
    .expect("fetched saved card");
    assert_eq!(selected.get_card_iin().as_deref(), Some("555555"));
    match selected {
        PaymentMethodData::CardWithOptionalCVC(card) => {
            assert_eq!(
                card.card_cvc.as_ref().map(|value| value.peek().as_str()),
                Some("123")
            );
            assert_eq!(
                card.card_holder_name
                    .as_ref()
                    .map(|value| value.peek().as_str()),
                Some("Updated name")
            );
        }
        _ => panic!("expected the fetched card"),
    }
}

#[test]
fn saved_card_selection_preserves_direct_payment_and_recurring_precedence() {
    assert!(matches!(
        payment_method_data_for_additional_info(
            Some(PaymentMethodData::Reward),
            None,
            Some(&PaymentMethodData::CardWithOptionalCVC(Default::default())),
        ),
        Some(PaymentMethodData::Reward)
    ));
    assert!(matches!(
        payment_method_data_for_additional_info(
            None,
            Some(&PaymentMethodData::Reward),
            Some(&PaymentMethodData::CardWithOptionalCVC(Default::default())),
        ),
        Some(PaymentMethodData::Reward)
    ));
}

#[test]
fn saved_card_selection_preserves_legacy_token_without_resolved_card() {
    assert!(matches!(
        payment_method_data_for_additional_info(
            Some(PaymentMethodData::CardToken(CardToken::default())),
            None,
            None,
        ),
        Some(PaymentMethodData::CardToken(_))
    ));
    assert!(payment_method_data_for_additional_info(None, None, None).is_none());
}
