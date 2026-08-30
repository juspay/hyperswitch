import { getCustomExchange } from "./Modifiers";

// Ilixium is UCS-only (routed entirely through the Unified Connector Service; see
// crates/hyperswitch_connectors/src/connectors/ilixium/transformers.rs, whose local
// TryFrom impl is a stub — it unconditionally returns NotImplemented for every
// payment method, so it is never exercised for this connector).
//
// Verified against real request/response pairs from
// https://github.com/juspay/hyperswitch/issues/13708 (manual capture confirm,
// three_ds confirm, capture, void) plus a live cypress run for No3DSManualCapture:
// No3DSManualCapture, "3DSManualCapture", Capture, PartialCapture, Void.
//
// A prior version of this file encoded Capture/PartialCapture as expected to fail
// with a "capture_method of manual, expected manual_multiple" error, based on a
// cypress run that actually hit that error. Issue #13708 shows a real, successful
// full capture on the exact same capture_method: "manual" shape, so that failure
// was very likely the UCS card_Capture rollout config not being active for that
// test's merchant at the time (see run-ilixium-ucs.sh's CYPRESS_METHOD_FLOW and
// the "Enable ucs config for capture" step in the issue) — not a real Ilixium or
// hyperswitch limitation. If Capture/PartialCapture fail again with that error,
// check the UCS rollout config for card_Capture is actually registered for the
// current test merchant before assuming connector behavior changed.
//
// No3DSAutoCapture is still unverified/broken: a live confirm with capture_method:
// "automatic" returned HTTP 200 status "succeeded" but a non-null
// error_message: "4" — not investigated further.
const verifiedCardDetails = {
  card_number: "9000100111111117",
  card_exp_month: "06",
  card_exp_year: "28",
  card_holder_name: "John Doe",
  card_cvc: "111",
};

// Ilixium requires the cardholder's date of birth as connector-specific metadata;
// confirm calls without it were not tested and may fail validation.
const ilixiumMetadata = {
  ilixium_date_of_birth: "01011990",
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "USD",
        amount: 1000,
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    // Verified (issue #13708 + live cypress run): authentication_type: "no_three_ds",
    // capture_method: "manual".
    No3DSManualCapture: {
      Request: {
        payment_method: "card",
        amount: 1000,
        payment_method_data: {
          card: verifiedCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        metadata: ilixiumMetadata,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    // Verified (issue #13708): authentication_type: "three_ds" with this test card
    // is frictionless — no next_action/challenge, goes straight to
    // requires_capture (whole_connector_response shows threeDSecureStatus:
    // "NOT_ENROLLED", i.e. this card isn't 3DS-enrolled on Ilixium's simulator).
    "3DSManualCapture": getCustomExchange({
      Request: {
        payment_method: "card",
        amount: 1000,
        payment_method_data: {
          card: verifiedCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        authentication_type: "three_ds",
        metadata: ilixiumMetadata,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    }),
    // BROKEN, not just unverified: a live confirm with capture_method: "automatic"
    // returned HTTP 200 status "succeeded" but a non-null error_message: "4" (no
    // error_code/full body captured). Left as a visible failure until the real
    // body is captured — do not treat this connector as supporting auto-capture
    // based on this entry.
    No3DSAutoCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        amount: 1000,
        payment_method_data: {
          card: verifiedCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        metadata: ilixiumMetadata,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          error_message: null, // known wrong — real value observed was "4"
        },
      },
    }),
    // Verified (issue #13708): full capture on a capture_method: "manual" payment
    // succeeds normally once the UCS card_Capture rollout config is active — see
    // the file header note above if this starts failing with a
    // manual/manual_multiple error again.
    Capture: getCustomExchange({
      Request: {
        amount_to_capture: 1000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          amount: 1000,
          amount_capturable: 0,
          amount_received: 1000,
        },
      },
    }),
    // UNVERIFIED: no partial-amount example exists in issue #13708 (only a
    // full-amount capture). Modeled on Capture above on the assumption partial
    // capture behaves the same way once UCS card_Capture is active; not
    // independently confirmed.
    PartialCapture: getCustomExchange({
      Request: {
        amount_to_capture: 500,
      },
      Response: {
        status: 200,
        body: {
          status: "partially_captured",
          amount: 1000,
          amount_capturable: 0,
          amount_received: 500,
        },
      },
    }),
    // Verified (issue #13708): cancellation_reason: "requested_by_customer".
    Void: {
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "cancelled",
        },
      },
    },
  },
};
