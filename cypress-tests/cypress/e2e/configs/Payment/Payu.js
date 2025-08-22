import { customerAcceptance } from "./Commons";

const successfulNo3DSCardDetails = {
  card_number: "4012001037141112",
  card_exp_month: "03",
  card_exp_year: "30",
  card_holder_name: "John Doe",
  card_cvc: "123",
};

const failedNo3DSCardDetails = {
  card_number: "4000000000000002",
  card_exp_month: "01",
  card_exp_year: "35",
  card_holder_name: "joseph Doe",
  card_cvc: "123",
};

const singleUseMandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    single_use: {
      amount: 8000,
      currency: "PLN",
    },
  },
};

const multiUseMandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    multi_use: {
      amount: 8000,
      currency: "PLN",
    },
  },
};

const polishBillingAddress = {
  address: {
    line1: "Marszałkowska 84/92",
    line2: "Apt 4B",
    line3: "Near Palace of Culture",
    city: "Warsaw",
    state: "Mazowieckie",
    zip: "00-514",
    country: "PL",
    first_name: "Jan",
    last_name: "Kowalski",
  },
  phone: {
    number: "9123456789",
    country_code: "+48",
  },
};

const paymentMethodData = {
  card: {
    last4: "1112",
    card_type: "DEBIT",
    card_network: "Visa",
    card_issuer: "VISA PRODUCTION SUPPORT CLIENT BID 1",
    card_issuing_country: "UNITEDSTATES",
    card_isin: "401200",
    card_extended_bin: null,
    card_exp_month: "03",
    card_exp_year: "30",
    card_holder_name: "John Doe",
    payment_checks: null,
    authentication_data: null,
  },
  billing: null,
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "PLN",
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
    PaymentIntentOffSession: {
      Request: {
        amount: 6000,
        authentication_type: "three_ds",
        currency: "PLN",
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
    PaymentIntentWithShippingCost: {
      Request: {
        currency: "PLN",
        shipping_cost: 50,
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
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
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
          status: "processing",
          shipping_cost: 50,
          amount_received: null,
          amount: 6000,
          net_amount: 6050,
          payment_method_data: paymentMethodData,
        },
      },
    },
    "3DSManualCapture": {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "PLN",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: polishBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
          payment_method_data: paymentMethodData,
        },
      },
    },
    "3DSAutoCapture": {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "PLN",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: polishBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
          payment_method_data: paymentMethodData,
        },
      },
    },
    No3DSManualCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "PLN",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: polishBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
          payment_method_data: paymentMethodData,
        },
      },
    },
    No3DSAutoCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "PLN",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: polishBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
          payment_method_data: paymentMethodData,
        },
      },
    },
    No3DSFailPayment: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: failedNo3DSCardDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: polishBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
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
          status: "processing",
          amount: 6000,
          amount_capturable: 6000,
          amount_received: 0,
          payment_method_data: paymentMethodData,
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
          status: "processing",
          amount: 6000,
          amount_capturable: 6000,
          amount_received: 0,
        },
      },
    },
    VoidAfterConfirm: {
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    PaymentSync: {
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    Refund: {
      //This connector doesn't support multiple refunds
      Configs: {
        TRIGGER_SKIP: true,
      },
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
      //This connector doesn't support multiple refunds
      Configs: {
        TRIGGER_SKIP: true,
      },
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
    manualPaymentRefund: {
      //This connector doesn't support multiple refunds
      Configs: {
        TRIGGER_SKIP: true,
      },
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
    manualPaymentPartialRefund: {
      //This connector doesn't support multiple refunds
      Configs: {
        TRIGGER_SKIP: true,
      },
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
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "PLN",
        mandate_data: singleUseMandateData,
        billing: polishBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    MandateSingleUse3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "PLN",
        mandate_data: singleUseMandateData,
        billing: polishBillingAddress,
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
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "PLN",
        mandate_data: singleUseMandateData,
        billing: polishBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    MandateSingleUseNo3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "PLN",
        mandate_data: singleUseMandateData,
        billing: polishBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    MandateMultiUseNo3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "PLN",
        mandate_data: multiUseMandateData,
        billing: polishBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    MandateMultiUseNo3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "PLN",
        mandate_data: multiUseMandateData,
        billing: polishBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    MandateMultiUse3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "PLN",
        mandate_data: multiUseMandateData,
        billing: polishBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    MandateMultiUse3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "PLN",
        mandate_data: multiUseMandateData,
        billing: polishBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    ZeroAuthMandate: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "PLN",
        mandate_data: singleUseMandateData,
        billing: polishBillingAddress,
      },
      Response: {
        status: 501,
        body: {
          error: {
            type: "invalid_request",
            message: "Setup Mandate flow for Payu is not implemented",
            code: "IR_00",
          },
        },
      },
    },
    ZeroAuthPaymentIntent: {
      Request: {
        amount: 0,
        setup_future_usage: "off_session",
        currency: "PLN",
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
        billing: polishBillingAddress,
      },
      Response: {
        status: 501,
        body: {
          error: {
            type: "invalid_request",
            message: "Setup Mandate flow for Payu is not implemented",
            code: "IR_00",
          },
        },
      },
    },
    SaveCardUseNo3DSAutoCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "PLN",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
        billing: polishBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
          payment_method_data: paymentMethodData,
        },
      },
    },
    SaveCardUseNo3DSAutoCaptureOffSession: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        currency: "PLN",
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
        billing: polishBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    SaveCardUse3DSAutoCaptureOffSession: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
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
    SaveCardUseNo3DSManualCaptureOffSession: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
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
        TRIGGER_SKIP: true,
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
    SaveCardConfirmManualCaptureOffSession: {
      Configs: {
        TRIGGER_SKIP: true,
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
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "PLN",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
        billing: polishBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
          payment_method_data: paymentMethodData,
        },
      },
    },
    PaymentMethodIdMandateNo3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "PLN",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
        billing: polishBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    PaymentMethodIdMandateNo3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "PLN",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
        billing: polishBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    PaymentMethodIdMandate3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "PLN",
        mandate_data: null,
        authentication_type: "three_ds",
        customer_acceptance: customerAcceptance,
        billing: polishBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    PaymentMethodIdMandate3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "PLN",
        mandate_data: null,
        authentication_type: "three_ds",
        customer_acceptance: customerAcceptance,
        billing: polishBillingAddress,
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
