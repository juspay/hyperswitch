import { getCustomExchange } from "./Modifiers";

const hostedPageCard = {
  card_number: "4546341111111119",
  card_exp_month: "12",
  card_exp_year: "30",
  card_holder_name: "Elavon PG Test",
  card_cvc: "123",
};

const billing = {
  address: {
    line1: "1467",
    city: "San Francisco",
    state: "CA",
    zip: "94122",
    country: "US",
    first_name: "Elavon",
    last_name: "Test",
  },
  email: "test@example.com",
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: getCustomExchange({
      Request: {
        amount: 1000,
        currency: "USD",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
    "3DSManualCapture": getCustomExchange({
      Request: {
        amount: 1000,
        currency: "USD",
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: hostedPageCard,
        },
        billing,
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
    SyncPayment: getCustomExchange({
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    }),
    Capture: getCustomExchange({
      Request: {
        amount_to_capture: 1000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          amount: 1000,
          amount_capturable: 0,
          amount_received: 1000,
        },
      },
    }),
    Refund: getCustomExchange({
      Request: {
        amount: 1000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),
    SyncRefund: getCustomExchange({
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),
    Void: getCustomExchange({
      Request: {
        cancellation_reason: "requested_by_customer",
      },
      Response: {
        status: 200,
        body: {
          status: "cancelled",
          capture_method: "manual",
        },
      },
    }),
  },
};
