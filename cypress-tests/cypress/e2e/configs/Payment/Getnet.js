const successfulNo3DSCardDetails = {
    card_number: "5413330300001006",
    card_exp_month: "02",
    card_exp_year: "2027",
    card_holder_name: "John Doe",
    card_cvc: "006",
    card_type: "visa"
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
          currency: "GBP",
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
          currency: "GBP",
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
          currency: "GBP",
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
          payment_method: "card",
          payment_method_data: {
            card: successfulNo3DSCardDetails,
          },
          customer_acceptance: null,
        },
        Response: {
          status: 200,
          body: {
            status: "succeeded",
            amount: 6500,
            amount_capturable: 0,
            amount_received: 6500,
          },
        },
      },
      PartialCapture: {
        Request: {},
        Response: {
          status: 200,
          body: {
            status: "partially_captured",
            amount: 6500,
            amount_capturable: 0,
            amount_received: 100,
          },
        },
      },
      Refund: {
        Configs: {
          TRIGGER_SKIP: true,
        },
        Request: {
          payment_method: "card",
          payment_method_data: {
            card: successfulNo3DSCardDetails,
          },
          customer_acceptance: null,
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
          payment_method: "card",
          payment_method_data: {
            card: successfulNo3DSCardDetails,
          },
          customer_acceptance: null,
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
          payment_method: "card",
          payment_method_data: {
            card: successfulNo3DSCardDetails,
          },
          customer_acceptance: null,
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
          payment_method: "card",
          payment_method_data: {
            card: successfulNo3DSCardDetails,
          },
          customer_acceptance: null,
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
  