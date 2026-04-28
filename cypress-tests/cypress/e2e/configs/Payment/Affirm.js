import { getCustomExchange } from "./Modifiers";

export const connectorDetails = {
  pay_later_pm: {
    PaymentIntent: getCustomExchange({
      Request: {
        currency: "USD",
        amount: 10000,
        capture_method: "manual",
        authentication_type: "three_ds",
        return_url: "https://webhook.site/d75a908f-ddbf-43e5-ac94-9d5120e12b92",
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "San Fransico",
            state: "California",
            zip: "94122",
            country: "US",
            first_name: "John",
            last_name: "Doe",
          },
        },
        shipping: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "San Fransico",
            state: "California",
            zip: "94122",
            country: "US",
            first_name: "John",
            last_name: "Doe",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
    Affirm: getCustomExchange({
      Request: {
        payment_method: "pay_later",
        payment_method_type: "affirm",
        payment_experience: "redirect_to_url",
        payment_method_data: {
          pay_later: {
            affirm_redirect: {
              billing_name: "John Doe",
              billing_email: "john.doe@example.com",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "San Fransico",
            state: "California",
            zip: "94122",
            country: "US",
            first_name: "John",
            last_name: "Doe",
          },
        },
        shipping: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "San Fransico",
            state: "California",
            zip: "94122",
            country: "US",
            first_name: "John",
            last_name: "Doe",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
    Capture: getCustomExchange({
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          amount_received: 10000,
        },
      },
    }),
    Refund: getCustomExchange({
      Request: {
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
        },
      },
    }),
  },
};
