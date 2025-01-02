const successfulNo3DSCardDetails = {
  card_number: "4000000000001091",
  card_exp_month: "12",
  card_exp_year: "27",
  card_holder_name: "joseph Doe",
  card_cvc: "123",
};

const singleUseMandateData = {
  customer_acceptance: {
    acceptance_type: "offline",
    accepted_at: "1963-05-03T04:07:52.723Z",
    online: {
      ip_address: "125.0.0.1",
      user_agent: "amet irure esse",
    },
  },
  mandate_type: {
    single_use: {
      amount: 8000,
      currency: "IDR",
    },
  },
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
      amount: 8000,
      currency: "IDR",
    },
  },
};
export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "IDR",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: {
          address: {
            line1: "1467",
            line2: "CA",
            line3: "CA",
            city: "Florence",
            state: "Tuscany",
            zip: "12345",
            country: "IT",
            first_name: "Max",
            last_name: "Mustermann",
          },
          email: "mauro.morandi@nexi.it",
          phone: {
            number: "9123456789",
            country_code: "+91",
          },
        },
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
        currency: "IDR",
        shipping_cost: 50,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          shipping_cost: 50,
          amount: 6500,
        },
      },
    },
    PaymentConfirmWithShippingCost: {
      Request: {
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
          shipping_cost: 50,
          amount: 6500,
        },
      },
    },
    No3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: {
            email: "mauro.morandi@nexi.it",
            phone: {
              number: "8056594427",
              country_code: "+91",
            },
          },
        },
        billing: {
          email: "mauro.morandi@nexi.it",
        },
        currency: "IDR",
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
          billing: {
            email: "mauro.morandi@nexi.it",
          },
        },
        billing: {
          email: "mauro.morandi@nexi.it",
          phone: {
            number: "8056594427",
            country_code: "+91",
          },
        },
        currency: "IDR",
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
    manualPaymentPartialRefund: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "IDR",
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
        currency: "IDR",
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    MandateMultiUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: {
            address: {
              line1: "1467",
              line2: "Harrison Street",
              line3: "Harrison Street",
              city: "San Fransico",
              state: "California",
              zip: "94122",
              country: "NL",
              first_name: "joseph",
              last_name: "Doe",
            },
            email: "johndoe@gmail.com",
          },
        },
        currency: "IDR",
        mandate_data: multiUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    MandateMultiUseNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: {
            address: {
              line1: "1467",
              line2: "Harrison Street",
              line3: "Harrison Street",
              city: "San Fransico",
              state: "California",
              zip: "94122",
              country: "NL",
              first_name: "joseph",
              last_name: "Doe",
            },
            email: "johndoe@gmail.com",
          },
        },
        currency: "IDR",
        mandate_data: multiUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    SaveCardUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: {
            email: "mauro.morandi@nexi.it",
            phone: {
              number: "8056594427",
              country_code: "+91",
            },
          },
        },
        currency: "IDR",
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
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    SaveCardUseNo3DSAutoCaptureOffSession: {
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: {
            address: {
              line1: "1467",
              line2: "Harrison Street",
              line3: "Harrison Street",
              city: "San Fransico",
              state: "California",
              zip: "94122",
              country: "NL",
              first_name: "joseph",
              last_name: "Doe",
            },
            phone: {
              number: "9123456789",
              country_code: "+91",
            },
            email: "mauro.morandi@nexi.it",
          },
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
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    SaveCardUseNo3DSManualCaptureOffSession: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: {
            address: {
              line1: "1467",
              line2: "Harrison Street",
              line3: "Harrison Street",
              city: "San Fransico",
              state: "California",
              zip: "94122",
              country: "NL",
              first_name: "joseph",
              last_name: "Doe",
            },
            phone: {
              number: "9123456789",
              country_code: "+91",
            },
            email: "mauro.morandi@nexi.it",
          },
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
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    SaveCardConfirmAutoCaptureOffSession: {
      Request: {
        setup_future_usage: "off_session",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    SaveCardConfirmManualCaptureOffSession: {
      Request: {
        setup_future_usage: "off_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    SaveCardUseNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: {
            address: {
              line1: "1467",
              line2: "Harrison Street",
              line3: "Harrison Street",
              city: "San Fransico",
              state: "California",
              zip: "94122",
              country: "NL",
              first_name: "joseph",
              last_name: "Doe",
            },
            email: "mauro.morandi@nexi.it",
            phone: {
              number: "8056594427",
              country_code: "+91",
            },
          },
        },
        currency: "IDR",
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
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    MandateSingleUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: {
            email: "mauro.morandi@nexi.it",
            phone: {
              number: "8056594427",
              country_code: "+91",
            },
          },
        },
        currency: "IDR",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    MandateSingleUseNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: {
            email: "mauro.morandi@nexi.it",
            phone: {
              number: "8056594427",
              country_code: "+91",
            },
          },
        },
        currency: "IDR",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    Capture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "IDR",
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
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "IDR",
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    VoidAfterConfirm: {
      Request: {},
      Response: {
        status: 501,
        body: {
          error: {
            type: "invalid_request",
            message: "Cancel/Void flow is not implemented",
            code: "IR_00",
          },
        },
      },
    },
    PartialRefund: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "IDR",
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
        currency: "IDR",
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    PaymentMethodIdMandateNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "IDR",
        billing: {
          email: "mauro.morandi@nexi.it",
          phone: {
            number: "8056594427",
            country_code: "+91",
          },
        },
        mandate_data: null,
        customer_acceptance: {
          acceptance_type: "offline",
          accepted_at: "1963-05-03T04:07:52.723Z",
          online: {
            ip_address: "125.0.0.1",
            user_agent: "amet irure esse",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    PaymentMethodIdMandateNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        billing: {
          email: "mauro.morandi@nexi.it",
          phone: {
            number: "8056594427",
            country_code: "+91",
          },
        },
        currency: "IDR",
        mandate_data: null,
        customer_acceptance: {
          acceptance_type: "offline",
          accepted_at: "1963-05-03T04:07:52.723Z",
          online: {
            ip_address: "125.0.0.1",
            user_agent: "amet irure esse",
          },
        },
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
