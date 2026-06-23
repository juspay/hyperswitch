import { customerAcceptance } from "./Commons";

export const connectorDetails = {
  pay_later_pm: {
    PaymentIntent: {
      Request: {
        currency: "USD",
        customer_acceptance: customerAcceptance,
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
    AffirmManualCapture: {
      Request: {
        payment_method: "pay_later",
        payment_method_type: "affirm",
        payment_experience: "redirect_to_url",
        payment_method_data: {
          pay_later: {
            affirm_redirect: {
              billing_email: "guest@juspay.in",
              billing_country: "US",
            },
          },
        },
        billing: {
          email: "guest@juspay.in",
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "San Francisco",
            state: "California",
            zip: "94122",
            country: "US",
            first_name: "joseph",
            last_name: "Doe",
          },
          phone: {
            number: "8056599999",
            country_code: "+1",
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
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    Capture: {
      Request: {
        amount_to_capture: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          amount_received: 6000,
        },
      },
    },
    Refund: {
      Request: {
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          amount: 6000,
        },
      },
    },
    PartialRefund: {
      Request: {
        amount: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          amount: 2000,
        },
      },
    },
    SyncRefund: {
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
  },
};
