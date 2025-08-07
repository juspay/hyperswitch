import { cardRequiredField, customerAcceptance } from "./Commons";
import { getCustomExchange } from "./Modifiers";

// Test card details for successful non-3DS transactions
// Based on Global Payments test cards
const successfulNo3DSCardDetails = {
  card_number: "4111111111111111",
  card_exp_month: "12",
  card_exp_year: "2030",
  card_holder_name: "joseph Doe",
  card_cvc: "123",
};

// Test card details for 3DS transactions
const successfulThreeDSTestCardDetails = {
  ...successfulNo3DSCardDetails,
  card_number: "4012000000061026", // Global Payments approved 3DS v2.0 test card with Challenge flow
};

// Test card details for failed transactions
const failedNo3DSCardDetails = {
  ...successfulNo3DSCardDetails,
  card_number: "4242420000000091", // Generic decline card
};

// Single-use mandate data structure
const singleUseMandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    single_use: {
      amount: 8000,
      currency: "EUR",
    },
  },
};

// Multi-use mandate data structure
const multiUseMandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    multi_use: {
      amount: 8000,
      currency: "EUR",
    },
  },
};

// Payment method data for non-3DS card transactions
const payment_method_data_no3ds = {
  card: {
    last4: "1111",
    card_type: "CREDIT",
    card_network: "Visa",
    card_issuer: "JP Morgan",
    card_issuing_country: "INDIA",
    card_isin: "411111",
    card_extended_bin: null,
    card_exp_month: "12",
    card_exp_year: "2030",
    card_holder_name: "joseph Doe",
    payment_checks: null,
    authentication_data: null,
  },
  billing: null,
};

// Required fields for payment methods
const requiredFields = {
  payment_methods: [
    {
      payment_method: "card",
      payment_method_types: [
        {
          payment_method_type: "credit",
          card_networks: [
            {
              eligible_connectors: ["globalpay"],
            },
          ],
          required_fields: cardRequiredField,
        },
        {
          payment_method_type: "debit",
          card_networks: [
            {
              eligible_connectors: ["globalpay"],
            },
          ],
          required_fields: cardRequiredField,
        },
      ],
    },
  ],
};

// Billing address for European payments
const billingAddressEurope = {
  address: {
    line1: "123 Payment Street",
    line2: "Apt 456",
    line3: "District 7",
    city: "Amsterdam",
    state: "North Holland",
    zip: "1011",
    country: "NL",
    first_name: "John",
    last_name: "Doe",
  },
  phone: {
    number: "1234567890",
    country_code: "+31",
  },
  email: "john.doe@example.com",
};

// Main connector details export
export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "EUR",
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
        currency: "EUR",
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
    SessionToken: {
      Response: {
        status: 200,
        body: {
          session_token: [],
        },
      },
    },
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
    // No3DS flows
    No3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "EUR",
        customer_acceptance: null,
        setup_future_usage: "on_session",
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
    No3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "EUR",
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
          error_code: "111",
          error_message:
            "(111)[ test system ] Strong Customer Authentication Required",
          unified_code: "UE_9000",
          unified_message: "Something went wrong",
        },
      },
    },
    "3DSManualCapture": {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "EUR",
        customer_acceptance: null,
        authentication_type: "three_ds",
        setup_future_usage: "on_session",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
            reason: "3DS flow is not supported by Globalpay",
          },
        },
      },
    },
    "3DSAutoCapture": {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "EUR",
        customer_acceptance: null,
        authentication_type: "three_ds",
        setup_future_usage: "on_session",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
            reason: "3DS flow is not supported by Globalpay",
          },
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
      ResponseCustom: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "You cannot cancel this payment because it has status succeeded",
            code: "IR_16",
          },
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
    // Mandate flows
    MandateSingleUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "EUR",
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
        currency: "EUR",
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
        currency: "EUR",
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
        currency: "EUR",
        mandate_data: multiUseMandateData,
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
        currency: "EUR",
        mandate_data: singleUseMandateData,
        authentication_type: "three_ds",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
            reason: "3DS flow is not supported by Globalpay",
          },
        },
      },
    },
    MandateSingleUse3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "EUR",
        mandate_data: singleUseMandateData,
        authentication_type: "three_ds",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
            reason: "3DS flow is not supported by Globalpay",
          },
        },
      },
    },
    MandateMultiUse3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "EUR",
        mandate_data: multiUseMandateData,
        authentication_type: "three_ds",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
            reason: "3DS flow is not supported by Globalpay",
          },
        },
      },
    },
    MandateMultiUse3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "EUR",
        mandate_data: multiUseMandateData,
        authentication_type: "three_ds",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
            reason: "3DS flow is not supported by Globalpay",
          },
        },
      },
    },
    MITAutoCapture: {
      Request: {
        currency: "EUR",
        billing: billingAddressEurope,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    MITManualCapture: {
      Request: {
        currency: "EUR",
        billing: billingAddressEurope,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
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
        currency: "EUR",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 501,
        body: {
          code: "IR_00",
          message: "Setup Mandate flow for Globalpay is not implemented",
          type: "invalid_request",
        },
      },
    },
    ZeroAuthPaymentIntent: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        amount: 0,
        setup_future_usage: "off_session",
        currency: "EUR",
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
        setup_future_usage: "off_session",
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
    // Saved card flows
    SaveCardUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "EUR",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          setup_future_usage: "on_session",
        },
      },
    },
    SaveCardUseNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "EUR",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          setup_future_usage: "on_session",
        },
      },
    },
    SaveCardUse3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "EUR",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
        authentication_type: "three_ds",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
            reason: "3DS flow is not supported by Globalpay",
          },
        },
      },
    },
    SaveCardUse3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "EUR",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
        authentication_type: "three_ds",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
            reason: "3DS flow is not supported by Globalpay",
          },
        },
      },
    },
    PaymentMethodIdMandateNo3DSAutoCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "EUR",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          setup_future_usage: "off_session",
        },
      },
    }),
    PaymentMethodIdMandateNo3DSManualCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "EUR",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          setup_future_usage: "off_session",
        },
      },
    }),
    SaveCardUseNo3DSAutoCaptureOffSession: {
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "EUR",
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          setup_future_usage: "off_session",
        },
      },
    },
    SaveCardUseNo3DSManualCaptureOffSession: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "EUR",
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
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
          setup_future_usage: "off_session",
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
          setup_future_usage: "off_session",
        },
      },
    },
    SaveCardConfirmAutoCaptureOffSessionWithoutBilling: {
      Request: {
        setup_future_usage: "off_session",
        billing: null,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          setup_future_usage: "off_session",
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
