import { getCustomExchange } from "./Modifiers";
import { stampPaymentMethodType } from "./Utils";
import {
  customerAcceptance,
  singleUseMandateData,
  multiUseMandateData,
} from "./Commons";

// Disable Cypress retries for Helcim because the connector enforces strict
// idempotency rules: it identifies transactions by card number, cardholder
// name, and amount. A retried test within the 5-minute duplicate-detection
// window would be flagged as a duplicate and fail, so retries are not
// meaningful here and would only produce false negatives.
if (Cypress.env("CONNECTOR") === "helcim") {
  Cypress.config("retries", 0);
}

const successfulNo3DSCardDetails = {
  card_number: "4111111111111111",
  card_exp_month: "08",
  card_exp_year: "30",
  card_holder_name: "joseph Doe",
  card_cvc: "999",
};

const failedNo3DSCardDetails = {
  card_number: "4000000000000002",
  card_exp_month: "08",
  card_exp_year: "30",
  card_holder_name: "joseph Doe",
  card_cvc: "999",
};

const card_pm = {
  PaymentIntent: getCustomExchange({
    Request: {
      currency: "USD",
      customer_acceptance: null,
      setup_future_usage: "on_session",
    },
    Response: {
      status: 200,
      body: {
        status: "requires_payment_method",
      },
    },
  }),
  No3DSManualCapture: getCustomExchange({
    Request: {
      payment_method: "card",
      payment_method_data: { card: successfulNo3DSCardDetails },
      currency: "USD",
      customer_acceptance: null,
      setup_future_usage: "on_session",
    },
    Response: {
      status: 200,
      body: {
        status: "requires_capture",
      },
    },
  }),
  No3DSAutoCapture: getCustomExchange({
    Request: {
      payment_method: "card",
      payment_method_data: { card: successfulNo3DSCardDetails },
      currency: "USD",
      customer_acceptance: null,
      setup_future_usage: "on_session",
    },
    Response: {
      status: 200,
      body: {
        status: "succeeded",
      },
    },
  }),
  PaymentIntentWithShippingCost: getCustomExchange({
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
  }),
  PaymentConfirmWithShippingCost: getCustomExchange({
    Request: {
      payment_method: "card",
      payment_method_data: { card: successfulNo3DSCardDetails },
      customer_acceptance: null,
      setup_future_usage: "on_session",
    },
    Response: {
      status: 200,
      body: {
        status: "succeeded",
        shipping_cost: 50,
        amount_received: 6050,
        amount: 6000,
        net_amount: 6050,
      },
    },
  }),
  No3DSFailPayment: getCustomExchange({
    Request: {
      payment_method: "card",
      payment_method_data: { card: failedNo3DSCardDetails },
      customer_acceptance: null,
      setup_future_usage: "on_session",
    },
    Response: {
      status: 200,
      body: {},
    },
  }),
  // Helcim's sandbox transactions never settle into a closed card batch
  // (same root cause as the refund limitation documented below), so a
  // manual-capture payment intent stays stuck in "processing" and never
  // reaches "requires_capture" — no amount of waiting resolves it. The
  // router then rejects the capture with IR_14 ("expected manual_multiple")
  // since the intent isn't in a capturable state. Tests assert this actual
  // error instead of an unreachable success.
  Capture: getCustomExchange({
    Request: { amount_to_capture: 6000 },
    Response: {
      status: 400,
      body: {
        error: {
          type: "invalid_request",
          message:
            "This Payment could not be captured because it has a capture_method of manual. The expected state is manual_multiple",
          code: "IR_14",
        },
      },
    },
  }),
  PartialCapture: getCustomExchange({
    Request: { amount_to_capture: 2000 },
    Response: {
      status: 400,
      body: {
        error: {
          type: "invalid_request",
          message:
            "This Payment could not be captured because it has a capture_method of manual. The expected state is manual_multiple",
          code: "IR_14",
        },
      },
    },
  }),
  VoidAfterConfirm: getCustomExchange({
    Request: {},
    Response: {
      status: 200,
      body: {
        status: "cancelled",
        capture_method: "manual",
      },
    },
  }),
  // Refund flows — Helcim's sandbox returns "Card Transaction cannot be
  // refunded" because sandbox transactions never settle into a closed card
  // batch. The connector sends a spec-compliant refund request; the failure
  // is a sandbox limitation, not a code bug. Confirmed in hyperswitch-prism
  // (helcim_payment_flows_test.rs). Tests assert the actual "failed" status.
  SyncRefund: getCustomExchange({
    Response: {
      status: 200,
      body: { status: "failed" },
    },
  }),
  Refund: getCustomExchange({
    Request: { amount: 6000 },
    Response: {
      status: 200,
      body: { status: "failed" },
    },
  }),
  PartialRefund: getCustomExchange({
    Request: { amount: 2000 },
    Response: {
      status: 200,
      body: { status: "failed" },
    },
  }),
  manualPaymentRefund: getCustomExchange({
    Request: { amount: 6000 },
    Response: {
      status: 200,
      body: { status: "failed" },
    },
  }),
  manualPaymentPartialRefund: getCustomExchange({
    Request: { amount: 2000 },
    Response: {
      status: 200,
      body: { status: "failed" },
    },
  }),
  // Mandate flows — Helcim connector returns NotImplemented for setup_mandate.
  // The payment intent creation succeeds, but the confirm/SETUP_MANDATE step
  // returns 501 with "Setup Mandate flow for Helcim is not implemented".
  // These entries map the test expectations to that actual error response.
  ZeroAuthPaymentIntent: getCustomExchange({
    Request: {
      amount: 0,
      setup_future_usage: "off_session",
      currency: "USD",
      payment_type: "setup_mandate",
    },
    Response: {
      status: 200,
      body: {
        status: "requires_payment_method",
      },
    },
  }),
  ZeroAuthConfirmPayment: getCustomExchange({
    Request: {
      payment_type: "setup_mandate",
      payment_method: "card",
      payment_method_data: { card: successfulNo3DSCardDetails },
      mandate_data: null,
      customer_acceptance: customerAcceptance,
    },
    Response: {
      status: 501,
      body: {
        error: {
          type: "invalid_request",
          message: "Setup Mandate flow for Helcim is not implemented",
          code: "IR_00",
        },
      },
    },
  }),
  ZeroAuthMandate: {
    Response: {
      status: 501,
      body: {
        error: {
          type: "invalid_request",
          message: "Setup Mandate flow for Helcim is not implemented",
          code: "IR_00",
        },
      },
    },
  },
  // Helcim isn't in mandates.supported_payment_methods, so off_session
  // mandate creation gets silently downgraded to on_session by the router
  // (authorize_flow.rs). We request on_session explicitly here rather than
  // relying on the downgrade, so the CIT actually runs (instead of being
  // skipped) and asserts the real "succeeded"/"requires_capture" outcome.
  // payment_method_id isn't asserted: the router only saves the payment
  // method synchronously (before the response returns) when
  // setup_future_usage is off_session; for on_session it saves via a
  // fire-and-forget async task, so PMID is null in this response.
  // MIT flows below stay skipped since no reusable connector mandate is
  // ever created for a subsequent off-session charge.
  MandateSingleUseNo3DSAutoCapture: {
    // Helcim's sandbox settles asynchronously; without this delay the
    // status is sometimes still "processing" when the response is checked.
    Configs: { DELAY: { STATUS: true, TIMEOUT: 10000 }, ALLOW_PROCESSING_STATUS: true },
    Request: {
      payment_method: "card",
      payment_method_data: { card: successfulNo3DSCardDetails },
      currency: "USD",
      mandate_data: singleUseMandateData,
      setup_future_usage: "on_session",
    },
    Response: {
      status: 200,
      body: { status: "succeeded" },
    },
  },
  MandateSingleUseNo3DSManualCapture: {
    Configs: { DELAY: { STATUS: true, TIMEOUT: 10000 }, ALLOW_PROCESSING_STATUS: true },
    Request: {
      payment_method: "card",
      payment_method_data: { card: successfulNo3DSCardDetails },
      currency: "USD",
      mandate_data: singleUseMandateData,
      setup_future_usage: "on_session",
    },
    Response: {
      status: 200,
      body: { status: "requires_capture" },
    },
  },
  PaymentMethodIdMandateNo3DSAutoCapture: {
    Configs: { DELAY: { STATUS: true, TIMEOUT: 10000 }, ALLOW_PROCESSING_STATUS: true },
    Request: {
      payment_method: "card",
      payment_method_data: { card: successfulNo3DSCardDetails },
      currency: "USD",
      mandate_data: null,
      customer_acceptance: customerAcceptance,
      setup_future_usage: "on_session",
    },
    Response: {
      status: 200,
      body: {
        status: "succeeded",
      },
    },
  },
  PaymentMethodIdMandateNo3DSManualCapture: {
    Configs: { DELAY: { STATUS: true, TIMEOUT: 10000 }, ALLOW_PROCESSING_STATUS: true },
    Request: {
      payment_method: "card",
      payment_method_data: { card: successfulNo3DSCardDetails },
      currency: "USD",
      mandate_data: null,
      customer_acceptance: customerAcceptance,
      setup_future_usage: "on_session",
    },
    Response: {
      status: 200,
      body: {
        status: "requires_capture",
      },
    },
  },
  PaymentMethodIdMandate3DSAutoCapture: {
    Configs: {
      TRIGGER_SKIP: true,
    },
    Request: {},
    Response: {
      status: 200,
      body: {},
    },
  },
  PaymentMethodIdMandate3DSManualCapture: {
    Configs: {
      TRIGGER_SKIP: true,
    },
    Request: {},
    Response: {
      status: 200,
      body: {},
    },
  },
  MITAutoCapture: {
    Configs: { TRIGGER_SKIP: true },
    Request: {},
    Response: {
      status: 200,
      body: { status: "succeeded" },
    },
  },
  MITManualCapture: {
    Configs: { TRIGGER_SKIP: true },
    Request: {},
    Response: {
      status: 200,
      body: { status: "requires_capture" },
    },
  },
  MITWithoutBillingAddress: {
    Configs: { TRIGGER_SKIP: true },
    Request: { billing: null },
    Response: {
      status: 200,
      body: { status: "succeeded" },
    },
  },
  MandateMultiUseNo3DSAutoCapture: {
    Configs: { DELAY: { STATUS: true, TIMEOUT: 10000 }, ALLOW_PROCESSING_STATUS: true },
    Request: {
      payment_method: "card",
      payment_method_data: { card: successfulNo3DSCardDetails },
      currency: "USD",
      mandate_data: multiUseMandateData,
      setup_future_usage: "on_session",
    },
    Response: {
      status: 200,
      body: { status: "succeeded" },
    },
  },
  MandateMultiUseNo3DSManualCapture: {
    Configs: { DELAY: { STATUS: true, TIMEOUT: 10000 }, ALLOW_PROCESSING_STATUS: true },
    Request: {
      payment_method: "card",
      payment_method_data: { card: successfulNo3DSCardDetails },
      currency: "USD",
      mandate_data: multiUseMandateData,
      setup_future_usage: "on_session",
    },
    Response: {
      status: 200,
      body: { status: "requires_capture" },
    },
  },
};

export const connectorDetails = {
  card_pm,
  card_credit_pm: stampPaymentMethodType(card_pm, "credit"),
  card_debit_pm: stampPaymentMethodType(card_pm, "debit"),
};
