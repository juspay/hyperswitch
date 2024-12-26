const successfulNo3DSCardDetails = {
    card_number: "6011016011011111",
    card_exp_month: "12",
    card_exp_year: "2025",
    card_holder_name: "John Doe",  // need to change
    card_cvc: "123",  // need to change
};

// need to change
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