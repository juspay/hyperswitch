import { getCustomExchange } from "./Modifiers";

const cardData = {
  card_number: "4111111111111111",
  card_exp_month: "08",
  card_exp_year: "30",
  card_holder_name: "joseph Doe",
  card_cvc: "999",
};

export const connectorDetails = {
  tax_connector: {
    PaymentProcessor: {
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "active",
        },
      },
    },
  },
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "USD",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    No3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: cardData,
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
    },
    CalculateTax: {
      Request: {
        shipping: {
          address: {
            city: "New York",
            country: "US",
            line1: "123 Main St",
            zip: "10001",
            state: "NY",
          },
          phone: {
            number: "1234567890",
            country_code: "+1",
          },
          email: "test@example.com",
        },
        payment_method_type: "credit",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          order_tax_amount: null,
          net_amount: null,
          shipping_cost: null,
          display_amount: null,
        },
      },
    },
    CalculateTaxEU: {
      Request: {
        shipping: {
          address: {
            city: "Berlin",
            country: "DE",
            line1: "Alexanderplatz 1",
            zip: "10178",
            state: "BE",
          },
          phone: {
            number: "1234567890",
            country_code: "+49",
          },
          email: "test@example.com",
        },
        payment_method_type: "debit",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          order_tax_amount: null,
          net_amount: null,
          shipping_cost: null,
          display_amount: null,
        },
      },
    },
    CalculateTaxSkip: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        shipping: {
          address: {
            city: "New York",
            country: "US",
            line1: "123 Main St",
            zip: "10001",
            state: "NY",
          },
          phone: {
            number: "1234567890",
            country_code: "+1",
          },
          email: "test@example.com",
        },
        payment_method_type: "credit",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          order_tax_amount: null,
        },
      },
    },
    CalculateTaxDisabled: {
      Request: {
        shipping: {
          address: {
            city: "New York",
            country: "US",
            line1: "123 Main St",
            zip: "10001",
            state: "NY",
          },
          phone: {
            number: "1234567890",
            country_code: "+1",
          },
          email: "test@example.com",
        },
        payment_method_type: "credit",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            code: "IR_01",
            message: "Tax calculation is not enabled for this merchant",
          },
        },
      },
    },
    CalculateTaxSucceededPayment: {
      Request: {
        shipping: {
          address: {
            city: "New York",
            country: "US",
            line1: "123 Main St",
            zip: "10001",
            state: "NY",
          },
          phone: {
            number: "1234567890",
            country_code: "+1",
          },
          email: "test@example.com",
        },
        payment_method_type: "credit",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            code: "IR_16",
            message:
              "You cannot calculate tax for this payment because the payment has already been succeeded",
          },
        },
      },
    },
    CalculateTaxUnconfirmedPayment: {
      Request: {
        shipping: {
          address: {
            city: "New York",
            country: "US",
            line1: "123 Main St",
            zip: "10001",
            state: "NY",
          },
          phone: {
            number: "1234567890",
            country_code: "+1",
          },
          email: "test@example.com",
        },
        payment_method_type: "credit",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            code: "IR_39",
            message:
              "You cannot calculate tax for this payment because the payment has not been confirmed",
          },
        },
      },
    },
    CalculateTaxWrongAuth: {
      Request: {
        shipping: {
          address: {
            city: "New York",
            country: "US",
            line1: "123 Main St",
            zip: "10001",
            state: "NY",
          },
          phone: {
            number: "1234567890",
            country_code: "+1",
          },
          email: "test@example.com",
        },
        payment_method_type: "credit",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            code: "IR_01",
            message: "API key not found",
          },
        },
      },
    },
    CalculateTaxMissingClientSecret: {
      Request: {
        shipping: {
          address: {
            city: "New York",
            country: "US",
            line1: "123 Main St",
            zip: "10001",
            state: "NY",
          },
          phone: {
            number: "1234567890",
            country_code: "+1",
          },
          email: "test@example.com",
        },
        payment_method_type: "credit",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            code: "IR_04",
            message: "Missing required param: client_secret",
          },
        },
      },
    },
  },
};
