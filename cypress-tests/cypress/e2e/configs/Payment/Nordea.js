import { getCustomExchange } from "./Modifiers";

export const connectorDetails = {
  bank_debit_pm: {
    PaymentIntent: (paymentMethodType) => {
      const currencyMap = { Sepa: "EUR", Ach: "USD", Becs: "AUD", Bacs: "GBP" };
      return getCustomExchange({
        Request: {
          currency: currencyMap[paymentMethodType] || "USD",
          setup_future_usage: "off_session",
        },
        Response: {
          status: 200,
          body: {
            status: "requires_payment_method",
          },
        },
      });
    },
    SepaDebit: getCustomExchange({
      Request: {
        payment_method: "bank_debit",
        payment_method_type: "sepa",
        payment_method_data: {
          bank_debit: {
            sepa_bank_debit: {
              iban: "DE89370400440532013000",
              bank_account_holder_name: "Test Customer",
            },
          },
        },
        billing: {
          address: {
            country: "FI",
          },
          email: "test@example.com",
        },
        metadata: {
          destination_account_number: "FI1410093000123458",
          account_type: "IBAN",
          merchant_name: "Test Merchant Oy",
        },
        currency: "EUR",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_confirmation",
        },
      },
    }),
  },
};
