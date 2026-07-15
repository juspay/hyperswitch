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
      Configs: {
        TRIGGER_SKIP: true,
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
