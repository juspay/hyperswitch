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

const cardMandateBaseRequest = {
  payment_method: "card",
  payment_method_data: { card: successfulNo3DSCardDetails },
  currency: "USD",
};

// Helcim's sandbox can't refund (transactions never settle into a closed
// card batch) — a sandbox limitation, not a code bug.
const helcimRefundFailureBody = {
  status: "failed",
  error_code: "No error code",
  error_message: "Card Transaction cannot be refunded",
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
  Capture: getCustomExchange({
    Request: { amount_to_capture: 6000 },
    Response: {
      status: 200,
      body: {
        status: "succeeded",
        amount: 6000,
        amount_capturable: 0,
        amount_received: 6000,
      },
    },
  }),
  PartialCapture: getCustomExchange({
    Request: { amount_to_capture: 2000 },
    Response: {
      status: 200,
      body: {
        status: "partially_captured",
        amount: 6000,
        amount_capturable: 0,
        amount_received: 2000,
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
  SyncRefund: getCustomExchange({
    Response: {
      status: 200,
      body: helcimRefundFailureBody,
    },
  }),
  Refund: getCustomExchange({
    Request: { amount: 6000 },
    Response: {
      status: 200,
      body: helcimRefundFailureBody,
    },
  }),
  PartialRefund: getCustomExchange({
    Request: { amount: 2000 },
    Response: {
      status: 200,
      body: helcimRefundFailureBody,
    },
  }),
  manualPaymentRefund: getCustomExchange({
    Request: { amount: 6000 },
    Response: {
      status: 200,
      body: helcimRefundFailureBody,
    },
  }),
  manualPaymentPartialRefund: getCustomExchange({
    Request: { amount: 2000 },
    Response: {
      status: 200,
      body: helcimRefundFailureBody,
    },
  }),
  // Helcim's SetupMandate flow returns 501 NotImplemented.
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
  // Helcim doesn't support mandates, so the router silently downgrades
  // off_session to on_session (after validation) and CIT succeeds for
  // real — don't override setup_future_usage here, just leave it unset.
  // MIT/list-mandate skip themselves at runtime (see commands.js) once
  // they see no real mandate was created.
  MandateSingleUseNo3DSAutoCapture: {
    Request: {
      ...cardMandateBaseRequest,
      mandate_data: singleUseMandateData,
    },
    Response: {
      status: 200,
      body: { status: "succeeded" },
    },
  },
  MandateSingleUseNo3DSManualCapture: {
    Request: {
      ...cardMandateBaseRequest,
      mandate_data: singleUseMandateData,
    },
    Response: {
      status: 200,
      body: { status: "requires_capture" },
    },
  },
  PaymentMethodIdMandateNo3DSAutoCapture: {
    Request: {
      ...cardMandateBaseRequest,
      mandate_data: null,
      customer_acceptance: customerAcceptance,
    },
    Response: {
      status: 200,
      body: {
        status: "succeeded",
      },
    },
  },
  PaymentMethodIdMandateNo3DSManualCapture: {
    Request: {
      ...cardMandateBaseRequest,
      mandate_data: null,
      customer_acceptance: customerAcceptance,
    },
    Response: {
      status: 200,
      body: {
        status: "requires_capture",
      },
    },
  },
  // Helcim declares three_ds: FeatureStatus::NotSupported (helcim.rs),
  // so the router rejects the request before it ever reaches the
  // connector. Kept skipped since there's no real 3DS flow to exercise,
  // but the Response documents the actual confirmed error.
  PaymentMethodIdMandate3DSAutoCapture: {
    Configs: {
      TRIGGER_SKIP: true,
    },
    Request: {},
    Response: {
      status: 400,
      body: {
        error: {
          type: "invalid_request",
          message: "Payment method type not supported",
          code: "IR_19",
          reason: "Cards 3DS is not supported by Helcim",
        },
      },
    },
  },
  PaymentMethodIdMandate3DSManualCapture: {
    Configs: {
      TRIGGER_SKIP: true,
    },
    Request: {},
    Response: {
      status: 400,
      body: {
        error: {
          type: "invalid_request",
          message: "Payment method type not supported",
          code: "IR_19",
          reason: "Cards 3DS is not supported by Helcim",
        },
      },
    },
  },
  // Helcim never creates a real mandate to reuse for MIT (same reason as
  // above), so these stay skipped via the standard TRIGGER_SKIP config,
  // same as Fiuu.js's MITAutoCapture.
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
    Request: {
      ...cardMandateBaseRequest,
      mandate_data: multiUseMandateData,
    },
    Response: {
      status: 200,
      body: { status: "succeeded" },
    },
  },
  MandateMultiUseNo3DSManualCapture: {
    Request: {
      ...cardMandateBaseRequest,
      mandate_data: multiUseMandateData,
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
