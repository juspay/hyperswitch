const successfulNo3DSCardDetails = {
  card_number: "4000000000001091",
  card_exp_month: "12",
  card_exp_year: "27",
  card_holder_name: "joseph Doe",
  card_cvc: "123",
};
const billingDetails = {
  email: "mauro.morandi@nexi.it",
  phone: {
    number: "9123456789",
    country_code: "+91",
  },
};
const customerAcceptance = {
  acceptance_type: "offline",
  accepted_at: "1963-05-03T04:07:52.723Z",
  online: {
    ip_address: "127.0.0.1",
    user_agent: "amet irure esse",
  },
};
const paymentMethodData3ds = {
  card: {
    last4: "1091",
    card_type: "CREDIT",
    card_network: "Visa",
    card_issuer: "INTL HDQTRS-CENTER OWNED",
    card_issuing_country: "UNITEDSTATES",
    card_isin: "400000",
    card_extended_bin: null,
    card_exp_month: "12",
    card_exp_year: "27",
    card_holder_name: "joseph Doe",
    payment_checks: null,
    authentication_data: null,
  },
  billing: {
    address: null,
    email: "mauro.morandi@nexi.it",
    phone: {
      number: "9123456789",
      country_code: "+91",
    },
  },
};

const singleUseMandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    single_use: {
      amount: 1600000,
      currency: "IDR",
    },
  },
};

const multiUseMandateData = {
  customer_acceptance: customerAcceptance,
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
        amount: 6000000,
        billing: billingDetails,
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
        shipping_cost: 100,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          shipping_cost: 100,
          amount: 6000000,
        },
      },
    },
    PaymentConfirmWithShippingCost: {
      Request: {
        amount: 6000000,
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
          status: "processing",
          shipping_cost: 100,
          amount: 6000000,
        },
      },
    },
    No3DSManualCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
      },
      Request: {
        payment_method: "card",
        amount: 6000000,
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: billingDetails,
        },
        currency: "IDR",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    No3DSAutoCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
      },
      Request: {
        payment_method: "card",
        amount: 6000000,
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: billingDetails,
        },
        currency: "IDR",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    manualPaymentPartialRefund: {
      Request: {
        amount: 2000000,
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
        amount: 6000000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    "3DSAutoCapture": {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: billingDetails,
        },
        currency: "IDR",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          setup_future_usage: "on_session",
          payment_method_data: paymentMethodData3ds,
        },
      },
    },
    MandateMultiUseNo3DSAutoCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: billingDetails,
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
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: billingDetails,
        },
        currency: "IDR",
        mandate_data: multiUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    SaveCardUseNo3DSAutoCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: billingDetails,
        },
        currency: "IDR",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    SaveCardUseNo3DSAutoCaptureOffSession: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
      },
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: billingDetails,
        },
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    SaveCardUseNo3DSManualCaptureOffSession: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: billingDetails,
        },
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    SaveCardConfirmAutoCaptureOffSession: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
      },
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
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
      },
      Request: {
        setup_future_usage: "off_session",
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    SaveCardUseNo3DSManualCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: billingDetails,
        },
        currency: "IDR",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    MandateSingleUseNo3DSAutoCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 1000,
        },
      },
      Request: {
        payment_method: "card",
        amount: 6000000,
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: billingDetails,
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
    "3DSManualCapture": {
      Request: {
        amount: 6000000,
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "IDR",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          setup_future_usage: "on_session",
          payment_method_data: paymentMethodData3ds,
        },
      },
    },
    MandateSingleUseNo3DSManualCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
      },
      Request: {
        payment_method: "card",
        amount: 6000000,
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: billingDetails,
        },
        currency: "IDR",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    Capture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
      },
      Request: {
        amount_to_capture: 6000000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          amount: 6000000,
          amount_capturable: 0,
          amount_received: 6000000,
        },
      },
    },
    PartialCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
      },
      Request: {
        amount_to_capture: 2000000,
      },
      Response: {
        status: 200,
        body: {
          status: "partially_captured",
          amount: 2000000,
          amount_capturable: 0,
          amount_received: 2000000,
        },
      },
    },
    Refund: {
      Request: {
        amount: 6000000,
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
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Cancel/Void flow is not supported",
            code: "IR_19",
          },
        },
      },
    },
    PartialRefund: {
      Request: {
        amount: 2000000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
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
    PaymentMethodIdMandateNo3DSAutoCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "IDR",
        billing: billingDetails,
        mandate_data: null,
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    PaymentMethodIdMandateNo3DSManualCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        billing: billingDetails,
        currency: "IDR",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
  },
};
