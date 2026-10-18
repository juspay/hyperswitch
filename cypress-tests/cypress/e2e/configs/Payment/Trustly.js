import { getCustomExchange } from "./Modifiers";

const billingAddressSE = {
  address: {
    line1: "1 Main St",
    city: "Stockholm",
    zip: "11122",
    country: "SE",
    first_name: "John",
    last_name: "Doe",
  },
  email: "test@example.com",
  phone: {
    number: "9123456789",
    country_code: "+46",
  },
};

export const connectorDetails = {
  bank_redirect_pm: {
    // Required supported-method config for the Trustly confirm flow. The CI
    // selection runs only this bank redirect flow for Trustly.
    Trustly: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "trustly",
        payment_method_data: {
          bank_redirect: {
            trustly: {
              country: "SE",
            },
          },
        },
        currency: "EUR", // Trustly requires EUR for Swedish bank redirect payments.
        billing: billingAddressSE,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method_type: "trustly",
          connector: "trustly",
        },
      },
      Configs: {
        skipPaymentMethodStatusAssertion: true,
      },
    }),
  },
};
