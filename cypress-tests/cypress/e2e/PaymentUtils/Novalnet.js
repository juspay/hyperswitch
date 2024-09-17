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

const multiUseMandateData = {
  customer_acceptance: {
    acceptance_type: "offline",
    accepted_at: "1963-05-03T04:07:52.723Z",
    online: {
      ip_address: "125.0.0.1",
      user_agent: "amet irure esse",
    },
  },
  mandate_type: {
    multi_use: {
      amount: 100,
      currency: "EUR",
    },
  },
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
    //TODO: Add No3DSManualCapture, No3DSAutoCapture
    // MandateMultiUseNo3DSManualCapture: {
    //   Request: {
    //     payment_method: "card",
    //     payment_method_data: {
    //       card: successfulNo3DSCardDetails,
    //     },
    //     currency: "USD",
    //     mandate_data: multiUseMandateData,
    //   },
    //   Response: {
    //     status: 200,
    //     body: {
    //       status: "requires_capture",
    //     },
    //   },
    // },
    MandateMultiUse3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
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
              last_name: "Mustermann"
          },
          email: "test@novalnet.de",
          phone: {
              number: "9123456789",
              country_code: "+91"
          }
        },
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "EUR",
        mandate_data: multiUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    MandateMultiUse3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
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
              last_name: "Mustermann"
          },
          email: "test@novalnet.de",
          phone: {
              number: "9123456789",
              country_code: "+91"
          }
        },
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "EUR",
        mandate_data: multiUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
  },
};
