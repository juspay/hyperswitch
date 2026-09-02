const successfulNo3DSCardDetails = {
  card_number: "5413330300001006",
  card_exp_month: "02",
  card_exp_year: "2027",
  card_holder_name: "John Doe",
  card_cvc: "006",
  card_network: "Mastercard",
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "GBP",
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
    PaymentIntentWithShippingCost: {
      Request: {
        currency: "GBP",
        shipping_cost: 50,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          amount: 6000,
          shipping_cost: 50,
        },
      },
    },
    PaymentConfirmWithShippingCost: {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "No eligible connector was found for the current payment method configuration",
            code: "IR_39",
          },
        },
      },
    },
    "3DSManualCapture": {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "GBP",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "No eligible connector was found for the current payment method configuration",
            code: "IR_39",
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
        currency: "GBP",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "No eligible connector was found for the current payment method configuration",
            code: "IR_39",
          },
        },
      },
    },
    No3DSManualCapture: {
      Request: {
        currency: "GBP",
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "No eligible connector was found for the current payment method configuration",
            code: "IR_39",
          },
        },
      },
    },
    No3DSAutoCapture: {
      Request: {
        currency: "GBP",
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "No eligible connector was found for the current payment method configuration",
            code: "IR_39",
          },
        },
      },
    },
    Capture: {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        customer_acceptance: null,
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
      Request: {
        amount: 6000,
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
        amount: 6000,
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
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
    manualPaymentPartialRefund: {
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
    SyncRefund: {
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    ZeroAuthMandate: {
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "No eligible connector was found for the current payment method configuration",
            code: "IR_39",
          },
        },
      },
    },
    ZeroAuthPaymentIntent: {
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
    ZeroAuthConfirmPayment: {
      Request: {
        payment_type: "setup_mandate",
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "No eligible connector was found for the current payment method configuration",
            code: "IR_39",
          },
        },
      },
    },
    SaveCardUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        amount: 5000,
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
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
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "No eligible connector was found for the current payment method configuration",
            code: "IR_39",
          },
        },
      },
    },
    SaveCardUseNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        amount: 5000,
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
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
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "No eligible connector was found for the current payment method configuration",
            code: "IR_39",
          },
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
        setup_future_usage: "off_session",
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
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "No eligible connector was found for the current payment method configuration",
            code: "IR_39",
          },
        },
      },
    },
    SaveCardUse3DSAutoCaptureOffSession: {
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "No eligible connector was found for the current payment method configuration",
            code: "IR_39",
          },
        },
      },
    },
    SaveCardUseNo3DSManualCaptureOffSession: {
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "No eligible connector was found for the current payment method configuration",
            code: "IR_39",
          },
        },
      },
    },
    SaveCardConfirmAutoCaptureOffSession: {
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "No eligible connector was found for the current payment method configuration",
            code: "IR_39",
          },
        },
      },
    },
    SaveCardConfirmManualCaptureOffSession: {
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "No eligible connector was found for the current payment method configuration",
            code: "IR_39",
          },
        },
      },
    },
    PaymentMethodIdMandateNo3DSAutoCapture: {
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "No eligible connector was found for the current payment method configuration",
            code: "IR_39",
          },
        },
      },
    },
    PaymentMethodIdMandateNo3DSManualCapture: {
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "No eligible connector was found for the current payment method configuration",
            code: "IR_39",
          },
        },
      },
    },
    PaymentMethodIdMandate3DSAutoCapture: {
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "No eligible connector was found for the current payment method configuration",
            code: "IR_39",
          },
        },
      },
    },
    PaymentMethodIdMandate3DSManualCapture: {
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "No eligible connector was found for the current payment method configuration",
            code: "IR_39",
          },
        },
      },
    },
  },
};
