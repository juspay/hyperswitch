import { getCustomExchange } from "./Modifiers";

const standardShippingAddress = {
  address: {
    city: "San Francisco",
    country: "US",
    line1: "123 Test St",
    state: "CA",
    zip: "94122",
  },
};

const standardShippingAddressEU = {
  address: {
    city: "Berlin",
    country: "DE",
    line1: "123 Test St",
    zip: "10115",
  },
};

export const connectorDetails = {
  tax_connector: {
    CalculateTax: getCustomExchange({
      Request: {
        shipping: standardShippingAddress,
        payment_method_type: "credit",
      },
      Response: {
        status: 200,
        body: {
          order_tax_amount: null,
        },
      },
    }),
    CalculateTaxEU: getCustomExchange({
      Request: {
        shipping: standardShippingAddressEU,
        payment_method_type: "debit",
      },
      Response: {
        status: 200,
        body: {
          order_tax_amount: null,
        },
      },
    }),
    CalculateTaxSkip: getCustomExchange({
      Request: {
        shipping: standardShippingAddress,
        payment_method_type: "credit",
      },
      Response: {
        status: 200,
        body: {
          order_tax_amount: null,
        },
      },
    }),
    CalculateTaxDisabled: getCustomExchange({
      Request: {
        shipping: standardShippingAddress,
        payment_method_type: "credit",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "Tax calculation is not enabled for this payment or the payment is not in a valid state",
            code: "IR_39",
          },
        },
      },
    }),
    CalculateTaxSucceededPayment: getCustomExchange({
      Request: {
        shipping: standardShippingAddress,
        payment_method_type: "credit",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "You cannot create a session update for this payment because it has status succeeded",
            code: "IR_16",
          },
        },
      },
    }),
    CalculateTaxMissingClientSecret: getCustomExchange({
      Request: {
        shipping: standardShippingAddress,
        payment_method_type: "credit",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Missing required param: client_secret",
            code: "IR_04",
          },
        },
      },
    }),
    CalculateTaxWrongAuth: getCustomExchange({
      Request: {
        shipping: standardShippingAddress,
        payment_method_type: "credit",
      },
      Response: {
        status: 401,
        body: {
          error: {
            type: "invalid_request",
            message: "API key not provided or invalid API key used",
            code: "IR_01",
          },
        },
      },
    }),
    CalculateTaxUnconfirmedPayment: getCustomExchange({
      Request: {
        shipping: standardShippingAddress,
        payment_method_type: "credit",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "No eligible connector was found for the current payment method configuration",
            code: "IR_39",
          },
        },
      },
    }),
  },
  card_pm: {
    PaymentIntent: getCustomExchange({
      Request: {
        currency: "USD",
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
          card: {
            card_number: "4111111111111111",
            card_exp_month: "08",
            card_exp_year: "30",
            card_holder_name: "joseph Doe",
            card_cvc: "999",
          },
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
  },
};
