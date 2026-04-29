import { customerAcceptance } from "./Commons";

const successfulNo3DSCardDetails = {
  card_number: "5267 3181 8797 5449",
  card_exp_month: "10",
  card_exp_year: "30",
  card_holder_name: "Joseph Doe",
  card_cvc: "999",
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          setup_future_usage: "on_session",
        },
      },
    },
    No3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method: "card",
        },
      },
    },
  },
  order_create_pm: {
    OrderCreate: {
      Request: {
        currency: "USD",
        amount: 6000,
        order_details: [
          {
            product_name: "Test Product",
            quantity: 1,
            amount: 6000,
          },
        ],
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          amount: 6000,
        },
      },
    },
    OrderCreateConfirm: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        amount: 6000,
        order_details: [
          {
            product_name: "Test Product",
            quantity: 1,
            amount: 6000,
          },
        ],
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          amount: 6000,
          payment_method: "card",
        },
      },
    },
  },
};
