import { cardRequiredField } from "./Commons";

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
  card_exp_month: "01",
  card_exp_year: "35",
  card_holder_name: "Test User",
  card_cvc: "123",
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
        {
          payment_method_type: "debit",
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
        currency: "EUR",
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
        currency: "EUR",
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
        currency: "EUR",
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
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_code: "card_declined",
          error_message: "Your card was declined.",
          unified_code: "UE_9000",
          unified_message: "Something went wrong",
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

    // Enhanced void payment (cancel authorized payment) - using order_id
    VoidPayment: {
      Request: {
        cancellation_reason: "customer_request",
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
          payment_method: "card",
          attempt_count: 1,
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
          status: "processing",
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

    // Enhanced void after auto capture - should fail gracefully
    VoidAfterAutoCapture: {
      Request: {
        cancellation_reason: "customer_request",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request_error",
            code: "IR_16",
            message:
              "You cannot cancel this payment because it has status succeeded.",
          },
        },
      },
    },

    // New: Void with missing identifiers (should fail gracefully)
    VoidMissingIdentifiers: {
      Request: {
        cancellation_reason: "test_scenario",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request_error",
            code: "request_encoding_failed",
            message: "Missing required identifiers for void operation",
          },
        },
      },
    },

    // New: Void with invalid order_id format (should use fallback)
    VoidInvalidOrderId: {
      Request: {
        cancellation_reason: "test_fallback",
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
        currency: "EUR",
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
