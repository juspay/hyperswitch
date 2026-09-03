import { getCustomExchange } from "./Modifiers";

const billingAddressGB = {
  address: {
    line1: "1 Principal Place",
    city: "London",
    zip: "EC2A 2FA",
    country: "GB",
    first_name: "John",
    last_name: "Doe",
  },
  email: "test@example.com",
  phone: {
    number: "7123456789",
    country_code: "+44",
  },
};

export const connectorDetails = {
  bank_redirect_pm: {
    // Required supported-method config for the Truelayer confirm flow. The CI
    // selection runs only this bank redirect flow for Truelayer.
    Truelayer: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "open_banking",
        payment_method_data: {
          bank_redirect: {
            open_banking: {},
          },
        },
        currency: "GBP", // Truelayer requires GBP for UK open banking payments.
        billing: billingAddressGB,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method_type: "open_banking",
          connector: "truelayer",
        },
      },
      Configs: {
        skipPaymentMethodStatusAssertion: true,
      },
    }),
    TruelayerNoBilling: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "open_banking",
        payment_method_data: {
          bank_redirect: {
            open_banking: {},
          },
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            // UCS maps missing top-level billing to the shared IR_04 error.
            message: "Missing required param: billing",
            code: "IR_04",
          },
        },
      },
    }),
  },
};
