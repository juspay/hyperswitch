import { getCustomExchange } from "./Modifiers";

export const connectorDetails = {
  pay_later_pm: {
    AutoCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
    ManualCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
    Klarna: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
    CaptureOnWrongStatus: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
    ConfirmWithoutPmData: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    PaymentIntent: (_paymentMethodType) =>
      getCustomExchange({
        Request: {
          currency: "ZAR",
          amount: 10000,
        },
        Response: {
          status: 200,
          body: {
            status: "requires_payment_method",
          },
        },
      }),
    Payjustnow: getCustomExchange({
      Request: {
        payment_method: "pay_later",
        payment_method_type: "payjustnow",
        payment_method_data: {
          pay_later: {
            payjustnow_redirect: {},
          },
        },
        billing: {
          email: "customer@payjustnow.co.za",
          address: {
            line1: "123 Main Street",
            line2: "",
            line3: "",
            city: "Johannesburg",
            state: "Gauteng",
            zip: "2001",
            country: "ZA",
            first_name: "Test",
            last_name: "Customer",
          },
        },
        shipping: {
          email: "customer@payjustnow.co.za",
          address: {
            line1: "123 Main Street",
            line2: "",
            line3: "",
            city: "Johannesburg",
            state: "Gauteng",
            zip: "2001",
            country: "ZA",
            first_name: "Test",
            last_name: "Customer",
          },
        },
        order_details: [
          {
            product_name: "Test Product",
            quantity: 1,
            amount: 10000,
          },
        ],
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
    Refund: getCustomExchange({
      Request: {
        amount: 10000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),
    PartialRefund: getCustomExchange({
      Request: {
        amount: 5000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),
    SyncRefund: getCustomExchange({
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),
  },
};
