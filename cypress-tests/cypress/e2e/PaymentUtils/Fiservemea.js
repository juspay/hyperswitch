const successfulNo3DSCardDetails = {
    card_number: "5204740000001002",
    card_exp_month: "10",
    card_exp_year: "24",
    card_holder_name: "Joseph Doe",
    card_cvc: "002",
  };
  
  export const connectorDetails = {
    card_pm: {
      PaymentIntent: {
        Request: {
          payment_method: "card",
          payment_method_data: {
            card: successfulNo3DSCardDetails,
          },
          currency: "EUR",
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
          payment_method: "card",
          payment_method_data: {
            card: successfulNo3DSCardDetails,
          },
          currency: "EUR",
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
          payment_method: "card",
          payment_method_data: {
            card: successfulNo3DSCardDetails,
          },
          currency: "EUR",
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
          currency: "EUR",
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
            card: successfulNo3DSCardDetails,
          },
          currency: "EUR",
          customer_acceptance: null,
        },
        Response: {
          status: 200,
          body: {
            status: "succeeded",
          },
        },
      },
      PartialRefund: {
        Request: {
          payment_method: "card",
          payment_method_data: {
            card: successfulNo3DSCardDetails,
          },
          currency: "EUR",
          customer_acceptance: null,
        },
        Response: {
          status: 200,
          body: {
            status: "succeeded",
          },
        },
      },
      SyncRefund: {
        Request: {
          payment_method: "card",
          payment_method_data: {
            card: successfulNo3DSCardDetails,
          },
          currency: "EUR",
          customer_acceptance: null,
        },
        Response: {
          status: 200,
          body: {
            status: "succeeded",
          },
        },
      },
      ZeroAuthMandate: {
        Response: {
          status: 501,
          body: {
            error: {
              type: "invalid_request",
              message: "Setup Mandate flow for Fiservemea is not implemented",
              code: "IR_00",
            },
          },
        },
      },
    },
  };
  