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
          status: "succeeded",
        },
      },
    }),
    // Helcim refunds fail with "Card Transaction cannot be refunded"
    Refund: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        amount: 6000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Card Transaction cannot be refunded",
            code: "IR_00",
          },
        },
      },
    }),
    PartialRefund: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        amount: 2000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Card Transaction cannot be refunded",
            code: "IR_00",
          },
        },
      },
    }),
    manualPaymentRefund: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        amount: 6000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Card Transaction cannot be refunded",
            code: "IR_00",
          },
        },
      },
    }),
    manualPaymentPartialRefund: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        amount: 2000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Card Transaction cannot be refunded",
            code: "IR_00",
          },
        },
      },
    }),
    SyncRefund: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Card Transaction cannot be refunded",
            code: "IR_00",
          },
        },
      },
    }),
  },
};
