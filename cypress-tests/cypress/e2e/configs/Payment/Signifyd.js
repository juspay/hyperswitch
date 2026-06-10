import { getCustomExchange } from "./Modifiers";

const successfulNo3DSCardDetails = {
  card_number: "4111111111111111",
  card_exp_month: "08",
  card_exp_year: "30",
  card_holder_name: "Joseph Doe",
  card_cvc: "999",
};

const orderDetails = [
  {
    product_name: "Test Product",
    quantity: 1,
    amount: 6540,
  },
];

const frmMetadata = {
  order_channel: "web",
};

export const connectorDetails = {
  card_pm: {
    FRMApprove: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        email: "testapproved@signifyd.com",
        frm_metadata: frmMetadata,
        order_details: orderDetails,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),
    FRMDecline: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        email: "testdeclined@signifyd.com",
        frm_metadata: frmMetadata,
        order_details: orderDetails,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
        },
      },
    }),
    FRMHold: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        email: "testpending@signifyd.com",
        frm_metadata: frmMetadata,
        order_details: orderDetails,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_merchant_action",
        },
      },
    }),
  },
};
