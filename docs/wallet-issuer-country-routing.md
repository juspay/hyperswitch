# Issuer-country routing for Apple Pay and Google Pay

The V1 confirm flow can populate `BackendInput.issuer_data.country` before evaluating
algorithmic routing. Both Hyperswitch's rule evaluator and the Decision Engine's
`/routing/hybrid` static request consume this field. The Decision Engine receives it as
`parameters.issuer_country` while the payment method remains `wallet` and the subtype
remains `apple_pay` or `google_pay`.

For example, a rule can combine these conditions with AND:

- `payment_method == wallet`
- `payment_method_type == apple_pay`
- `issuer_country == UnitedStatesOfAmerica`

## Country source

Issuer country is read from the `country_code` of the `cards_info` record for the eight-digit
BIN of the decrypted wallet payment number. This supports Google Pay `PAN_ONLY` and
`CRYPTOGRAM_3DS`, and Apple Pay's decrypted device PAN. For tokenized payments, the BIN
belongs to the device/network token; the BIN database must contain appropriate issuer
metadata for that token range. This does not recover the original funding PAN or infer
its country from a token range without a matching record. The lookup uses eight digits only,
matching wallet eligibility; it does not fall back to a six-digit BIN record.

Already-decrypted wallet data can be looked up directly. For an encrypted token, the
flow uses the same wallet-specific MCA recorded in the attempt's `pre_routing_results`
that wallet eligibility uses to resolve decryption keys. Only configured Hyperswitch-side
decryption is attempted. An unknown session MCA or connector-side-only decryption leaves
issuer country absent. This routing-only operation does not replace the encrypted payment
data or change the later connector-specific decryption and validation flow. The selected
connector must still support the original token under its wallet configuration; this does
not translate tokens between incompatible connector-specific wallet sessions.

## Missing data and routing precedence

Missing BIN records, invalid country codes, storage failures, and routing-only decryption
failures leave issuer country unset. No billing-country or wallet-display-name fallback is
used. A rule requiring issuer country therefore does not match when it is unavailable;
subsequent rules and the configured default keep their existing behavior.

This enrichment applies when a V1 confirm/create-with-confirm operation invokes
algorithmic connector selection. Existing predetermined connectors, explicit
straight-through routing, and session/pre-routing connector precedence are unchanged.
It does not reroute an already-pinned wallet session, add issuer-country support to
non-card-backed wallets, or change V2's routing input construction.
