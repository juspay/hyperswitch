import { getCustomExchange } from "./Modifiers";

const successfulNo3DSCardDetails = {
  card_number: "4012000033330026",
  card_exp_month: "01",
  card_exp_year: "50",
  card_holder_name: "joseph Doe",
  card_cvc: "123",
};

const successfulThreeDSTestCardDetails = {
  card_number: "4868719460707704",
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
      amount: 8000,
      currency: "USD",
    },
  },
};

const multiUseMandateData = {
  customer_acceptance: customerAcceptance,
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
    PaymentIntentWithShippingCost: {
      Request: {
        currency: "USD",
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
          status: "failed",
          error_code: "AMOUNT_MISMATCH",
          error_message:
            "description - Should equal item_total + tax_total + shipping + handling + insurance - shipping_discount - discount., value - 65.50, field - value;",
        },
      },
    },
    "3DSManualCapture": {
      Configs: {
        TRIGGER_SKIP: true,
      },
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
      Configs: {
        TRIGGER_SKIP: true,
      },
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
          status: "succeeded",
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
          status: "succeeded",
        },
      },
    },
    manualPaymentRefund: {
      Request: {
        amount: 6000,
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
        amount: 2000,
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
    ZeroAuthMandate: {
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
    ZeroAuthPaymentIntent: {
      Request: {
        amount: 0,
        setup_future_usage: "off_session",
        currency: "USD",
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
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          setup_future_usage: "off_session",
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
    PaymentIntentOffSession: {
      Request: {
        currency: "USD",
        amount: 6000,
        authentication_type: "no_three_ds",
        customer_acceptance: null,
        setup_future_usage: "off_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          setup_future_usage: "off_session",
        },
      },
    },
    SaveCardUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
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
    SaveCardUseNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
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
          status: "succeeded",
        },
      },
    },
    SaveCardUseNo3DSManualCaptureOffSession: {
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
          status: "requires_capture",
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
    MandateSingleUse3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
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
        },
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
          status: "succeeded",
        },
      },
    },
    MITManualCapture: {
      Request: {},
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
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        mandate_data: null,
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
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
        mandate_data: null,
        authentication_type: "three_ds",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
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
        mandate_data: null,
        authentication_type: "three_ds",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
  },
  bank_redirect_pm: {
    PaymentIntent: getCustomExchange({
      Request: {
        currency: "EUR",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
    Ideal: {
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "ideal",
        payment_method_data: {
          bank_redirect: {
            ideal: {
              bank_name: "ing",
            },
          },
        },
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
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    Giropay: {
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "giropay",
        payment_method_data: {
          bank_redirect: {
            giropay: {
              bank_name: "",
              bank_account_bic: "",
              bank_account_iban: "",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "San Fransico",
            state: "California",
            zip: "94122",
            country: "DE",
            first_name: "joseph",
            last_name: "Doe",
          },
          phone: {
            number: "9123456789",
            country_code: "+91",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_code: "PERMISSION_DENIED",
        },
      },
    },
    Sofort: {
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "sofort",
        payment_method_data: {
          bank_redirect: {
            sofort: {
              preferred_language: "en",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "San Fransico",
            state: "California",
            zip: "94122",
            country: "AT",
            first_name: "joseph",
            last_name: "Doe",
          },
          phone: {
            number: "9123456789",
            country_code: "+91",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_code: "PERMISSION_DENIED",
        },
      },
    },
    Eps: {
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "eps",
        payment_method_data: {
          bank_redirect: {
            eps: {
              bank_name: "ing",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "San Fransico",
            state: "California",
            zip: "94122",
            country: "AT",
            first_name: "joseph",
            last_name: "Doe",
          },
          phone: {
            number: "9123456789",
            country_code: "+91",
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
