import { getCustomExchange } from "./Modifiers";
import { connectorDetails as commonsConnectorDetails } from "./Commons";

// Citigate is a UCS-only connector (see PR #13704): the integration itself
// lives in hyperswitch-prism, this repo only registers enum/auth/config
// plumbing. Scope per the PR is card only, one-time payments only
// (Authorize + PSync, non-3DS and 3DS redirect) — no mandates, no
// recurring/MIT, and no separate Capture/Void endpoint was exercised in the
// PR's manual E2E evidence, so those flows are skipped below rather than
// asserted against a guessed response.

// Citigate's sandbox rejects a repeat Authorize with the same card + amount
// as "Duplicate transaction" (observed on identical back-to-back requests
// within the same run). A Cypress retry of a failed test would resend that
// exact same request and get flagged, so disable retries here rather than
// chase a false negative — same approach as Helcim.
if (Cypress.env("CONNECTOR") === "citigate") {
  Cypress.config("retries", 0);
}

const successfulNo3DSCardDetails = {
  card_number: "4111111111111111",
  card_exp_month: "12",
  card_exp_year: "2030",
  card_holder_name: "John Phillips",
  card_cvc: "123",
  card_network: "Visa",
};

const successfulThreeDSTestCardDetails = {
  card_number: "4111111111111111",
  card_exp_month: "12",
  card_exp_year: "2030",
  card_holder_name: "John Phillips",
  card_cvc: "123",
  card_network: "Visa",
};

// Citigate rejects Authorize with IR_04 "Missing required field:
// payment_method_data.billing.phone" — the shared cypress fixture
// (create-confirm-body.json) sets billing.address but not billing.phone, so
// every card_pm request below needs an explicit override.
//
// Citigate's sandbox also flags "Duplicate transaction" whenever a request
// looks identical to one already accepted (same card, amount, and
// cardholder identity) within its detection window. Several spec files
// (e.g. 04-NoThreeDSAutoCapture.cy.js) read the *same* connectorDetails
// entry — same object reference — for more than one `it()` block, so a
// plain precomputed billing object is byte-identical across those calls
// and still collides. `citigateBillingAddress` is used as a getter below so
// each read recomputes a fresh identity. `Cypress.config("retries", 0")`
// above stops a Cypress *retry* from resending the exact same request, and
// the DELAY configs add a cooldown between the real Authorize calls that
// run one after another in the same spec.
let citigateIdentitySeq = 0;
const citigateBillingAddress = () => {
  citigateIdentitySeq += 1;
  const nonce = `${Date.now()}-${citigateIdentitySeq}`;
  return {
    address: {
      line1: "92 West Broadway",
      city: "New York",
      state: "NY",
      zip: "10007",
      country: "US",
      first_name: "John",
      last_name: `Phillips-${nonce}`,
    },
    phone: {
      number: "8056594427",
      country_code: "+1",
    },
    email: `john.phillips.${nonce}@gmail.com`,
  };
};

// Commons.js's generic `card_pm` has well over a hundred scenario keys
// covering capabilities (capture, refund, mandates, zero-auth, card
// blocking, payment-method listing, ...) far beyond this PR's stated scope.
// mergeConnectorDetails (Utils.js) silently falls back to Commons's version
// of any key this file doesn't define — which carries the *generic*
// "not implemented" expectation most connectors share, not anything
// verified against Citigate specifically. A request still fires for real
// in that case (TRIGGER_SKIP only works when a connector config actually
// sets it), so an undefined key is worse than an explicit skip: it asserts
// an unverified guess against live connector behavior instead of skipping.
//
// Router-level input-validation keys (bad card number/CVC/expiry/currency/
// payment method, missing params, bad publishable key) are excluded here:
// those fail identically for every connector before the request ever
// reaches Citigate, so Commons's generic expectation already holds and
// doesn't need an override.
const ROUTER_LEVEL_VALIDATION_KEYS = new Set([
  "InvalidCardCvv",
  "InvalidCardNumber",
  "InvalidCurrency",
  "InvalidExpiryMonth",
  "InvalidExpiryYear",
  "InvalidPaymentMethod",
  "InvalidPublishableKey",
  "MissingRequiredParam",
]);

// Keys this file defines explicitly below, with real behavior captured
// from the PR's manual E2E evidence or this session's own live testing.
// Everything else in Commons's card_pm gets auto-skipped instead of
// silently inheriting a generic, unverified expectation.
const EXPLICITLY_VERIFIED_CARD_PM_KEYS = new Set([
  "PaymentIntent",
  "PaymentIntentWithShippingCost",
  "PaymentConfirmWithShippingCost",
  "No3DSAutoCapture",
  "No3DSManualCapture",
  "No3DSFailPayment",
  "3DSAutoCapture",
  "3DSManualCapture",
  "Capture",
  "PartialCapture",
  "Void",
  "Refund",
  "PartialRefund",
  "SyncRefund",
  "manualPaymentRefund",
  "manualPaymentPartialRefund",
  "MandateSingleUseNo3DSAutoCapture",
  "MandateSingleUseNo3DSManualCapture",
  "MandateMultiUseNo3DSAutoCapture",
  "MandateMultiUseNo3DSManualCapture",
  "MITAutoCapture",
  "MITManualCapture",
  "MITWithoutBillingAddress",
  "PaymentMethodIdMandateNo3DSAutoCapture",
  "PaymentMethodIdMandateNo3DSManualCapture",
  "PaymentMethodIdMandate3DSAutoCapture",
  "PaymentMethodIdMandate3DSManualCapture",
  "SaveCardUseNo3DSAutoCapture",
  "SaveCardUseNo3DSAutoCaptureOffSession",
  "ZeroAuthMandate",
]);

const autoSkippedCardPm = Object.fromEntries(
  Object.keys(commonsConnectorDetails.card_pm)
    .filter(
      (key) =>
        !EXPLICITLY_VERIFIED_CARD_PM_KEYS.has(key) &&
        !ROUTER_LEVEL_VALIDATION_KEYS.has(key)
    )
    .map((key) => [
      key,
      getCustomExchange({
        Request: {},
        Configs: { TRIGGER_SKIP: true },
      }),
    ])
);

export const connectorDetails = {
  card_pm: {
    ...autoSkippedCardPm,
    // Shared by every spec that confirms in two steps (04-, 05-, 06-, 07-,
    // 08-, 09-, 16-, ...). Routing to Citigate's 3DS MID (connector_2) has
    // to happen here, at intent creation — the confirm step can't
    // retroactively move an already-created intent to a different
    // profile/MCA, so CONNECTOR_CREDENTIAL on "3DSAutoCapture"/
    // "3DSManualCapture" alone isn't enough.
    //
    // Routed by the actual `authentication_type` ("three_ds" vs
    // "no_three_ds") the calling spec passes to createPaymentIntentTest,
    // not by which spec file is running — a filename allowlist has to be
    // kept in sync by hand every time a new spec exercises this same
    // shared "PaymentIntent" key, and silently falls back to the wrong MID
    // (connector_1) when it isn't (see #14656 CI failure in
    // 09-RefundPayment.cy.js for exactly this).
    PaymentIntent: {
      Configs: {
        CONNECTOR_CREDENTIAL: {
          byAuthenticationType: "three_ds",
          value: "connector_2",
        },
      },
      Request: {
        currency: "USD",
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    PaymentIntentWithShippingCost: {
      Request: {
        currency: "USD",
        shipping_cost: 50,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          shipping_cost: 50,
        },
      },
    },
    PaymentConfirmWithShippingCost: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 30000,
        },
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        customer_acceptance: null,
        get billing() {
          return citigateBillingAddress();
        },
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          shipping_cost: 50,
        },
      },
    },
    // Matches the non-3DS Authorize captured in the PR's manual E2E run:
    // gateway ResponseCode "0" ("Approved") comes back synchronously and
    // hyperswitch maps it straight to `succeeded`.
    No3DSAutoCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 30000,
        },
      },
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        get billing() {
          return citigateBillingAddress();
        },
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    // No separate Capture endpoint was exercised against the Citigate
    // sandbox in the PR, so manual capture is treated as unimplemented
    // rather than asserted against a guessed status.
    No3DSManualCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        get billing() {
          return citigateBillingAddress();
        },
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
    // This generic cross-connector scenario (23-Variations.cy.js) wasn't
    // exercised against the Citigate sandbox in the PR's manual E2E
    // evidence, and the default "not implemented" expectation most
    // connectors share here doesn't hold for Citigate in any verified
    // way (confirmCallTest would need a real billing.phone override just
    // to reach the gateway at all — Citigate rejects every request
    // missing it with its own validation error, before any connector-level
    // behavior is exercised). Skip rather than guess.
    No3DSFailPayment: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
        get billing() {
          return citigateBillingAddress();
        },
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
    // Matches the 3DS Authorize captured in the PR's manual E2E run: gateway
    // ResponseCode "600" ("Redirect required") maps to
    // `requires_customer_action` with a `redirect_to_url` next action.
    //
    // Citigate's sandbox uses a *separate* MID for 3DS (creds.json
    // connector_2, "Dummy2") from the non-3DS one (connector_1, "Dummy1")
    // used everywhere else in this file — confirmed by the PR's own manual
    // E2E evidence, which created a second profile/MCA for the 3DS run.
    // Without CONNECTOR_CREDENTIAL routing here, this falls back to
    // connector_1 and Citigate approves outright instead of challenging.
    "3DSAutoCapture": {
      Configs: {
        CONNECTOR_CREDENTIAL: {
          value: "connector_2",
        },
        DELAY: {
          STATUS: true,
          TIMEOUT: 30000,
        },
      },
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        authentication_type: "three_ds",
        get billing() {
          return citigateBillingAddress();
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    "3DSManualCapture": getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        authentication_type: "three_ds",
        get billing() {
          return citigateBillingAddress();
        },
      },
      Configs: {
        CONNECTOR_CREDENTIAL: {
          value: "connector_2",
        },
        TRIGGER_SKIP: true,
      },
    }),
    Capture: getCustomExchange({
      Request: {
        amount_to_capture: 6000,
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
    PartialCapture: getCustomExchange({
      Request: {
        amount_to_capture: 2000,
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
    // Cancelling a payment that's still `requires_payment_method` (never
    // reached the connector) is handled entirely router-side — it doesn't
    // depend on Citigate implementing a Void endpoint, so this isn't
    // TRIGGER_SKIP'd like the post-authorize Capture/PartialCapture cases.
    // (voidCallTest also doesn't honor TRIGGER_SKIP at all — it always
    // fires the request regardless of this config.)
    Void: {
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "cancelled",
        },
      },
    },
    // Refund/PSync-of-refund were not part of the PR's manual E2E evidence
    // either; skip rather than assert a guessed shape.
    Refund: getCustomExchange({
      Request: {
        amount: 6000,
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
    PartialRefund: getCustomExchange({
      Request: {
        amount: 2000,
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
    manualPaymentRefund: getCustomExchange({
      Request: {
        amount: 6000,
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
    manualPaymentPartialRefund: getCustomExchange({
      Request: {
        amount: 2000,
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
    SyncRefund: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
    // PR scope explicitly excludes mandates/recurring/MIT for Citigate.
    MandateSingleUseNo3DSAutoCapture: getCustomExchange({
      Request: {
        amount: 6000,
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
    MandateSingleUseNo3DSManualCapture: getCustomExchange({
      Request: {
        amount: 6000,
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
    MandateMultiUseNo3DSAutoCapture: getCustomExchange({
      Request: {
        amount: 6000,
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
    MandateMultiUseNo3DSManualCapture: getCustomExchange({
      Request: {
        amount: 6000,
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
    // Missing from this file entirely, which is worse than TRIGGER_SKIP'd:
    // getConnectorDetails merges in Commons.js's generic default for any key
    // a connector doesn't override, so a request fired for real here with no
    // override still carries the generic "not implemented" expectation
    // instead of being skipped. Zero-auth/multi-use mandates are explicitly
    // out of scope (same as the other Mandate*/MIT* keys above).
    ZeroAuthMandate: getCustomExchange({
      Request: {
        amount: 0,
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
    MITAutoCapture: getCustomExchange({
      Request: { amount: 6000 },
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
    MITManualCapture: getCustomExchange({
      Request: { amount: 6000 },
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
    MITWithoutBillingAddress: getCustomExchange({
      Request: { amount: 6000, billing: null },
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
    PaymentMethodIdMandateNo3DSAutoCapture: getCustomExchange({
      Request: {
        amount: 6000,
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: null,
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
    PaymentMethodIdMandateNo3DSManualCapture: getCustomExchange({
      Request: {
        amount: 6000,
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: null,
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
    PaymentMethodIdMandate3DSAutoCapture: getCustomExchange({
      Request: {
        amount: 6000,
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        mandate_data: null,
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
    PaymentMethodIdMandate3DSManualCapture: getCustomExchange({
      Request: {
        amount: 6000,
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        mandate_data: null,
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
    SaveCardUseNo3DSAutoCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        setup_future_usage: "on_session",
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
    SaveCardUseNo3DSAutoCaptureOffSession: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        setup_future_usage: "off_session",
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
  },
};
