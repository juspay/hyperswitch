import { standardBillingAddress } from "./Commons";
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
      Configs: {
        TRIGGER_SKIP: true,
      },
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
        billing: standardBillingAddress,
      },
    }),
    Refund: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
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
          status: "pending",
        },
      },
    },
    SyncRefund: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
  },
};