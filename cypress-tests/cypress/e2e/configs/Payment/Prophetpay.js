import { getCustomExchange } from "./Modifiers";

export const connectorDetails = {
  card_redirect_pm: {
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
    CardRedirect: getCustomExchange({
      Request: {
        payment_method: "card_redirect",
        payment_method_type: "card_redirect",
        payment_method_data: {
          card_redirect: {
            card_redirect: {},
          },
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
    Refund: getCustomExchange({
      Request: {
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
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
    "3DSAutoCapture": getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
    }),
    "3DSManualCapture": getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
    }),
    No3DSManualCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
    }),
    MandateSingleUse3DSAutoCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
    }),
    MandateSingleUse3DSManualCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
    }),
    MandateSingleUseNo3DSAutoCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
    }),
    MandateSingleUseNo3DSManualCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
    }),
    MandateMultiUseNo3DSAutoCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
    }),
    MandateMultiUseNo3DSManualCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
    }),
    ZeroAuthMandate: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
    }),
    ZeroAuthPaymentIntent: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
    }),
  },
};
