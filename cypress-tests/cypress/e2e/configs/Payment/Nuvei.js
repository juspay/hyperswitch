import { customerAcceptance } from "./Commons";
import { getCurrency } from "./Modifiers";

const successfulNo3DSCardDetails = {
  card_number: "4444333322221111",
  card_exp_month: "12",
  card_exp_year: "30",
  card_holder_name: "joseph Doe",
  card_cvc: "123",
};
// Test card details based on Nuvei test cards (from Rust tests)
const successfulThreeDSCardDetails = {
  card_number: "4000027891380961",
  card_exp_month: "10",
  card_exp_year: "30",
  card_holder_name: "CL-BRW1",
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

// Billing address for manual capture flows
const billingAddress = {
  address: {
    line1: "1467",
    line2: "Harrison Street",
    line3: "Harrison Street",
    city: "San Francisco",
    state: "CA",
    zip: "94122",
    country: "US",
    first_name: "John",
    last_name: "Doe",
  },
  phone: {
    number: "9123456789",
    country_code: "+1",
  },
  email: "test@example.com",
};

// Note: payment_method_data object removed as Nuvei returns dynamic card metadata
// Tests validate that payment_method_data exists and is not empty (via commands.js)

export const connectorDetails = {
  card_pm: {
    // Basic payment intent creation
    PaymentIntent: {
      Request: {
        currency: "USD",
        amount: 11500,
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
    },
    // Payment intent with shipping cost
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
          shipping_cost: 50,
          amount_received: 11550,
          amount: 11500,
          net_amount: 11550,
        },
      },
    },
    // No 3DS automatic capture
    No3DSAutoCapture: {
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
        },
      },
    },
    // No 3DS manual capture
    No3DSManualCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        billing: billingAddress,
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
          // payment_method_data removed - Nuvei returns dynamic card metadata (issuer, country) that varies per transaction
        },
      },
    },
    // 3DS automatic capture
    "3DSAutoCapture": {
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
          // we are removing payment_method_data from the response as authentication_data is different every time.
        },
      },
    },
    // 3DS manual capture
    "3DSManualCapture": {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSCardDetails,
        },
        billing: billingAddress,
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
        },
      },
    },
    // Capture payment
    Capture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
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
    },
    // Partial capture
    PartialCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
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
    },
    // Void payment
    Void: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
      },
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "cancelled",
        },
      },
    },
    // Refund payment
    Refund: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        amount: 11500,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    // Partial refund - Nuvei limitation: only supports single refund per sale
    // Note: Tests that attempt multiple partial refunds will fail after the first one
    // TRIGGER_SKIP is used to skip tests that would fail due to Nuvei's "only one refund per sale" limitation
    PartialRefund: {
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
    },
    // Manual payment refund
    manualPaymentRefund: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        amount: 11500,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    // Manual payment partial refund - Nuvei limitation: only supports single refund per sale
    // Note: Tests that attempt multiple partial refunds will fail after the first one
    // TRIGGER_SKIP is used to skip tests that would fail due to Nuvei's "only one refund per sale" limitation
    manualPaymentPartialRefund: {
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
    },
    // Sync refund
    SyncRefund: {
      Response: {
        status: 200,
        body: {
          status: "succeeded",
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
        currency: "USD",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    ZeroAuthPaymentIntent: {
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
        currency: "USD",
        customer_acceptance: customerAcceptance,
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
        TRIGGER_SKIP: true,
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
        currency: "USD",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
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
        currency: "USD",
        mandate_data: multiUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          // Note: payment_method_data removed from response validation as Nuvei returns dynamic card metadata
          payment_method: "card",
        },
      },
    },
    MITAutoCapture: {
      Request: {
        amount_to_capture: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    // Save card scenarios
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
          status: "succeeded",
        },
      },
    },
    SaveCardUseNo3DSManualCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        billing: billingAddress,
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
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
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
    SaveCardConfirmAutoCaptureOffSession: {
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
    },
    SaveCardConfirmManualCaptureOffSession: {
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
    },
    // Payment method ID mandate scenarios
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
    PaymentMethodIdMandateNo3DSManualCapture: {
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
    },
    PaymentMethodIdMandate3DSAutoCapture: {
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
    },
    PaymentMethodIdMandate3DSManualCapture: {
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
    },
  },
  // Bank redirect payment methods
  bank_redirect_pm: {
    PaymentIntent: (paymentMethodType) => ({
      Request: {
        amount: 11500,
        currency: getCurrency(paymentMethodType),
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
    BlikPaymentIntent: {
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
    },
    Blik: {
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
            message:
              "No eligible connector was found for the current payment method configuration",
            code: "IR_39",
          },
        },
      },
    },
    Ideal: {
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "ideal",
        currency: "EUR",
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
          status: "requires_customer_action",
          error_code: null,
          error_message: null,
        },
      },
    },
    Giropay: {
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "giropay",
        currency: "EUR", // Giropay requires EUR currency
        payment_method_data: {
          bank_redirect: {
            giropay: {
              country: "DE", // Germany required for Giropay
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
            country: "DE", // Germany required for Giropay
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
          status: "requires_customer_action", // Bank redirect requires customer action
          error_code: null,
          error_message: null,
        },
      },
    },
    Sofort: {
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "sofort",
        currency: "EUR", // Sofort requires EUR currency
        payment_method_data: {
          bank_redirect: {
            sofort: {
              country: "DE", // Germany required for Sofort
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
            state: "Berlin",
            zip: "10115",
            country: "DE", // Germany required for Sofort
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
          status: "requires_customer_action", // Bank redirect requires customer action
          error_code: null,
          error_message: null,
        },
      },
    },
    Eps: {
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "eps",
        amount: 11500,
        currency: "EUR",
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
          status: "requires_customer_action",
          error_code: null,
          error_message: null,
        },
      },
    },
  },
};
