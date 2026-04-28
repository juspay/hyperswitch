import { customerAcceptance } from "./Commons";
import { getCustomExchange } from "./Modifiers";

// Test card details for successful non-3DS transactions
const successfulNo3DSCardDetails = {
  card_number: "4111111111111111",
  card_exp_month: "08",
  card_exp_year: "30",
  card_holder_name: "joseph Doe",
  card_cvc: "999",
};

// Test card details for 3DS transactions
const successfulThreeDSTestCardDetails = {
  card_number: "4111111111111111",
  card_exp_month: "10",
  card_exp_year: "30",
  card_holder_name: "morino",
  card_cvc: "999",
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: getCustomExchange({
      Request: {
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
    // Hyperpg payment may stay in "processing" status
    No3DSAutoCapture: getCustomExchange({
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
          status: "processing",
        },
      },
    }),
    // Refund will fail if payment is still processing
    Refund: getCustomExchange({
      Request: {
        amount: 6000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "This Payment could not be refund because it has a status of processing. The expected state is succeeded, partially_captured",
            code: "IR_14",
          },
        },
      },
    }),
    PartialRefund: getCustomExchange({
      Request: {
        amount: 2000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "This Payment could not be refund because it has a status of processing. The expected state is succeeded, partially_captured",
            code: "IR_14",
          },
        },
      },
    }),
    manualPaymentRefund: getCustomExchange({
      Request: {
        amount: 6000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "This Payment could not be refund because it has a status of processing. The expected state is succeeded, partially_captured",
            code: "IR_14",
          },
        },
      },
    }),
    manualPaymentPartialRefund: getCustomExchange({
      Request: {
        amount: 2000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "This Payment could not be refund because it has a status of processing. The expected state is succeeded, partially_captured",
            code: "IR_14",
          },
        },
      },
    }),
    SyncRefund: getCustomExchange({
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    }),
  },
};
