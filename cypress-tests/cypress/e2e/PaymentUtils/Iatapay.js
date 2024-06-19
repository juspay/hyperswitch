import { getCustomExchange } from "./Commons";

export const connectorDetails = {
    upi_pm: {
      PaymentIntent: {
        Request: {
          currency: "INR",
          customer_acceptance: null,
          setup_future_usage: "on_session",
        },
        Response: {
          status: 200,
          body: {
            status: "requires_payment_method",
          },
        },
      },
      UpiAutoCapture: {
        Request: {
            payment_method: "upi",
            payment_method_type: "upi_collect",
            payment_method_data: {
                upi: {
                upi_intent: {},
              },
            },
        },
        Response: {
          status: 200,
          body: {
            status: "requires_customer_action",
          },
        },
      },
    }
}
