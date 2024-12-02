import { getCustomExchange } from "./Commons";

const successfulNo3DSCardDetails = {
  card_number: "4111111111111111",
  card_exp_month: "08",
  card_exp_year: "25",
  card_holder_name: "joseph Doe",
  card_cvc: "999",
};

const successfulThreeDSTestCardDetails = {
  card_number: "4349940199004549",
  card_exp_month: "12",
  card_exp_year: "30",
  card_holder_name: "joseph Doe",
  card_cvc: "396",
};

const multiUseMandateData = {
  customer_acceptance: {
    acceptance_type: "offline",
    accepted_at: "1963-05-03T04:07:52.723Z",
    online: {
      ip_address: "125.0.0.1",
      user_agent: "amet irure esse",
    },
  },
  mandate_type: {
    multi_use: {
      amount: 8000,
      currency: "EUR",
    },
  },
};

const singleUseMandateData = {
  customer_acceptance: {
    acceptance_type: "offline",
    accepted_at: "1963-05-03T04:07:52.723Z",
    online: {
      ip_address: "125.0.0.1",
      user_agent: "amet irure esse",
    },
  },
  mandate_type: {
    multi_use: {
      amount: 8000,
      currency: "EUR",
    },
  },
};

const customerAcceptance = {
  acceptance_type: "offline",
  accepted_at: "1963-05-03T04:07:52.723Z",
  online: {
    ip_address: "125.0.0.1",
    user_agent: "amet irure esse",
  },
};

const payment_method_data_no3ds = {
  card: {
    last4: "1111",
    card_type: null,
    card_network: null,
    card_issuer: null,
    card_issuing_country: null,
    card_isin: "411111",
    card_extended_bin: null,
    card_exp_month: "08",
    card_exp_year: "25",
    card_holder_name: null,
    payment_checks: null,
    authentication_data: null,
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
  }
}

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "EUR",
        amount: 6500,
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
        amount: 6500,
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
        trigger_skip: true,
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    "3DSAutoCapture": {
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
        trigger_skip: true,
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
        status: 501,
        body: {
          error: {
            type: "invalid_request",
            message: "No threeds is not supported",
            code: "IR_00",
          },
        },
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
        status: 501,
        body: {
          error: {
            type: "invalid_request",
            message: "No threeds is not supported",
            code: "IR_00",
          },
        },
      }
    },
    Capture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        trigger_skip: true,
        body: {
          status: "processing",
          amount: 6500,
          amount_capturable: 6500,
          amount_received: null,
        },
      },
    },
    PartialCapture: {
      Request: {},
      Response: {
        status: 200,
        trigger_skip: true,
        body: {
          status: "processing",
          amount: 6500,
          amount_capturable: 6500,
          amount_received: 100,
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
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        customer_acceptance: null,
      },
      Response: {
        trigger_skip: true,
        status: 200,
        body: {
          status: "pending",
        },
      },
    },
    PartialRefund: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        customer_acceptance: null,
      },
      Response: {
        trigger_skip: true,
        status: 200,
        body: {
          status: "pending",
        },
      },
    },
    SyncRefund: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    MandateMultiUse3DSAutoCapture: {
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
        trigger_skip: true,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    MandateMultiUse3DSManualCapture: {
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
        trigger_skip: true,

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
        status: 501,
        body: {
          error: {
            type: "invalid_request",
            message: "No threeds is not supported",
            code: "IR_00",
          },
        },
      }
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
        status: 501,
        body: {
          error: {
            type: "invalid_request",
            message: "No threeds is not supported",
            code: "IR_00",
          },
        },
      }
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
        status: 501,
        body: {
          error: {
            type: "invalid_request",
            message: "No threeds is not supported",
            code: "IR_00",
          },
        },
      }
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
        status: 501,
        body: {
          error: {
            type: "invalid_request",
            message: "No threeds is not supported",
            code: "IR_00",
          },
        },
      }
    },
    manualPaymentRefund: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "EUR",
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        trigger_skip: true,
        body: {
          status: "pending",
        },
      },
    },
    ZeroAuthMandate: {
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
        trigger_skip: true,
        body: {
          status: "processing",
        },
      },
    },
    PaymentMethodIdMandateNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "EUR",
        amount: 6500,
        mandate_data: null,
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        trigger_skip: true,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    PaymentMethodIdMandateNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "EUR",
        amount: 6500,
        mandate_data: null,
        customer_acceptance: customerAcceptance
      },
      Response: {
        status: 200,
        trigger_skip: true,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    PaymentMethodIdMandate3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "EUR",
        amount: 6500,
        mandate_data: null,
        authentication_type: "three_ds",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        trigger_skip: true,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    PaymentMethodIdMandate3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "EUR",
        amount: 6500,
        mandate_data: null,
        authentication_type: "three_ds",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        trigger_skip: true,
        body: {
          status: "requires_customer_action",
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
        currency: "EUR",
        billing: billingAddress,
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        trigger_skip: true,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    SaveCardUseNo3DSAutoCaptureOffSession: {
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
        trigger_skip: true,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    SaveCardUseNo3DSManualCaptureOffSession: {
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
        trigger_skip: true,
        body: {
          status: "requires_customer_action",
        },
      },
    }
  },
};
