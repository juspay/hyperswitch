import { getCustomExchange } from "../Payment/Modifiers";
import {
  successfulNo3DSCardDetails,
  standardBillingAddress,
} from "../Payment/Commons";

// SIGNIFYD SANDBOX LIMITATION:
// Signifyd sandbox always returns "Accept" decision regardless of test emails.
// All FRM scenarios (Approve/Decline/Hold) return status "succeeded".
// To test actual decline/hold behavior, configure Signifyd production credentials.

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
        shipping: standardBillingAddress,
        billing: standardBillingAddress,
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
