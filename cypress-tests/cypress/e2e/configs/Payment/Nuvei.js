import {
  customerAcceptance,
  cardRequiredField,
  successfulNo3DSCardDetails,
} from "./Commons";
import { getCustomExchange } from "./Modifiers";

// Mandate data for supported mandate flows
const singleUseMandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    single_use: {
      amount: 8000,
      currency: "USD",
      start_date: "2022-09-10T00:00:00Z",
      end_date: "2023-09-10T00:00:00Z",
      metadata: {
        frequency: "1",
      },
    },
  },
};

const multiUseMandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    multi_use: {
      amount: 8000,
      currency: "USD",
      start_date: "2022-09-10T00:00:00Z",
      end_date: "2023-09-10T00:00:00Z",
      metadata: {
        frequency: "13",
      },
    },
  },
};

// Test card details based on Nuvei test cards (from Rust tests)
const successfulThreeDSCardDetails = {
  card_number: "4000027891380961",
  card_exp_month: "10",
  card_exp_year: "25",
  card_holder_name: "CL-BRW1",
  card_cvc: "123",
};

// Payment method data objects for responses
const payment_method_data_no3ds = {
  card: {
    last4: "1111",
    card_type: "CREDIT",
    card_network: "Visa",
    card_issuer: "JP Morgan",
    card_issuing_country: "INDIA",
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

const payment_method_data_3ds = {
  card: {
    last4: "0961",
    card_type: "CREDIT",
    card_network: "Visa",
    card_issuer: "RIVER VALLEY CREDIT UNION",
    card_issuing_country: "UNITEDSTATES",
    card_isin: "400002",
    card_extended_bin: null,
    card_exp_month: "10",
    card_exp_year: "25",
    card_holder_name: "CL-BRW1",
    payment_checks: null,
    authentication_data: null,
  },
  billing: null,
};

export const connectorDetails = {
  card_pm: {
    // Basic payment intent creation
    PaymentIntent: getCustomExchange({
      Request: {
        currency: "USD",
        amount: 11500,
        authentication_type: "three_ds",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          setup_future_usage: "on_session",
          amount: 11500,
        },
      },
    }),

    // Payment intent with shipping cost
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
          amount: 11500,
        },
      },
    }),

    // Payment confirmation with shipping cost
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
          shipping_cost: 50,
          amount_received: 11550,
          amount: 11500,
          net_amount: 11550,
        },
      },
    }),

    // No 3DS automatic capture
    No3DSAutoCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        amount: 11500,
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
    }),

    // No 3DS manual capture
    No3DSManualCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        amount: 11500,
        currency: "USD",
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
    }),

    // 3DS automatic capture
    "3DSAutoCapture": getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSCardDetails,
        },
        amount: 11500,
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          setup_future_usage: "on_session",
          payment_method_data: payment_method_data_3ds,
        },
      },
    }),

    // 3DS manual capture
    "3DSManualCapture": getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSCardDetails,
        },
        amount: 11500,
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          setup_future_usage: "on_session",
          payment_method_data: payment_method_data_3ds,
        },
      },
    }),

    // Capture payment
    Capture: getCustomExchange({
      Request: {
        amount_to_capture: 11500,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          amount: 11500,
          amount_capturable: 0,
          amount_received: 11500,
        },
      },
    }),

    // Partial capture
    PartialCapture: getCustomExchange({
      Request: {
        amount_to_capture: 5000,
      },
      Response: {
        status: 200,
        body: {
          status: "partially_captured",
          amount: 11500,
          amount_capturable: 0,
          amount_received: 5000,
        },
      },
    }),

    // Void payment
    Void: getCustomExchange({
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "cancelled",
        },
      },
    }),

    // Refund payment
    Refund: getCustomExchange({
      Request: {
        amount: 11500,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),

    // Partial refund - Nuvei limitation: only supports single refund per sale
    // Note: Tests that attempt multiple partial refunds will fail after the first one
    // TRIGGER_SKIP is used to skip tests that would fail due to Nuvei's "only one refund per sale" limitation
    PartialRefund: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        amount: 5000,
      },
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
        amount: 11500,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),

    // Manual payment partial refund - Nuvei limitation: only supports single refund per sale
    // Note: Tests that attempt multiple partial refunds will fail after the first one
    // TRIGGER_SKIP is used to skip tests that would fail due to Nuvei's "only one refund per sale" limitation
    manualPaymentPartialRefund: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        amount: 5000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),

    // Sync refund
    SyncRefund: getCustomExchange({
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),

    // Mandate scenarios - Single Use
    MandateSingleUseNo3DSAutoCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        amount: 11500,
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
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        amount: 11500,
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

    MandateSingleUse3DSAutoCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSCardDetails,
        },
        amount: 11500,
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
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSCardDetails,
        },
        amount: 11500,
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

    // Mandate scenarios - Multi Use
    MandateMultiUseNo3DSAutoCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        amount: 11500,
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
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        amount: 11500,
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

    MandateMultiUse3DSAutoCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSCardDetails,
        },
        amount: 11500,
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
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSCardDetails,
        },
        amount: 11500,
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

    // MIT (Merchant Initiated Transaction) scenarios
    MITAutoCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        amount: 11500,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method_id: null,
        },
      },
    }),

    MITManualCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    }),

    // Zero auth scenarios
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
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Setup Mandate flow for Nuvei is not implemented",
            code: "IR_00",
          },
        },
      },
    },

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
          setup_future_usage: "off_session",
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
    }),

    // Save card scenarios
    SaveCardUseNo3DSAutoCapture: getCustomExchange({
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
    }),

    SaveCardUseNo3DSManualCapture: getCustomExchange({
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

    SaveCardUseNo3DSAutoCaptureOffSession: getCustomExchange({
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
    }),

    SaveCardUse3DSAutoCaptureOffSession: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulThreeDSCardDetails,
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
    }),

    SaveCardConfirmAutoCaptureOffSession: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
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
        TRIGGER_SKIP: true,
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

    // Payment scenarios for dynamic fields testing
    PaymentWithoutBilling: getCustomExchange({
      Request: {
        currency: "USD",
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
    }),

    PaymentWithBilling: getCustomExchange({
      Request: {
        currency: "USD",
        setup_future_usage: "on_session",
        billing: {
          address: {
            line1: "1467",
            line2: "CA",
            line3: "Harrison Street",
            city: "San Francisco",
            state: "CA",
            zip: "94122",
            country: "US",
            first_name: "John",
            last_name: "Doe",
          },
          phone: {
            number: "9111222333",
            country_code: "+1",
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
    }),

    PaymentWithFullName: getCustomExchange({
      Request: {
        currency: "USD",
        setup_future_usage: "on_session",
        billing: {
          address: {
            first_name: "John",
            last_name: "Doe",
          },
          phone: {
            number: "9111222333",
            country_code: "+1",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),

    PaymentWithBillingEmail: getCustomExchange({
      Request: {
        currency: "USD",
        setup_future_usage: "on_session",
        email: "hyperswitch_sdk_demo_id1@gmail.com",
        billing: {
          address: {
            first_name: "John",
            last_name: "Doe",
          },
          phone: {
            number: "9111222333",
            country_code: "+1",
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
    }),

    // Payment method ID mandate scenarios
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
          status: "succeeded",
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
        status: 200,
        body: {
          status: "requires_capture",
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
          card: successfulThreeDSCardDetails,
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

    PaymentMethodIdMandate3DSManualCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSCardDetails,
        },
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
  },

  // Bank redirect payment methods
  bank_redirect_pm: {
    PaymentIntent: getCustomExchange({
      Request: {
        amount: 11500,
        currency: "EUR",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),

    BlikPaymentIntent: getCustomExchange({
      Request: {
        amount: 11500,
        currency: "PLN",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),

    Blik: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "blik",
        payment_method_data: {
          bank_redirect: {
            blik: {},
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "Warsaw",
            state: "Mazovia",
            zip: "00-001",
            country: "PL",
            first_name: "John",
            last_name: "Doe",
          },
          phone: {
            number: "9123456789",
            country_code: "+48",
          },
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
            reason: "bank_redirect blik is not supported by nuvei",
          },
        },
      },
    }),

    Ideal: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "ideal",
        payment_method_data: {
          bank_redirect: {
            ideal: {
              bank_name: "ing",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "Amsterdam",
            state: "North Holland",
            zip: "1012",
            country: "NL",
            first_name: "John",
            last_name: "Doe",
          },
          phone: {
            number: "9123456789",
            country_code: "+31",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
          error_message: "Connector did not respond in specified time",
          error_code: "TIMEOUT",
          unified_code: "UE_9000",
          unified_message: "Something went wrong",
        },
      },
    }),

    Giropay: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "giropay",
        payment_method_data: {
          bank_redirect: {
            giropay: {},
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "Berlin",
            state: "Berlin",
            zip: "10115",
            country: "DE",
            first_name: "John",
            last_name: "Doe",
          },
          phone: {
            number: "9123456789",
            country_code: "+49",
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

    Sofort: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "sofort",
        payment_method_data: {
          bank_redirect: {
            sofort: {
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
            state: "Berlin",
            zip: "10115",
            country: "DE",
            first_name: "John",
            last_name: "Doe",
          },
          phone: {
            number: "9123456789",
            country_code: "+49",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
          error_message: "Connector did not respond in specified time",
          error_code: "TIMEOUT",
          unified_code: "UE_9000",
          unified_message: "Something went wrong",
        },
      },
    }),

    Eps: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "eps",
        amount: 11500,
        payment_method_data: {
          bank_redirect: {
            eps: {
              country: "AT",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "Vienna",
            state: "Vienna",
            zip: "1010",
            country: "AT",
            first_name: "John",
            last_name: "Doe",
          },
          phone: {
            number: "9123456789",
            country_code: "+43",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
          error_message: "Connector did not respond in specified time",
          error_code: "TIMEOUT",
          unified_code: "UE_9000",
          unified_message: "Something went wrong",
        },
      },
    }),
  },

  // Wallet payment methods
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

    ApplePay: getCustomExchange({
      Request: {
        payment_method: "wallet",
        payment_method_type: "apple_pay",
        payment_method_data: {
          wallet: {
            apple_pay: {
              payment_data: "test_payment_data",
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
    }),

    GooglePay: getCustomExchange({
      Request: {
        payment_method: "wallet",
        payment_method_type: "google_pay",
        payment_method_data: {
          wallet: {
            google_pay: {
              payment_token: "test_payment_token",
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
    }),

    PayPal: getCustomExchange({
      Request: {
        payment_method: "wallet",
        payment_method_type: "paypal",
        payment_method_data: {
          wallet: {
            paypal: {},
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
  },

  // Pay Later payment methods
  pay_later_pm: {
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

    Klarna: getCustomExchange({
      Request: {
        payment_method: "pay_later",
        payment_method_type: "klarna",
        payment_method_data: {
          pay_later: {
            klarna: {},
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "San Francisco",
            state: "California",
            zip: "94122",
            country: "US",
            first_name: "John",
            last_name: "Doe",
          },
          phone: {
            number: "9123456789",
            country_code: "+1",
          },
          email: "john.doe@example.com",
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),

    AfterpayClearpay: getCustomExchange({
      Request: {
        payment_method: "pay_later",
        payment_method_type: "afterpay_clearpay",
        payment_method_data: {
          pay_later: {
            afterpay_clearpay: {},
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "San Francisco",
            state: "California",
            zip: "94122",
            country: "US",
            first_name: "John",
            last_name: "Doe",
          },
          phone: {
            number: "9123456789",
            country_code: "+1",
          },
          email: "john.doe@example.com",
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
  },

  // Payment method list configurations
  pm_list: {
    PmListResponse: {
      PmListNull: {
        payment_methods: [],
      },
      pmListDynamicFieldWithoutBilling: {
        payment_methods: [
          {
            payment_method: "card",
            payment_method_types: [
              {
                payment_method_type: "credit",
                card_networks: [],
                required_fields: cardRequiredField,
              },
            ],
          },
        ],
      },
      pmListDynamicFieldWithBilling: {
        payment_methods: [
          {
            payment_method: "card",
            payment_method_types: [
              {
                payment_method_type: "credit",
                card_networks: [],
                required_fields: cardRequiredField,
              },
            ],
          },
        ],
      },
      pmListDynamicFieldWithNames: {
        payment_methods: [
          {
            payment_method: "card",
            payment_method_types: [
              {
                payment_method_type: "credit",
                card_networks: [],
                required_fields: cardRequiredField,
              },
            ],
          },
        ],
      },
      pmListDynamicFieldWithEmail: {
        payment_methods: [
          {
            payment_method: "card",
            payment_method_types: [
              {
                payment_method_type: "credit",
                card_networks: [],
                required_fields: cardRequiredField,
              },
            ],
          },
        ],
      },
    },
  },
};

// Export payment methods enabled configuration
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
          "UnionPay",
          "Interac",
          "JCB",
          "DinersClub",
          "Discover",
          "CartesBancaires",
        ],
        minimum_amount: 0,
        maximum_amount: 68607706,
        recurring_enabled: true,
        installment_payment_enabled: true,
      },
      {
        payment_method_type: "debit",
        card_networks: [
          "Visa",
          "Mastercard",
          "AmericanExpress",
          "UnionPay",
          "Interac",
          "JCB",
          "DinersClub",
          "Discover",
          "CartesBancaires",
        ],
        minimum_amount: 0,
        maximum_amount: 68607706,
        recurring_enabled: true,
        installment_payment_enabled: true,
      },
    ],
  },
  {
    payment_method: "pay_later",
    payment_method_types: [
      {
        payment_method_type: "klarna",
        minimum_amount: 0,
        maximum_amount: 68607706,
        recurring_enabled: false,
        installment_payment_enabled: false,
      },
      {
        payment_method_type: "afterpay_clearpay",
        minimum_amount: 0,
        maximum_amount: 68607706,
        recurring_enabled: false,
        installment_payment_enabled: false,
      },
    ],
  },
  {
    payment_method: "bank_redirect",
    payment_method_types: [
      {
        payment_method_type: "ideal",
        minimum_amount: 0,
        maximum_amount: 68607706,
        recurring_enabled: false,
        installment_payment_enabled: false,
      },
      {
        payment_method_type: "giropay",
        minimum_amount: 0,
        maximum_amount: 68607706,
        recurring_enabled: false,
        installment_payment_enabled: false,
      },
      {
        payment_method_type: "sofort",
        minimum_amount: 0,
        maximum_amount: 68607706,
        recurring_enabled: false,
        installment_payment_enabled: false,
      },
      {
        payment_method_type: "eps",
        minimum_amount: 0,
        maximum_amount: 68607706,
        recurring_enabled: false,
        installment_payment_enabled: false,
      },
    ],
  },
  {
    payment_method: "wallet",
    payment_method_types: [
      {
        payment_method_type: "apple_pay",
        minimum_amount: 0,
        maximum_amount: 68607706,
        recurring_enabled: false,
        installment_payment_enabled: false,
      },
      {
        payment_method_type: "google_pay",
        minimum_amount: 0,
        maximum_amount: 68607706,
        recurring_enabled: false,
        installment_payment_enabled: false,
      },
      {
        payment_method_type: "paypal",
        minimum_amount: 0,
        maximum_amount: 68607706,
        recurring_enabled: false,
        installment_payment_enabled: false,
      },
    ],
  },
];
