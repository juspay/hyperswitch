import { customerAcceptance, standardBillingAddress } from "./Commons";
import { getCustomExchange } from "./Modifiers";

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

const singleUseMandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    single_use: {
      amount: 6000,
      currency: "USD",
    },
  },
};

const multiUseMandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    multi_use: {
      amount: 6000,
      currency: "USD",
    },
  },
};

const paymentScenarios = {
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
  SaveCardConfirmAutoCaptureOffSessionWithoutBilling: {
    Request: {
      setup_future_usage: "off_session",
      billing: null,
    },
    Response: {
      status: 200,
      body: { status: "succeeded" },
    },
  },
  SaveCardConfirmAutoCaptureOffSession: {
    Request: { setup_future_usage: "off_session" },
    Response: {
      status: 200,
      body: { status: "succeeded" },
    },
  },
  SaveCardConfirmManualCaptureOffSession: {
    Request: { setup_future_usage: "off_session" },
    Response: {
      status: 200,
      body: { status: "requires_capture" },
    },
  },
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
      body: { status: "succeeded" },
    },
  }),
};

const refundScenarios = {
  manualPaymentRefund: getCustomExchange({
    Configs: {
      DELAY: { STATUS: true, TIMEOUT: 10000 },
    },
    Request: { amount: 6000 },
    Response: {
      status: 200,
      body: { status: "succeeded" },
    },
  }),
  manualPaymentPartialRefund: getCustomExchange({
    Configs: {
      DELAY: { STATUS: true, TIMEOUT: 10000 },
    },
    Request: { amount: 2000 },
    Response: {
      status: 200,
      body: { status: "succeeded" },
    },
  }),
  Refund: getCustomExchange({
    Configs: {
      DELAY: { STATUS: true, TIMEOUT: 10000 },
    },
    Request: { amount: 6000 },
    Response: {
      status: 200,
      body: { status: "succeeded" },
    },
  }),
  PartialRefund: getCustomExchange({
    Request: { amount: 2000 },
    Response: {
      status: 200,
      body: { status: "succeeded" },
    },
  }),
};

const mandateScenarios = {
  MandateMultiUseNo3DSAutoCapture: {
    Request: {
      payment_method: "card",
      payment_method_data: {
        card: successfulNo3DSCardDetails,
        billing: standardBillingAddress,
      },
      currency: "USD",
      mandate_data: multiUseMandateData,
    },
    Response: {
      status: 200,
      body: { status: "succeeded" },
    },
  },
  MandateMultiUseNo3DSManualCapture: {
    Request: {
      payment_method: "card",
      payment_method_data: {
        card: successfulNo3DSCardDetails,
        billing: standardBillingAddress,
      },
      currency: "USD",
      mandate_data: multiUseMandateData,
    },
    Response: {
      status: 200,
      body: { status: "requires_capture" },
    },
  },
  MandateSingleUseNo3DSAutoCapture: {
    Request: {
      payment_method: "card",
      payment_method_data: {
        card: successfulNo3DSCardDetails,
        billing: standardBillingAddress,
      },
      currency: "USD",
      mandate_data: singleUseMandateData,
    },
    Response: {
      status: 200,
      body: { status: "succeeded" },
    },
  },
  MandateSingleUseNo3DSManualCapture: {
    Request: {
      payment_method: "card",
      payment_method_data: {
        card: successfulNo3DSCardDetails,
        billing: standardBillingAddress,
      },
      currency: "USD",
      mandate_data: singleUseMandateData,
    },
    Response: {
      status: 200,
      body: { status: "requires_capture" },
    },
  },
  PaymentMethodIdMandateNo3DSAutoCapture: {
    Request: {
      payment_method: "card",
      payment_method_data: {
        card: successfulNo3DSCardDetails,
        billing: standardBillingAddress,
      },
      currency: "USD",
      mandate_data: null,
      customer_acceptance: customerAcceptance,
    },
    Response: {
      status: 200,
      body: { status: "succeeded" },
    },
  },
  PaymentMethodIdMandateNo3DSManualCapture: {
    Request: {
      payment_method: "card",
      payment_method_data: {
        card: successfulNo3DSCardDetails,
        billing: standardBillingAddress,
      },
      currency: "USD",
      mandate_data: null,
      customer_acceptance: customerAcceptance,
    },
    Response: {
      status: 200,
      body: { status: "requires_capture" },
    },
  },
  ZeroAuthMandate: {
    Configs: { TRIGGER_SKIP: true },
    Request: {
      payment_method: "card",
      payment_method_data: { card: failedNo3DSCardDetails },
      currency: "USD",
      mandate_data: singleUseMandateData,
    },
    Response: {
      status: 200,
      body: { status: "succeeded" },
    },
  },
  SaveCardUseNo3DSAutoCapture: {
    Request: {
      payment_method: "card",
      payment_method_data: {
        card: successfulNo3DSCardDetails,
        billing: standardBillingAddress,
      },
      currency: "USD",
      setup_future_usage: "on_session",
      customer_acceptance: customerAcceptance,
    },
    Response: {
      status: 200,
      body: { status: "succeeded" },
    },
  },
  SaveCardUseNo3DSAutoCaptureOffSession: {
    Request: {
      payment_method: "card",
      payment_method_data: {
        card: successfulNo3DSCardDetails,
        billing: standardBillingAddress,
      },
      setup_future_usage: "off_session",
      customer_acceptance: customerAcceptance,
    },
    Response: {
      status: 200,
      body: { status: "succeeded" },
    },
  },
  SaveCardUseNo3DSManualCapture: {
    Request: {
      payment_method: "card",
      payment_method_data: {
        card: successfulNo3DSCardDetails,
        billing: standardBillingAddress,
      },
      currency: "USD",
      setup_future_usage: "on_session",
      customer_acceptance: customerAcceptance,
    },
    Response: {
      status: 200,
      body: { status: "requires_capture" },
    },
  },
  SaveCardUseNo3DSManualCaptureOffSession: {
    Request: {
      payment_method: "card",
      payment_method_data: {
        card: successfulNo3DSCardDetails,
        billing: standardBillingAddress,
      },
      setup_future_usage: "off_session",
      customer_acceptance: customerAcceptance,
    },
    Response: {
      status: 200,
      body: { status: "requires_capture" },
    },
  },
  MITManualCapture: {
    Request: {},
    Response: {
      status: 200,
      body: { status: "requires_capture" },
    },
  },
};

function stampPaymentMethodType(scenarios, paymentMethodType) {
  const cloned = JSON.parse(JSON.stringify(scenarios));
  for (const scenario of Object.values(cloned)) {
    if (scenario.Request && typeof scenario.Request === "object") {
      scenario.Request.payment_method_type = paymentMethodType;
    }
  }
  return cloned;
}

export const connectorDetails = {
  card_pm: { ...paymentScenarios, ...mandateScenarios, ...refundScenarios },
  card_credit_pm: {
    ...stampPaymentMethodType(paymentScenarios, "credit"),
    ...stampPaymentMethodType(mandateScenarios, "credit"),
    ...refundScenarios,
  },
  card_debit_pm: {
    ...stampPaymentMethodType(paymentScenarios, "debit"),
    ...stampPaymentMethodType(mandateScenarios, "debit"),
    ...refundScenarios,
  },
};

// Rotate cards to avoid Helcim's duplicate-decline window.
const helcimTestCards = [
  "4111111111111111",
  "4000000000000002",
  "4242424242424242",
  "4012888888881881",
  "4000056655665556",
  "4532015112830366",
  "4000000000000127",
  "4000000000000119",
  "4111111111111129",
  "4111111111111137",
  "4111111111111145",
  "4111111111111152",
  "4000000000000259",
  "4000000000003238",
  "5555555555554444",
  "5105105105105100",
  "5200828282828210",
  "5100000000000008",
  "4111111111111160",
  "4000000000000340",
];

export function injectHelcimTestCard(body, globalState) {
  if (globalState.get("connectorId") !== "helcim") return;
  if (!body.payment_method_data?.card) return;

  const testOffset = globalState.get("helcimCardIndex") ?? 0;
  const timeOffset = Math.floor(Date.now() / 1000) % helcimTestCards.length;
  const idx = (timeOffset + testOffset) % helcimTestCards.length;
  globalState.set("helcimCardIndex", testOffset + 1);

  const ts = Date.now();
  const rnd = Math.floor(Math.random() * 100000);
  const uniqueSuffix = `${ts.toString(36)}_${rnd}`;
  body.payment_method_data.card.card_number = helcimTestCards[idx];
  body.payment_method_data.card.card_holder_name = `HelcimTest ${uniqueSuffix}`;
}
