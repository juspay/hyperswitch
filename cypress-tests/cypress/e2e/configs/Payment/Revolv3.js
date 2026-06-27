import { getCustomExchange } from "./Modifiers";

// Test card details for Revolv3 connector
const successfulNo3DSCardDetails = {
  card_number: "4111111111111111",
  card_exp_month: "12",
  card_exp_year: "2030",
  card_holder_name: "Test User",
  card_cvc: "123",
};

const successfulNo3DSDebitCardDetails = {
  card_number: "4000000000000002",
  card_exp_month: "12",
  card_exp_year: "2030",
  card_holder_name: "Test User",
  card_cvc: "123",
};

const successfulNo3DSCreditCardDetails = {
  card_number: "5555555555554444",
  card_exp_month: "12",
  card_exp_year: "2030",
  card_holder_name: "Test User",
  card_cvc: "123",
};

const usdBillingAddress = {
  address: {
    line1: "123 Main Street",
    line2: "Apt 4B",
    city: "New York",
    state: "NY",
    zip: "10001",
    country: "US",
    first_name: "Test",
    last_name: "User",
  },
  phone: {
    number: "8056594427",
    country_code: "+1",
  },
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "USD",
        customer_acceptance: null,
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
        billing: usdBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
          shipping_cost: 50,
          amount: 6000,
        },
      },
    },
    // No3DS Auto Capture - Generic Card (defaults to debit behavior)
    No3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: usdBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
          amount: 6000,
          amount_capturable: 6000,
        },
      },
    },
    // No3DS Auto Capture - Debit Card
    No3DSAutoCaptureDebit: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSDebitCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: usdBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
          amount: 6000,
          amount_capturable: 6000,
        },
      },
    },
    // No3DS Auto Capture - Credit Card
    No3DSAutoCaptureCredit: {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCreditCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: usdBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
          amount: 6000,
        },
      },
    },
    // No3DS Manual Capture - Revolv3 is async, returns processing not requires_capture
    No3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: usdBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
          amount: 6000,
          amount_capturable: 6000,
        },
      },
    },
    // Capture for async connectors - returns processing since payment stays processing
    Capture: {
      Request: {
        amount_to_capture: 5000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "This Payment could not be captured because it has a capture_method of manual. The expected state is manual_multiple",
            code: "IR_14",
          },
        },
      },
    },
    PartialCapture: {
      Request: {
        amount_to_capture: 2000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "This Payment could not be captured because it has a capture_method of manual. The expected state is manual_multiple",
            code: "IR_14",
          },
        },
      },
    },
    // Refund flows - blocked because payment stays in processing state
    Refund: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        amount: 6540,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "This Payment could not be refund because it has a status of processing. The expected state is succeeded, partially_captured",
            code: "IR_14",
          },
        },
      },
    },
    PartialRefund: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        amount: 3000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "This Payment could not be refund because it has a status of processing. The expected state is succeeded, partially_captured",
            code: "IR_14",
          },
        },
      },
    },
    manualPaymentRefund: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        amount: 5000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "This Payment could not be refund because it has a status of processing. The expected state is succeeded, partially_captured",
            code: "IR_14",
          },
        },
      },
    },
    manualPaymentPartialRefund: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        amount: 2000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "This Payment could not be refund because it has a status of processing. The expected state is succeeded, partially_captured",
            code: "IR_14",
          },
        },
      },
    },
    SyncRefund: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "This Payment could not be refund because it has a status of processing. The expected state is succeeded, partially_captured",
            code: "IR_14",
          },
        },
      },
    },
    VoidAfterConfirm: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "You cannot cancel this payment because it has status processing",
            code: "IR_16",
          },
        },
      },
    },
    // 3DS flows - not supported or return processing
    "3DSManualCapture": {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: usdBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
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
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: usdBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    // Mandate flows - not fully supported
    ZeroAuthMandate: {
      Configs: {
        TRIGGER_SKIP: true,
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
    MandateSingleUseNo3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        setup_future_usage: "on_session",
        billing: usdBillingAddress,
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
        currency: "USD",
        setup_future_usage: "on_session",
        billing: usdBillingAddress,
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
        currency: "USD",
        setup_future_usage: "on_session",
        billing: usdBillingAddress,
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
        currency: "USD",
        setup_future_usage: "on_session",
        billing: usdBillingAddress,
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
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        setup_future_usage: "on_session",
        billing: usdBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    SaveCardUseNo3DSAutoCaptureOffSession: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulNo3DSDebitCardDetails,
        },
        currency: "USD",
        setup_future_usage: "off_session",
        billing: usdBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
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
        currency: "USD",
        setup_future_usage: "on_session",
        billing: usdBillingAddress,
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
        currency: "USD",
        setup_future_usage: "on_session",
        billing: usdBillingAddress,
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
