import { getCustomExchange } from "./_Reusable.js";

const successfulNo3DSCardDetails = {
  card_number: "4111111111111111",
  card_exp_month: "08",
  card_exp_year: "28",
  card_holder_name: "joseph Doe",
  card_cvc: "999",
};

const billingAddress = {
  address: {
    line1: "1467",
    line2: "Harrison Street",
    line3: "Harrison Street",
    city: "San Fransico",
    state: "California",
    zip: "94122",
    country: "US",
    first_name: "joseph",
    last_name: "Doe",
  },
  email: "example@example.com",
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: getCustomExchange({
      Request: {
        amount_details: {
          order_amount: 1001,
          currency: "USD",
        },
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),

    No3DSAutoCapture: getCustomExchange({
      Request: {
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        payment_method_type: "card",
        payment_method_subtype: "credit",
        customer_acceptance: null,
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),
  },
};
