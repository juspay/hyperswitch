import { getCustomExchange } from "./Modifiers";

// SIGNIFYD SANDBOX LIMITATION:
// Signifyd sandbox always returns "Accept" decision regardless of test emails.
// All FRM scenarios (Approve/Decline/Hold) return status "succeeded".
// To test actual decline/hold behavior, configure Signifyd production credentials.

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
    amount: 6000,
  },
];

const frmMetadata = {
  order_channel: "web",
};

const shippingAddress = {
  address: {
    line1: "1467 Harrison St",
    city: "San Francisco",
    state: "CA",
    zip: "94103",
    country: "US",
    first_name: "John",
    last_name: "Doe",
  },
};

const billingAddress = {
  address: {
    line1: "1467 Harrison St",
    city: "San Francisco",
    state: "CA",
    zip: "94103",
    country: "US",
    first_name: "John",
    last_name: "Doe",
  },
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
        shipping: shippingAddress,
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
