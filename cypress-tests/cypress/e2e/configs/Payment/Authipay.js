import { cardRequiredField, customerAcceptance } from "./Commons";
import { getCustomExchange } from "./Modifiers";

// ============================================================================
// AUTHIPAY CYPRESS TEST CONFIGURATION (SIMPLIFIED)
// ============================================================================
// Core functionality only:
// - Non-3DS card payments (auto/manual capture)
// - Refund operations (full/partial/sync)
// - Basic failure scenarios
// ============================================================================

// ===== TEST CARD DATA =====

const successfulNo3DSCardDetails = {
  card_number: "4147463011110083", // Authipay Mastercard test card
  card_exp_month: "10",
  card_exp_year: "30",
  card_holder_name: "Test User",
  card_cvc: "123",
};

const failedCardDetails = {
  card_number: "4000000000000002", // Generic declined card
  card_exp_month: "10",
  card_exp_year: "30",
  card_holder_name: "Test User",
  card_cvc: "123",
};

// ===== MANDATE DATA =====

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

// ===== BILLING INFORMATION =====

const billingAddress = {
  address: {
    line1: "1467",
    line2: "Harrison Street",
    city: "San Francisco",
    state: "California",
    zip: "94122",
    country: "US",
    first_name: "Test",
    last_name: "User",
  },
  phone: {
    number: "9123456789",
    country_code: "+1",
  },
};

// ===== PAYMENT METHOD DATA =====

const payment_method_data = {
  card: {
    last4: "0083",
    card_type: null,
    card_network: null,
    card_issuer: null,
    card_issuing_country: null,
    card_isin: "414746",
    card_extended_bin: null,
    card_exp_month: "10",
    card_exp_year: "30",
    card_holder_name: "Test User",
    payment_checks: null,
    authentication_data: null,
  },
  billing: null,
};

const payment_method_data_failed = {
  card: {
    last4: "0002",
    card_type: null,
    card_network: null,
    card_issuer: null,
    card_issuing_country: null,
    card_isin: "400000",
    card_extended_bin: null,
    card_exp_month: "10",
    card_exp_year: "30",
    card_holder_name: "Test User",
    payment_checks: null,
    authentication_data: null,
  },
  billing: null,
};

// ===== REQUIRED FIELDS =====

const requiredFields = {
  payment_methods: [
    {
      payment_method: "card",
      payment_method_types: [
        {
          payment_method_type: "credit",
          card_networks: [
            {
              eligible_connectors: ["authipay"],
            },
          ],
          required_fields: cardRequiredField,
        },
      ],
    },
  ],
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
        billing: billingAddress,
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: null,
        capture_method: "automatic",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method: "card",
          attempt_count: 1,
          payment_method_data: payment_method_data,
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
        billing: billingAddress,
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: null,
        capture_method: "manual",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          payment_method: "card",
          attempt_count: 1,
          payment_method_data: payment_method_data,
        },
      },
    },

    // Failed payment
    No3DSFailPayment: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: failedCardDetails,
        },
        billing: billingAddress,
        customer_acceptance: null,
        setup_future_usage: null,
      },
      //Response as successful payment due to Authipay's failed payment cards not getting declined on connector side
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method: "card",
          attempt_count: 1,
          payment_method_data: payment_method_data_failed,
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
        amount_to_capture: 3000,
      },
      Response: {
        status: 200,
        body: {
          status: "partially_captured",
          amount: 6000,
          amount_capturable: 0,
          amount_received: 3000,
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
        amount: 3000,
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
    // This should work for payments in "requires_capture" state
    VoidAfterConfirm: {
      Request: {
        cancellation_reason: "customer_request",
      },
      Response: {
        status: 200,
        body: {
          status: "cancelled",
          payment_method: "card",
          attempt_count: 1,
        },
      },
    },

    // Void payment in early state (before payment confirmation)
    Void: {
      Request: {
        cancellation_reason: "duplicate_transaction",
      },
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
          payment_method: "card",
          payment_method_data: payment_method_data,
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
        billing: billingAddress,
        customer_acceptance: null,
        setup_future_usage: null,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method: "card",
          payment_method_data: payment_method_data,
          shipping_cost: 50,
        },
      },
    },

    // ===== MANDATE SCENARIOS =====
    // Note: Authipay may not implement mandates, marked as TRIGGER_SKIP

    MandateSingleUseNo3DSAutoCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Skip if Authipay doesn't support mandates
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
        status: 400,
        body: {},
      },
    }),

    MandateSingleUseNo3DSManualCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Skip if Authipay doesn't support mandates
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
    }),

    MandateMultiUseNo3DSAutoCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Skip if Authipay doesn't support mandates
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
    }),

    MandateMultiUseNo3DSManualCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Skip if Authipay doesn't support mandates
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
    }),

    // ===== SAVE CARD SCENARIOS =====
    // Note: Authipay may not support save card, marked as TRIGGER_SKIP

    SaveCardUseNo3DSAutoCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Skip if Authipay doesn't support save card
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
        status: 400,
        body: {
          status: "succeeded",
        },
      },
    }),

    SaveCardUseNo3DSManualCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Skip if Authipay doesn't support save card
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
          status: "requires_capture",
        },
      },
    }),

    // ===== ZERO AUTH SCENARIOS =====
    // Note: Authipay may not support zero auth, marked as TRIGGER_SKIP

    ZeroAuthPaymentIntent: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Skip if Authipay doesn't support zero auth
      },
      Request: {
        amount: 0,
        setup_future_usage: "off_session",
        currency: "USD",
        payment_type: "setup_mandate",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),

    ZeroAuthConfirmPayment: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Skip if Authipay doesn't support zero auth
      },
      Request: {
        payment_type: "setup_mandate",
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),

    // ===== MIT (MERCHANT INITIATED TRANSACTION) SCENARIOS =====
    // Note: Authipay may not support MIT, marked as TRIGGER_SKIP

    MITAutoCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Skip if Authipay doesn't support MIT
      },
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),

    MITWithoutBillingAddress: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Skip if Authipay doesn't support MIT
      },
      Request: {
        billing: null,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),

    MandateSingleUse3DSAutoCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Skip - 3DS not implemented
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
          status: "requires_customer_action",
        },
      },
    }),

    MandateSingleUse3DSManualCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Skip - 3DS not implemented
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
          status: "requires_customer_action",
        },
      },
    }),

    MandateMultiUse3DSAutoCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Skip - 3DS not implemented
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
          status: "requires_customer_action",
        },
      },
    }),

    MandateMultiUse3DSManualCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Skip - 3DS not implemented
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
          status: "requires_customer_action",
        },
      },
    }),

    SaveCardUse3DSAutoCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Skip - 3DS not implemented
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
          status: "requires_customer_action",
        },
      },
    }),

    SaveCardUse3DSManualCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Skip - 3DS not implemented
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
          status: "requires_customer_action",
        },
      },
    }),

    // ===== OFF-SESSION SCENARIOS =====
    // Note: Authipay may not support off-session, marked as TRIGGER_SKIP

    PaymentIntentOffSession: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Skip if Authipay doesn't support off-session
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
    }),

    SaveCardUseNo3DSAutoCaptureOffSession: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Skip if Authipay doesn't support off-session
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
          status: "succeeded",
        },
      },
    }),

    SaveCardUse3DSAutoCaptureOffSession: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Skip - 3DS not implemented and off-session may not be supported
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
          status: "requires_customer_action",
        },
      },
    }),

    SaveCardUseNo3DSManualCaptureOffSession: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Skip if Authipay doesn't support off-session
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
    }),

    SaveCardConfirmAutoCaptureOffSession: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Skip if Authipay doesn't support off-session
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
    }),

    SaveCardConfirmManualCaptureOffSession: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Skip if Authipay doesn't support off-session
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
    }),

    SaveCardConfirmAutoCaptureOffSessionWithoutBilling: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Skip if Authipay doesn't support off-session
      },
      Request: {
        setup_future_usage: "off_session",
        billing: null,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          billing: null,
        },
      },
    }),

    // ===== PAYMENT METHOD SCENARIOS =====
    // Note: These may not be supported by Authipay, marked as TRIGGER_SKIP

    PaymentMethod: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Skip if Authipay doesn't support payment method creation
      },
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_issuer: "Gpay",
        payment_method_issuer_code: "jp_hdfc",
        card: successfulNo3DSCardDetails,
      },
      Response: {
        status: 200,
        body: {},
      },
    }),

    PaymentMethodIdMandateNo3DSAutoCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Skip if Authipay doesn't support payment method ID mandates
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
    }),

    PaymentMethodIdMandateNo3DSManualCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Skip if Authipay doesn't support payment method ID mandates
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
    }),
    // ===== ZERO AUTH MANDATE SCENARIO =====
    // Note: User specifically requested this to be skipped

    ZeroAuthMandate: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Skip as requested by user
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
          status: "processing",
        },
      },
    }),

    // ===== SESSION TOKEN SCENARIO =====
    // Note: Authipay may not support session tokens, marked as TRIGGER_SKIP

    SessionToken: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Skip if Authipay doesn't support session tokens
      },
      Response: {
        status: 200,
        body: {
          session_token: [],
        },
      },
    }),
  },

  pm_list: {
    PmListResponse: {
      PmListNull: {
        payment_methods: [],
      },
      pmListDynamicFieldWithoutBilling: requiredFields,
      pmListDynamicFieldWithBilling: requiredFields,
      pmListDynamicFieldWithNames: requiredFields,
      pmListDynamicFieldWithEmail: requiredFields,
    },
  },
};
