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
            message: "Three_ds payments is not supported by Jpmorgan",
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
        billing: {
          address: {
            line1: "1467",
            line2: "CA",
            line3: "CA",
            city: "Musterhausen",
            state: "California",
            zip: "12345",
            country: "US",
            first_name: "John",
            last_name: "Doe",
          },
          email: "test@jpmorgan.us",
          phone: {
            number: "9123456789",
            country_code: "+91",
          },
        },
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
    /*Refund: {
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
        },
      },
    },
    manualPaymentRefund: {
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
        },
      },
    },
    manualPaymentPartialRefund: {
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
        },
      },
    },
    PartialRefund: {
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
        },
      },
    },*/
    Refund: {
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
          // status: "succeeded",
          type: "invalid_request",
        message: "Refunds is not implemented",
        code: "IR_00"
        },
      },
    },
    manualPaymentRefund: {
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
        code: "IR_00"
        },
      },
    },
    manualPaymentPartialRefund: {
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
        code: "IR_00"
        },
      },
    },
    PartialRefund: {
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
        code: "IR_00"
        },
      },
    },
    /*SyncRefund: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        trigger_skip: true,
        body: {
          status: "succeeded",
        },
      },
    }*/
    SyncRefund: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        customer_acceptance: null,
      },
      Response: {
        status: 404,
        body: {
          type: "undefined",
          message: "Refund does not exist in our records.",
          code: "HE_02"
        },
      },
    }
  },
};
