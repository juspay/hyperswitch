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
    // Globepay connector is not fully integrated - no eligible connector found
    No3DSAutoCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
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
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "No eligible connector was found for the current payment method configuration",
            code: "IR_00",
          },
        },
      },
    }),
    Refund: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
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
    PartialRefund: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        amount: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
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
        status: 200,
        body: {
          status: "pending",
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
        status: 200,
        body: {
          status: "pending",
        },
      },
    }),
    SyncRefund: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
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
