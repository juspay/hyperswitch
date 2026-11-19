use api_models::enums::Connector;
use common_enums::{Currency, PaymentMethod};
use error_stack::report;

use crate::{
    configs::settings::CrossCurrencyMitConfig,
    core::errors::{ApiErrorResponse, RouterResult},
};

/// Validate the selected connector mandate on both initial attempts and retries.
///
/// Only card connector mandates can opt out of currency equality. Legacy mandates
/// with amount limits are validated separately and must not use this exception.
pub(super) fn validate_connector_mandate_currency(
    config: &CrossCurrencyMitConfig,
    connector: Connector,
    payment_method: Option<PaymentMethod>,
    original_currency: Option<Currency>,
    payment_currency: Currency,
) -> RouterResult<()> {
    let Some(original_currency) = original_currency else {
        return Ok(());
    };

    if original_currency == payment_currency {
        return Ok(());
    }

    if payment_method == Some(PaymentMethod::Card) && config.connector_list.contains(&connector) {
        router_env::logger::info!(
            %connector,
            %original_currency,
            %payment_currency,
            "allowing cross-currency card connector mandate"
        );
        return Ok(());
    }

    Err(report!(ApiErrorResponse::MandateValidationFailed {
        reason: "Cross currency mandates not supported".into(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stripe_config() -> CrossCurrencyMitConfig {
        CrossCurrencyMitConfig {
            connector_list: [Connector::Stripe].into_iter().collect(),
        }
    }

    #[test]
    fn default_config_preserves_currency_validation() {
        let result = validate_connector_mandate_currency(
            &CrossCurrencyMitConfig::default(),
            Connector::Stripe,
            Some(PaymentMethod::Card),
            Some(Currency::USD),
            Currency::CAD,
        );
        assert!(matches!(
            result
                .expect_err("cross-currency mandates must be rejected by default")
                .current_context(),
            ApiErrorResponse::MandateValidationFailed { .. }
        ));
    }

    #[test]
    fn configured_card_connector_allows_both_currency_directions() {
        for (original, requested) in [
            (Currency::USD, Currency::CAD),
            (Currency::CAD, Currency::USD),
        ] {
            assert!(validate_connector_mandate_currency(
                &stripe_config(),
                Connector::Stripe,
                Some(PaymentMethod::Card),
                Some(original),
                requested,
            )
            .is_ok());
        }
    }

    #[test]
    fn same_currency_and_missing_original_currency_remain_allowed() {
        for original in [Some(Currency::USD), None] {
            assert!(validate_connector_mandate_currency(
                &CrossCurrencyMitConfig::default(),
                Connector::Stripe,
                Some(PaymentMethod::Card),
                original,
                Currency::USD,
            )
            .is_ok());
        }
    }

    #[test]
    fn each_selected_connector_is_checked_independently() {
        let config = stripe_config();
        assert!(validate_connector_mandate_currency(
            &config,
            Connector::Stripe,
            Some(PaymentMethod::Card),
            Some(Currency::USD),
            Currency::CAD,
        )
        .is_ok());
        assert!(validate_connector_mandate_currency(
            &config,
            Connector::Cybersource,
            Some(PaymentMethod::Card),
            Some(Currency::USD),
            Currency::CAD,
        )
        .is_err());
    }

    #[test]
    fn non_card_and_unknown_payment_methods_remain_restricted() {
        for method in [
            Some(PaymentMethod::BankDebit),
            Some(PaymentMethod::Wallet),
            None,
        ] {
            assert!(validate_connector_mandate_currency(
                &stripe_config(),
                Connector::Stripe,
                method,
                Some(Currency::USD),
                Currency::EUR,
            )
            .is_err());
        }
    }

    #[test]
    fn connector_config_deserializes_and_rejects_unsupported_currency_rules() {
        let config: CrossCurrencyMitConfig =
            serde_json::from_str(r#"{"connector_list":["stripe"]}"#)
                .expect("connector list should deserialize");
        assert!(config.connector_list.contains(&Connector::Stripe));
        let empty: CrossCurrencyMitConfig =
            serde_json::from_str("{}").expect("empty config should deserialize");
        assert!(empty.connector_list.is_empty());
        assert!(serde_json::from_str::<CrossCurrencyMitConfig>(
            r#"{"connector_list":["stripe"],"currencies":["USD","CAD"]}"#
        )
        .is_err());
    }
}
