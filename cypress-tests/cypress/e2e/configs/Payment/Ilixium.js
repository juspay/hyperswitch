import { getCustomExchange } from "./Modifiers";

const verifiedCardDetails = {
  card_number: "9000100111111117",
  card_exp_month: "06",
  card_exp_year: "28",
  card_holder_name: "John Doe",
  card_cvc: "111",
};

const threeDsCardDetails = {
  card_number: "9001100511111112",
  card_exp_month: "06",
  card_exp_year: "28",
  card_holder_name: "John Doe",
  card_cvc: "111",
};

const ilixiumMetadata = {
  ilixium_date_of_birth: "01011990",
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "USD",
        amount: 1000,
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    No3DSManualCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        amount: 1000,
        payment_method_data: {
          card: verifiedCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        metadata: ilixiumMetadata,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    "3DSManualCapture": getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        amount: 1000,
        payment_method_data: {
          card: threeDsCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        authentication_type: "three_ds",
        metadata: ilixiumMetadata,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
    "3DSAutoCapture": getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        amount: 1000,
        payment_method_data: {
          card: threeDsCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        authentication_type: "three_ds",
        metadata: ilixiumMetadata,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_message: "4",
          error_code: "4",
        },
      },
    }),
    No3DSAutoCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        amount: 1000,
        payment_method_data: {
          card: verifiedCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        metadata: ilixiumMetadata,
      },
      // Creds we currently have only supports manual capture. Therefore mapped error code for auto capture.
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_message: "4",
          error_code: "4",
        },
      },
    }),
    PaymentConfirmWithShippingCost: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: verifiedCardDetails,
        },
        customer_acceptance: null,
        metadata: ilixiumMetadata,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_message: "4",
          error_code: "4",
        },
      },
    }),
    Capture: getCustomExchange({
      Request: {
        amount_to_capture: 1000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          amount: 1000,
          amount_capturable: 0,
          amount_received: 1000,
        },
      },
    }),
    PartialCapture: getCustomExchange({
      Request: {
        amount_to_capture: 500,
      },
      Response: {
        status: 200,
        body: {
          status: "partially_captured",
          amount: 1000,
          amount_capturable: 0,
          amount_received: 500,
        },
      },
    }),
    Void: {
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "cancelled",
        },
      },
    },
    No3DSFailPayment: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: verifiedCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        metadata: ilixiumMetadata,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_message: "4",
          error_code: "4",
        },
      },
    }),
    SaveCardUseNo3DSAutoCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: verifiedCardDetails,
        },
        currency: "USD",
        setup_future_usage: "on_session",
        metadata: ilixiumMetadata,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_message: "4",
          error_code: "4",
        },
      },
    }),
    SaveCardUseNo3DSAutoCaptureOffSession: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: verifiedCardDetails,
        },
        setup_future_usage: "off_session",
        metadata: ilixiumMetadata,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_message: "4",
          error_code: "4",
        },
      },
    }),
    SaveCardUse3DSAutoCaptureOffSession: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: threeDsCardDetails,
        },
        setup_future_usage: "off_session",
        metadata: ilixiumMetadata,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_message: "4",
          error_code: "4",
        },
      },
    }),
    SaveCardUseNo3DSManualCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: verifiedCardDetails,
        },
        currency: "USD",
        setup_future_usage: "on_session",
        metadata: ilixiumMetadata,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    }),
    SaveCardUseNo3DSManualCaptureOffSession: getCustomExchange({
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: verifiedCardDetails,
        },
        amount: 1000,
        setup_future_usage: "off_session",
        metadata: ilixiumMetadata,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    }),
    SaveCardConfirmManualCaptureOffSession: getCustomExchange({
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        setup_future_usage: "off_session",
      },
      Response: {
        status: 400,
        body: {
          error: {
            message:
              "No eligible connector was found for the current payment method configuration",
            type: "invalid_request",
          },
        },
      },
    }),
    manualPaymentRefund: getCustomExchange({
      Request: {
        amount: 500,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),
    manualPaymentPartialRefund: getCustomExchange({
      Request: {
        amount: 200,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
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
    PaymentMethodIdMandateNo3DSAutoCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: verifiedCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        metadata: ilixiumMetadata,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_message: "4",
          error_code: "4",
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
          card: verifiedCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        metadata: ilixiumMetadata,
      },
    }),
    PaymentMethodIdMandate3DSAutoCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: threeDsCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        metadata: ilixiumMetadata,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_message: "4",
          error_code: "4",
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
          card: threeDsCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        metadata: ilixiumMetadata,
      },
    }),
  },
};
