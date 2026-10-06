import { getCustomExchange } from "./Modifiers";
import { standardBillingAddress } from "./Commons";

export const connectorDetails = {
  wallet_pm: {
    Mifinity: getCustomExchange({
      Request: {
        payment_method: "wallet",
        payment_method_type: "mifinity",
        authentication_type: "no_three_ds",
        billing: {
          ...standardBillingAddress,
          address: {
            ...standardBillingAddress.address,
            country: "GB",
          },
          phone: {
            number: "1234567890",
            country_code: "+44",
          },
          email: "test@example.com",
        },
        payment_method_data: {
          wallet: {
            mifinity: {
              date_of_birth: "1990-01-01",
              language_preference: "en",
            },
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method_type: "mifinity",
          connector: "mifinity",
        },
      },
    }),
  },
};
