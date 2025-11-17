import { customerAcceptance, cardRequiredField } from "./Commons";
import { getCustomExchange } from "./Modifiers";

// Card details for non-3DS payment
const successfulNo3DSCardDetails = {
  card_number: "4111111111111111", // Visa test card
  card_exp_month: "10",
  card_exp_year: "2050",
  card_holder_name: "Test User",
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
// Finix doesn't support 3DS, but keeping this for common pattern
const failedNo3DSCardDetails = {
  card_number: "4000000000000002", // Failed card
  card_exp_month: "01",
  card_exp_year: "2035",
  card_holder_name: "Test User",
  card_cvc: "123",
};

// Payment method data for non-3DS
const payment_method_data_no3ds = {
  card: {
    last4: "1111",
    card_type: "CREDIT",
    card_network: "Visa",
    card_issuer: "JP Morgan",
    card_issuing_country: "INDIA",
    card_isin: "411111",
    card_extended_bin: null,
    card_exp_month: "10",
    card_exp_year: "2050",
    card_holder_name: "Test User",
    payment_checks: null,
    authentication_data: null,
  },
  billing: null,
};

const requiredFields = {
  payment_methods: [
    {
      payment_method: "card",
      payment_method_types: [
        {
          payment_method_type: "credit",
          card_networks: [
            {
              eligible_connectors: ["finix"],
            },
          ],
          required_fields: cardRequiredField,
        },
      ],
    },
  ],
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
    PaymentIntentWithShippingCost: {
      Request: {
        currency: "USD",
        amount: 11500,
        shipping_cost: 50,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          shipping_cost: 50,
          amount: 11500,
        },
      },
    },
    // Payment confirmation with shipping cost
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
        },
      },
    },
    SessionToken: {
      Response: {
        status: 200,
        body: {
          session_token: [],
        },
      },
    },
    // Non-3DS Manual Capture - Card is authorized but requires explicit capture
    No3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        capture_method: "manual",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          payment_method: "card",
          attempt_count: 1,
          payment_method_data: payment_method_data_no3ds,
        },
      },
    },
    // Non-3DS Auto Capture - Card is authorized and captured automatically
    "3DSAutoCapture": {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card", // not implemented
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
      },
      Response: {
        status: 401,
        body: {
          status: "requires_customer_action",
        },
      },
    },

    MandateSingleUseNo3DSAutoCapture: getCustomExchange({
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
    MITManualCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        currency: "USD",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    MITAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        currency: "USD",
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
    SaveCardUseNo3DSAutoCaptureOffSession: getCustomExchange({
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
    }),
    SaveCardConfirmAutoCaptureOffSession: {
      Configs: {},
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
    SaveCardUseNo3DSManualCapture: getCustomExchange({
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
          status: "requires_capture",
        },
      },
    }),
    ZeroAuthPaymentIntent: {
      Configs: {
        TRIGGER_SKIP: true, // Skip if Authipay doesn't support save card
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
    ZeroAuthMandate: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 401,
        body: {
          status: "succeeded",
        },
      },
    }),
    "3DSManualCapture": {
      Configs: {
        TRIGGER_SKIP: true, // not implemented
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
      },
      Response: {
        status: 401,
        body: {
          status: "requires_customer_action",
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
          payment_method: "card",
          attempt_count: 1,
          payment_method_data: payment_method_data_no3ds,
        },
      },
    },
    // Failed Non-3DS payment
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
          error_code: "DO_NOT_HONOR",
          error_message:
            "The card was declined for an unknown reason. The cardholder needs to contact their issuing bank for more information.",
          unified_code: "UE_9000",
          unified_message: "Something went wrong",
        },
      },
    },
    // Capture a previously authorized payment
    Capture: {
      Request: {
        amount_to_capture: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "processing", // capture takes 1-2 min to process
          amount: 6000,
          amount_capturable: 6000,
        },
      },
    },
    // Partial capture of an authorized payment
    PartialCapture: {
      Request: {
        amount_to_capture: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
          amount: 6000,
        },
      },
    },
    // Void (cancel) an authorized payment
    Void: {
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "cancelled",
        },
      },
    },
    // Full refund of a captured payment
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
    // Partial refund of a captured payment
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
    manualPaymentRefund: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        amount: 6000,
      },
      Response: {
        status: 400,

        body: {
          type: "invalid_request",
          message:
            "This Payment could not be refund because it has a status of processing. The expected state is succeeded, partially_captured",
          code: "IR_14",
        },
      },
    },
    PaymentIntentOffSession: {
      Configs: {
        CONNECTOR_CREDENTIAL: {
          specName: ["connectorAgnosticNTID"],
          value: "connector_2",
        },
      },
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
    PaymentMethodIdMandateNo3DSAutoCapture: {
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
          type: "invalid_request",
          message:
            "This Payment could not be refund because it has a status of processing. The expected state is succeeded, partially_captured",
          code: "IR_14",
        },
      },
    },
    // Sync refund status (check refund status)
    SyncRefund: {
      Response: {
        status: 200,
        body: {
          status: "pending",
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
