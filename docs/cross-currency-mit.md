# Cross currency card MIT configuration

HS normally rejects a connector-mandate payment if the requested currency differs from the mandate's stored original currency. The `cross_currency_mit_supported_connectors` configuration allows selected connectors to bypass this equality check for saved cards.

```toml
[cross_currency_mit_supported_connectors]
connector_list = ["stripe"]
```

The equivalent environment variable is `ROUTER__CROSS_CURRENCY_MIT_SUPPORTED_CONNECTORS__CONNECTOR_LIST=stripe` (comma-separated for multiple connectors).

The list is empty by default. Adding a connector applies to all merchants using that connector in the deployment. It is an operational opt-in, not a statement that every account, card network, issuer, or currency combination is supported. Verify the intended processor and merchant-account configuration before enabling it. Changes take effect when the router loads the updated application configuration.

## Validation behavior

`payments::helpers::validate_connector_mandate_currency` owns the comparison and relaxation decision. Both connector-mandate routing paths call it. The automatic retry path passes the configuration into `get_mandate_reference_id`, which checks the connector selected for that attempt.

| Condition | Result of this validation |
| --- | --- |
| Original currency absent | Allow, preserving existing behavior |
| Original and requested currencies match | Allow |
| Currencies differ, payment method is card, and connector is configured | Allow |
| Any other currency mismatch | Reject with HE_03, Mandate Validation Failed |

The card-only condition is a conservative scope of this configuration, not an inherent limitation of connector mandates. HS also routes saved wallets through connector mandates. A connector-wide exception without this condition would relax those payment-method flows too, even though their cross-currency behavior has not been established.

Bank debits, wallets, and unknown payment methods do not receive this exception. Legacy mandate amount and currency validation remains separate. NTI paths retain their existing behavior. A rejection stops the attempt; it does not automatically skip to another connector.

This configuration does not convert amounts or change stored mandate data. Other validations and downstream authorization still apply. In particular, enabling a connector does not establish that specialized mandates with explicit currency restrictions can be reused across currencies.

## Adding a currency dimension later

This release implements only connector eligibility. If a narrower policy becomes necessary, extend `CrossCurrencyMitSupportedConnectors` with an optional per-connector currency restriction and evaluate it inside the same shared validator. Existing connector-list configurations can retain their current meaning when no additional restriction is supplied.

For a configured set such as USD/CAD, require both the original and requested currency to be in the set. An explicit empty set should permit no currency mismatch. If eligibility differs by direction, use ordered currency pairs instead of inferring all combinations from a set. Currency restrictions are not accepted by the current configuration schema; unknown fields are rejected to avoid silently ignoring an intended restriction.

A connector's general presentment-currency list is insufficient evidence of cross-currency MIT support. Before introducing currency rules, identify the policy owner and obtain evidence scoped to the merchant account and flow. No dedicated public cross-currency MIT currency matrix was identified in the Stripe documentation reviewed. [Stripe currencies](https://docs.stripe.com/currencies) and [Country Specs](https://docs.stripe.com/api/country_specs/object) describe general payment-currency support, rather than original-to-subsequent currency combinations.

Connector-only relaxation is broader than a USD/CAD pilot: it permits other currency mismatches on the configured card flow. Choose that scope explicitly rather than assuming that a successful USD/CAD test proves every currency combination.
