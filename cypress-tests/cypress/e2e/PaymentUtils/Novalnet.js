const successfulNo3DSCardDetails = {
    card_number: "4000000000001091",
    card_exp_month: "12",
    card_exp_year: "25",
    card_holder_name: "Joseph Doe",
    card_cvc: "123",
  };

const successfulThreeDSTestCardDetails = {
  card_number: "4000000000001091",
  card_exp_month: "12",
  card_exp_year: "25",
  card_holder_name: "Joseph Doe",
  card_cvc: "123",
  };
  
  
  export const connectorDetails = {
    card_pm: {
      PaymentIntent: {
        Request: {
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
      "3DSManualCapture": {
        Request: {
          payment_method: "card",
          payment_method_data: {
            card: successfulThreeDSTestCardDetails,
          },
          currency: "EUR",
          customer_acceptance: null,
          setup_future_usage: "on_session",
        },
        Response: {
          status: 200,
          trigger_skip: true,
          body: {
            status: "requires_capture",
          },
        },
      },
      "3DSAutoCapture": {
        Request: {
          payment_method: "card",
          payment_method_data: {
            card: successfulThreeDSTestCardDetails,
          },
          currency: "EUR",
          customer_acceptance: null,
          setup_future_usage: "on_session",
        },
        Response: {
          status: 200,
          trigger_skip: true,
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
    },
  };
  