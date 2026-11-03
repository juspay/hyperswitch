import { customerAcceptance } from "./Commons";
import { getCustomExchange } from "./Modifiers";

// RealEx sandbox test cards; the decline PAN always declines.
const successfulNo3DSCardDetails = {
  card_number: "4263970000005262",
  card_exp_month: "10",
  card_exp_year: "30",
  card_holder_name: "John Doe",
  card_cvc: "123",
};

const successfulThreeDSTestCardDetails = {
  card_number: "4263970000005262",
  card_exp_month: "10",
  card_exp_year: "30",
  card_holder_name: "John Doe",
  card_cvc: "123",
};

const declinedCardDetails = {
  card_number: "4000120000001154",
  card_exp_month: "10",
  card_exp_year: "30",
  card_holder_name: "John Doe",
  card_cvc: "123",
};

const threeDSNotConfiguredMessage =
  "That Account ID is not configured for 3D Secure";

export const connectorDetails = {
  card_pm: {
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
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          shipping_cost: 50,
          amount: 6000,
          net_amount: 6050,
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
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    No3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    "3DSAutoCapture": {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_message: threeDSNotConfiguredMessage,
        },
      },
    },
    "3DSManualCapture": {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_message: threeDSNotConfiguredMessage,
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
    // No multi-capture: amount_capturable drops to 0 after any capture.
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
    // RealEx allows one refund per payment (512 on a second); specs refund twice.
    PartialRefund: {
      Configs: { TRIGGER_SKIP: true },
      Request: {
        amount: 3000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          amount: 3000,
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
    manualPaymentRefund: {
      Request: {
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          amount: 6000,
        },
      },
    },
    // One refund per payment, same as PartialRefund.
    manualPaymentPartialRefund: {
      Configs: { TRIGGER_SKIP: true },
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
    SyncPayment: {
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    No3DSFailPayment: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: declinedCardDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_code: "101",
          error_message: "[ test system ] Declined",
        },
      },
    },
    // SetupMandate/RepeatPayment are `not_implemented` for this connector.
    MandateSingleUseNo3DSAutoCapture: {
      Configs: { TRIGGER_SKIP: true },
      Request: {},
      Response: { status: 200, body: { status: "failed" } },
    },
    MandateMultiUseNo3DSAutoCapture: {
      Configs: { TRIGGER_SKIP: true },
      Request: {},
      Response: { status: 200, body: { status: "failed" } },
    },
    MandateSingleUseNo3DSManualCapture: {
      Configs: { TRIGGER_SKIP: true },
      Request: {},
      Response: { status: 200, body: { status: "failed" } },
    },
    MandateMultiUseNo3DSManualCapture: {
      Configs: { TRIGGER_SKIP: true },
      Request: {},
      Response: { status: 200, body: { status: "failed" } },
    },
    MITAutoCapture: getCustomExchange({
      Configs: { TRIGGER_SKIP: true },
      Request: { amount: 6000 },
    }),
    MITAutoCaptureWithCustomerAcceptance: getCustomExchange({
      Configs: { TRIGGER_SKIP: true },
      Request: {
        amount: 6000,
        customer_acceptance: {
          acceptance_type: "offline",
          accepted_at: "1963-05-03T04:07:52.723Z",
          online: {
            ip_address: "127.0.0.1",
            user_agent: "amet irure esse",
          },
        },
      },
    }),
    MITManualCapture: getCustomExchange({
      Configs: { TRIGGER_SKIP: true },
      Request: { amount: 6000 },
    }),
    MITWithoutBillingAddress: getCustomExchange({
      Configs: { TRIGGER_SKIP: true },
      Request: { amount: 6000, billing: null },
    }),
    PaymentMethodIdMandateNo3DSAutoCapture: getCustomExchange({
      Configs: { TRIGGER_SKIP: true },
      Request: {
        amount: 6000,
        payment_method: "card",
        payment_method_data: { card: successfulNo3DSCardDetails },
        currency: "USD",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
      },
    }),
    PaymentMethodIdMandateNo3DSManualCapture: getCustomExchange({
      Configs: { TRIGGER_SKIP: true },
      Request: {
        amount: 6000,
        payment_method: "card",
        payment_method_data: { card: successfulNo3DSCardDetails },
        currency: "USD",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
      },
    }),
    PaymentMethodIdMandate3DSAutoCapture: getCustomExchange({
      Configs: { TRIGGER_SKIP: true },
      Request: {
        amount: 6000,
        payment_method: "card",
        payment_method_data: { card: successfulThreeDSTestCardDetails },
        currency: "USD",
        mandate_data: null,
        authentication_type: "three_ds",
        customer_acceptance: customerAcceptance,
      },
    }),
    PaymentMethodIdMandate3DSManualCapture: getCustomExchange({
      Configs: { TRIGGER_SKIP: true },
      Request: {
        amount: 6000,
        payment_method: "card",
        payment_method_data: { card: successfulThreeDSTestCardDetails },
        mandate_data: null,
        authentication_type: "three_ds",
        customer_acceptance: customerAcceptance,
      },
    }),
    SaveCardUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: { card: successfulNo3DSCardDetails },
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
      },
      Response: { status: 200, body: { status: "succeeded" } },
    },
    SaveCardUseNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: { card: successfulNo3DSCardDetails },
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
      },
      Response: { status: 200, body: { status: "requires_capture" } },
    },
    // Off-session save-card needs RepeatPayment, which isn't implemented.
    SaveCardUseNo3DSAutoCaptureOffSession: getCustomExchange({
      Configs: { TRIGGER_SKIP: true },
      Request: {
        payment_method: "card",
        payment_method_data: { card: successfulNo3DSCardDetails },
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
      },
    }),
    SaveCardUseNo3DSManualCaptureOffSession: getCustomExchange({
      Configs: { TRIGGER_SKIP: true },
      Request: {
        payment_method: "card",
        payment_method_data: { card: successfulNo3DSCardDetails },
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
      },
    }),
    SaveCardConfirmAutoCaptureOffSession: getCustomExchange({
      Configs: { TRIGGER_SKIP: true },
      Request: { setup_future_usage: "off_session" },
    }),
    SaveCardConfirmManualCaptureOffSession: getCustomExchange({
      Configs: { TRIGGER_SKIP: true },
      Request: { setup_future_usage: "off_session" },
    }),
    // 3DS hits the same account-level 3DS block as 3DSAutoCapture.
    SaveCardUse3DSAutoCaptureOffSession: {
      Request: {
        payment_method: "card",
        payment_method_data: { card: successfulThreeDSTestCardDetails },
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_message: threeDSNotConfiguredMessage,
        },
      },
    },
    ZeroAuthMandate: {
      Configs: { TRIGGER_SKIP: true },
      Request: {},
      Response: { status: 200, body: { status: "processing" } },
    },
    ZeroAuthPaymentIntent: {
      Configs: { TRIGGER_SKIP: true },
      Request: { amount: 0, currency: "USD" },
      Response: { status: 200, body: { status: "requires_payment_method" } },
    },
  },
};
