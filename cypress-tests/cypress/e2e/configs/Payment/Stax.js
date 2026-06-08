import {
  cardRequiredField,
  customerAcceptance,
  multiUseMandateData,
  singleUseMandateData,
} from "./Commons";
import { getCustomExchange } from "./Modifiers";

// Test card details for stax SnapPay
const successfulNo3DSCardDetails = {
  card_number: "4111111111111111", // Visa test card from stax documentation
  card_exp_month: "12",
  card_exp_year: "30",
  card_holder_name: "Joseph Doe",
  card_cvc: "123",
};

const successfulThreeDSCardDetails = {
  ...successfulNo3DSCardDetails,
  card_number: "5555555555554444", // Visa test card from stax documentation
};

const failedCardDetails = {
  ...successfulNo3DSCardDetails,
  card_number: "4012888888881881", // Standard decline test card for stax - "Do Not Honor" response
};

export const fullNameRequiredField = {
  "billing.address.last_name": {
    required_field: "payment_method_data.billing.address.last_name",
    display_name: "card_holder_name",
    field_type: "user_full_name",
    value: "Doe",
  },
  "billing.address.first_name": {
    required_field: "payment_method_data.billing.address.first_name",
    display_name: "card_holder_name",
    field_type: "user_full_name",
    value: "joseph",
  },
};

export const billingRequiredField = {};

const requiredFields = {
  payment_methods: [
    {
      payment_method: "card",
      payment_method_types: [
        {
          payment_method_type: "credit",
          card_networks: [
            {
              eligible_connectors: ["stax"],
            },
          ],
          required_fields: cardRequiredField,
        },
      ],
    },
  ],
};

const payment_method_data_no3ds = {
  card: {
    last4: "1111",
    card_type: "DEBIT",
    card_network: "Visa",
    card_issuer: "Conotoxia Sp Z Oo",
    card_issuing_country: "POLAND",
    card_isin: "411111",
    card_extended_bin: null,
    card_exp_month: "12",
    card_exp_year: "30",
    card_holder_name: "Joseph Doe",
    payment_checks: null,
    authentication_data: null,
    auth_code: null,
  },
  billing: null,
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          setup_future_usage: "on_session",
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
  },
};
