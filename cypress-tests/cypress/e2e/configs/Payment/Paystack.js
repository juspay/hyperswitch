import { getCustomExchange } from "./Modifiers";

const bankRedirectEftData = {
  payment_method: "bank_redirect",
  payment_method_type: "eft",
  payment_method_data: {
    bank_redirect: {
      eft: {
        provider: "ozow",
      },
    },
  },
};

export const connectorDetails = {
  bank_redirect_pm: {
    PaymentIntent: (paymentMethodType) => {
      const currencyMap = { Eft: "USD" };
      return getCustomExchange({
        Request: {
          currency: currencyMap[paymentMethodType] || "USD",
        },
        Response: {
          status: 200,
          body: {
            status: "requires_payment_method",
          },
        },
      });
    },
    Eft: getCustomExchange({
      Request: {
        ...bankRedirectEftData,
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
  },
};
