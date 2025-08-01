import { cardRequiredField, customerAcceptance } from "./Commons";
import { getCustomExchange } from "./Modifiers";

// ============================================================================
// SILVERFLOW CYPRESS TEST CONFIGURATION
// ============================================================================
// Core functionality based on Silverflow API specification:
// - Non-3DS card payments (auto/manual capture)
// - Refund operations (full/partial/sync)
// - Void operations
// - Basic failure scenarios
// - Card tokenization
// ============================================================================

// ===== TEST CARD DATA =====

const successfulNo3DSCardDetails = {
  card_number: "4111111111111111", // Visa test card
  card_exp_month: "12",
  card_exp_year: "30",
  card_holder_name: "Test User",
  card_cvc: "123",
};

const successfulAmexCardDetails = {
  card_number: "378282246310005", // American Express test card
  card_exp_month: "12",
  card_exp_year: "30",
  card_holder_name: "Test User",
  card_cvc: "1234",
};

const failedCardDetails = {
  card_number: "4000000000000002", // Generic declined card
  card_exp_month: "12",
  card_exp_year: "30",
  card_holder_name: "Test User",
  card_cvc: "123",
};

// ===== BILLING INFORMATION =====

const billingAddress = {
  address: {
    line1: "1467 Harrison Street",
    line2: "Apt 12",
    city: "San Francisco",
    state: "California",
    zip: "94122",
    country: "US",
    first_name: "Test",
    last_name: "User",
  },
  phone: {
    number: "4155551234",
    country_code: "+1",
  },
};

// ===== PAYMENT METHOD DATA =====

const payment_method_data_visa = {
  card: {
    last4: "1111",
    card_type: "CREDIT",
    card_network: "Visa",
    card_issuer: "JP Morgan",
    card_issuing_country: "INDIA",
    card_isin: "411111",
    card_extended_bin: null,
    card_exp_month: "12",
    card_exp_year: "30",
    card_holder_name: "Test User",
    payment_checks: null,
    authentication_data: null,
  },
  billing: null,
};

const payment_method_data_amex = {
  card: {
    last4: "0005",
    card_type: "CREDIT",
    card_network: "AmericanExpress",
    card_issuer: null,
    card_issuing_country: null,
    card_isin: "378282",
    card_extended_bin: null,
    card_exp_month: "12",
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
    card_type: "CREDIT",
    card_network: "Visa",
    card_issuer: "INTL HDQTRS-CENTER OWNED",
    card_issuing_country: "UNITEDSTATES",
    card_isin: "400000",
    card_extended_bin: null,
    card_exp_month: "12",
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
              eligible_connectors: ["silverflow"],
            },
          ],
          required_fields: cardRequiredField,
        },
        {
          payment_method_type: "debit",
          card_networks: [
            {
              eligible_connectors: ["silverflow"],
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
          payment_method_data: payment_method_data_visa,
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
          payment_method_data: payment_method_data_visa,
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
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_code: null,
          error_message: null,
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

    // Void payment (cancel authorized payment)
    Void: {
      Request: {
        cancellation_reason: "requested_by_customer",
      },
      Response: {
        status: 200,
        body: {
          status: "cancelled",
          payment_method: null,
          attempt_count: 1,
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
          status: "succeeded",
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
          status: "succeeded",
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
          status: "succeeded",
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
          status: "succeeded",
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
          payment_method_data: payment_method_data_visa,
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
          payment_method_data: payment_method_data_visa,
          shipping_cost: 50,
          amount_received: 6050,
          amount: 6000,
          net_amount: 6050,
        },
      },
    },

    // American Express card test
    AmexCardPayment: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulAmexCardDetails,
        },
        billing: billingAddress,
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: null,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method: "card",
          attempt_count: 1,
          payment_method_data: payment_method_data_amex,
        },
      },
    },

    // ===== UNSUPPORTED FEATURES (MARKED AS TRIGGER_SKIP) =====
    // Silverflow doesn't support 3DS, mandates, save card, etc.=

    MandateSingleUseNo3DSAutoCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Mandates not supported
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: {
          customer_acceptance: customerAcceptance,
          mandate_type: {
            single_use: {
              amount: 8000,
              currency: "USD",
            },
          },
        },
      },
      Response: {
        status: 400,
        body: {},
      },
    }),

    MandateSingleUseNo3DSManualCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Mandates not supported
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: {
          customer_acceptance: customerAcceptance,
          mandate_type: {
            single_use: {
              amount: 8000,
              currency: "USD",
            },
          },
        },
        capture_method: "manual",
      },
      Response: {
        status: 400,
        body: {},
      },
    }),

    MandateMultiUseNo3DSAutoCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Mandates not supported
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: {
          customer_acceptance: customerAcceptance,
          mandate_type: {
            multi_use: {
              amount: 8000,
              currency: "USD",
            },
          },
        },
      },
      Response: {
        status: 400,
        body: {},
      },
    }),

    MandateMultiUseNo3DSManualCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Mandates not supported
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: {
          customer_acceptance: customerAcceptance,
          mandate_type: {
            multi_use: {
              amount: 8000,
              currency: "USD",
            },
          },
        },
        capture_method: "manual",
      },
      Response: {
        status: 400,
        body: {},
      },
    }),

    SaveCardUseNo3DSAutoCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Skip if Silverflow doesn't support save card
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
        TRIGGER_SKIP: true, // Skip if Silverflow doesn't support save card
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
        capture_method: "manual",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    }),

    // ===== 3DS SAVE CARD SCENARIOS =====
    // Note: Silverflow doesn't support 3DS or save card, marked as TRIGGER_SKIP

    SaveCardUse3DSAutoCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Skip - 3DS and save card not implemented
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
        TRIGGER_SKIP: true, // Skip - 3DS and save card not implemented
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
        capture_method: "manual",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),

    // ===== OFF-SESSION SAVE CARD SCENARIOS =====
    // Note: Silverflow doesn't support off-session or save card, marked as TRIGGER_SKIP

    SaveCardUseNo3DSAutoCaptureOffSession: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Skip if Silverflow doesn't support off-session save card
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
        TRIGGER_SKIP: true, // Skip - 3DS and off-session save card not supported
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
        TRIGGER_SKIP: true, // Skip if Silverflow doesn't support off-session save card
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
        capture_method: "manual",
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
        TRIGGER_SKIP: true, // Skip if Silverflow doesn't support off-session save card
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
        TRIGGER_SKIP: true, // Skip if Silverflow doesn't support off-session save card
      },
      Request: {
        setup_future_usage: "off_session",
        capture_method: "manual",
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
        TRIGGER_SKIP: true, // Skip if Silverflow doesn't support off-session save card
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

    ZeroAuthPaymentIntent: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Zero auth not supported
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
        TRIGGER_SKIP: true, // Zero auth not supported
      },
      Request: {
        payment_type: "setup_mandate",
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request_error",
            code: "feature_not_supported",
            message: "Zero auth is not supported",
          },
        },
      },
    }),

    ZeroAuthMandate: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Zero auth not supported
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: {
          customer_acceptance: customerAcceptance,
          mandate_type: {
            single_use: {
              amount: 8000,
              currency: "USD",
            },
          },
        },
      },
      Response: {
        status: 400,
        body: {},
      },
    }),

    MITAutoCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Skip if Silverflow doesn't support MIT
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
        TRIGGER_SKIP: true, // Skip if Silverflow doesn't support MIT
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

    // ===== 3DS MANDATE SCENARIOS =====
    // Note: Silverflow doesn't support 3DS or mandates, marked as TRIGGER_SKIP

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
        mandate_data: {
          customer_acceptance: customerAcceptance,
          mandate_type: {
            single_use: {
              amount: 8000,
              currency: "USD",
            },
          },
        },
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
        mandate_data: {
          customer_acceptance: customerAcceptance,
          mandate_type: {
            single_use: {
              amount: 8000,
              currency: "USD",
            },
          },
        },
        capture_method: "manual",
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
        mandate_data: {
          customer_acceptance: customerAcceptance,
          mandate_type: {
            multi_use: {
              amount: 8000,
              currency: "USD",
            },
          },
        },
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
        mandate_data: {
          customer_acceptance: customerAcceptance,
          mandate_type: {
            multi_use: {
              amount: 8000,
              currency: "USD",
            },
          },
        },
        capture_method: "manual",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),

    // ===== PAYMENT METHOD SCENARIOS =====
    // Note: These may not be supported by Silverflow, marked as TRIGGER_SKIP

    PaymentMethodIdMandateNo3DSAutoCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Skip if Silverflow doesn't support payment method ID mandates
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
        TRIGGER_SKIP: true, // Skip if Silverflow doesn't support payment method ID mandates
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
        capture_method: "manual",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    }),

    // ===== VOID SCENARIOS =====
    // Note: Adding void scenarios to match Authipay

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

    PaymentIntentOffSession: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Off-session not supported
      },
      Request: {
        currency: "USD",
        setup_future_usage: "off_session",
      },
      Response: {
        status: 200,
        body: {},
      },
    }),

    SessionToken: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Session tokens not supported
      },
      Response: {
        status: 200,
        body: {
          session_token: [],
        },
      },
    }),

    PaymentMethod: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Payment method creation not supported
      },
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        card: successfulNo3DSCardDetails,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request_error",
            code: "feature_not_supported",
            message: "Payment method creation is not supported",
          },
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
