const successfulNo3DSCardDetails = {
  card_number: "6011016011016011",
  card_exp_month: "10",
  card_exp_year: "2027",
  card_holder_name: "John Doe",
  card_cvc: "123",
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
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
    },
    "3DSManualCapture": {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 501,
        body: {
          error: {
            type: "invalid_request",
            message: "3DS payments is not supported by Jpmorgan",
            code: "IR_00",
          },
        },
      },
    },

    "3DSAutoCapture": {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 501,
        body: {
          error: {
            type: "invalid_request",
            message: "Three_ds payments is not supported by Jpmorgan",
            code: "IR_00",
          },
        },
      },
    },
    No3DSManualCapture: {
      Request: {
        currency: "USD",
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
          status: "requires_capture",
        },
      },
    },
    No3DSAutoCapture: {
      Request: {
        currency: "USD",
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
          amount: 6000,
          amount_capturable: 0,
          amount_received: 6000,
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
          amount: 6000,
          amount_capturable: 0,
          amount_received: 2000,
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
        status: 501,
        body: {
          type: "invalid_request",
          message: "Refunds is not implemented",
          code: "IR_00",
        },
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
        status: 501,
        body: {
          type: "invalid_request",
          message: "Refunds is not implemented",
          code: "IR_00",
        },
      },
    },
    manualPaymentPartialRefund: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        amount: 2000,
      },
      Response: {
        status: 501,
        body: {
          type: "invalid_request",
          message: "Refunds is not implemented",
          code: "IR_00",
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
        status: 501,
        body: {
          type: "invalid_request",
          message: "Refunds is not implemented",
          code: "IR_00",
        },
      },
    },
    SyncRefund: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Response: {
        status: 404,
        body: {
          type: "invalid_request",
          message: "Refund does not exist in our records.",
          code: "HE_02",
        },
      },
    },
  },
};
