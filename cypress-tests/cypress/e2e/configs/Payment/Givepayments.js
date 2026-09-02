import {
  customerAcceptance,
  multiUseMandateData,
  singleUseMandateData,
} from "./Commons";

const generateGivepaymentsEmail = () => "venkatakarthik.m@juspay.in";

const successfulNo3DSCardDetails = {
  card_number: "4111111111111111",
  card_exp_month: "12",
  card_exp_year: "2029",
  card_holder_name: "John Doe",
  card_cvc: "123",
  card_network: "Visa",
};

const billingWithEmail = (email) => ({
  address: {
    line1: "1467",
    line2: "Harrison Street",
    city: "San Francisco",
    state: "California",
    zip: "94122",
    country: "US",
    first_name: "John",
    last_name: "Doe",
  },
  email,
});

const captureMethodNotSupportedError = {
  error: {
    type: "invalid_request",
    message:
      "Capture method not supported. Givepayments connector supports Automatic capture method. is not implemented",
    code: "IR_00",
  },
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
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
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
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
        setup_future_usage: "on_session",
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
          amount: 6000,
          shipping_cost: 50,
          net_amount: 6050,
        },
      },
    },
    No3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
          amount: 6000,
          net_amount: 6000,
        },
      },
    },
    "3DSAutoCapture": {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
          amount: 6000,
          net_amount: 6000,
        },
      },
    },
    "3DSManualCapture": {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
      },
      Response: {
        status: 501,
        body: captureMethodNotSupportedError,
      },
    },
    No3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
      },
      Response: {
        status: 501,
        body: captureMethodNotSupportedError,
      },
    },
    Refund: {
      Configs: {
        POLL_BEFORE: true,
      },
      Request: {
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
          amount: 6000,
        },
      },
    },
    PartialRefund: {
      Configs: {
        POLL_BEFORE: true,
      },
      Request: {
        amount: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
          amount: 2000,
        },
      },
    },
    manualPaymentRefund: {
      Configs: {
        POLL_BEFORE: true,
      },
      Request: {
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
          amount: 6000,
        },
      },
    },
    manualPaymentPartialRefund: {
      Configs: {
        POLL_BEFORE: true,
      },
      Request: {
        amount: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
          amount: 2000,
        },
      },
    },
    SyncRefund: {
      Response: {
        status: 200,
        body: {
          status: "pending",
        },
      },
    },
    MandateSingleUseNo3DSAutoCapture: {
      Configs: {
        POLL_AFTER: true,
      },
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
      },
      Response: {
        status: 200,
        body: {
          amount: 6000,
        },
      },
    },
    MandateSingleUseNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
      },
      Response: {
        status: 501,
        body: captureMethodNotSupportedError,
      },
    },
    MandateMultiUseNo3DSAutoCapture: {
      Configs: {
        POLL_AFTER: true,
      },
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: multiUseMandateData,
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
      },
      Response: {
        status: 200,
        body: {
          amount: 6000,
        },
      },
    },
    MandateMultiUseNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: multiUseMandateData,
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
      },
      Response: {
        status: 501,
        body: captureMethodNotSupportedError,
      },
    },
    MITAutoCapture: {
      Configs: {
        POLL_BEFORE: true,
      },
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          amount: 6000,
          net_amount: 6000,
        },
      },
    },
    MITWithoutBillingAddress: {
      Configs: {
        POLL_BEFORE: true,
      },
      Request: { billing: null },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          amount: 6000,
          net_amount: 6000,
        },
      },
    },
    MITManualCapture: {
      Request: {},
      Response: {
        status: 501,
        body: captureMethodNotSupportedError,
      },
    },
    PaymentMethodIdMandateNo3DSAutoCapture: {
      Configs: {
        POLL_AFTER: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
      },
      Response: {
        status: 200,
        body: {
          amount: 6000,
        },
      },
    },
    PaymentMethodIdMandateNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
      },
      Response: {
        status: 501,
        body: captureMethodNotSupportedError,
      },
    },
    PaymentMethodIdMandate3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    PaymentMethodIdMandate3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    SaveCardUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    SaveCardUseNo3DSAutoCaptureOffSession: {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
  },
};
