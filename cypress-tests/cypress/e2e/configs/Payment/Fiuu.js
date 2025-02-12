const successfulNo3DSCardDetails = {
  card_number: "5105105105105100",
  card_exp_month: "12",
  card_exp_year: "2030",
  card_holder_name: "joseph Doe",
  card_cvc: "123",
};

const successfulThreeDSTestCardDetails = {
  card_number: "5105105105105100",
  card_exp_month: "12",
  card_exp_year: "2031",
  card_holder_name: "joseph Doe",
  card_cvc: "444",
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
      currency: "USD",
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
      currency: "USD",
    },
  },
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
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
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
    "3DSAutoCapture": {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
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
    No3DSManualCapture: {
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
        currency: "USD",
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
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
        },
      },
    },
    PartialRefund: {
      Request: {
        amount: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
        },
      },
    },
    SyncRefund: {
      Response: {
        status: 200,
        body: {
          status: "pending",
        },
      },
    },
    MandateSingleUse3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    MandateSingleUse3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    MandateSingleUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
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
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
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
        currency: "USD",
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
        },
        currency: "USD",
        mandate_data: multiUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    MandateMultiUse3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
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
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        mandate_data: multiUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    MITAutoCapture: {
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_code: "The currency not allow for the RecordType",
          error_message: "The currency not allow for the RecordType",
        },
      },
    },
    MITManualCapture: {
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_code: "The currency not allow for the RecordType",
          error_message: "The currency not allow for the RecordType",
        },
      },
    },
    PaymentIntentOffSession: {
      Request: {
        amount: 6000,
        authentication_type: "no_three_ds",
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "off_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    PaymentMethodIdMandateNo3DSAutoCapture: {
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
        currency: "USD",
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
    SaveCardUseNo3DSAutoCapture: {
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
        status: 200,
        body: {
          status: "succeeded",
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
            email: "johndoe@gmail.com",
          },
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
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    SaveCardUseNo3DSAutoCaptureOffSession: {
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
            email: "johndoe@gmail.com",
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
    PaymentMethodIdMandateNo3DSManualCapture: {
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
        currency: "USD",
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
    PaymentMethodIdMandate3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
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
        currency: "USD",
        mandate_data: null,
        authentication_type: "three_ds",
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
          status: "requires_customer_action",
        },
      },
    },
    PaymentMethodIdMandate3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
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
        mandate_data: null,
        authentication_type: "three_ds",
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
          status: "requires_customer_action",
        },
      },
    },
  },
};
