const successfulNo3DSCardDetails = {
  card_number: "4111111111111111",
  card_exp_month: "08",
  card_exp_year: "35",
  card_holder_name: "joseph Doe",
  card_cvc: "999",
};

const successfulThreeDSTestCardDetails = {
  card_number: "4349940199004549",
  card_exp_month: "12",
  card_exp_year: "35",
  card_holder_name: "joseph Doe",
  card_cvc: "396",
};

const customerAcceptance = {
  acceptance_type: "offline",
  accepted_at: "1963-05-03T04:07:52.723Z",
  online: {
    ip_address: "125.0.0.1",
    user_agent: "amet irure esse",
  },
};

const multiUseMandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    multi_use: {
      amount: 8000,
      currency: "EUR",
    },
  },
};

const singleUseMandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    multi_use: {
      amount: 8000,
      currency: "EUR",
    },
  },
};

const billingAddress = {
  address: {
    line1: "1467",
    line2: "Harrison Street",
    line3: "Harrison Street",
    city: "San Fransico",
    state: "California",
    zip: "94122",
    country: "IT",
    first_name: "joseph",
    last_name: "Doe",
  },
  email: "mauro.morandi@nexi.it",
  phone: {
    number: "9123456789",
    country_code: "+91",
  },
};

const no3DSNotSupportedResponseBody = {
  error: {
    type: "invalid_request",
    message: "No threeds is not supported",
    code: "IR_00",
  },
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "EUR",
        amount: 6000,
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    PaymentIntentOffSession: {
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
    "3DSManualCapture": {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        billing: billingAddress,
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    "3DSAutoCapture": {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        billing: billingAddress,
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    No3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "EUR",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: billingAddress,
      },
      Response: {
        status: 400,
        body: no3DSNotSupportedResponseBody,
      },
    },
    No3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "EUR",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: billingAddress,
      },
      Response: {
        status: 400,
        body: no3DSNotSupportedResponseBody,
      },
    },
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
          status: "processing",
          amount: 6000,
          amount_capturable: 6000,
          amount_received: null,
        },
      },
    },
    PartialCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        amount_to_capture: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
          amount: 6000,
          amount_capturable: 6000,
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
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
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
          status: "pending",
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
    MandateMultiUse3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "EUR",
        mandate_data: multiUseMandateData,
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    MandateMultiUse3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "EUR",
        mandate_data: multiUseMandateData,
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    MandateMultiUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "EUR",
        mandate_data: multiUseMandateData,
        billing: billingAddress,
      },
      Response: {
        status: 400,
        body: no3DSNotSupportedResponseBody,
      },
    },
    MandateMultiUseNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "EUR",
        mandate_data: multiUseMandateData,
        billing: billingAddress,
      },
      Response: {
        status: 400,
        body: no3DSNotSupportedResponseBody,
      },
    },
    MandateSingleUse3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "EUR",
        mandate_data: singleUseMandateData,
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    MandateSingleUse3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "EUR",
        mandate_data: singleUseMandateData,
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    MandateSingleUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "EUR",
        mandate_data: singleUseMandateData,
        billing: billingAddress,
      },
      Response: {
        status: 400,
        body: no3DSNotSupportedResponseBody,
      },
    },
    MandateSingleUseNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "EUR",
        mandate_data: singleUseMandateData,
        billing: billingAddress,
      },
      Response: {
        status: 400,
        body: no3DSNotSupportedResponseBody,
      },
    },
    manualPaymentRefund: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
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
          card: successfulThreeDSTestCardDetails,
        },
        currency: "EUR",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
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
          card: successfulThreeDSTestCardDetails,
        },
        currency: "EUR",
        amount: 6000,
        mandate_data: null,
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
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
          card: successfulThreeDSTestCardDetails,
        },
        currency: "EUR",
        amount: 6000,
        mandate_data: null,
        customer_acceptance: customerAcceptance,
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
          card: successfulThreeDSTestCardDetails,
        },
        currency: "EUR",
        amount: 6000,
        mandate_data: null,
        authentication_type: "three_ds",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
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
          card: successfulThreeDSTestCardDetails,
        },
        currency: "EUR",
        amount: 6000,
        mandate_data: null,
        authentication_type: "three_ds",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    SaveCardUseNo3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "EUR",
        billing: billingAddress,
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    SaveCardUseNo3DSAutoCaptureOffSession: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        currency: "EUR",
        billing: billingAddress,
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    SaveCardUseNo3DSManualCaptureOffSession: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        currency: "EUR",
        billing: billingAddress,
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
  },
};
