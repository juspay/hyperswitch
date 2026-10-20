import { customerAcceptance } from "./Commons";
import { getCustomExchange } from "./Modifiers";

// RealEx sandbox test cards, confirmed live against integ.hyperswitch.io
// (PR #13980 test session). 4263970000005262 is Visa credit; the decline PAN
// below is RealEx's own standard "always declined" test number.
const successfulNo3DSCardDetails = {
  card_number: "4263970000005262",
  card_exp_month: "10",
  card_exp_year: "30",
  card_holder_name: "John Doe",
  card_cvc: "123",
};

const successfulThreeDSTestCardDetails = {
  card_number: "4263970000005262",
  card_exp_month: "10",
  card_exp_year: "30",
  card_holder_name: "John Doe",
  card_cvc: "123",
};

const declinedCardDetails = {
  card_number: "4000120000001154",
  card_exp_month: "10",
  card_exp_year: "30",
  card_holder_name: "John Doe",
  card_cvc: "123",
};

export const connectorDetails = {
  card_pm: {
    // Confirmed live: shipping_cost is supported end to end — net_amount
    // correctly computes as amount + shipping_cost. The spec drives this
    // from fixtures.createPaymentBody (amount: 6000), not a connector
    // override, so the expected amount/net_amount here must match that
    // fixture's default, not the 1000 used in ad hoc manual testing.
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
          amount: 6000,
        },
      },
    },
    PaymentConfirmWithShippingCost: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          shipping_cost: 50,
          amount: 6000,
          net_amount: 6050,
        },
      },
    },
    No3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    No3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    // Hyperswitch correctly calls UCS's PreAuthenticate for this attempt
    // instead of a plain XML auth (confirmed live via Loki — exactly one
    // UCS call, flow: "PreAuthenticate"). But the sandbox account available
    // for testing isn't 3DS-provisioned at all, so the attempt fails at
    // RealEx's own account check before any challenge/redirect exists to
    // assert on. This is the real, live-verified outcome, not a guess — see
    // pr-13980-test-results.md. Once a 3DS-enabled RealEx account is
    // available, this should be rewritten to assert the actual challenge
    // flow (requires_customer_action -> handleRedirection -> succeeded).
    "3DSAutoCapture": {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_message: "That Account ID is not configured for 3D Secure",
        },
      },
    },
    "3DSManualCapture": {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_message: "That Account ID is not configured for 3D Secure",
        },
      },
    },
    Capture: {
      Request: {
        amount_to_capture: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          amount: 6000,
          amount_capturable: 0,
          amount_received: 6000,
        },
      },
    },
    // Confirmed live: the connector doesn't support multi-capture —
    // amount_capturable goes straight to 0 after any capture, not to the
    // remaining uncaptured amount.
    PartialCapture: {
      Request: {
        amount_to_capture: 3000,
      },
      Response: {
        status: 200,
        body: {
          status: "partially_captured",
          amount: 6000,
          amount_capturable: 0,
          amount_received: 3000,
        },
      },
    },
    Void: {
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "cancelled",
        },
      },
    },
    Refund: {
      Request: {
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    // Confirmed live: RealEx only allows a single refund per payment —
    // 512 "This transaction has already been rebated and cannot be rebated
    // again" on any second refund attempt, regardless of amount. The specs
    // that use this key always call it twice against the same payment, and
    // can't express a different expectation per call, so this is skipped
    // rather than asserting a scenario the connector can't actually do.
    PartialRefund: {
      Configs: { TRIGGER_SKIP: true },
      Request: {
        amount: 3000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          amount: 3000,
        },
      },
    },
    SyncRefund: {
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    manualPaymentRefund: {
      Request: {
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          amount: 6000,
        },
      },
    },
    // Same single-refund-per-payment limitation as PartialRefund above.
    manualPaymentPartialRefund: {
      Configs: { TRIGGER_SKIP: true },
      Request: {
        amount: 3000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    SyncPayment: {
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    // RealEx's own standard "always declined" test PAN — confirmed live
    // (error_code "101", "[ test system ] Declined").
    DeclineCardPayment: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: declinedCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_code: "101",
          error_message: "[ test system ] Declined",
        },
      },
    },
    // 23-Variations.cy.js's own fail-payment scenario — same decline PAN
    // and verified result as DeclineCardPayment above, just under the key
    // that spec reads.
    No3DSFailPayment: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: declinedCardDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_code: "101",
          error_message: "[ test system ] Declined",
        },
      },
    },
    // Ground truth, not a guess: connector-service's
    // macro_connector_flow_status_impls! for GlobalpaymentsRealex
    // (crates/integrations/connector-integration/src/connectors/
    // globalpayments_realex.rs) explicitly lists MandateRevoke, SetupMandate,
    // RepeatPayment, and PaymentMethodToken under `not_implemented`. Mandate
    // setup (CIT) is SetupMandate; MIT/subsequent-payment reuse is
    // RepeatPayment; mandate listing/revocation is MandateRevoke. Only the
    // six core flows are wired: Authorize, PSync, Capture, Void, Refund,
    // RSync (see that file's own comment above the macro call).
    MandateSingleUseNo3DSAutoCapture: {
      Configs: { TRIGGER_SKIP: true },
      Request: {},
      Response: { status: 200, body: { status: "failed" } },
    },
    MandateMultiUseNo3DSAutoCapture: {
      Configs: { TRIGGER_SKIP: true },
      Request: {},
      Response: { status: 200, body: { status: "failed" } },
    },
    MandateSingleUseNo3DSManualCapture: {
      Configs: { TRIGGER_SKIP: true },
      Request: {},
      Response: { status: 200, body: { status: "failed" } },
    },
    MandateMultiUseNo3DSManualCapture: {
      Configs: { TRIGGER_SKIP: true },
      Request: {},
      Response: { status: 200, body: { status: "failed" } },
    },
    // MIT-via-payment_method_id and MIT-via-mandate-id confirms — same
    // RepeatPayment: not_implemented ground truth as the Mandate* keys
    // above. Request shapes copied from Commons.js's own definitions for
    // these keys (used here instead of inherited, per the one-level-deep
    // merge rule — see cypress-automation-standards.md §3b).
    MITAutoCapture: getCustomExchange({
      Configs: { TRIGGER_SKIP: true },
      Request: { amount: 6000 },
    }),
    MITAutoCaptureWithCustomerAcceptance: getCustomExchange({
      Configs: { TRIGGER_SKIP: true },
      Request: {
        amount: 6000,
        customer_acceptance: {
          acceptance_type: "offline",
          accepted_at: "1963-05-03T04:07:52.723Z",
          online: {
            ip_address: "127.0.0.1",
            user_agent: "amet irure esse",
          },
        },
      },
    }),
    // Not in Commons.js at all (connector-specific-only key); shape
    // mirrors Barclaycard's own MITManualCapture.
    MITManualCapture: getCustomExchange({
      Configs: { TRIGGER_SKIP: true },
      Request: { amount: 6000 },
    }),
    MITWithoutBillingAddress: getCustomExchange({
      Configs: { TRIGGER_SKIP: true },
      Request: { amount: 6000, billing: null },
    }),
    PaymentMethodIdMandateNo3DSAutoCapture: getCustomExchange({
      Configs: { TRIGGER_SKIP: true },
      Request: {
        amount: 6000,
        payment_method: "card",
        payment_method_data: { card: successfulNo3DSCardDetails },
        currency: "USD",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
      },
    }),
    PaymentMethodIdMandateNo3DSManualCapture: getCustomExchange({
      Configs: { TRIGGER_SKIP: true },
      Request: {
        amount: 6000,
        payment_method: "card",
        payment_method_data: { card: successfulNo3DSCardDetails },
        currency: "USD",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
      },
    }),
    PaymentMethodIdMandate3DSAutoCapture: getCustomExchange({
      Configs: { TRIGGER_SKIP: true },
      Request: {
        amount: 6000,
        payment_method: "card",
        payment_method_data: { card: successfulThreeDSTestCardDetails },
        currency: "USD",
        mandate_data: null,
        authentication_type: "three_ds",
        customer_acceptance: customerAcceptance,
      },
    }),
    PaymentMethodIdMandate3DSManualCapture: getCustomExchange({
      Configs: { TRIGGER_SKIP: true },
      Request: {
        amount: 6000,
        payment_method: "card",
        payment_method_data: { card: successfulThreeDSTestCardDetails },
        mandate_data: null,
        authentication_type: "three_ds",
        customer_acceptance: customerAcceptance,
      },
    }),
    SaveCardUseNo3DSAutoCapture: {
      Configs: { TRIGGER_SKIP: true },
      Request: {
        payment_method: "card",
        payment_method_data: { card: successfulNo3DSCardDetails },
        customer_acceptance: customerAcceptance,
      },
      Response: { status: 200, body: { status: "succeeded" } },
    },
    // Off-session / stored-credential (MIT-style) saved-card confirms.
    // Confirmed live (14-SaveCardFlow.cy.js) AND at the source: UCS's own
    // RepeatPayment flow for this connector is `not_implemented` (see the
    // ground-truth note above MandateSingleUseNo3DSAutoCapture). Response
    // is left to getCustomExchange's default (501, "Selected payment method
    // through <connector> is not implemented") rather than hardcoded here,
    // matching how every other not-implemented case in this codebase gets
    // its message from the one place that generates it (Modifiers.js) —
    // confirmed byte-identical to the live response.
    SaveCardUseNo3DSAutoCaptureOffSession: getCustomExchange({
      Configs: { TRIGGER_SKIP: true },
      Request: {
        payment_method: "card",
        payment_method_data: { card: successfulNo3DSCardDetails },
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
      },
    }),
    SaveCardUseNo3DSManualCaptureOffSession: getCustomExchange({
      Configs: { TRIGGER_SKIP: true },
      Request: {
        payment_method: "card",
        payment_method_data: { card: successfulNo3DSCardDetails },
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
      },
    }),
    SaveCardConfirmAutoCaptureOffSession: getCustomExchange({
      Configs: { TRIGGER_SKIP: true },
      Request: { setup_future_usage: "off_session" },
    }),
    SaveCardConfirmManualCaptureOffSession: getCustomExchange({
      Configs: { TRIGGER_SKIP: true },
      Request: { setup_future_usage: "off_session" },
    }),
    // Not skipped — real, live-verified behavior (24-PaymentMethods.cy.js).
    // authentication_type: three_ds routes through PreAuthenticate first,
    // same as 3DSAutoCapture/3DSManualCapture above, so this hits the same
    // account-level 3DS block before SetupMandate/RepeatPayment would ever
    // matter.
    SaveCardUse3DSAutoCaptureOffSession: {
      Request: {
        payment_method: "card",
        payment_method_data: { card: successfulThreeDSTestCardDetails },
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_message: "That Account ID is not configured for 3D Secure",
        },
      },
    },
    ZeroAuthMandate: {
      Configs: { TRIGGER_SKIP: true },
      Request: {},
      Response: { status: 200, body: { status: "processing" } },
    },
    ZeroAuthPaymentIntent: {
      Configs: { TRIGGER_SKIP: true },
      Request: { amount: 0, currency: "USD" },
      Response: { status: 200, body: { status: "requires_payment_method" } },
    },
  },
};
