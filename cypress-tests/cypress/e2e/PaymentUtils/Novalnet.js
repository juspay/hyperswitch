const successfulNo3DSCardDetails = {
  card_number: "4200000000000000",
  card_exp_month: "12",
  card_exp_year: "25",
  card_holder_name: "Max Mustermann",
  card_cvc: "123",
};

const successfulThreeDSTestCardDetails = {
  card_number: "4000000000001091",
  card_exp_month: "12",
  card_exp_year: "25",
  card_holder_name: "Max Mustermann",
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
        billing: {
          address: {
            line1: "1467",
            line2: "CA",
            line3: "CA",
            city: "Musterhausen",
            state: "California",
            zip: "12345",
            country: "DE",
            first_name: "Max",
            last_name: "Mustermann",
          },
          email: "test@novalnet.de",
          phone: {
            number: "9123456789",
            country_code: "+91",
          },
        },
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
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
    "3DSAutoCapture": {
      Request: {
        payment_method: "card",
        billing: {
          address: {
            line1: "1467",
            line2: "CA",
            line3: "CA",
            city: "Musterhausen",
            state: "California",
            zip: "12345",
            country: "DE",
            first_name: "Max",
            last_name: "Mustermann",
          },
          email: "test@novalnet.de",
          phone: {
            number: "9123456789",
            country_code: "+91",
          },
        },
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    //TODO: Add No3DSManualCapture, No3DSAutoCapture
    // No3DSManualCapture: {
    //   Request: {
    //     payment_method: "card",
    //     payment_method_data: {
    //       card: successfulNo3DSCardDetails,
    //     },
    //     customer_acceptance: null,
    //     setup_future_usage: "on_session",
    //   },
    //   Response: {
    //     status: 200,
    //     body: {
    //       status: "requires_capture",
    //     },
    //   },
    // },
    // No3DSAutoCapture: {
    //   Request: {
    //     payment_method: "card",
    //     payment_method_data: {
    //       card: successfulNo3DSCardDetails,
    //     },
    //     customer_acceptance: null,
    //     setup_future_usage: "on_session",
    //   },
    //   Response: {
    //     status: 200,
    //     body: {
    //       status: "succeeded",
    //     },
    //   },
    // },
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
          card: successfulThreeDSTestCardDetails,
        },
        amount: 6500,
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    Refund_7000: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        amount: 7000,
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    PartialRefundForFullCapture3DS1: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        amount: 5000,
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    PartialRefundForFullCapture3DS2: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        amount: 1500,
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    RefundforPartialCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        amount: 100,
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
          card: successfulThreeDSTestCardDetails,
        },
        amount: 1200,
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    PartialRefundForPartialCapture3DS: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        amount: 50,
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    PartialRefundNo3DS: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        amount: 3000,
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
  },
};
