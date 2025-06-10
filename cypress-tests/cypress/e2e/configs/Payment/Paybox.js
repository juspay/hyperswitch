const successfulNo3DSCardDetails = {
  card_number: "1111222233334444",
  card_exp_month: "05",
  card_exp_year: "27",
  card_holder_name: "joseph Doe",
  card_cvc: "222",
};

const successfulThreeDSTestCardDetails = {
  card_number: "4000000000001091",
  card_exp_month: "01",
  card_exp_year: "50",
  card_holder_name: "joseph Doe",
  card_cvc: "123",
};

const customerAcceptance = {
  acceptance_type: "offline",
  accepted_at: "1963-05-03T04:07:52.723Z",
  online: {
    ip_address: "125.0.0.1",
    user_agent: "amet irure esse",
  },
};

const singleUseMandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    single_use: {
      amount: 6000,
      currency: "EUR",
    },
  },
};

const multiUseMandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    multi_use: {
      amount: 6000,
      currency: "EUR",
    },
  },
};

const captureNotSupported = {
  type: "invalid_request",
  message: "Payment method type not supported",
  code: "IR_19",
  reason:
    "Capture Not allowed in case of Creating the Subscriber is not supported by Paybox",
};

const billingData = {
  address: {
    line1: "1467",
    line2: "Harrison Street\nApt 101",
    line3: "Harrison Street\nApt 101",
    city: "Paris",
    state: "asd",
    zip: "8008",
    country: "FR",
    first_name: "joseph",
    last_name: "Doe",
  },
  phone: {
    number: "9123456789",
    country_code: "+91",
  },
  email: "example@email.com",
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "EUR",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: billingData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    PaymentIntentOffSession: {
      Request: {
        currency: "EUR",
        amount: 6000,
        authentication_type: "no_three_ds",
        customer_acceptance: null,
        setup_future_usage: "off_session",
        billing: billingData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          setup_future_usage: "off_session",
        },
      },
    },
    No3DSManualCapture: {
      Request: {
        currency: "EUR",
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: billingData,
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
        // Auto capture with different currency, so we need to pass currency in here
        currency: "EUR",
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: billingData,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
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
        billing: billingData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          setup_future_usage: "on_session",
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
        billing: billingData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          setup_future_usage: "on_session",
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
    VoidAfterConfirm: {
      Request: {},
      Response: {
        status: 501,
        body: {
          status: "cancelled",
          error: {
            type: "invalid_request",
            message: "Cancel/Void flow is not implemented",
            code: "IR_00",
          },
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
          status: "succeeded",
        },
      },
    },
    PaymentIntentWithShippingCost: {
      Request: {
        currency: "EUR",
        shipping_cost: 50,
        billing: billingData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          shipping_cost: 50,
          amount: 6000,
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
          amount_received: 6050,
          amount: 6000,
          net_amount: 6050,
        },
      },
    },
    MandateSingleUse3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "EUR",
        mandate_data: singleUseMandateData,
        billing: billingData,
      },
      Response: {
        status: 400,
        body: {
          error: captureNotSupported,
        },
      },
    },
    MandateSingleUse3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "EUR",
        mandate_data: singleUseMandateData,
        billing: billingData,
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
        currency: "EUR",
        mandate_data: singleUseMandateData,
        billing: billingData,
      },
      Response: {
        status: 400,
        body: {
          error: captureNotSupported,
        },
      },
    },
    MandateSingleUseNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "EUR",
        mandate_data: singleUseMandateData,
        billing: billingData,
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
        },
        currency: "EUR",
        mandate_data: multiUseMandateData,
        billing: billingData,
      },
      Response: {
        status: 200,
        body: {
          error: captureNotSupported,
        },
      },
    },
    MandateMultiUseNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "EUR",
        mandate_data: multiUseMandateData,
        billing: billingData,
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
        currency: "EUR",
        mandate_data: multiUseMandateData,
        billing: billingData,
      },
      Response: {
        status: 400,
        body: {
          error: captureNotSupported,
        },
      },
    },
    MandateMultiUse3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "EUR",
        mandate_data: multiUseMandateData,
        billing: billingData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    MITAutoCapture: {
      Request: {
        currency: "EUR",
        amount: 6000,
        billing: billingData,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    MITManualCapture: {
      Request: {
        currency: "EUR",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    PaymentMethodIdMandateNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "EUR",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
        billing: billingData,
      },
      Response: {
        status: 400,
        body: {
          error: captureNotSupported,
        },
      },
    },
    PaymentMethodIdMandateNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "EUR",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
        billing: billingData,
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
        },
        currency: "EUR",
        mandate_data: null,
        authentication_type: "three_ds",
        customer_acceptance: customerAcceptance,
        billing: billingData,
      },
      Response: {
        status: 400,
        body: {
          error: captureNotSupported,
        },
      },
    },
    PaymentMethodIdMandate3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        mandate_data: null,
        authentication_type: "three_ds",
        customer_acceptance: customerAcceptance,
        billing: billingData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    ZeroAuthMandate: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "EUR",
        mandate_data: singleUseMandateData,
        billing: billingData,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    ZeroAuthPaymentIntent: {
      Request: {
        amount: 0,
        setup_future_usage: "off_session",
        currency: "EUR",
        billing: billingData,
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
        billing: billingData,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    SaveCardUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "EUR",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
        billing: billingData,
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
        },
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
        billing: billingData,
      },
      Response: {
        status: 400,
        body: {
          error: captureNotSupported,
        },
      },
    },
    SaveCardUse3DSAutoCaptureOffSession: {
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
        billing: billingData,
      },
      Response: {
        status: 400,
        body: {
          error: captureNotSupported,
        },
      },
    },
    SaveCardUseNo3DSManualCaptureOffSession: {
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "EUR",
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
        billing: billingData,
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
        billing: billingData,
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
        },
        currency: "EUR",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
        billing: billingData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    No3DSFailPayment: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "EUR",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: billingData,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    PaymentWithoutBilling: {
      Request: {
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        authentication_type: "no_three_ds",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    PaymentWithBilling: {
      Request: {
        currency: "USD",
        setup_future_usage: "on_session",
        billing: billingData,
        email: "hyperswitch.example@gmail.com",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    PaymentWithFullName: {
      Request: {
        currency: "USD",
        setup_future_usage: "on_session",
        billing: {
          address: {
            first_name: "joseph",
            last_name: "Doe",
          },
          phone: {
            number: "9111222333",
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
    PaymentWithBillingEmail: {
      Request: {
        currency: "USD",
        setup_future_usage: "on_session",
        email: "hyperswitch_sdk_demo_id1@gmail.com",
        billing: {
          address: {
            first_name: "joseph",
            last_name: "Doe",
          },
          phone: {
            number: "9111222333",
            country_code: "+91",
          },
          email: "hyperswitch.example@gmail.com",
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
  },
};
