import {
  customerAcceptance,
  multiUseMandateData,
  singleUseMandateData,
} from "./Commons";

const successfulNo3DSCardDetails = {
  card_number: "4242424242424242", // Non-3DS PowerTranz test card
  card_exp_month: "12",
  card_exp_year: "30",
  card_holder_name: "John Smith",
  card_cvc: "123",
};

const successfulThreeDSTestCardDetails = {
  card_number: "4012000000020071", // 3DS PowerTranz test card
  card_exp_month: "12",
  card_exp_year: "30",
  card_holder_name: "John Smith",
  card_cvc: "123",
};

const failedNo3DSCardDetails = {
  card_number: "5555666666662222", // Decline test card
  card_exp_month: "12",
  card_exp_year: "30",
  card_holder_name: "John Smith",
  card_cvc: "123",
};

const payment_method_data_3ds = {
  card: {
    last4: "0071",
    card_type: "DEBIT",
    card_network: "Visa",
    card_issuer: "VISA PRODUCTION SUPPORT CLIENT BID 1",
    card_issuing_country: "UNITEDSTATES",
    card_isin: "401200",
    card_extended_bin: null,
    card_exp_month: "12",
    card_exp_year: "30",
    card_holder_name: "John Smith",
    payment_checks: null,
    authentication_data: null,
  },
  billing: null,
};

const payment_method_data_no3ds = {
  card: {
    last4: "4242",
    card_type: "CREDIT",
    card_network: "Visa",
    card_issuer: "STRIPE PAYMENTS UK LIMITED",
    card_issuing_country: "UNITEDKINGDOM",
    card_isin: "424242",
    card_extended_bin: null,
    card_exp_month: "12",
    card_exp_year: "30",
    card_holder_name: "John Smith",
    payment_checks: null,
    authentication_data: null,
  },
  billing: null,
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
    PaymentIntentOffSession: {
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
    "3DSManualCapture": {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        authentication_type: "three_ds",
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
    },
    "3DSAutoCapture": {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        authentication_type: "three_ds",
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
    },

    No3DSManualCapture: {
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
          error_code: "05",
          error_message: "Do Not Honor",
          unified_code: "UE_9000",
          unified_message: "Something went wrong",
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
          status: "failed",
          amount: 6000,
          amount_capturable: 6000,
          error_code: "12",
          error_message: "Partial capture not allowed",
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
      Configs: {
        TRIGGER_SKIP: true,
      },
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
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        payment_type: "setup_mandate",
      },
      Response: {
        status: 501,
        body: {
          error: {
            code: "IR_00",
            message: "Setup Mandate flow for Powertranz is not implemented",
            type: "invalid_request",
          },
        },
      },
    },
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
    MandateSingleUse3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
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
        currency: "USD",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
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
        currency: "USD",
        mandate_data: multiUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
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
        currency: "USD",
        mandate_data: multiUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    PaymentMethodIdMandateNo3DSAutoCapture: {
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
          card: successfulThreeDSTestCardDetails,
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
    SaveCardConfirmAutoCaptureOffSessionWithoutBilling: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        setup_future_usage: "off_session",
        billing: null,
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
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        authentication_type: "three_ds",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
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
          card: successfulThreeDSTestCardDetails,
        },
        mandate_data: null,
        authentication_type: "three_ds",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
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
        status: 501,
        body: {
          error: {
            code: "IR_00",
            message: "Setup Mandate flow for Powertranz is not implemented",
            type: "invalid_request",
          },
        },
      },
    },
  },
};
