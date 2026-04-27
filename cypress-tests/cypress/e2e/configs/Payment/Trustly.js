import { getCustomExchange } from "./Modifiers";

export const connectorDetails = {
  bank_redirect_pm: {
    PaymentIntent: (_paymentMethodType) =>
      getCustomExchange({
        Request: {
          currency: "EUR",
        },
        Response: {
          status: 200,
          body: {
            status: "requires_payment_method",
          },
        },
      }),
    Trustly: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "trustly",
        payment_method_data: {
          bank_redirect: {
            trustly: {
              country: "NL",
            },
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
    Refund: getCustomExchange({
      Request: {
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
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
          status: "pending",
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
