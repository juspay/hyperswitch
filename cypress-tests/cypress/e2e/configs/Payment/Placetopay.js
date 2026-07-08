const successfulNo3DSCardDetails = {
  card_number: "4110760000000081",
  card_exp_month: "12",
  card_exp_year: "2030",
  card_holder_name: "Test User",
  card_cvc: "123",
};

const browserInfo = {
  ip_address: "127.0.0.1",
  user_agent:
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
};

const simplifiedCustomerAcceptance = {
  acceptance_type: "offline",
};

const notImplementedConfirmRequest = {
  payment_method: "card",
  payment_method_data: {
    card: successfulNo3DSCardDetails,
  },
};

const manualCaptureConfirmRequest = {
  payment_method: "card",
  payment_method_type: "credit",
  payment_method_data: {
    card: successfulNo3DSCardDetails,
  },
  currency: "COP",
  browser_info: browserInfo,
  customer_acceptance: simplifiedCustomerAcceptance,
  setup_future_usage: "on_session",
  billing: {
    address: {
      line1: "Calle 93B No 17-25",
      city: "Bogota",
      state: "Bogota",
      zip: "110111",
      country: "CO",
      first_name: "Test",
      last_name: "User",
    },
    email: "test@placetopay.com",
  },
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "COP",
        description: "placetopay test",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
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
        currency: "COP",
        browser_info: browserInfo,
        customer_acceptance: simplifiedCustomerAcceptance,
        setup_future_usage: "on_session",
        billing: {
          address: {
            line1: "Calle 93B No 17-25",
            city: "Bogota",
            state: "Bogota",
            zip: "110111",
            country: "CO",
            first_name: "Test",
            last_name: "User",
          },
          email: "test@placetopay.com",
        },
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
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
          status: "succeeded",
        },
      },
      ResponseCustom: {
        status: 200,
        body: {
          status: "succeeded",
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
          status: "succeeded",
        },
      },
      ResponseCustom: {
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
      ResponseCustom: {
        status: 200,
        body: {
          status: "succeeded",
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
          status: "succeeded",
        },
      },
      ResponseCustom: {
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
      ResponseCustom: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    No3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: manualCaptureConfirmRequest,
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    "3DSAutoCapture": {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: notImplementedConfirmRequest,
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    "3DSManualCapture": {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: notImplementedConfirmRequest,
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
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
        amount_to_capture: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "partially_captured",
        },
      },
    },
    PaymentIntentWithShippingCost: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        currency: "COP",
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
    },
    PaymentConfirmWithShippingCost: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: notImplementedConfirmRequest,
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          shipping_cost: 50,
          amount: 6000,
        },
      },
    },
  },
};
