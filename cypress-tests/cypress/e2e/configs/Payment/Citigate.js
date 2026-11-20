import { getCustomExchange } from "./Modifiers";

// UCS-only connector (#13704): card, one-time Authorize + PSync only.

// Sandbox rejects a repeated card + amount as a duplicate, so no retries.
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

// Requires billing.phone; a fresh identity per read avoids duplicate flags.
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

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
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
    // Capture is out of scope (#13704).
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
    // Not verified against the Citigate sandbox.
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
    // 3DS uses a separate MID (connector_2).
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
    // Cancelling an unconfirmed intent is router-side; no connector Void.
    Void: {
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "cancelled",
        },
      },
    },
    // Refunds are out of scope (#13704).
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
    // Mandates, MIT and saved cards are out of scope (#13704).
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
