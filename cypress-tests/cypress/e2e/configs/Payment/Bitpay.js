import { standardBillingAddress } from "./Commons";

export const connectorDetails = {
  crypto_pm: {
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
    CryptoCurrency: {
      Request: {
        payment_method: "crypto",
        payment_method_type: "crypto_currency",
        payment_method_data: {
          crypto: {
            network: "bitcoin",
            pay_currency: "BTC",
          },
        },
        billing: standardBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    Refund: {
      Response: {
        status: 501,
        body: {
          error: {
            type: "invalid_request",
            message: "Refund flow not Implemented",
            code: "HE_00",
          },
        },
      },
    },
  },
  card_pm: {
    ZeroAuthMandate: {
      Response: {
        status: 501,
        body: {
          error: {
            type: "invalid_request",
            message: "Setup Mandate flow for Bitpay is not implemented",
            code: "IR_00",
          },
        },
      },
    },
  },
};
