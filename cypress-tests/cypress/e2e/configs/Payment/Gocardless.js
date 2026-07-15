import { customerAcceptance } from "./Commons";
import { getCustomExchange } from "./Modifiers";

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        currency: "USD",
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
  },
  bank_debit_pm: {
    PaymentIntent: (paymentMethodType) => {
      const currencyMap = { Ach: "USD" };
      return {
        Request: {
          currency: currencyMap[paymentMethodType] || "USD",
        },
        Response: {
          status: 200,
          body: {
            status: "requires_payment_method",
          },
        },
      };
    },
    Sepa: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
    SepaDebitMandate: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
    Ach: getCustomExchange({
      Request: {
        payment_method: "bank_debit",
        payment_method_type: "ach",
        payment_method_data: {
          bank_debit: {
            ach_bank_debit: {
              account_number: "000123456789",
              routing_number: "110000000",
              bank_account_holder_name: "Test Account",
            },
          },
        },
        billing: {
          address: {
            country: "US",
            first_name: "Test",
            last_name: "Account",
          },
          email: "test@example.com",
        },
      },
      Response: {
        status: 200,
        body: { status: "processing" },
      },
    }),
    AchMandate: getCustomExchange({
      Request: {
        payment_method: "bank_debit",
        payment_method_type: "ach",
        payment_method_data: {
          bank_debit: {
            ach_bank_debit: {
              account_number: "000123456789",
              routing_number: "110000000",
              bank_account_holder_name: "Test Account",
            },
          },
        },
        billing: {
          address: {
            country: "US",
            first_name: "Test",
            last_name: "Account",
          },
          email: "test@example.com",
        },
        setup_future_usage: "off_session",
        mandate_data: {
          customer_acceptance: {
            acceptance_type: "online",
            accepted_at: "1963-05-03T04:07:52.723Z",
            online: {
              ip_address: "127.0.0.1",
              user_agent:
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/70.0.3538.110 Safari/537.36",
            },
          },
          mandate_type: {
            multi_use: {
              amount: 8000,
              currency: "USD",
            },
          },
        },
      },
      Response: {
        status: 200,
        body: { status: "processing" },
      },
    }),
    Becs: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
    Bacs: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
  },
};
