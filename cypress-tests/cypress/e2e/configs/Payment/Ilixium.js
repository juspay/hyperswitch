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

// Ilixium requires the cardholder's date of birth on every authorisation. It travels as
// `customer.date_of_birth` (ISO-8601), which the router forwards to UCS on
// `Customer.date_of_birth`; the Ilixium transformer reformats it to the `ddmmyyyy` the
// processor wants. Absent it, Ilixium answers `VA8` and the payment is REJECTED.
const ilixiumCustomer = {
  date_of_birth: "1990-01-01",
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
        customer: ilixiumCustomer,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    // UNVERIFIED — inferred from No3DSManualCapture by only changing
    // capture_method; not confirmed against a live run.
    No3DSAutoCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        amount: 1000,
        payment_method_data: {
          card: verifiedNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        customer: ilixiumCustomer,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),
    // UNVERIFIED — standard full-capture shape, not confirmed against a live run.
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
  },
};
