import { getCustomExchange } from "./Modifiers";

const ThreeDSChallengeTestCardDetails = {
  card_number: "4548817212493017",
  card_exp_month: "12",
  card_exp_year: "25",
  card_holder_name: "Joseph",
  card_cvc: "123",
};

const threeDSFrictionlessTestCardDetails = {
  card_number: "4548814479727229",
  card_exp_month: "12",
  card_exp_year: "25",
  card_holder_name: "Joseph",
  card_cvc: "123",
};

const Address = {
  address: {
    line1: "1467",
    line2: "Harrison Street",
    line3: "Harrison Street",
    city: "San Fransico",
    state: "Ceuta",
    zip: "94122",
    country: "ES",
    first_name: "joseph",
    last_name: "Doe",
  },
  email: "mauro.morandi@nexi.it",
  phone: {
    number: "9123456789",
    country_code: "+91",
  },
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "EUR",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: Address,
        shipping: Address,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    PaymentConfirmWithShippingCost: getCustomExchange({
      Request: {
        currency: "EUR",

        payment_method: "card",
        payment_method_data: {
          card: ThreeDSChallengeTestCardDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: Address,
        shipping: Address,
      },
    }),
    No3DSManualCapture: getCustomExchange({
      Request: {
        currency: "EUR",
        payment_method: "card",
        payment_method_data: {
          card: ThreeDSChallengeTestCardDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: Address,
        shipping: Address,
      },
    }),
    MandateSingleUseNo3DSManualCapture: getCustomExchange({
      Request: {
        currency: "EUR",

        payment_method: "card",
        payment_method_data: {
          card: ThreeDSChallengeTestCardDetails,
        },
        customer_acceptance: null,
        billing: Address,
        shipping: Address,
      },
    }),
    MandateSingleUseNo3DSAutoCapture: getCustomExchange({
      Request: {
        currency: "EUR",

        payment_method: "card",
        payment_method_data: {
          card: ThreeDSChallengeTestCardDetails,
        },
        customer_acceptance: null,
        billing: Address,
        shipping: Address,
      },
    }),
    MandateMultiUseNo3DSAutoCapture: getCustomExchange({
      Request: {
        currency: "EUR",

        payment_method: "card",
        payment_method_data: {
          card: ThreeDSChallengeTestCardDetails,
        },
        customer_acceptance: null,
        billing: Address,
        shipping: Address,
      },
    }),
    ZeroAuthMandate: {
      Request: {
        currency: "EUR",
        payment_method_type: "credit",
        payment_method_data: {
          card: ThreeDSChallengeTestCardDetails,
        },
        customer_acceptance: null,
        billing: Address,
        shipping: Address,
        payment_type: "setup_mandate",
      },
      Response: {
        status: 501,
        body: {
          error: {
            type: "invalid_request",
            message: "Setup Mandate flow for Redsys is not implemented",
            code: "IR_00",
          },
        },
      },
    },
    "3DSManualCapture": {
      Request: {
        currency: "EUR",
        authentication_type: "three_ds",
        payment_method_type: "credit",
        payment_method: "card",
        payment_method_data: {
          card: ThreeDSChallengeTestCardDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: Address,
        shipping: Address,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    "3DSAutoCapture": {
      Request: {
        currency: "EUR",
        payment_method: "card",
        payment_method_data: {
          card: ThreeDSChallengeTestCardDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: Address,
        shipping: Address,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    No3DSAutoCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        amount: 5000,
        payment_method_data: {
          card: ThreeDSChallengeTestCardDetails,
        },
        currency: "EUR",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: Address,
        shipping: Address,
      },
    }),
    Capture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        amount_to_capture: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          amount: 6000,
        },
      },
    },
    PartialCapture: {
      Request: {
        amount_to_capture: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "partially_captured",
          amount_received: 2000,
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
        amount: 5000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    PartialRefund: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        amount: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    manualPaymentRefund: {
      Request: {
        amount: 5000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
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
          status: "succeeded",
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
    ZeroAuthPaymentIntent: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        amount: 0,
        setup_future_usage: "off_session",
        currency: "EUR",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          setup_future_usage: "off_session",
        },
      },
    },
    ZeroAuthConfirmPayment: {
      Request: {
        payment_type: "setup_mandate",
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: ThreeDSChallengeTestCardDetails,
        },
      },
      Response: {
        status: 501,
        body: {
          error: {
            type: "invalid_request",
            message: "Setup Mandate flow for redsys is not implemented",
            code: "IR_00",
          },
        },
      },
    },
    SaveCardUseNo3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        currency: "EUR",
        billing: Address,
        shipping: Address,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    SaveCardUseNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        amount: 5000,
        payment_method_data: {
          card: ThreeDSChallengeTestCardDetails,
        },
        currency: "EUR",
        setup_future_usage: "on_session",
        customer_acceptance: {
          acceptance_type: "offline",
          accepted_at: "1963-05-03T04:07:52.723Z",
          online: {
            ip_address: "127.0.0.1",
            user_agent: "amet irure esse",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    PaymentIntentOffSession: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        currency: "EUR",
        amount: 6000,
        authentication_type: "no_three_ds",
        customer_acceptance: null,
        setup_future_usage: "off_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          setup_future_usage: "off_session",
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
          card: threeDSFrictionlessTestCardDetails,
        },
        billing: Address,
        shipping: Address,
        currency: "EUR",
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    PaymentMethodIdMandate3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: threeDSFrictionlessTestCardDetails,
        },
        billing: Address,
        shipping: Address,
        currency: "EUR",
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
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
          card: ThreeDSChallengeTestCardDetails,
        },
        billing: Address,
        shipping: Address,
        currency: "EUR",
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
        },
      },
    },
    No3DSFailPayment: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: ThreeDSChallengeTestCardDetails,
        },
        billing: Address,
        shipping: Address,
        currency: "EUR",
        amount: 6000,
      },
    }),
    SaveCardUseNo3DSAutoCaptureOffSession: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: ThreeDSChallengeTestCardDetails,
        },
        billing: Address,
        shipping: Address,
        currency: "EUR",
        amount: 6000,
      },
    }),
    PaymentWithBilling: {
      Request: {
        currency: "USD",
        setup_future_usage: "on_session",
        billing: Address,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
  },
};
