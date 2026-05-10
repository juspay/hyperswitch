import { getCustomExchange } from "./Modifiers";

export const connectorDetails = {
  pay_later_pm: {
    PaymentIntent: getCustomExchange({
      Request: {
        amount: 6540,
        currency: "USD",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_confirmation",
        },
      },
    }),
    No3DSAutoCapture: getCustomExchange({
      Request: {
        payment_method: "pay_later",
        payment_method_type: "affirm",
        payment_experience: "redirect_to_url",
        billing: {
          email: "guest@juspay.in",
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "San Francisco",
            state: "CA",
            zip: "94122",
            country: "US",
            first_name: "joseph",
            last_name: "Doe",
          },
        },
        shipping: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "San Francisco",
            state: "CA",
            zip: "94122",
            country: "US",
            first_name: "joseph",
            last_name: "Doe",
          },
        },
        order_details: [
          {
            product_name: "Test Product",
            quantity: 1,
            amount: 6540,
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
        amount: 6540,
        reason: "Customer request",
      },
      Response: {
        status: 200,
        body: {
          status: "success",
        },
      },
    }),
    PartialRefund: getCustomExchange({
      Request: {
        amount: 3000,
        reason: "Partial refund - customer request",
      },
      Response: {
        status: 200,
        body: {
          status: "success",
        },
      },
    }),
    SyncRefund: getCustomExchange({
      Response: {
        status: 200,
        body: {
          status: "success",
        },
      },
    }),
  },
};
