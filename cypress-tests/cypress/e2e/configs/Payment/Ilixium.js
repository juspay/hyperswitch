import { getCustomExchange } from "./Modifiers";

// Verified against a live confirm call on integ.hyperswitch.io (capture_method:
// "manual", authentication_type: "no_three_ds") — this is the only card/response
// combination actually confirmed working for Ilixium; everything else in this file
// is inferred from that single reference call and is UNVERIFIED. Ilixium is
// UCS-only (routed entirely through the Unified Connector Service; see
// crates/hyperswitch_connectors/src/connectors/ilixium/transformers.rs, whose local
// TryFrom impl is a stub — it unconditionally returns NotImplemented for every
// payment method, so it is never exercised for this connector) and its local
// request-building/response-parsing code is not something this repo can be used to
// verify decline behavior, 3DS support, or refund/void semantics against. Treat
// anything not explicitly marked "verified" below as a starting point to confirm
// against a live run, not as trusted data.
const verifiedNo3DSCardDetails = {
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
    // Verified: matches the confirmed live request (capture_method: "manual",
    // authentication_type: "no_three_ds") except for fields confirmCallTest
    // supplies itself (client_secret, confirm, profile_id).
    No3DSManualCapture: {
      Request: {
        payment_method: "card",
        amount: 1000,
        payment_method_data: {
          card: verifiedNo3DSCardDetails,
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
    // BROKEN, not just unverified: a live run against capture_method: "automatic"
    // returned HTTP 200 with status "succeeded" but a non-null error_message: "4"
    // (no error_code/full body captured yet). Whatever that means server-side,
    // it isn't the clean success this entry used to claim. Left as a visible
    // failure (rather than removed) until the real body is captured and this can
    // be encoded correctly — do not mark this connector as supporting
    // auto-capture based on this entry.
    No3DSAutoCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        amount: 1000,
        payment_method_data: {
          card: verifiedNo3DSCardDetails,
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
    // Verified: same hyperswitch-level validation as PartialCapture below — the
    // error text is about capture_method state generally (not "partial" vs
    // "full"), and a live run confirmed it fires identically here.
    Capture: getCustomExchange({
      Request: {
        amount_to_capture: 1000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "This Payment could not be captured because it has a capture_method of manual. The expected state is manual_multiple",
          },
        },
      },
    }),
    // Verified: a live run's capture request against a plain "manual"
    // capture_method payment returned this exact error (hyperswitch-level
    // validation, not Ilixium-specific — it wants capture_method:
    // "manual_multiple" to allow calling /capture at all here). error.code is
    // unconfirmed — not present in the assertion trail we have, since the
    // message mismatch stopped the check before code was compared.
    PartialCapture: getCustomExchange({
      Request: {
        amount_to_capture: 500,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "This Payment could not be captured because it has a capture_method of manual. The expected state is manual_multiple",
          },
        },
      },
    }),
  },
};
