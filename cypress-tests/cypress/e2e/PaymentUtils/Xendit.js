const successfulNo3DSCardDetails = {
  card_number: "4000000000001091",
  card_exp_month: "12",
  card_exp_year: "27",
  card_holder_name: "joseph Doe",
  card_cvc: "123",
};
const billing_details = {
  email: "mauro.morandi@nexi.it",
  phone: {
    number: "9123456789",
    country_code: "+91",
  },
};
const customer_acceptance = {
  acceptance_type: "offline",
  accepted_at: "1963-05-03T04:07:52.723Z",
  online: {
    ip_address: "127.0.0.1",
    user_agent: "amet irure esse",
  }
};
const payment_method_data_3ds = {
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
      number: "8056594427",
      country_code: "+91"
    },
  },
};

const singleUseMandateData = {
  customer_acceptance: customer_acceptance,
  mandate_type: {
    single_use: {
      amount: 1600000,
      currency: "IDR",
    },
  },
};

const multiUseMandateData = {
  customer_acceptance: customer_acceptance,
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
        amount: 6500000,
        billing: billing_details
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
          amount: 6500000,
        },
      },
    },
    PaymentConfirmWithShippingCost: {
      Request: {
        amount: 6500000,
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
          amount: 6500000,
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
        amount: 6500000,
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: billing_details,
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
        amount: 6500000,
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: billing_details
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
          billing: billing_details
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
          payment_method_data: payment_method_data_3ds,
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
          billing: billing_details
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
          billing: billing_details,
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
          billing: billing_details,
        },
        currency: "IDR",
        setup_future_usage: "on_session",
        customer_acceptance: customer_acceptance,
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
          billing: billing_details
        },
        setup_future_usage: "off_session",
        customer_acceptance: customer_acceptance,
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
          billing: billing_details
        },
        setup_future_usage: "off_session",
        customer_acceptance: customer_acceptance,
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
          billing: billing_details
        },
        currency: "IDR",
        setup_future_usage: "on_session",
        customer_acceptance: customer_acceptance,
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
        amount: 6500000,
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: billing_details
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
        amount: 6500000,
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
          payment_method_data: payment_method_data_3ds,
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
        amount: 6500000,
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: billing_details
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
        payment_method: "card",
        amount: 6500000,
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
          amount: 6500000,
          amount_capturable: 0,
          amount_received: 6500000,
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
        amount: 6500000,
      },
      Response: {
        status: 200,
        body: {
          status: "partially_captured",
          amount: 6500000,
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
            message: "Cancel/Void flow is not supported",
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
        billing: billing_details,
        mandate_data: null,
        customer_acceptance: customer_acceptance,
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
        billing: billing_details,
        currency: "IDR",
        mandate_data: null,
        customer_acceptance: customer_acceptance,
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
