import { customerAcceptance,cardRequiredField } from "./Commons.js";
import { getCustomExchange } from "./Modifiers.js";

const defaultBillingDetails = {
          address: {
            line1: "Calle 123",
            line2: "Apt 1",
            line3:null,
            city: "Bogotá",
            state: "Cundinamarca",
            zip: "110111",
            country: "CO",
            first_name: "John",
            last_name: "Doe",
          },
          phone: {
            number: "3001234567",
            country_code: "+57",
          },
          email: "john.doe@example.com",
        }
    
// Test card details for Placetopay - using standard test cards
const successfulNo3DSCardDetails = {
  card_number: "4111111111111111", // Visa test card
  card_exp_month: "12",
  card_exp_year: "30",
  card_holder_name: "Test User",
  card_cvc: "123",
};

const successfulMastercardDetails = {
  card_number: "5555555555554444", // Mastercard test card
  card_exp_month: "12",
  card_exp_year: "30",
  card_holder_name: "Test User",
  card_cvc: "123",
};

const successfulAmexDetails = {
  card_number: "378282246310005", // American Express test card
  card_exp_month: "12",
  card_exp_year: "30",
  card_holder_name: "Test User",
  card_cvc: "1234",
};

const payment_method_data_amex = {
  card: {
    last4: "0005",
    card_type: "CREDIT",
    card_network: "AmericanExpress",
    card_issuer: "Test Bank",
    card_issuing_country: "US",
    card_isin: "378282",
    card_extended_bin: null,
    card_exp_month: "12",
    card_exp_year: "30",
    card_holder_name: "Test User",
    payment_checks: null,
    authentication_data: null,
  },
};

const failedCardDetails = {
  card_number: "4000000000000002", // Standard decline test card
  card_exp_month: "12",
  card_exp_year: "30",
  card_holder_name: "Test User",
  card_cvc: "123",
};

// Payment method data for responses
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

const payment_method_data_mastercard = {
  card: {
    last4: "4444",
    card_type: "CREDIT",
    card_network: "Mastercard",
    card_issuer: "Test Bank",
    card_issuing_country: "US",
    card_isin: "555555",
    card_extended_bin: null,
    card_exp_month: "12",
    card_exp_year: "30",
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
              eligible_connectors: ["placetopay"],
            },
          ],
          required_fields: cardRequiredField,
        },
      ],
    },
  ],
};

export const connectorDetails = {
  authDetails: {
    placetopay: {
      connector_account_details: {
        auth_type: "BodyKey",
        api_key: "e2ffa4ba4e5538a5ff78bed4b156531e",
        key1: "B383FT602fEXF49x"
      }
    }
  },
  card_pm: {
    PaymentIntent: getCustomExchange({
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
    }),

    PaymentIntentWithShippingCost: getCustomExchange({
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
    }),

    PaymentConfirmWithShippingCost: getCustomExchange({
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
          payment_method: "card",
          payment_method_data: payment_method_data_visa,
          shipping_cost: 50,
          amount: 6000,
          amount_received: 6050,
          net_amount: 6050,
        },
      },
    }),

    // No 3DS Auto Capture - Placetopay doesn't support 3DS
    No3DSAutoCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        billing: defaultBillingDetails,
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method: "card",
          payment_method_data: payment_method_data_visa,
        }
      },
    }),

    // No 3DS Manual Capture - NOT supported by Placetopay
    No3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true, // Skip manual capture tests as Placetopay doesn't support manual capture
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        billing: {
          address: {
            line1: "Calle 123",
            line2: "Apt 1",
            city: "Bogotá",
            state: "Cundinamarca",
            zip: "110111",
            country: "CO",
            first_name: "John",
            last_name: "Doe",
          },
          phone: {
            number: "3001234567",
            country_code: "+57",
          },
          email: "john.doe@example.com",
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
          payment_method_data: payment_method_data_visa,
        },
      },
    },

    // Mastercard test
    MastercardAutoCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulMastercardDetails,
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
          payment_method_data: payment_method_data_mastercard,
        },
      },
    }),

    // American Express test
    AmexAutoCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulAmexDetails,
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
          payment_method_data: payment_method_data_amex,
        },
      },
    }),

    // Failed payment test
    No3DSFailPayment: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: failedCardDetails,
        },
        billing: defaultBillingDetails,
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
        },
      },
    }),

    // Capture flow
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
          billing : defaultBillingDetails,
        },
      },
    },

    // Partial Capture - Placetopay supports this
    PartialCapture:{
      Request: {
        amount_to_capture: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "partially_captured",
          amount: 6000,
          amount_capturable: 4000,
          amount_received: 2000,
        },
      },
    },

    // Void payment
    Void: getCustomExchange({
      Request: {
        cancellation_reason: "requested_by_customer",
      },
      Response: {
        status: 200,
        body: {
          status: "cancelled",
        },
      },
    }),

    VoidAfterConfirm: getCustomExchange({
      Request: {
        cancellation_reason: "requested_by_customer",
      },
      Response: {
        status: 200,
        body: {
          status: "cancelled",
        },
      },
    }),

    // Refund - Placetopay supports full refunds only
    Refund: getCustomExchange({
      Request: {
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),

    // Partial Refund - NOT supported by Placetopay, should fail
    PartialRefund: {
      Configs: {
        TRIGGER_SKIP: true, // Skip this test as Placetopay doesn't support partial refunds
      },
      Request: {
        amount: 2000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Partial Refund",
            code: "IR_00",
          },
        },
      },
    },

    // Sync Refund
    SyncRefund: getCustomExchange({
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),

    // Manual payment refund
    manualPaymentRefund: getCustomExchange({
      Request: {
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),

    // Manual payment partial refund - NOT supported
    manualPaymentPartialRefund: {
      Configs: {
        TRIGGER_SKIP: true, // Skip this test as Placetopay doesn't support partial refunds
      },
      Request: {
        amount: 2000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Partial Refund",
            code: "IR_00",
          },
        },
      },
    },

    // Sync Payment
    SyncPayment: getCustomExchange({
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method: "card",
          payment_method_data: payment_method_data_visa,
        },
      },
    }),

    // 3DS flows - NOT supported by Placetopay, should be skipped
    "3DSAutoCapture": {
      Configs: {
        TRIGGER_SKIP: true, // Skip 3DS tests as Placetopay doesn't support 3DS
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
        status: 400,
        body: {
          status: "NotSupported",
        },
      },
    },

    "3DSManualCapture": {
      Configs: {
        TRIGGER_SKIP: true, // Skip 3DS tests as Placetopay doesn't support 3DS
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        billing: defaultBillingDetails,
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "3DS authentication is not supported",
            code: "IR_00",
          },
        },
      },
    },

    // Mandate flows - NOT supported by Placetopay, should be skipped
    MandateSingleUseNo3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true, // Skip mandate tests as Placetopay doesn't support mandates
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        billing:defaultBillingDetails,
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
          status: "succeeded",
        },
      },
    },

    MandateSingleUseNo3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true, // Skip mandate tests
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        billing: defaultBillingDetails,
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
          status: "succeeded",
        },
      },
    },

    MandateMultiUseNo3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true, // Skip mandate tests
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        billing: defaultBillingDetails,
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
          status:"succeeded"
        },
      },
    },

    MandateMultiUseNo3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true, // Skip mandate tests
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
        body: {
          error: {
            type: "invalid_request",
            message: "Setup Mandate flow for Placetopay is not implemented",
            code: "IR_00",
          },
        },
      },
    },

    MandateMultiUse3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true, // Skip mandate tests
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
        body: {
          error: {
            type: "invalid_request",
            message: "Setup Mandate flow for Placetopay is not implemented",
            code: "IR_00",
          },
        },
      },
    },

    MandateMultiUse3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true, // Skip mandate tests
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
        body: {
          error: {
            type: "invalid_request",
            message: "Setup Mandate flow for Placetopay is not implemented",
            code: "IR_00",
          },
        },
      },
    },

    // Save card flows - NOT supported by Placetopay
    SaveCardUseNo3DSAutoCapture: {
      // Configs: {
      //   TRIGGER_SKIP: true, // Skip save card tests as Placetopay doesn't support tokenization
      // },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        billing: defaultBillingDetails,
        currency: "USD",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          billing: defaultBillingDetails,
        },
      },
    },

    SaveCardUseNo3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true, // Skip save card tests
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        billing: defaultBillingDetails,
        currency: "USD",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          billing: defaultBillingDetails,
        },
      },
    },

    SaveCardUseNo3DSAutoCaptureOffSession: {
      Configs: {
        TRIGGER_SKIP: true, // Skip save card tests
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        billing: defaultBillingDetails,
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          billing: defaultBillingDetails,
        },
      },
    },

    SaveCardUse3DSAutoCaptureOffSession: {
      Configs: {
        TRIGGER_SKIP: true, // Skip save card tests
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
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Card tokenization is not supported",
            code: "IR_00",
          },
        },
      },
    },

    SaveCardUseNo3DSManualCaptureOffSession: {
      Configs: {
        TRIGGER_SKIP: true, // Skip save card tests
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
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Card tokenization is not supported",
            code: "IR_00",
          },
        },
      },
    },

    SaveCardConfirmAutoCaptureOffSession: {
      Configs: {
        TRIGGER_SKIP: true, // Skip save card tests
      },
      Request: {
        setup_future_usage: "off_session",
        billing: defaultBillingDetails,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Card tokenization is not supported",
            code: "IR_00",
          },
        },
      },
    },

    SaveCardConfirmManualCaptureOffSession: {
      Configs: {
        TRIGGER_SKIP: true, // Skip save card tests
      },
      Request: {
        setup_future_usage: "off_session",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Card tokenization is not supported",
            code: "IR_00",
          },
        },
      },
    },

    SaveCardConfirmAutoCaptureOffSessionWithoutBilling: {
      Configs: {
        TRIGGER_SKIP: true, // Skip save card tests
      },
      Request: {
        setup_future_usage: "off_session",
        billing: null,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Card tokenization is not supported",
            code: "IR_00",
          },
        },
      },
    },

    // Zero Auth flows - NOT supported by Placetopay
    ZeroAuthMandate: {
      Configs: {
        TRIGGER_SKIP: true, // Skip zero auth tests
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        billing: defaultBillingDetails,
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
        body: {
          error: {
            type: "invalid_request",
            message: "Setup Mandate flow for Placetopay is not implemented",
            code: "IR_00",
          },
        },
      },
    },

    ZeroAuthPaymentIntent: {
      Configs: {
        TRIGGER_SKIP: true, // Skip zero auth tests
      },
      Request: {
        currency: "USD",
        amount: 0,
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
        TRIGGER_SKIP: true, // Skip zero auth tests
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
            type: "invalid_request",
            message: "Setup Mandate flow for Placetopay is not implemented",
            code: "IR_00",
          },
        },
      },
    },

    // MIT flows - NOT supported by Placetopay
    MITAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true, // Skip MIT tests
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "MIT payments are not supported",
            code: "IR_00",
          },
        },
      },
    },

    MITManualCapture: {
      Configs: {
        TRIGGER_SKIP: true, // Skip MIT tests
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "MIT payments are not supported",
            code: "IR_00",
          },
        },
      },
    },

    MITWithoutBillingAddress: {
      Configs: {
        TRIGGER_SKIP: true, // Skip MIT tests
      },
      Request: {
        billing: null,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "MIT payments are not supported",
            code: "IR_00",
          },
        },
      },
    },

    // Payment Method ID flows - NOT supported by Placetopay
    PaymentMethodIdMandateNo3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true, // Skip payment method ID tests
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        billing: defaultBillingDetails,
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
        TRIGGER_SKIP: true, // Skip payment method ID tests
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
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method tokenization is not supported",
            code: "IR_00",
          },
        },
      },
    },

    PaymentMethodIdMandate3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true, // Skip payment method ID tests
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        billing: defaultBillingDetails,
        currency: "USD",
        mandate_data: null,
        authentication_type: "three_ds",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method tokenization is not supported",
            code: "IR_00",
          },
        },
      },
    },

    PaymentMethodIdMandate3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true, // Skip payment method ID tests
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
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method tokenization is not supported",
            code: "IR_00",
          },
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
