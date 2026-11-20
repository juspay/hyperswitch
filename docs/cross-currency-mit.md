# Cross-currency MIT

Allow connector mandates to be reused across currencies for selected connectors:

```toml
[cross_currency_mit_supported_connectors]
connector_list = ["stripe"]
```

Or set `ROUTER__CROSS_CURRENCY_MIT_SUPPORTED_CONNECTORS__CONNECTOR_LIST=stripe` (comma-separated for multiple connectors). Reload the router configuration for changes to take effect.

- Empty by default: currency mismatches return HE_03.
- Listed connectors: mismatches are allowed across payment methods and currencies, for all merchants in the deployment.
- Matching currencies or a missing original currency pass this check as before.
- Both connector-mandate routing paths and automatic retries use the shared validator in `payments/helpers.rs`.
- Legacy mandate amount/currency checks and NTI paths are unchanged.

This config bypasses HS currency equality validation; it does not convert amounts or guarantee connector/account support. Confirm that support before enabling. Future currency restrictions can extend the same config and validator.
