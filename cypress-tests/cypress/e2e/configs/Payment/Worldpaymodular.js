import { getCustomExchange } from "./Modifiers";

const successfulWalletData = {
  apple_pay: {},
  google_pay: {},
};

export const connectorDetails = {
  wallet_pm: {
    PaymentIntent: getCustomExchange({
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
    }),
    GooglePay: getCustomExchange({
      Request: {
        payment_method: "wallet",
        payment_method_type: "google_pay",
        payment_method_data: {
          wallet: {
            google_pay_redirect: {},
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
    ApplePay: getCustomExchange({
      Request: {
        payment_method: "wallet",
        payment_method_type: "apple_pay",
        payment_method_data: {
          wallet: {
            apple_pay_redirect: {},
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
  },
};
