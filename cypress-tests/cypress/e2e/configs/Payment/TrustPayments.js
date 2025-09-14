import { customerAcceptance } from "./Commons";

const successfulNo3DSCardDetails = {
  card_number: "4111111111111111", // TrustPayments Visa test card
  card_exp_month: "12",
  card_exp_year: "30",
  card_holder_name: "John Doe",
  card_cvc: "123",
};

const failedCardDetails = {
  card_number: "5555666666662222", // Generic declined card
  card_exp_month: "12",
  card_exp_year: "30",
  card_holder_name: "John Doe",
  card_cvc: "123",
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

// ===== MAIN CONNECTOR DETAILS =====

export const connectorDetails = {
  card_pm: {
    // Basic payment intent
    PaymentIntent: {
      Request: {
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: null,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          setup_future_usage: null,
        },
      },
    },

    // Successful automatic capture
    No3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: null,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },

    // Successful manual capture (requires capture)
    No3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: null,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },

    // Failed payment - TrustPayments should handle card declines
    No3DSFailPayment: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: failedCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: null,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
        },
      },
    },

    // Capture operation
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

    // Partial capture
    PartialCapture: {
      Request: {
        amount_to_capture: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "partially_captured",
          amount: 6000,
        },
      },
    },

    // Full refund
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

    // Partial refund
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

    // Refund sync
    SyncRefund: {
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },

    // Manual payment refund
    manualPaymentRefund: {
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

    // Manual payment partial refund
    manualPaymentPartialRefund: {
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

    // Void payment failure (trying to void already captured payment)
    VoidPaymentFailure: {
      Request: {
        cancellation_reason: "merchant_request",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request_error",
            code: "processing_error",
            message:
              "You cannot cancel this PaymentIntent because it has a status of succeeded.",
          },
        },
      },
    },

    // Void after confirm (manual capture scenario)
    VoidAfterConfirm: {
      Request: {
        cancellation_reason: "customer_request",
      },
      Response: {
        status: 200,
        body: {
          status: "cancelled",
        },
      },
    },

    // Void payment in early state
    Void: {
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "cancelled",
        },
      },
    },

    // Payment sync
    SyncPayment: {
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },

    // Payment with shipping cost
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
        setup_future_usage: null,
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

    // ===== MANDATE SCENARIOS (NOT SUPPORTED - SKIP) =====

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
      Configs: {
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
        },
      },
    },
    MITManualCapture: {
      Configs: {
        TRIGGER_SKIP: true, // Skip if TrustPayments doesn't support mandates
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
        status: 501,
        body: {
          error: {
            type: "invalid_request_error",
            code: "connector_error",
            message: "Payment method not supported",
          },
        },
      },
    },

    // ===== SAVE CARD SCENARIOS (NOT SUPPORTED - SKIP) =====

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
        TRIGGER_SKIP: true,
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
        status: 501,
        body: {
          error: {
            type: "invalid_request_error",
            code: "connector_error",
            message: "Payment method not supported",
          },
        },
      },
    },

    // ===== ZERO AUTH SCENARIOS (NOT SUPPORTED - SKIP) =====

    ZeroAuthPaymentIntent: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        amount: 0,
        setup_future_usage: "off_session",
        currency: "USD",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },

    ZeroAuthConfirmPayment: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_type: "setup_mandate",
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
      },
      Response: {
        status: 501,
        body: {
          error: {
            type: "invalid_request_error",
            code: "connector_error",
            message: "Payment method not supported",
          },
        },
      },
    },

    ZeroAuthMandate: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
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

    // ===== MIT (MERCHANT INITIATED TRANSACTION) SCENARIOS (NOT SUPPORTED - SKIP) =====

    MITAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true, // Skip if Authipay doesn't support mandates
      },
      Request: {},
      Response: {
        status: 501,
        body: {
          error: {
            type: "invalid_request_error",
            code: "connector_error",
            message: "Payment method not supported",
          },
        },
      },
    },

    MITWithoutBillingAddress: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        billing: null,
      },
      Response: {
        status: 501,
        body: {
          error: {
            type: "invalid_request_error",
            code: "connector_error",
            message: "Payment method not supported",
          },
        },
      },
    },

    // ===== OFF-SESSION SCENARIOS (NOT SUPPORTED - SKIP) =====

    PaymentIntentOffSession: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        currency: "USD",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
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
          status: "requires_capture",
        },
      },
    },

    // ===== PAYMENT METHOD SCENARIOS (NOT SUPPORTED - SKIP) =====

    PaymentMethod: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_issuer: "Gpay",
        payment_method_issuer_code: "jp_hdfc",
        card: successfulNo3DSCardDetails,
      },
      Response: {
        status: 501,
        body: {
          error: {
            type: "invalid_request_error",
            code: "connector_error",
            message: "Payment method not supported",
          },
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
  },
};
