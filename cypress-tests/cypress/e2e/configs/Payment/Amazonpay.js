import { getCustomExchange } from "./Modifiers";
import { standardBillingAddress } from "./Commons";

export const connectorDetails = {
  wallet_pm: {
    AmazonPay: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "wallet",
        payment_method_type: "amazon_pay",
        authentication_type: "no_three_ds",
        billing: standardBillingAddress,
        payment_method_data: {
          wallet: {
            amazon_pay: {
              checkout_session_id: "test_checkout_session_id",
            },
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method_type: "amazon_pay",
          connector: "amazonpay",
        },
      },
    }),
  },
};
