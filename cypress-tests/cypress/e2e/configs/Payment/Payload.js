import {
  customerAcceptance,
  singleUseMandateData,
  multiUseMandateData,
} from "./Commons";

const successfulNo3DSCardDetails = {
  card_number: "4242424242424242",
  card_exp_month: "12",
  card_exp_year: "25",
  card_holder_name: "John Doe",
  card_cvc: "123",
};

const successfulThreeDSTestCardDetails = {
  ...successfulNo3DSCardDetails,
};

const failedNo3DSCardDetails = {
  card_number: "4111111111119903",
  card_exp_month: "01",
  card_exp_year: "25",
  card_holder_name: "John Doe",
  card_cvc: "123",
};

const payment_method_data_no3ds = {
  card: {
    last4: "4242",
    card_type: "CREDIT",
    card_network: "Visa",
    card_issuer: "STRIPE PAYMENTS UK LIMITED",
    card_issuing_country: "UNITEDKINGDOM",
    card_isin: "424242",
    card_extended_bin: null,
    card_exp_month: "12",
    card_exp_year: "25",
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
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          setup_future_usage: "on_session",
        },
      },
    },
    PaymentIntentOffSession: {
      Request: {
        currency: "USD",
        customer_acceptance: null,
        amount: 6000,
        authentication_type: "no_three_ds",
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
          status: "succeeded",
          shipping_cost: 50,
          amount_received: 6050,
          amount: 6000,
          net_amount: 6050,
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
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "3DS authentication is not supported by Payload",
            code: "IR_00",
          },
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
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "3DS authentication is not supported by Payload",
            code: "IR_00",
          },
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
          status: "requires_capture", // Manual capture should require explicit capture
          payment_method: "card",
          attempt_count: 1,
        },
      },
    },
    No3DSAutoCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 15000,
        },
      },
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
          payment_method: "card",
          attempt_count: 1,
        },
      },
    },
    No3DSFailPayment: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: failedNo3DSCardDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_code: "card_declined",
          error_message: "Your card was declined",
          unified_code: "UE_9000",
          unified_message: "Something went wrong",
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
    SyncRefund: {
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    SaveCardUseNo3DSAutoCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 15000,
        },
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
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
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 15000,
        },
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture", // Keep this as requires_capture for manual flows
        },
      },
    },
    PaymentMethodIdMandateNo3DSAutoCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 15000,
        },
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
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
    SaveCardUseNo3DSAutoCaptureOffSession: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 15000,
        },
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
          status: "succeeded",
        },
      },
    },
    SaveCardUse3DSAutoCaptureOffSession: {
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
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "3DS authentication is not supported by Payload",
            code: "IR_00",
          },
        },
      },
    },
    SaveCardUseNo3DSManualCaptureOffSession: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 15000,
        },
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
          status: "requires_capture",
        },
      },
    },
    SaveCardConfirmAutoCaptureOffSession: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 15000,
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
          TIMEOUT: 15000,
        },
      },
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
    SaveCardConfirmAutoCaptureOffSessionWithoutBilling: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 15000,
        },
      },
      Request: {
        setup_future_usage: "off_session",
        billing: null,
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
          TIMEOUT: 15000,
        },
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
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
    MandateSingleUseNo3DSAutoCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 15000,
        },
        TRIGGER_SKIP: true,
      },
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
          mandate_id: null,
          payment_method_data: payment_method_data_no3ds,
          payment_method: "card",
          connector: "payload",
        },
      },
    },
    MandateSingleUseNo3DSManualCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 15000,
        },
        TRIGGER_SKIP: true,
      },
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
          mandate_id: null,
          payment_method_data: payment_method_data_no3ds,
          payment_method: "card",
          connector: "payload",
        },
      },
    },
    MandateMultiUseNo3DSAutoCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 15000,
        },
        TRIGGER_SKIP: true,
      },
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
          status: "succeeded",
          payment_method_data: payment_method_data_no3ds,
          payment_method: "card",
          connector: "payload",
        },
      },
    },
    MandateMultiUseNo3DSManualCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 15000,
        },
        TRIGGER_SKIP: true,
      },
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
          payment_method_data: payment_method_data_no3ds,
          payment_method: "card",
          connector: "payload",
        },
      },
    },
    ZeroAuthPaymentIntent: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 15000,
        },
        TRIGGER_SKIP: true,
      },
      Request: {
        currency: "BRL",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    ZeroAuthMandate: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 15000,
        },
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 501,
        body: {
          code: "IR_00",
          message: "Setup Mandate flow for Payload is not implemented",
          type: "invalid_request",
        },
      },
    },
  },
};
