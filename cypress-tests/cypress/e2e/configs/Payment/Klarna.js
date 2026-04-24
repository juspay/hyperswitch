import { getCustomExchange } from "./Modifiers";

export const connectorDetails = {
  pay_later_pm: {
    Klarna: getCustomExchange({
      Request: {
        payment_method: "pay_later",
        payment_method_type: "klarna",
        payment_experience: "redirect_to_url",
        payment_method_data: {
          pay_later: {
            klarna_redirect: {
              billing_email: "guest@juspay.in",
              billing_country: "DE",
            },
          },
        },
        billing: {
          email: "guest@juspay.in",
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "Berlin",
            state: "Berlin",
            zip: "10115",
            country: "DE",
            first_name: "joseph",
            last_name: "Doe",
          },
        },
        order_details: [
          {
            product_name: "Test Product",
            quantity: 1,
            amount: 6000,
          },
        ],
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
    Capture: getCustomExchange({
      Request: {
        amount_to_capture: 6000,
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
    Refund: getCustomExchange({
      Request: {
        amount: 6000,
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
    PartialRefund: getCustomExchange({
      Request: {
        amount: 2000,
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
    SyncRefund: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
  },
};
