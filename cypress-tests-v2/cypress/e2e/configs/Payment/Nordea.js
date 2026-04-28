import { getCustomExchange } from "./_Reusable.js";

const sepaBankDebitDetails = {
  iban: "FI1410093000123458",
  bank_account_holder_name: "Test User",
};

const billingAddress = {
  address: {
    line1: "1467",
    line2: "Harrison Street",
    line3: "Harrison Street",
    city: "San Fransico",
    state: "California",
    zip: "94122",
    country: "FI",
    first_name: "joseph",
    last_name: "Doe",
  },
  email: "example@example.com",
};

export const connectorDetails = {
  bank_debit_pm: {
    PaymentIntent: getCustomExchange({
      Request: {
        currency: "EUR",
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
    SepaDebit: getCustomExchange({
      Request: {
        payment_method: "bank_debit",
        payment_method_type: "sepa",
        currency: "EUR",
        payment_method_data: {
          bank_debit: {
            sepa_bank_debit: sepaBankDebitDetails,
          },
        },
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method_type: "sepa",
        },
      },
    }),
  },
};
