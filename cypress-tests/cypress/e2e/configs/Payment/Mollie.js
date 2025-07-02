import {
  customerAcceptance,
  cardRequiredField,
  connectorDetails as commonConnectorDetails,
} from "./Commons.js";
import { getCustomExchange, getCurrency } from "./Modifiers.js";

// Mollie test card details based on their test environment
const successfulNo3DSCardDetails = {
  card_number: "4111111111111111", // Visa test card
  card_exp_month: "12",
  card_exp_year: "30",
  card_holder_name: "Test User",
  card_cvc: "123",
};

const successfulThreeDSTestCardDetails = {
  card_number: "4000000000003220", // 3DS test card
  card_exp_month: "12",
  card_exp_year: "30",
  card_holder_name: "Test User",
  card_cvc: "123",
};

const failedCardDetails = {
  card_number: "4000000000000002", // Declined card
  card_exp_month: "12",
  card_exp_year: "30",
  card_holder_name: "Test User",
  card_cvc: "123",
};

// Payment method list configuration for dynamic fields
const requiredFields = {
  payment_methods: [
    {
      payment_method: "card",
      payment_method_types: [
        {
          payment_method_type: "credit",
          card_networks: [
            {
              eligible_connectors: ["mollie"],
            },
          ],
          required_fields: cardRequiredField,
        },
      ],
    },
  ],
};

export const payment_methods_enabled = [
  {
    payment_method: "card",
    payment_method_types: [
      {
        payment_method_type: "credit",
        card_networks: [
          "Visa",
          "Mastercard",
          "AmericanExpress",
          "Discover",
          "JCB",
          "DinersClub",
          "UnionPay",
          "Interac",
          "CartesBancaires",
        ],
        minimum_amount: 0,
        maximum_amount: 68607706,
        recurring_enabled: false, // Mandates not supported by Mollie
        installment_payment_enabled: true,
      },
      {
        payment_method_type: "debit",
        card_networks: [
          "Visa",
          "Mastercard",
          "AmericanExpress",
          "Discover",
          "JCB",
          "DinersClub",
          "UnionPay",
          "Interac",
          "CartesBancaires",
        ],
        minimum_amount: 0,
        maximum_amount: 68607706,
        recurring_enabled: false, // Mandates not supported by Mollie
        installment_payment_enabled: true,
      },
    ],
  },
  {
    payment_method: "bank_redirect",
    payment_method_types: [
      {
        payment_method_type: "ideal",
        minimum_amount: 1,
        maximum_amount: 68607706,
        recurring_enabled: false,
        installment_payment_enabled: true,
      },
      {
        payment_method_type: "giropay",
        minimum_amount: 1,
        maximum_amount: 68607706,
        recurring_enabled: false,
        installment_payment_enabled: true,
      },
      {
        payment_method_type: "eps",
        minimum_amount: 1,
        maximum_amount: 68607706,
        recurring_enabled: false,
        installment_payment_enabled: true,
      },
      {
        payment_method_type: "sofort",
        minimum_amount: 1,
        maximum_amount: 68607706,
        recurring_enabled: false,
        installment_payment_enabled: true,
      },
      {
        payment_method_type: "przelewy24",
        minimum_amount: 1,
        maximum_amount: 68607706,
        recurring_enabled: false,
        installment_payment_enabled: true,
      },
      {
        payment_method_type: "bancontact_card",
        minimum_amount: 1,
        maximum_amount: 68607706,
        recurring_enabled: false,
        installment_payment_enabled: true,
      },
    ],
  },
  {
    payment_method: "wallet",
    payment_method_types: [
      {
        payment_method_type: "paypal",
        minimum_amount: 1,
        maximum_amount: 68607706,
        recurring_enabled: false,
        installment_payment_enabled: true,
      },
      {
        payment_method_type: "apple_pay",
        minimum_amount: 1,
        maximum_amount: 68607706,
        recurring_enabled: false,
        installment_payment_enabled: true,
      },
    ],
  },
  {
    payment_method: "bank_debit",
    payment_method_types: [
      {
        payment_method_type: "sepa",
        minimum_amount: 1,
        maximum_amount: 68607706,
        recurring_enabled: false,
        installment_payment_enabled: true,
      },
    ],
  },
  // Note: bank_transfer payment methods are not supported by Mollie
  // Removing this section so the test framework will use our explicit
  // "not implemented" configurations in bank_transfer_pm
];

export const connectorDetails = {
  card_pm: {
    PaymentIntent: getCustomExchange({
      Request: {
        currency: "USD", // Mollie's primary currency
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
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
          status: "requires_customer_action", // Mollie auto-captures by default
          payment_method: "card",
          shipping_cost: 50,
          amount: 6000,
        },
      },
    }),

    No3DSAutoCapture: getCustomExchange({
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
          status: "requires_customer_action", // Mollie requires redirection for payment completion
          payment_method: "card",
        },
      },
    }),

    "3DSAutoCapture": getCustomExchange({
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
        status: 200,
        body: {
          status: "requires_customer_action", // Mollie requires redirection for 3DS
          payment_method: "card",
        },
      },
    }),

    No3DSFailPayment: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: failedCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_code: "card_declined",
          error_message: "Your card was declined",
        },
      },
    }),

    // Manual capture not supported by Mollie - skip these tests
    No3DSManualCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
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
          error: {
            type: "invalid_request",
            message: "No eligible connector was found for the current payment method configuration",
          },
        },
      },
    }),

    "3DSManualCapture": getCustomExchange({
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
            message: "No eligible connector was found for the current payment method configuration",
          },
        },
      },
    }),

    // Capture operations not supported
    Capture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        amount_to_capture: 6000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Capture flow not supported by Mollie",
          },
        },
      },
    }),

    PartialCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        amount_to_capture: 2000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Capture flow not supported by Mollie",
          },
        },
      },
    }),

    // Void operations not supported
    Void: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          
        },
      },
    }),

    VoidAfterConfirm: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Void flow not supported by Mollie",
          },
        },
      },
    }),

    // Refund operations - supported by Mollie
    Refund: getCustomExchange({
      Request: {
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "pending", // Mollie refunds are typically async
        },
      },
    }),

    PartialRefund: getCustomExchange({
      Request: {
        amount: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
        },
      },
    }),

    manualPaymentRefund: getCustomExchange({
      Request: {
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
        },
      },
    }),

    manualPaymentPartialRefund: getCustomExchange({
      Request: {
        amount: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
        },
      },
    }),

    SyncRefund: getCustomExchange({
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),

    // All mandate scenarios should be skipped - Mollie doesn't support mandates
    MandateSingleUseNo3DSAutoCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
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
          status:"requires_customer_action", // Mollie auto-captures by default
        },
      },
    }),

    MandateSingleUseNo3DSManualCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
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
        body: {
          error: {
            type: "invalid_request",
            message: "No eligible connector was found for the current payment method configuration",
          },
        },
      },
    }),

    MandateSingleUse3DSAutoCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
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
        body: {
          error: {
            type: "invalid_request",
            message: "Mandates not supported by Mollie",
          },
        },
      },
    }),

    MandateMultiUseNo3DSAutoCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
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
          
        },
      },
    }),

    MandateMultiUseNo3DSManualCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
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
            message: "No eligible connector was found for the current payment method configuration",
          },
        },
      },
    }),

    MandateMultiUse3DSAutoCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
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
            message: "Mandates not supported by Mollie",
          },
        },
      },
    }),

    // Zero auth and MIT scenarios - not supported
    ZeroAuthMandate: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
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
        body: {
          error: {
            type: "invalid_request",
            message: "Missing required param: browser_info",
          },
        },
      },
    }),

    ZeroAuthPaymentIntent: getCustomExchange({
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
    }),

    ZeroAuthConfirmPayment: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
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
            message: "Zero auth not supported by Mollie",
          },
        },
      },
    }),

    // Save card scenarios - may work with tokenization
    SaveCardUseNo3DSAutoCapture: {
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
          status: "requires_customer_action", // Mollie requires redirection
          payment_method: "card",
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
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Missing required param: payment_method_data",
          },
        },
      },
    }),

    SaveCardUse3DSAutoCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action", // Mollie requires redirection for 3DS
        },
      },
    }),

    SaveCardUseNo3DSAutoCaptureOffSession: getCustomExchange({
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
        status: 400,
        body: {
          
        },
      },
    }),

    SaveCardUseNo3DSManualCaptureOffSession: getCustomExchange({
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
        status: 400,
        body: {
          
        },
      },
    }),

    SaveCardConfirmAutoCaptureOffSession: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        setup_future_usage: "off_session",
        currency: "USD",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 400,
        body: {
          
        },
      },
    }),

    SaveCardConfirmManualCaptureOffSession: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        setup_future_usage: "off_session",
        currency: "USD",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 400,
        body: {
          
        },
      },
    }),

    // MIT scenarios - not supported without mandates
    MITAutoCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "A payment token or payment method data or ctp service details is required",
          },
        },
      },
    }),

    MITManualCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Manual capture not supported by Mollie",
          },
        },
      },
    }),

    // Capture scenarios - not supported by Mollie
    CaptureGreaterAmount: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        amount_to_capture: 6000000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Capture flow not supported by Mollie",
          },
        },
      },
    }),

    CaptureCapturedAmount: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        amount_to_capture: 6000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "This Payment could not be captured because it has a payment.status of requires_customer_action. The expected state is requires_capture, partially_captured_and_capturable, processing",
          },
        },
      },
    }),

    MITWithoutBillingAddress: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        billing: null,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "MIT not supported by Mollie without mandates",
          },
        },
      },
    }),

    ConfirmSuccessfulPayment: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "You cannot confirm this payment because it has status requires_customer_action",
            code: "IR_16",
          },
        },
      },
    }),

    RefundGreaterAmount: getCustomExchange({
      Request: {
        amount: 6000000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "The refund amount exceeds the amount captured",
            code: "IR_13",
          },
        },
      },
    }),

    DuplicatePaymentID: getCustomExchange({
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
          error: {
            type: "invalid_request",
            message: "The payment with the specified payment_id already exists in our records",
            code: "HE_01",
          },
        },
      },
    }),

    DuplicateRefundID: getCustomExchange({
      Request: {
        amount: 2000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Duplicate refund request. Refund already attempted with the refund ID",
            code: "HE_01",
          },
        },
      },
    }),

    // Validation error scenarios
    InvalidCardNumber: getCustomExchange({
      Request: {
        currency: "USD",
        payment_method: "card",
        payment_method_type: "debit",
        setup_future_usage: "on_session",
        payment_method_data: {
          card: {
            card_number: "123456",
            card_exp_month: "10",
            card_exp_year: "25",
            card_holder_name: "Test User",
            card_cvc: "123",
          },
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            error_type: "invalid_request",
            message: "Json deserialize error: invalid card number length",
            code: "IR_06",
          },
        },
      },
    }),

    InvalidExpiryMonth: getCustomExchange({
      Request: {
        currency: "USD",
        payment_method: "card",
        payment_method_type: "debit",
        setup_future_usage: "on_session",
        payment_method_data: {
          card: {
            card_number: "4111111111111111",
            card_exp_month: "00",
            card_exp_year: "2030",
            card_holder_name: "Test User",
            card_cvc: "123",
          },
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Invalid Expiry Month",
            code: "IR_16",
          },
        },
      },
    }),

    InvalidExpiryYear: getCustomExchange({
      Request: {
        currency: "USD",
        payment_method: "card",
        payment_method_type: "debit",
        setup_future_usage: "on_session",
        payment_method_data: {
          card: {
            card_number: "4111111111111111",
            card_exp_month: "01",
            card_exp_year: "2023",
            card_holder_name: "Test User",
            card_cvc: "123",
          },
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Invalid Expiry Year",
            code: "IR_16",
          },
        },
      },
    }),

    InvalidCardCvv: getCustomExchange({
      Request: {
        currency: "USD",
        payment_method: "card",
        payment_method_type: "debit",
        setup_future_usage: "on_session",
        payment_method_data: {
          card: {
            card_number: "4111111111111111",
            card_exp_month: "01",
            card_exp_year: "2030",
            card_holder_name: "Test User",
            card_cvc: "123456",
          },
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Invalid card_cvc length",
            code: "IR_16",
          },
        },
      },
    }),

    InvalidCurrency: getCustomExchange({
      Request: {
        currency: "United",
        payment_method: "card",
        payment_method_type: "debit",
        setup_future_usage: "on_session",
        payment_method_data: {
          card: {
            card_number: "4111111111111111",
            card_exp_month: "01",
            card_exp_year: "2030",
            card_holder_name: "Test User",
            card_cvc: "123",
          },
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            error_type: "invalid_request",
            message: "Json deserialize error: unknown variant `United`, expected one of `AED`, `AFN`, `ALL`, `AMD`, `ANG`, `AOA`, `ARS`, `AUD`, `AWG`, `AZN`, `BAM`, `BBD`, `BDT`, `BGN`, `BHD`, `BIF`, `BMD`, `BND`, `BOB`, `BRL`, `BSD`, `BTN`, `BWP`, `BYN`, `BZD`, `CAD`, `CDF`, `CHF`, `CLF`, `CLP`, `CNY`, `COP`, `CRC`, `CUC`, `CUP`, `CVE`, `CZK`, `DJF`, `DKK`, `DOP`, `DZD`, `EGP`, `ERN`, `ETB`, `USD`, `FJD`, `FKP`, `GBP`, `GEL`, `GHS`, `GIP`, `GMD`, `GNF`, `GTQ`, `GYD`, `HKD`, `HNL`, `HRK`, `HTG`, `HUF`, `IDR`, `ILS`, `INR`, `IQD`, `IRR`, `ISK`, `JMD`, `JOD`, `JPY`, `KES`, `KGS`, `KHR`, `KMF`, `KPW`, `KRW`, `KWD`, `KYD`, `KZT`, `LAK`, `LBP`, `LKR`, `LRD`, `LSL`, `LYD`, `MAD`, `MDL`, `MGA`, `MKD`, `MMK`, `MNT`, `MOP`, `MRU`, `MUR`, `MVR`, `MWK`, `MXN`, `MYR`, `MZN`, `NAD`, `NGN`, `NIO`, `NOK`, `NPR`, `NZD`, `OMR`, `PAB`, `PEN`, `PGK`, `PHP`, `PKR`, `PLN`, `PYG`, `QAR`, `RON`, `RSD`, `RUB`, `RWF`, `SAR`, `SBD`, `SCR`, `SDG`, `SEK`, `SGD`, `SHP`, `SLE`, `SLL`, `SOS`, `SRD`, `SSP`, `STD`, `STN`, `SVC`, `SYP`, `SZL`, `THB`, `TJS`, `TMT`, `TND`, `TOP`, `TRY`, `TTD`, `TWD`, `TZS`, `UAH`, `UGX`, `USD`, `UYU`, `UZS`, `VES`, `VND`, `VUV`, `WST`, `XAF`, `XCD`, `XOF`, `XPF`, `YER`, `ZAR`, `ZMW`, `ZWL`",
            code: "IR_06",
          },
        },
      },
    }),

    InvalidCaptureMethod: getCustomExchange({
      Request: {
        currency: "USD",
        capture_method: "auto",
        payment_method: "card",
        payment_method_type: "debit",
        setup_future_usage: "on_session",
        payment_method_data: {
          card: {
            card_number: "4111111111111111",
            card_exp_month: "01",
            card_exp_year: "2030",
            card_holder_name: "Test User",
            card_cvc: "123",
          },
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            error_type: "invalid_request",
            message: "Json deserialize error: unknown variant `auto`, expected one of `automatic`, `manual`, `manual_multiple`, `scheduled`",
            code: "IR_06",
          },
        },
      },
    }),

    InvalidPaymentMethod: getCustomExchange({
      Request: {
        currency: "USD",
        payment_method: "this_supposed_to_be_a_card",
        payment_method_type: "debit",
        setup_future_usage: "on_session",
        payment_method_data: {
          card: {
            card_number: "4111111111111111",
            card_exp_month: "01",
            card_exp_year: "2030",
            card_holder_name: "Test User",
            card_cvc: "123",
          },
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            error_type: "invalid_request",
            message: "Json deserialize error: unknown variant `this_supposed_to_be_a_card`, expected one of `card`, `card_redirect`, `pay_later`, `wallet`, `bank_redirect`, `bank_transfer`, `crypto`, `bank_debit`, `reward`, `real_time_payment`, `upi`, `voucher`, `gift_card`, `open_banking`, `mobile_payment`",
            code: "IR_06",
          },
        },
      },
    }),

    InvalidAmountToCapture: getCustomExchange({
      Request: {
        currency: "USD",
        amount_to_capture: 10000,
        payment_method: "card",
        payment_method_type: "debit",
        setup_future_usage: "on_session",
        payment_method_data: {
          card: {
            card_number: "4111111111111111",
            card_exp_month: "01",
            card_exp_year: "2030",
            card_holder_name: "Test User",
            card_cvc: "123",
          },
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "amount_to_capture contains invalid data. Expected format is amount_to_capture lesser than amount",
            code: "IR_05",
          },
        },
      },
    }),

    MissingRequiredParam: getCustomExchange({
      Request: {
        currency: "USD",
        payment_method_type: "debit",
        setup_future_usage: "on_session",
        payment_method_data: {
          card: {
            card_number: "4111111111111111",
            card_exp_month: "01",
            card_exp_year: "2030",
            card_holder_name: "Test User",
            card_cvc: "123",
          },
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Missing required param: payment_method",
            code: "IR_04",
          },
        },
      },
    }),

    PaymentIntentErrored: getCustomExchange({
      Request: {
        currency: "USD",
      },
      Response: {
        status: 422,
        body: {
          error: {
            type: "invalid_request",
            message: "A payment token or payment method data or ctp service details is required",
            code: "IR_06",
          },
        },
      },
    }),

    // Payment method ID scenarios
    PaymentMethodIdMandateNo3DSAutoCapture: getCustomExchange({
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
        currency: "USD",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 400,
        body: {
          status: "requires_capture",
          payment_method: "card",
        },
      },
    }),

    PaymentMethodIdMandate3DSAutoCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        authentication_type: "three_ds",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 400,
        body: {
          
        },
      },
    }),

    PaymentMethodIdMandate3DSManualCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
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
            message: "No eligible connector was found for the current payment method configuration",
          },
        },
      },
    }),

  },

  bank_redirect_pm: {
    PaymentIntent: (paymentMethodType) => {
      // For BLIK, return skip configuration since it's not supported by Mollie
      if (paymentMethodType === "Blik") {
        return getCustomExchange({
          Configs: {
            TRIGGER_SKIP: true,
          },
          Request: {
            currency: "PLN",
          },
          Response: {
            status: 200,
            body: {
              status: "requires_payment_method",
            },
          },
        });
      }
      
      // For other payment methods, return the standard PaymentIntent configuration
      return getCustomExchange({
        Request: {
          currency: "USD",
        },
        Response: {
          status: 200,
          body: {
            status: "requires_payment_method",
          },
        },
      });
    },

    Ideal: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "ideal",
        payment_method_data: {
          bank_redirect: {
            ideal: {
              bank_name: "ing",
              country: "NL",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "Amsterdam",
            state: "NH",
            zip: "1012",
            country: "NL",
            first_name: "John",
            last_name: "Doe",
          },
        },
        currency: "USD",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method: "bank_redirect",
        },
      },
    }),

    Giropay: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "giropay",
        payment_method_data: {
          bank_redirect: {
            giropay: {
              bank_name: "",
              bank_account_bic: "",
              bank_account_iban: "",
              preferred_language: "en",
              country: "DE",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "Berlin",
            state: "BE",
            zip: "10115",
            country: "DE",
            first_name: "John",
            last_name: "Doe",
          },
        },
        currency: "USD",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method: "bank_redirect",
        },
      },
    }),

    Eps: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "eps",
        payment_method_data: {
          bank_redirect: {
            eps: {
              bank_name: "bank_austria",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "Vienna",
            state: "VI",
            zip: "1010",
            country: "AT",
            first_name: "John",
            last_name: "Doe",
          },
        },
        currency: "USD",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method: "bank_redirect",
        },
      },
    }),

    Sofort: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "sofort",
        payment_method_data: {
          bank_redirect: {
            sofort: {
              country: "DE",
              preferred_language: "en",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "Berlin",
            state: "BE",
            zip: "10115",
            country: "DE",
            first_name: "John",
            last_name: "Doe",
          },
        },
        currency: "USD",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method: "bank_redirect",
        },
      },
    }),

    Przelewy24: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "przelewy24",
        payment_method_data: {
          bank_redirect: {
            przelewy24: {
              bank_name: "citi",
              billing_details: {
                email: "guest@juspay.in",
              },
            },
          },
        },
        currency: "PLN",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method: "bank_redirect",
        },
      },
    }),

    Bancontact: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "bancontact_card",
        payment_method_data: {
          bank_redirect: {
            bancontact_card: {},
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "Brussels",
            state: "BR",
            zip: "1000",
            country: "BE",
            first_name: "John",
            last_name: "Doe",
          },
        },
        currency: "USD",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),

    Blik: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "blik",
        payment_method_data: {
          bank_redirect: {
            blik: {
              blik_code: "777987",
            },
          },
        },
        currency: "PLN",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "You cannot confirm this payment because it has status failed, you can pass `retry_action` as `manual_retry` in request to try this payment again",
          },
        },
      },
    }),
  },

  wallet_pm: {
    PaymentIntent: getCustomExchange({
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

    PaypalRedirect: getCustomExchange({
      Request: {
        payment_method: "wallet",
        payment_method_type: "paypal",
        payment_method_data: {
          wallet: {
            paypal_redirect: {},
          },
        },
        currency: "USD",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),

    ApplePay: getCustomExchange({
      Request: {
        payment_method: "wallet",
        payment_method_type: "apple_pay",
        payment_method_data: {
          wallet: {
            apple_pay: {
              payment_data: "test_payment_data_string",
            },
          },
        },
        currency: "USD",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),
  },

  bank_debit_pm: {
    PaymentIntent: getCustomExchange({
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

    SepaDirectDebit: getCustomExchange({
      Request: {
        payment_method: "bank_debit",
        payment_method_type: "sepa",
        payment_method_data: {
          bank_debit: {
            sepa_bank_debit: {
              iban: "DE89370400440532013000",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "Berlin",
            state: "BE",
            zip: "10115",
            country: "DE",
            first_name: "John",
            last_name: "Doe",
          },
        },
        currency: "USD",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
  },

  bank_transfer_pm: {
    PaymentIntent: (paymentMethodType) => {
      return getCustomExchange({
        Request: {
          currency: getCurrency(paymentMethodType),
        },
        Response: {
          status: 200,
          body: {
            status: "requires_payment_method",
          },
        },
      });
    },

    Pix: getCustomExchange({
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "pix",
        payment_method_data: {
          bank_transfer: {
            pix: {},
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "San Fransico",
            state: "California",
            zip: "94122",
            country: "BR",
            first_name: "john",
            last_name: "doe",
          },
        },
        currency: "BRL",
      },
      ResponseCustom: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Selected payment method through mollie is not implemented",
            code: "IR_39",
          },
        },
      },
    }, commonConnectorDetails.bank_transfer_pm.Pix),

    InstantBankTransferFinland: getCustomExchange({
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "instant_bank_transfer_finland",
        payment_method_data: {
          bank_transfer: {
            instant_bank_transfer_finland: {},
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "San Fransico",
            state: "California",
            zip: "94122",
            country: "FI",
            first_name: "john",
            last_name: "doe",
          },
        },
        currency: "EUR",
      },
      ResponseCustom: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Selected payment method through mollie is not implemented",
            code: "IR_39",
          },
        },
      },
    }, commonConnectorDetails.bank_transfer_pm.InstantBankTransferFinland),

    InstantBankTransferPoland: getCustomExchange({
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "instant_bank_transfer_poland",
        payment_method_data: {
          bank_transfer: {
            instant_bank_transfer_poland: {},
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "San Fransico",
            state: "California",
            zip: "94122",
            country: "PL",
            first_name: "john",
            last_name: "doe",
          },
        },
        currency: "PLN",
      },
      ResponseCustom: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Selected payment method through mollie is not implemented",
            code: "IR_39",
          },
        },
      },
    }, commonConnectorDetails.bank_transfer_pm.InstantBankTransferPoland),
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

  // Return URL validation scenarios
  return_url_variations: {
    return_url_too_long: getCustomExchange({
      Request: {
        customer_id: "customer_1234567890",
        return_url: "http://example.com/" + "a".repeat(2031),
      },
      Response: {
        status: 400,
        body: {
          error: {
            message: "return_url must be at most 2048 characters long. Received 2050 characters",
            code: "IR_06",
            type: "invalid_request",
          },
        },
      },
    }),

    return_url_invalid_format: getCustomExchange({
      Request: {
        return_url: "not_a_valid_url",
      },
      Response: {
        status: 400,
        body: {
          error: {
            message: 'Json deserialize error: relative URL without a base: "not_a_valid_url" at line 1 column 357',
            code: "IR_06",
            error_type: "invalid_request",
          },
        },
      },
    }),
  },

  // Mandate ID validation
  mandate_id_too_long: getCustomExchange({
    Request: {
      mandate_id: "mnd_" + "a".repeat(63),
      off_session: true,
    },
    Response: {
      status: 400,
      body: {
        error: {
          message: "mandate_id must be at most 64 characters long. Received 67 characters",
          code: "IR_06",
          type: "invalid_request",
        },
      },
    },
  }),
};
