import { getCustomExchange } from "./Modifiers";

const billingAddress = {
  address: {
    line1: "1467",
    line2: "Harrison Street",
    line3: "Harrison Street",
    city: "San Fransico",
    state: "California",
    zip: "94122",
    country: "GB",
    first_name: "john",
    last_name: "doe",
  },
};

export const connectorDetails = {
  bank_redirect_pm: {
    OpenBankingUk: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        amount: 6000,
        currency: "GBP",
        payment_method_type: "open_banking_uk",
        payment_method_data: {
          bank_redirect: {
            open_banking_uk: {
              issuer: "citi",
              country: "GB",
            },
          },
        },
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method_type: "open_banking_uk",
          payment_method_type_display_name: "Open Banking",
          connector: "volt",
        },
      },
    }),
  },
};
