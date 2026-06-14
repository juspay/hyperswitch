import {
  customerAcceptance,
  standardBillingAddress,
} from "./Commons";
import { getCustomExchange } from "./Modifiers";

const creditCard1 = { card_number: "4111111111111111", card_exp_month: "08", card_exp_year: "30", card_holder_name: "Alpha One", card_cvc: "999" };
const creditCard2 = { card_number: "4000056655665556", card_exp_month: "08", card_exp_year: "30", card_holder_name: "Beta Two", card_cvc: "999" };
const creditCard3 = { card_number: "378282246310005", card_exp_month: "08", card_exp_year: "30", card_holder_name: "Gamma Three", card_cvc: "999" };
const creditCard4 = { card_number: "371449635398431", card_exp_month: "08", card_exp_year: "30", card_holder_name: "Delta Four", card_cvc: "999" };
const creditCard5 = { card_number: "5555555555554444", card_exp_month: "08", card_exp_year: "30", card_holder_name: "Epsilon Five", card_cvc: "999" };
const creditCard6 = { card_number: "4532015112830366", card_exp_month: "08", card_exp_year: "30", card_holder_name: "Zeta Six", card_cvc: "999" };
const creditCard7 = { card_number: "5200828282828210", card_exp_month: "08", card_exp_year: "30", card_holder_name: "Eta Seven", card_cvc: "999" };
const creditCard8 = { card_number: "3566002020360505", card_exp_month: "08", card_exp_year: "30", card_holder_name: "Theta Eight", card_cvc: "999" };

const debitCard1 = { card_number: "4000000000000002", card_exp_month: "08", card_exp_year: "30", card_holder_name: "Iota Nine", card_cvc: "999" };
const debitCard2 = { card_number: "4242424242424242", card_exp_month: "08", card_exp_year: "30", card_holder_name: "Kappa Ten", card_cvc: "999" };
const debitCard3 = { card_number: "5105105105105100", card_exp_month: "08", card_exp_year: "30", card_holder_name: "Lambda Eleven", card_cvc: "999" };
const debitCard4 = { card_number: "6011111111111117", card_exp_month: "08", card_exp_year: "30", card_holder_name: "Mu Twelve", card_cvc: "999" };
const debitCard5 = { card_number: "4012888888881881", card_exp_month: "08", card_exp_year: "30", card_holder_name: "Nu Thirteen", card_cvc: "999" };
const debitCard6 = { card_number: "4000000000000127", card_exp_month: "08", card_exp_year: "30", card_holder_name: "Xi Fourteen", card_cvc: "999" };
const debitCard7 = { card_number: "4000000000000119", card_exp_month: "08", card_exp_year: "30", card_holder_name: "Omicron Fifteen", card_cvc: "999" };
const debitCard8 = { card_number: "5100000000000008", card_exp_month: "08", card_exp_year: "30", card_holder_name: "Pi Sixteen", card_cvc: "999" };

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
      payment_method_data: { card: null },
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
      payment_method_data: { card: null },
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
      payment_method_data: { card: null },
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
      payment_method_data: { card: null },
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
      payment_method_data: { card: null, billing: standardBillingAddress },
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
      payment_method_data: { card: null, billing: standardBillingAddress },
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
      payment_method_data: { card: null, billing: standardBillingAddress },
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
      payment_method_data: { card: null, billing: standardBillingAddress },
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
      payment_method_data: { card: null, billing: standardBillingAddress },
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
      payment_method_data: { card: null, billing: standardBillingAddress },
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
      payment_method_data: { card: null },
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
      payment_method_data: { card: null, billing: standardBillingAddress },
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
      payment_method_data: { card: null, billing: standardBillingAddress },
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
      payment_method_data: { card: null, billing: standardBillingAddress },
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
      payment_method_data: { card: null, billing: standardBillingAddress },
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

function assignCards(scenarios, cards) {
  const result = {};
  const [c1, c2, c3, c4, c5, c6, c7, c8] = cards;

  for (const [key, val] of Object.entries(scenarios)) {
    result[key] = JSON.parse(JSON.stringify(val));

    if (result[key].Request?.payment_method_data?.card === null) {
      const cardMap = {
        No3DSManualCapture: c1,
        No3DSAutoCapture: c2,
        PaymentConfirmWithShippingCost: c3,
        No3DSFailPayment: c4,
        MandateMultiUseNo3DSAutoCapture: c5,
        MandateMultiUseNo3DSManualCapture: c6,
        MandateSingleUseNo3DSAutoCapture: c7,
        MandateSingleUseNo3DSManualCapture: c8,
        PaymentMethodIdMandateNo3DSAutoCapture: c5,
        PaymentMethodIdMandateNo3DSManualCapture: c6,
        ZeroAuthMandate: c4,
        SaveCardUseNo3DSAutoCapture: c7,
        SaveCardUseNo3DSAutoCaptureOffSession: c8,
        SaveCardUseNo3DSManualCapture: c1,
        SaveCardUseNo3DSManualCaptureOffSession: c2,
      };
      if (cardMap[key]) {
        result[key].Request.payment_method_data.card = cardMap[key];
      }
    }
  }
  return result;
}

const creditCards = [creditCard1, creditCard2, creditCard3, creditCard4, creditCard5, creditCard6, creditCard7, creditCard8];
const debitCards = [debitCard1, debitCard2, debitCard3, debitCard4, debitCard5, debitCard6, debitCard7, debitCard8];

const creditPaymentRaw = assignCards(paymentScenarios, creditCards);
const debitPaymentRaw = assignCards(paymentScenarios, debitCards);
const creditMandateRaw = assignCards(mandateScenarios, creditCards);
const debitMandateRaw = assignCards(mandateScenarios, debitCards);

function stampType(raw, type) {
  const out = {};
  for (const [key, val] of Object.entries(raw)) {
    out[key] = JSON.parse(JSON.stringify(val));
    if (out[key].Request) {
      out[key].Request.payment_method_type = type;
    }
  }
  return out;
}

function mergeRefunds(pmScenarios, refundScens) {
  return { ...pmScenarios, ...JSON.parse(JSON.stringify(refundScens)) };
}

export const connectorDetails = {
  card_pm: mergeRefunds(creditPaymentRaw, refundScenarios),
  card_credit_pm: mergeRefunds(stampType(creditPaymentRaw, "credit"), refundScenarios),
  card_debit_pm: mergeRefunds(stampType(debitPaymentRaw, "debit"), refundScenarios),
};
