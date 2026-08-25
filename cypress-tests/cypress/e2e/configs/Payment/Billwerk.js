import { cardRequiredField, customerAcceptance } from "./Commons";
import { getCustomExchange } from "./Modifiers";

// Test card details for successful non-3DS transactions
const successfulNo3DSCardDetails = {
  card_number: "340000000000009",
  card_exp_month: "12",
  card_exp_year: "2099",
  card_holder_name: "joseph Doe",
  card_cvc: "0009",
};

const blockedPaymentErrorBodyForIssuingCountry = {
  status: 200,
  expectBlockedPayment: true,
  body: {
    error: {
      type: "blocked",
      message:
        "Cards issued in your region aren't supported for this transaction, please try a different card",
      code: "HE_03",
      reason: "Blocked",
    },
  },
};
export const blockedPaymentErrorBodyForDebitCard = {
  status: 200,
  expectBlockedPayment: true,
  body: {
    error: {
      type: "blocked",
      message:
        "Debit cards are not accepted for this transaction, please try a different card",
      code: "HE_03",
      reason: "Blocked",
    },
  },
};

export const blockedPaymentErrorBodyForCardSubtype = {
  status: 200,
  expectBlockedPayment: true,
  body: {
    error: {
      type: "blocked",
      message:
        "This card is not accepted for this transaction, please try a different card",
      code: "HE_03",
      reason: "Blocked",
    },
  },
};

export const blockedPaymentErrorBodyForBinUnavailable = {
  status: 200,
  expectBlockedPayment: true,
  body: {
    error: {
      type: "blocked",
      message:
        "We couldn't verify this card's information, please try a different card",
      code: "HE_03",
      reason: "Blocked",
    },
  },
};

// Payment method card details
const PaymentMethodCardDetails = {
  card_number: "4111111145551142",
  card_exp_month: "03",
  card_exp_year: "30",
  card_holder_name: "Joseph Doe",
};

// Test card details for successful 3DS transactions
const successfulThreeDSTestCardDetails = {
  card_number: "4000002500003155",
  card_exp_month: "10",
  card_exp_year: "2030",
  card_holder_name: "joseph Doe",
  card_cvc: "123",
};

// Test card details for failed transactions
const failedNo3DSCardDetails = {
  card_number: "340000000000009",
  card_exp_month: "12",
  card_exp_year: "2030",
  card_holder_name: "Test User",
  card_cvc: "100",
};

// Payment method data structure for non-3DS card payments
const payment_method_data_no3ds = {
  card: {
    last4: "0009",
    card_type: "CREDIT",
    card_network: "AmericanExpress",
    card_issuer: "AMERICAN EXPRESS COMPANY",
    card_issuing_country: "INDIA",
    card_isin: "340000",
    card_extended_bin: null,
    card_exp_month: "12",
    card_exp_year: "2099",
    card_holder_name: "joseph Doe",
    payment_checks: null,
    authentication_data: null,
    auth_code: null,
  },
  billing: null,
};

// Required fields for card payments
const requiredFields = {
  payment_methods: [
    {
      payment_method: "card",
      payment_method_types: [
        {
          payment_method_type: "credit",
          card_networks: [
            {
              eligible_connectors: ["billwerk"],
            },
          ],
          required_fields: cardRequiredField,
        },
      ],
    },
  ],
};

// Mandate data structures (for reference - will be skipped)
const singleUseMandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    single_use: {
      amount: 8000,
      currency: "DKK",
    },
  },
};

const multiUseMandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    multi_use: {
      amount: 8000,
      currency: "DKK",
    },
  },
};

// Standard billing address for billwerk payments
const billingAddress = {
  address: {
    line1: "123 Payment Street",
    line2: "Apt 456",
    line3: "District 7",
    city: "Copenhagen",
    state: "Capital Region",
    zip: "1050",
    country: "DK", // Using Denmark (DK) as it's supported by billwerk
    first_name: "Test",
    last_name: "User",
  },
  phone: {
    number: "12345678",
    country_code: "+45", // Denmark country code
  },
  email: "test.user@example.com",
};

const threeDsNotSupportedResponse = {
  error: {
    type: "invalid_request",
    message: "Three_ds payments through Billwerk is not implemented",
    code: "IR_00",
  },
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "DKK",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: billingAddress,
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
        currency: "DKK",
        customer_acceptance: null,
        amount: 6000,
        authentication_type: "no_three_ds",
        setup_future_usage: "off_session",
        billing: billingAddress,
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
        currency: "DKK",
        shipping_cost: 50,
        billing: billingAddress,
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
        billing: billingAddress,
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
    // 3DS flows for billwerk - Not implemented
    "3DSManualCapture": {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "DKK",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: billingAddress,
      },
      Response: {
        status: 501,
        body: threeDsNotSupportedResponse,
      },
    },
    "3DSAutoCapture": {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "DKK",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: billingAddress,
      },
      Response: {
        status: 501,
        body: threeDsNotSupportedResponse,
      },
    },
    // Non-3DS flows
    No3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "DKK",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: billingAddress,
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
        currency: "DKK",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: billingAddress,
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
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_code: "hard_declined",
          error_message: "credit_card_expired",
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
    // No mandate flows for billwerk as specified in requirements
    SaveCardUseNo3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "DKK",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
        billing: billingAddress,
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
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "DKK",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          setup_future_usage: "on_session",
        },
      },
    },
    // 3DS Save Card flows - Not implemented
    SaveCardUse3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "DKK",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
        billing: billingAddress,
      },
      Response: {
        status: 501,
        body: threeDsNotSupportedResponse,
      },
    },
    SaveCardUse3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "DKK",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
        billing: billingAddress,
      },
      Response: {
        status: 501,
        body: threeDsNotSupportedResponse,
      },
    },
    // Payment method mandate flows - Skipped for billwerk
    PaymentMethodIdMandateNo3DSAutoCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "DKK",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          setup_future_usage: "on_session",
        },
      },
    }),
    PaymentMethodIdMandateNo3DSManualCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "DKK",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          setup_future_usage: "on_session",
        },
      },
    }),
    // 3DS Payment Method Mandate flows - Not implemented
    PaymentMethodIdMandate3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "DKK",
        mandate_data: null,
        authentication_type: "three_ds",
        customer_acceptance: customerAcceptance,
        billing: billingAddress,
      },
      Response: {
        status: 501,
        body: threeDsNotSupportedResponse,
      },
    },
    PaymentMethodIdMandate3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        mandate_data: null,
        authentication_type: "three_ds",
        customer_acceptance: customerAcceptance,
        billing: billingAddress,
      },
      Response: {
        status: 501,
        body: threeDsNotSupportedResponse,
      },
    },

    // Mandate flows - Skipped for billwerk
    MandateSingleUse3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "DKK",
        mandate_data: singleUseMandateData,
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          setup_future_usage: "on_session",
        },
      },
    },
    MandateSingleUse3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "DKK",
        mandate_data: singleUseMandateData,
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          setup_future_usage: "on_session",
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
        currency: "DKK",
        mandate_data: singleUseMandateData,
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          setup_future_usage: "on_session",
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
        currency: "DKK",
        mandate_data: singleUseMandateData,
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          setup_future_usage: "on_session",
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
        currency: "DKK",
        mandate_data: multiUseMandateData,
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          setup_future_usage: "on_session",
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
        currency: "DKK",
        mandate_data: multiUseMandateData,
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          setup_future_usage: "on_session",
        },
      },
    },
    MandateMultiUse3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "DKK",
        mandate_data: multiUseMandateData,
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          setup_future_usage: "on_session",
        },
      },
    },
    MandateMultiUse3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "DKK",
        mandate_data: multiUseMandateData,
        billing: billingAddress,
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
        currency: "DKK",
        mandate_data: singleUseMandateData,
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
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
        currency: "DKK",
        billing: billingAddress,
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
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        billing: billingAddress,
        mandate_data: null,
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
    MITAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          setup_future_usage: "on_session",
        },
      },
    },
    MITAutoCaptureWithCustomerAcceptance: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        customer_acceptance: {
          acceptance_type: "offline",
          accepted_at: "1963-05-03T04:07:52.723Z",
          online: {
            ip_address: "127.0.0.1",
            user_agent: "amet irure esse",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          setup_future_usage: "on_session",
        },
      },
    },
    MITManualCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          setup_future_usage: "on_session",
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
        currency: "DKK",
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          setup_future_usage: "on_session",
        },
      },
    },
    SaveCardUse3DSAutoCaptureOffSession: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "DKK",
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
        billing: billingAddress,
      },
      Response: {
        status: 501,
        body: threeDsNotSupportedResponse,
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
        currency: "DKK",
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          setup_future_usage: "on_session",
        },
      },
    },
    SaveCardConfirmAutoCaptureOffSession: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        setup_future_usage: "off_session",
        billing: billingAddress,
        card_cvc: "0009",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          setup_future_usage: "on_session",
        },
      },
    },
    SaveCardConfirmManualCaptureOffSession: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        setup_future_usage: "off_session",
        billing: billingAddress,
        card_cvc: "0009",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          setup_future_usage: "on_session",
        },
      },
    },
    SaveCardConfirmAutoCaptureOffSessionWithoutBilling: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        setup_future_usage: "off_session",
        billing: null,
        card_cvc: "0009",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          setup_future_usage: "on_session",
        },
      },
    },
    PaymentWithoutBilling: {
      Request: {
        currency: "DKK",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        authentication_type: "no_three_ds",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    PaymentWithBilling: {
      Request: {
        currency: "DKK",
        setup_future_usage: "on_session",
        billing: {
          address: {
            line1: "1467",
            line2: "CA",
            line3: "Harrison Street",
            city: "San Fransico",
            state: "CA",
            zip: "94122",
            country: "DK",
            first_name: "joseph",
            last_name: "Doe",
          },
          phone: {
            number: "9111222333",
            country_code: "+91",
          },
        },
        email: "hyperswitch.example@gmail.com",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    PaymentWithFullName: {
      Request: {
        currency: "DKK",
        setup_future_usage: "on_session",
        billing: {
          address: {
            first_name: "joseph",
            last_name: "Doe",
          },
          phone: {
            number: "9111222333",
            country_code: "+91",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    PaymentWithBillingEmail: {
      Request: {
        currency: "DKK",
        setup_future_usage: "on_session",
        email: "hyperswitch_sdk_demo_id1@gmail.com",
        billing: {
          address: {
            first_name: "joseph",
            last_name: "Doe",
          },
          phone: {
            number: "9111222333",
            country_code: "+91",
          },
          email: "hyperswitch.example@gmail.com",
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    PaymentMethod: {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_issuer: "Gpay",
        payment_method_issuer_code: "dk_billwerk",
        card: PaymentMethodCardDetails,
      },
      Response: {
        status: 200,
        body: {},
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
  payment_method_blocking_pm: {
    BlockIssuingCountry: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: {
            card_number: "4000000000000002",
            card_exp_month: "03",
            card_exp_year: "30",
            card_holder_name: "joseph Doeeee",
            card_cvc: "737",
            card_network: "Visa",
          },
        },
        billing: billingAddress,
        currency: "MYR",
      },
      Response: blockedPaymentErrorBodyForIssuingCountry,
    }),
    BlockCardType: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: {
            card_number: "4111111111111111",
            card_exp_month: "03",
            card_exp_year: "30",
            card_holder_name: "joseph Doeeee",
            card_cvc: "737",
            card_network: "Visa",
          },
        },
        billing: billingAddress,
        currency: "MYR",
      },
      Response: blockedPaymentErrorBodyForDebitCard,
    }),
    BlockCardSubtype: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: {
            card_number: "378282246310005",
            card_exp_month: "03",
            card_exp_year: "30",
            card_holder_name: "joseph Doeeee",
            card_cvc: "737",
            card_network: "Visa",
          },
        },
      },
      Response: blockedPaymentErrorBodyForCardSubtype,
    }),
    BlockIfBinInfoUnavailable: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: {
            card_number: "6304000000000000",
            card_exp_month: "03",
            card_exp_year: "30",
            card_holder_name: "joseph Doeeee",
            card_cvc: "737",
            card_network: "Visa",
          },
        },
      },
      Response: blockedPaymentErrorBodyForBinUnavailable,
    }),
  },
};
