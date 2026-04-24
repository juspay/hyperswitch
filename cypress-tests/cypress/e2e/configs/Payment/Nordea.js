import { standardBillingAddress } from "./Commons";

const sepaBankDebitDetails = {
  iban: "FI1410093000123458",
  bank_account_holder_name: "Test User",
};

export const connectorDetails = {
  bank_debit_pm: {
    PaymentIntent: {
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
    },
    SepaDebit: {
      Request: {
        payment_method: "bank_debit",
        payment_method_type: "sepa",
        currency: "EUR",
        payment_method_data: {
          bank_debit: {
            sepa_bank_debit: sepaBankDebitDetails,
          },
        },
        billing: {
          ...standardBillingAddress,
          address: {
            ...standardBillingAddress.address,
            country: "FI",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method_type: "sepa",
        },
      },
    },
  },
};
