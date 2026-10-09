import { getCustomExchange } from "./Modifiers";

const billingAddressGB = {
  address: {
    line1: "1467 Harrison Street",
    city: "London",
    zip: "EC1A 1BB",
    country: "GB",
    first_name: "John",
    last_name: "Doe",
  },
  phone: {
    number: "9999999999",
    country_code: "+44",
  },
};

export const connectorDetails = {
  open_banking_pm: {
    PaymentIntent: {
      Request: {
        currency: "GBP",
        customer_acceptance: null,
        setup_future_usage: "off_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    OpenBankingPIS: getCustomExchange({
      Request: {
        payment_method: "open_banking",
        payment_method_type: "open_banking_pis",
        payment_method_data: {
          open_banking: {
            open_banking_pis: {},
          },
        },
        currency: "GBP",
        billing: billingAddressGB,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method_type: "open_banking_pis",
          connector: "plaid",
        },
      },
    }),
    PostSessionTokens: getCustomExchange({
      Request: {
        payment_method_type: "open_banking_pis",
        payment_method: "open_banking",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
    OpenBankingPISNoBilling: getCustomExchange({
      Request: {
        payment_method: "open_banking",
        payment_method_type: "open_banking_pis",
        payment_method_data: {
          open_banking: {
            open_banking_pis: {},
          },
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            code: "IR_04",
          },
        },
      },
    }),
    SyncPayment: getCustomExchange({
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method_type: "open_banking_pis",
          connector: "plaid",
        },
      },
    }),
  },
};
