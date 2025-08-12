import { cardRequiredField } from "./Commons";
import { getCustomExchange } from "./Modifiers";

// ============================================================================
// MPGS CYPRESS TEST CONFIGURATION
// ============================================================================
// Core functionality based on MPGS API specification:
// - Non-3DS card payments (auto/manual capture)
// - Refund operations (full/partial/sync)
// - Void operations
// - Basic failure scenarios
// - Multiple card networks (Mastercard, Visa, Diners, JCB, Discover)
// ============================================================================

// ===== MPGS TEST CARD DATA =====
// Using official MPGS test cards from their documentation

const successfulNo3DSCardDetails = {
  card_number: "4111111111111111", // Visa test card
  card_exp_month: "08",
  card_exp_year: "30",
  card_holder_name: "joseph Doe",
  card_cvc: "999",
};

const successfulVisaCardDetails = {
  card_number: "4508750015741019", // Visa test card
  card_exp_month: "01",
  card_exp_year: "39",
  card_holder_name: "Test User",
  card_cvc: "100",
};

const successfulDinersCardDetails = {
  card_number: "30123400000000", // Diners Club test card
  card_exp_month: "01",
  card_exp_year: "39",
  card_holder_name: "Test User",
  card_cvc: "100",
};

const successfulJCBCardDetails = {
  card_number: "3528000000000007", // JCB test card
  card_exp_month: "01",
  card_exp_year: "39",
  card_holder_name: "Test User",
  card_cvc: "100",
};

const successfulDiscoverCardDetails = {
  card_number: "6011003179988686", // Discover test card
  card_exp_month: "01",
  card_exp_year: "39",
  card_holder_name: "Test User",
  card_cvc: "100",
};

const failedCardDetails = {
  card_number: "4000000000000002", // Generic declined card
  card_exp_month: "05",
  card_exp_year: "39",
  card_holder_name: "Test User",
  card_cvc: "100",
};

const expiredCardDetails = {
  card_number: "5123450000000008", // Mastercard with expired date
  card_exp_month: "04",
  card_exp_year: "27",
  card_holder_name: "Test User",
  card_cvc: "100",
};

const insufficientFundsCardDetails = {
  card_number: "5123450000000008", // Mastercard with insufficient funds amount
  card_exp_month: "01",
  card_exp_year: "39",
  card_holder_name: "Test User",
  card_cvc: "100",
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

const payment_method_data_mastercard = {
  card: {
    last4: "1111",
    card_type: null,
    card_network: null,
    card_issuer: null,
    card_issuing_country: null,
    card_isin: "411111",
    card_extended_bin: null,
    card_exp_month: "08",
    card_exp_year: "30",
    card_holder_name: "joseph Doe",
    payment_checks: null,
    authentication_data: null,
  },
  billing: null,
};

const payment_method_data_visa = {
  card: {
    last4: "1019",
    card_type: "CREDIT",
    card_network: "Visa",
    card_issuer: "JPMORGAN CHASE BANK, N.A.",
    card_issuing_country: "UNITEDSTATES",
    card_isin: "450875",
    card_extended_bin: null,
    card_exp_month: "01",
    card_exp_year: "39",
    card_holder_name: "Test User",
    payment_checks: null,
    authentication_data: null,
  },
  billing: null,
};

const payment_method_data_diners = {
  card: {
    last4: "0000",
    card_type: "CREDIT",
    card_network: "DinersClub",
    card_issuer: null,
    card_issuing_country: null,
    card_isin: "301234",
    card_extended_bin: null,
    card_exp_month: "01",
    card_exp_year: "39",
    card_holder_name: "Test User",
    payment_checks: null,
    authentication_data: null,
  },
  billing: null,
};

const payment_method_data_jcb = {
  card: {
    last4: "0007",
    card_type: "CREDIT",
    card_network: "JCB",
    card_issuer: null,
    card_issuing_country: null,
    card_isin: "352800",
    card_extended_bin: null,
    card_exp_month: "01",
    card_exp_year: "39",
    card_holder_name: "Test User",
    payment_checks: null,
    authentication_data: null,
  },
  billing: null,
};

const payment_method_data_discover = {
  card: {
    last4: "8686",
    card_type: "CREDIT",
    card_network: "Discover",
    card_issuer: null,
    card_issuing_country: null,
    card_isin: "601100",
    card_extended_bin: null,
    card_exp_month: "01",
    card_exp_year: "39",
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
    card_exp_month: "05",
    card_exp_year: "39",
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
              eligible_connectors: ["mpgs"],
            },
          ],
          required_fields: cardRequiredField,
        },
        {
          payment_method_type: "debit",
          card_networks: [
            {
              eligible_connectors: ["mpgs"],
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
  authDetails: {
    connector_name: "mpgs",
    profile_id: "{{profile_id}}",
    connector_account_details: {
      auth_type: "BodyKey",
      api_key: "mpgs-secret",
      key1: "TESTMPGS",
    },
    test_mode: true,
    disabled: false,
    payment_methods_enabled: [
      {
        payment_method: "card",
        payment_method_types: [
          {
            payment_method_type: "credit",
            card_networks: [
              "AmericanExpress",
              "Discover",
              "JCB",
              "Mastercard",
              "Visa",
              "DinersClub",
            ],
            minimum_amount: 1,
            maximum_amount: 68607706,
            recurring_enabled: false,
            installment_payment_enabled: false,
          },
          {
            payment_method_type: "debit",
            card_networks: [
              "AmericanExpress",
              "Discover",
              "JCB",
              "Mastercard",
              "Visa",
              "DinersClub",
            ],
            minimum_amount: 1,
            maximum_amount: 68607706,
            recurring_enabled: false,
            installment_payment_enabled: false,
          },
        ],
      },
    ],
    metadata: {
      city: "NY",
      unit: "245",
      endpoint_prefix: "AD",
      merchant_name: "Cypress Test MPGS",
      account_name: "transaction_processing",
    },
  },
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
          payment_method_data: payment_method_data_mastercard,
          error_code: null,
          error: null,
          error_message: null,
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
          payment_method_data: payment_method_data_mastercard,
          error_code: null,
          error_message: null,
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
          error_code: null,
          error_message: null,
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
          error_code: null,
          error_message: null,
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
          error_code: null,
          error_message: null,
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
      Request: {},
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
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method: "card",
          payment_method_data: payment_method_data_mastercard,
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
          payment_method_data: payment_method_data_mastercard,
          shipping_cost: 50,
          amount_received: 6050,
          amount: 6000,
          net_amount: 6050,
          error_code: null,
          error_message: null,
        },
      },
    },

    // Visa card test
    VisaCardPayment: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulVisaCardDetails,
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
          payment_method_data: payment_method_data_visa,
        },
      },
    },

    // Diners Club card test
    DinersCardPayment: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulDinersCardDetails,
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
          payment_method_data: payment_method_data_diners,
        },
      },
    },

    // JCB card test
    JCBCardPayment: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulJCBCardDetails,
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
          payment_method_data: payment_method_data_jcb,
        },
      },
    },

    // Discover card test
    DiscoverCardPayment: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulDiscoverCardDetails,
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
          payment_method_data: payment_method_data_discover,
        },
      },
    },

    // Expired card test
    ExpiredCardPayment: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: expiredCardDetails,
        },
        billing: billingAddress,
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: null,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_code: "expired_card",
          error_message: "Your card has expired.",
          payment_method: "card",
          attempt_count: 1,
          payment_method_data: payment_method_data_mastercard,
        },
      },
    },

    // Insufficient funds test
    InsufficientFundsPayment: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: insufficientFundsCardDetails,
        },
        billing: billingAddress,
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: null,
        amount: 120, // Special amount for insufficient funds (1.20)
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_code: "insufficient_funds",
          error_message: "Your card has insufficient funds.",
          payment_method: "card",
          attempt_count: 1,
          payment_method_data: payment_method_data_mastercard,
        },
      },
    },

    // ===== VOID SCENARIOS =====

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

    // ===== 3DS SCENARIOS (NOT SUPPORTED BY MPGS) =====
    // MPGS doesn't support 3DS, so these return "not implemented" errors
    
    "3DSAutoCapture": getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // 3DS not supported
      },
    }),

    "3DSManualCapture": getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // 3DS not supported  
      },
    }),

    // ===== UNSUPPORTED FEATURES (MARKED AS TRIGGER_SKIP) =====
    // MPGS doesn't support 3DS, mandates, save card, etc.
    // These are marked with TRIGGER_SKIP so tests will be skipped

    MandateSingleUseNo3DSManualCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Mandates not supported
      },
    }),

    MandateMultiUseNo3DSAutoCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Mandates not supported
      },
    }),

    MandateMultiUseNo3DSManualCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Mandates not supported
      },
    }),

    SaveCardUseNo3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true, // Save card not supported
      },
    },

    SaveCardUseNo3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true, // Save card not supported
      },
    },

    SaveCardUse3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true, // 3DS and save card not supported
      },
    },

    SaveCardUse3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true, // 3DS and save card not supported
      },
    },

    SaveCardUseNo3DSAutoCaptureOffSession: {
      Configs: {
        TRIGGER_SKIP: true, // Off-session save card not supported
      },
    },

    SaveCardUse3DSAutoCaptureOffSession: {
      Configs: {
        TRIGGER_SKIP: true, // 3DS and off-session save card not supported
      },
    },

    SaveCardUseNo3DSManualCaptureOffSession: {
      Configs: {
        TRIGGER_SKIP: true, // Off-session save card not supported
      },
    },

    SaveCardConfirmAutoCaptureOffSession: {
      Configs: {
        TRIGGER_SKIP: true, // Off-session save card not supported
      },
    },

    SaveCardConfirmManualCaptureOffSession: {
      Configs: {
        TRIGGER_SKIP: true, // Off-session save card not supported
      },
    },

    SaveCardConfirmAutoCaptureOffSessionWithoutBilling: {
      Configs: {
        TRIGGER_SKIP: true, // Off-session save card not supported
      },
    },

    ZeroAuthPaymentIntent: {
      Configs: {
        TRIGGER_SKIP: true, // Zero auth not supported
      },
    },

    ZeroAuthConfirmPayment: {
      Configs: {
        TRIGGER_SKIP: true, // Zero auth not supported
      },
    },

    ZeroAuthMandate: {
      Configs: {
        TRIGGER_SKIP: true, // Zero auth not supported
      },
    },

    MITAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true, // MIT not supported
      },
    },

    MITWithoutBillingAddress: {
      Configs: {
        TRIGGER_SKIP: true, // MIT not supported
      },
    },

    MandateSingleUse3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true, // 3DS not supported
      },
    },

    MandateSingleUse3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true, // 3DS not supported
      },
    },

    MandateMultiUse3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true, // 3DS not supported
      },
    },

    MandateMultiUse3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true, // 3DS not supported
      },
    },

    PaymentMethodIdMandateNo3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true, // Payment method ID mandates not supported
      },
    },

    PaymentMethodIdMandateNo3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true, // Payment method ID mandates not supported
      },
    },

    PaymentIntentOffSession: {
      Configs: {
        TRIGGER_SKIP: true, // Off-session not supported
      },
    },

    SessionToken: {
      Configs: {
        TRIGGER_SKIP: true, // Session tokens not supported
      },
    },

    PaymentMethod: {
      Configs: {
        TRIGGER_SKIP: true, // Payment method creation not supported
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
