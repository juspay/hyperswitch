import { getCustomExchange } from "./Modifiers";

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
        },
      },
    },
    No3DSAutoCapture: getCustomExchange({
      Configs: {
        ASSERT_BILLING_NOT_NULL: false,
        TRIGGER_SKIP: true, // Dwolla is an ACH/bank transfer processor, card payments not supported
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: {
            card_number: "4111111111111111",
            card_exp_month: "08",
            card_exp_year: "30",
            card_holder_name: "joseph Doe",
            card_cvc: "999",
          },
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),
    No3DSManualCapture: getCustomExchange({
      Configs: {
        ASSERT_BILLING_NOT_NULL: false,
        TRIGGER_SKIP: true, // Dwolla is an ACH/bank transfer processor, card payments not supported
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: {
            card_number: "4111111111111111",
            card_exp_month: "08",
            card_exp_year: "30",
            card_holder_name: "joseph Doe",
            card_cvc: "999",
          },
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    }),
    Capture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Card payments not supported by Dwolla
      },
      Request: {
        amount_to_capture: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),
    PartialCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Card payments not supported by Dwolla
      },
      Request: {
        amount_to_capture: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "partially_captured",
        },
      },
    }),
    Refund: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Card payments not supported by Dwolla
      },
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
      Configs: {
        TRIGGER_SKIP: true, // Card payments not supported by Dwolla
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
    }),
    manualPaymentRefund: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Card payments not supported by Dwolla
      },
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
    manualPaymentPartialRefund: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Card payments not supported by Dwolla
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
    }),
    SyncRefund: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Card payments not supported by Dwolla
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),
  },
};
