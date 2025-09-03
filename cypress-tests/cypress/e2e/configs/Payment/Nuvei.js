import { customerAcceptance } from "./Commons";
import { getCurrency } from "./Modifiers";

const successfulNo3DSCardDetails = {
  card_number: "4111111111111111",
  card_exp_month: "08",
  card_exp_year: "30",
  card_holder_name: "joseph Doe",
  card_cvc: "999",
};
// Test card details based on Nuvei test cards (from Rust tests)
const successfulThreeDSCardDetails = {
  card_number: "4000027891380961",
  card_exp_month: "10",
  card_exp_year: "25",
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
// Payment method data objects for responses
const payment_method_data_no3ds = {
  card: {
    authentication_data: {},
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
    payment_checks: {
      avs_description: null,
      avs_result_code: "",
      cvv_2_reply_code: "",
      cvv_2_description: null,
      merchant_advice_code: "",
      merchant_advice_code_description: null,
    },
  },
  billing: null,
};

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
          payment_method_data: payment_method_data_no3ds,
        },
      },
    },
    // No 3DS manual capture
    No3DSManualCapture: {
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
        },
      },
    },
    // Capture payment
    Capture: {
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
          payment_method_data: payment_method_data_no3ds,
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
        currency: "EUR", // iDEAL requires EUR currency
        payment_method_data: {
          bank_redirect: {
            ideal: {
              bank_name: "ing", // Maps to INGBNL2A in Nuvei
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
            country: "NL", // Netherlands required for iDEAL
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
          status: "requires_customer_action", // Bank redirect requires customer action
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
        currency: "EUR", // EPS requires EUR currency
        payment_method_data: {
          bank_redirect: {
            eps: {
              country: "AT", // Austria required for EPS
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
            country: "AT", // Austria required for EPS
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
        },
      },
    },
  },
};
