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

const notImplementedResponse = {
  status: 400,
  body: {
    error: {
      type: "invalid_request",
      message:
        "No eligible connector was found for the current payment method configuration",
    },
  },
};

const notImplementedConfirmRequest = {
  payment_method: "card",
  payment_method_data: {
    card: successfulNo3DSCardDetails,
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
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "COP",
        browser_info: browserInfo,
        customer_acceptance: simplifiedCustomerAcceptance,
        setup_future_usage: "on_session",
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
    No3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: notImplementedConfirmRequest,
      Response: notImplementedResponse,
    },
    "3DSAutoCapture": {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: notImplementedConfirmRequest,
      Response: notImplementedResponse,
    },
    "3DSManualCapture": {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: notImplementedConfirmRequest,
      Response: notImplementedResponse,
    },
    Capture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: notImplementedConfirmRequest,
      Response: notImplementedResponse,
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
      Response: notImplementedResponse,
    },
  },
};
