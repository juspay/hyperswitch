import { getCustomExchange } from "./Modifiers";

// Placeholder test cards; swap for real Etisalat sandbox test cards once
// available. Etisalat only supports credit/debit for Visa, Mastercard,
// AmericanExpress, JCB and DinersClub (see crates/connector_configs/toml).
const successfulNo3DSCardDetails = {
  card_number: "4242424242424242",
  card_exp_month: "01",
  card_exp_year: "30",
  card_holder_name: "joseph Doe",
  card_cvc: "123",
};

const successfulThreeDSTestCardDetails = {
  card_number: "4000000000001091",
  card_exp_month: "01",
  card_exp_year: "30",
  card_holder_name: "joseph Doe",
  card_cvc: "123",
};

// pm_filters.etisalat (config/development.toml) restricts eligibility to
// AE, BH, KW, OM, QA, SA, EG — without a matching billing country the
// connector is excluded from routing (IR_39 "No eligible connector").
const billing = {
  address: {
    line1: "1467",
    city: "Dubai",
    state: "Dubai",
    zip: "00000",
    country: "AE",
    first_name: "John",
    last_name: "Doe",
  },
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "USD",
        amount: 6000,
        customer_acceptance: null,
        billing,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    No3DSManualCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        billing,
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
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        billing,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),
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
        billing,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
    "3DSAutoCapture": getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        authentication_type: "three_ds",
        billing,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
    PaymentConfirmWithShippingCost: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        customer_acceptance: null,
        billing,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),
    Capture: getCustomExchange({
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
    }),
    PartialCapture: getCustomExchange({
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
    }),
    Void: {
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "cancelled",
        },
      },
    },
    No3DSFailPayment: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        billing,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
        },
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
        billing,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),
    SaveCardUseNo3DSAutoCaptureOffSession: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        setup_future_usage: "off_session",
        billing,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),
    SaveCardUse3DSAutoCaptureOffSession: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        setup_future_usage: "off_session",
        billing,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
    SaveCardUseNo3DSManualCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        setup_future_usage: "on_session",
        billing,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    }),
    SaveCardUseNo3DSManualCaptureOffSession: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        setup_future_usage: "off_session",
        billing,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    }),
    SaveCardConfirmManualCaptureOffSession: getCustomExchange({
      Request: {
        setup_future_usage: "off_session",
        billing,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    }),
    manualPaymentRefund: getCustomExchange({
      Request: {
        amount: 3000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),
    manualPaymentPartialRefund: getCustomExchange({
      Request: {
        amount: 1500,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),
    SyncRefund: getCustomExchange({
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
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
        billing,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
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
        billing,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
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
        billing,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
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
        billing,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
  },
};
