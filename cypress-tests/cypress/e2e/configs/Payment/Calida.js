import { standardBillingAddress } from "./Commons";

const bluecodeBilling = {
  ...standardBillingAddress,
  address: {
    ...standardBillingAddress.address,
    country: "AT",
  },
};

export const connectorDetails = {
  wallet_pm: {
    Bluecode: {
      Request: {
        payment_method: "wallet",
        payment_method_type: "bluecode",
        authentication_type: "no_three_ds",
        billing: bluecodeBilling,
        payment_method_data: {
          wallet: {
            bluecode_redirect: {},
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method_type: "bluecode",
          connector: "calida",
        },
      },
    },
  },
};
