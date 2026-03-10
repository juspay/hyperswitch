import {
  customerAcceptance,
  multiUseMandateData,
  singleUseMandateData,
} from "./Commons";
import { getCustomExchange } from "./Modifiers";

// Test card details for successful non-3DS transactions
const successfulNo3DSCardDetails = {
  card_number: "4111111111111111",
  card_exp_month: "12",
  card_exp_year: "2030",
  card_holder_name: "John Doe",
  card_cvc: "123",
};

// Test card details for failed transactions
const failedNo3DSCardDetails = {
  ...successfulNo3DSCardDetails,
  card_number: "4005550000000019",
};

// Payment method card details
const PaymentMethodCardDetails = {
  card_number: "4111111111111111",
  card_exp_month: "03",
  card_exp_year: "30",
  card_holder_name: "John Doe",
};

// Payment method data structure for non-3DS card payments
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
    card_holder_name: "John Doe",
    payment_checks: null,
    authentication_data: null,
  },
  billing: null,
};

// Standard billing address for US payments
const billingAddressUS = {
  address: {
    line1: "123 Payment Street",
    line2: "Apt 456",
    line3: "District 7",
    city: "New York",
    state: "NY",
    zip: "10001",
    country: "US",
    first_name: "John",
    last_name: "Doe",
  },
  phone: {
    number: "1234567890",
    country_code: "+1",
  },
  email: "john.doe@example.com",
};

const refundErrorResponse = {
  code: "IR_14",
  message:
    "This Payment could not be refund because it has a status of processing. The expected state is succeeded, partially_captured",
  type: "invalid_request",
};

const threeDsNotSupportedResponse = {
  error: {
    type: "invalid_request",
    message: "Three_ds payments through Forte are not implemented",
    code: "IR_00",
  },
};
export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: billingAddressUS,
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
        currency: "USD",
        customer_acceptance: null,
        amount: 6000,
        authentication_type: "no_three_ds",
        setup_future_usage: "off_session",
        billing: billingAddressUS,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          setup_future_usage: "off_session",
        },
      },
    },
    PaymentIntentWithShippingCost: {
      Request: {
        currency: "USD",
        shipping_cost: 50,
        billing: billingAddressUS,
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
        billing: billingAddressUS,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
          shipping_cost: 50,
          amount_received: null,
          amount: 6000,
          net_amount: 6050,
        },
      },
    },
    // 3DS flows for Forte - Not supported
    "3DSManualCapture": {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: billingAddressUS,
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
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: billingAddressUS,
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
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: billingAddressUS,
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
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: billingAddressUS,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
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
        billing: billingAddressUS,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
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
          status: "processing",
          amount: 6000,
          amount_capturable: 6000,
          amount_received: null,
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
          status: "processing",
          amount: 6000,
          amount_capturable: 6000,
          amount_received: null,
        },
      },
    },
    Void: getCustomExchange({
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
          code: "IR_16",
          message:
            "You cannot cancel this payment because it has status processing",
          type: "invalid_request",
        },
      },
    }),
    VoidAfterConfirm: {
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    Refund: {
      Request: {
        amount: 6000,
      },
      Response: {
        status: 400,
        body: {
          error: refundErrorResponse,
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
          status: "failed",
          error_message:
            "A reverse action can only be performed on original transaction that are of sale action type.",
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
          status: "failed",
          error_message:
            "A reverse action can only be performed on original transaction that are of sale action type.",
        },
      },
    },
    PartialRefund: {
      Request: {
        amount: 2000,
      },
      Response: {
        status: 400,
        body: {
          error: refundErrorResponse,
        },
      },
    },
    SyncRefund: {
      Response: {
        status: 400,
        body: {
          error: refundErrorResponse,
        },
      },
    },
    // Payments will go processing state, so card wont be saved hence skipping the tests
    SaveCardUseNo3DSAutoCapture: {
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
        billing: billingAddressUS,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
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
        currency: "USD",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
        billing: billingAddressUS,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          setup_future_usage: "on_session",
        },
      },
    },
    // 3DS Save Card flows - Not implemented
    SaveCardUse3DSAutoCapture: {
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
        billing: billingAddressUS,
      },
      Response: {
        status: 501,
        body: threeDsNotSupportedResponse,
      },
    },
    SaveCardUse3DSManualCapture: {
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
        billing: billingAddressUS,
      },
      Response: {
        status: 501,
        body: threeDsNotSupportedResponse,
      },
    },
    // Payment method mandate flows - Skipped for Forte
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
        billing: billingAddressUS,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
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
        currency: "USD",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
        billing: billingAddressUS,
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
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        authentication_type: "three_ds",
        customer_acceptance: customerAcceptance,
        billing: billingAddressUS,
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
          card: successfulNo3DSCardDetails,
        },
        mandate_data: null,
        authentication_type: "three_ds",
        customer_acceptance: customerAcceptance,
        billing: billingAddressUS,
      },
      Response: {
        status: 501,
        body: threeDsNotSupportedResponse,
      },
    },

    // Mandate flows - Skipped for Forte
    MandateSingleUse3DSAutoCapture: {
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
        billing: billingAddressUS,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
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
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
        billing: billingAddressUS,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
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
        mandate_data: singleUseMandateData,
        billing: billingAddressUS,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
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
        currency: "USD",
        mandate_data: singleUseMandateData,
        billing: billingAddressUS,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
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
        billing: billingAddressUS,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
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
        billing: billingAddressUS,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
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
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: multiUseMandateData,
        billing: billingAddressUS,
      },
      Response: {
        status: 501,
        body: {
          error: {
            type: "invalid_request",
            message: "Mandate payments through Forte are not supported",
            code: "IR_00",
          },
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
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: multiUseMandateData,
        billing: billingAddressUS,
      },
      Response: {
        status: 501,
        body: {
          error: {
            type: "invalid_request",
            message: "Mandate payments through Forte are not supported",
            code: "IR_00",
          },
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
        mandate_data: singleUseMandateData,
        billing: billingAddressUS,
      },
      Response: {
        status: 501,
        body: {
          error: {
            type: "invalid_request",
            message: "Setup Mandate flow for Forte is not implemented",
            code: "IR_00",
          },
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
        currency: "USD",
        billing: billingAddressUS,
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
        billing: billingAddressUS,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
          setup_future_usage: "on_session",
        },
      },
    },
    MITAutoCapture: {
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "processing",
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
        currency: "USD",
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
        billing: billingAddressUS,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
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
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
        billing: billingAddressUS,
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
        currency: "USD",
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
        billing: billingAddressUS,
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
        billing: billingAddressUS,
        card_cvc: "123",
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
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
        billing: billingAddressUS,
        card_cvc: "123",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
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
        card_cvc: "123",
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
          setup_future_usage: "on_session",
        },
      },
    },
    PaymentMethod: {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_issuer: "Gpay",
        payment_method_issuer_code: "us_forte",
        card: PaymentMethodCardDetails,
      },
      Response: {
        status: 200,
        body: {},
      },
    },
    CaptureCapturedAmount: {
      Request: {
        Request: {
          amount_to_capture: 6000,
        },
      },
      Response: {
        status: 400,
        body: {
          code: "IR_14",
          message:
            "This Payment could not be captured because it has a capture_method of automatic. The expected state is manual_multiple",
          type: "invalid_request",
        },
      },
    },
    ConfirmSuccessfulPayment: {
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
            message:
              "You cannot confirm this payment because it has status processing",
            code: "IR_16",
          },
        },
      },
    },
    RefundGreaterAmount: {
      Request: {
        amount: 6000000,
      },
      Response: {
        status: 400,
        body: {
          error: refundErrorResponse,
        },
      },
    },
  },
};
