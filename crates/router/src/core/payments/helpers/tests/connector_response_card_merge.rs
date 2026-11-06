use hyperswitch_domain_models::router_data::AdditionalPaymentMethodConnectorResponse;

use super::super::add_connector_response_to_additional_payment_data;

/// A connector that reports every card attribute it can, as Worldpay does when `<cardBin>` is
/// enabled on the account.
fn connector_card_response() -> AdditionalPaymentMethodConnectorResponse {
    AdditionalPaymentMethodConnectorResponse::Card {
        authentication_data: None,
        payment_checks: None,
        card_network: None,
        domestic_network: None,
        auth_code: Some("123456".to_string()),
        processor_card_network: Some(common_enums::CardNetwork::Visa),
        card_subtype: Some("ELECTRON".to_string()),
        card_segment_type: Some(common_enums::CardSegmentType::Commercial),
        funding_source: Some(common_enums::FundingSource::Debit),
        card_type: Some(common_enums::CardType::Debit),
        issuer_name: Some("CONNECTOR ISSUER".to_string()),
        issuer_country: Some(common_enums::CountryAlpha2::GB),
    }
}

fn merge(
    additional_card_info: api_models::payments::AdditionalCardInfo,
) -> api_models::payments::AdditionalCardInfo {
    let merged = add_connector_response_to_additional_payment_data(
        api_models::payments::AdditionalPaymentData::Card(Box::new(additional_card_info)),
        connector_card_response(),
    );

    match merged {
        api_models::payments::AdditionalPaymentData::Card(card) => *card,
        other => panic!("expected card additional payment data, got {other:?}"),
    }
}

#[test]
fn connector_card_attributes_fill_an_empty_card_record() {
    let card = merge(api_models::payments::AdditionalCardInfo {
        card_isin: Some("444433".to_string()),
        ..Default::default()
    });

    assert_eq!(card.card_issuer.as_deref(), Some("CONNECTOR ISSUER"));
    assert_eq!(card.card_type.as_deref(), Some("DEBIT"));
    assert_eq!(card.card_subtype.as_deref(), Some("ELECTRON"));
    assert_eq!(
        card.card_segment_type,
        Some(common_enums::CardSegmentType::Commercial)
    );
    assert_eq!(
        card.funding_source,
        Some(common_enums::FundingSource::Debit)
    );
    assert_eq!(card.card_issuing_country.as_deref(), Some("GB"));
    assert_eq!(card.card_issuing_country_code.as_deref(), Some("GB"));
    assert_eq!(card.card_network, Some(common_enums::CardNetwork::Visa));
    assert_eq!(card.auth_code.as_deref(), Some("123456"));
    assert_eq!(card.card_isin.as_deref(), Some("444433"));
}

#[test]
fn resolved_values_win_and_the_connector_only_fills_the_gaps() {
    let card = merge(api_models::payments::AdditionalCardInfo {
        card_issuer: Some("CURATED ISSUER".to_string()),
        card_type: Some("CREDIT".to_string()),
        card_issuing_country: Some("IN".to_string()),
        ..Default::default()
    });

    assert_eq!(card.card_issuer.as_deref(), Some("CURATED ISSUER"));
    assert_eq!(card.card_type.as_deref(), Some("CREDIT"));
    assert_eq!(card.card_issuing_country.as_deref(), Some("IN"));
    // The columns the BIN record left null are filled from the connector rather than staying
    // empty, which is the case that matters for BINs whose subtype is not recorded.
    assert_eq!(card.card_subtype.as_deref(), Some("ELECTRON"));
    assert_eq!(
        card.funding_source,
        Some(common_enums::FundingSource::Debit)
    );
    assert_eq!(
        card.card_segment_type,
        Some(common_enums::CardSegmentType::Commercial)
    );
    assert_eq!(card.auth_code.as_deref(), Some("123456"));
}

#[test]
fn a_cobadge_resolved_network_is_not_overwritten() {
    let card = merge(api_models::payments::AdditionalCardInfo {
        card_network: Some(common_enums::CardNetwork::RuPay),
        ..Default::default()
    });

    // Debit routing selects the rail the payment authorizes on, so the connector's plain scheme
    // network must not replace it.
    assert_eq!(card.card_network, Some(common_enums::CardNetwork::RuPay));
    assert_eq!(card.card_issuer.as_deref(), Some("CONNECTOR ISSUER"));
}
