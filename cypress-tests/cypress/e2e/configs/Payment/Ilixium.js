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
// Two prior versions of this file misdiagnosed a "capture_method of manual,
// expected manual_multiple" error hit in live cypress runs: first as genuine
// connector behavior (wrong — issue #13708 shows a real successful capture on
// the same shape), then as a UCS rollout-config gap (also wrong — Ilixium is
// UCS-only at the deployment-config level, so unified_connector_service.rs's
// decide_execution_path forces every flow through UCS unconditionally,
// independent of any rollout config). The real cause, traced via
// crates/router/src/core/payments/helpers.rs's validate_status_with_capture_method:
// it rejects a plain "manual" capture while the intent is in
// IntentStatus::Processing, and Ilixium's authorize apparently settles from
// Processing to RequiresCapture asynchronously. Cypress's fast confirm->retrieve->
// capture sequence can land the capture call inside that window; #13708's manual
// Postman flow never hit it because of the natural delay between clicking through
// steps by hand. Capture/PartialCapture below carry a DELAY config for this reason
// (see their own comments) rather than a data/response fix.
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
    // succeeds. The manual/manual_multiple error seen in earlier cypress runs
    // traced back to crates/router/src/core/payments/helpers.rs's
    // validate_status_with_capture_method: it rejects a plain "manual" capture
    // while the intent sits in IntentStatus::Processing, and Ilixium's authorize
    // apparently settles from Processing to RequiresCapture asynchronously — a
    // race the manual Postman flow in #13708 never hit simply because of the
    // natural delay between clicking through steps by hand. DELAY here gives
    // that settling time to complete before capturing, same pattern already used
    // by Nuvei's Capture/PartialCapture/Void entries in this file's sibling
    // configs for the same kind of async post-authorize settling.
    Capture: getCustomExchange({
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
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
    // capture behaves the same way; not independently confirmed. Same DELAY as
    // Capture above, for the same Processing-state race.
    PartialCapture: getCustomExchange({
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
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
