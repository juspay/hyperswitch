import { customerAcceptance } from "./Commons.js";

const successfulNo3DSCardDetails = {
  card_number: "5185570141917102",
  card_exp_month: "01",
  card_exp_year: "50",
  card_holder_name: "Joseph Doe",
  card_cvc: "123",
};

const billingDetails = {
  address: {
    line1: "1467",
    line2: "CA",
    line3: "CA",
    city: "Florence",
    state: "Tuscany",
    zip: "12345",
    first_name: "Max",
    last_name: "Mustermann",
  },
  email: "mauro.morandi@nexi.it",
  phone: {
    number: "9123456789",
    country_code: "+91",
  },
};

const paymentMethodDataNo3DSResponse = {
  card: {
    last4: "7102",
    card_type: "DEBIT",
    card_network: "Visa",
    card_issuer: "MASTERCARD INTERNATIONAL",
    card_issuing_country: "UNITEDSTATES",
    card_isin: "518557",
    card_extended_bin: null,
    card_exp_month: "01",
    card_exp_year: "50",
    card_holder_name: "Joseph Doe",
    payment_checks: null,
    authentication_data: null,
  },
  billing: null,
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Request: {
        amount: 6000,
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          setup_future_usage: "on_session",
        },
      },
    },
    PaymentIntentOffSession: {
      // Need to Skip this because on confirming Off Session Payments, even though it succeeds, it does not yield a `connector_mandate_id`
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        setup_future_usage: "off_session",
        currency: "USD",
        billing: billingDetails,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    No3DSManualCapture: {
      Request: {
        amount: 6000,
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          payment_method_data: paymentMethodDataNo3DSResponse,
        },
      },
    },
    No3DSAutoCapture: {
      Request: {
        amount: 6000,
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method_data: paymentMethodDataNo3DSResponse,
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
        },
      },
    },
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
          status: "failed",
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
          status: "failed",
        },
      },
    },
    manualPaymentPartialRefund: {
      Request: {
        amount: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
        },
      },
    },
    PartialRefund: {
      Request: {
        amount: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
        },
      },
    },
    SyncRefund: {
      Response: {
        status: 200,
        body: {
          status: "failed",
        },
      },
    },
    IncrementalAuth: {
      Request: {
        amount: 8000,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          amount: 8000,
          amount_capturable: 8000,
          amount_received: null,
        },
      },
    },
    ZeroAuthMandate: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    ZeroAuthPaymentIntent: {
      // Need to Skip this because on confirming Off Session Payments, even though it succeeds, it does not yield a `connector_mandate_id`
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        amount: 0,
        setup_future_usage: "off_session",
        currency: "USD",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          setup_future_usage: "off_session",
        },
      },
    },
    SaveCardUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    SaveCardUseNo3DSAutoCaptureOffSession: {
      Request: {
        setup_future_usage: "off_session",
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method_data: paymentMethodDataNo3DSResponse,
        },
      },
    },
    SaveCardUseNo3DSManualCaptureOffSession: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        setup_future_usage: "off_session",
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          payment_method_data: paymentMethodDataNo3DSResponse,
        },
      },
    },
    SaveCardUseNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          payment_method_data: paymentMethodDataNo3DSResponse,
        },
      },
    },
    PaymentIntentWithShippingCost: {
      Request: {
        currency: "USD",
        amount: 6500,
        shipping_cost: 50,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          shipping_cost: 50,
          amount: 6500,
          net_amount: 6550,
          amount_capturable: 6550,
        },
      },
    },
    PaymentConfirmWithShippingCost: {
      Request: {
        amount: 6500,
        shipping_cost: 50,
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          shipping_cost: 50,
          amount_received: 6550,
          amount: 6500,
          net_amount: 6550,
          payment_method_data: paymentMethodDataNo3DSResponse,
        },
      },
    },
    MandateMultiUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method_data: paymentMethodDataNo3DSResponse,
        },
      },
    },
    MandateMultiUseNo3DSManualCapture: {
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          payment_method_data: paymentMethodDataNo3DSResponse,
        },
      },
    },
    MITManualCapture: {
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    MandateSingleUseNo3DSAutoCapture: {
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    MandateSingleUseNo3DSManualCapture: {
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    PaymentMethodIdMandateNo3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method_data: paymentMethodDataNo3DSResponse,
        },
      },
    },
    PaymentMethodIdMandateNo3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          payment_method_data: paymentMethodDataNo3DSResponse,
        },
      },
    },
  },
};
