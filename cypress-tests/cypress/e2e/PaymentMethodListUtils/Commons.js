import State from "../../utils/State";

const globalState = new State({
  connectorId: Cypress.env("CONNECTOR"),
  baseUrl: Cypress.env("BASEURL"),
  adminApiKey: Cypress.env("ADMINAPIKEY"),
  connectorAuthFilePath: Cypress.env("CONNECTOR_AUTH_FILE_PATH"),
});

export const card_credit_enabled = [
  {
    payment_method: "card",
    payment_method_types: [
      {
        payment_method_type: "credit",
        card_networks: ["Visa"],
        minimum_amount: 0,
        maximum_amount: 68607706,
        recurring_enabled: false,
        installment_payment_enabled: true,
      },
    ],
  },
];

export const card_credit_enabled_in_USD = [
  {
    payment_method: "card",
    payment_method_types: [
      {
        payment_method_type: "credit",
        card_networks: ["Visa"],
        minimum_amount: 0,
        accepted_currencies: {
          type: "enable_only",
          list: ["USD"],
        },
        maximum_amount: 68607706,
        recurring_enabled: false,
        installment_payment_enabled: true,
      },
    ],
  },
];

export const card_credit_enabled_in_US = [
  {
    payment_method: "card",
    payment_method_types: [
      {
        payment_method_type: "credit",
        card_networks: ["Visa"],
        minimum_amount: 0,
        accepted_countries: {
          type: "enable_only",
          list: ["US"],
        },
        maximum_amount: 68607706,
        recurring_enabled: false,
        installment_payment_enabled: true,
      },
    ],
  },
];

export const bank_redirect_ideal_enabled = [
  {
    payment_method: "bank_redirect",
    payment_method_types: [
      {
        payment_method_type: "ideal",
        payment_experience: null,
        card_networks: null,
        accepted_countries: null,
        minimum_amount: 0,
        maximum_amount: 68607706,
        recurring_enabled: true,
        installment_payment_enabled: false,
      },
    ],
  },
];

export const bank_redirect_ideal_and_credit_enabled = [
  {
    payment_method: "card",
    payment_method_types: [
      {
        payment_method_type: "credit",
        card_networks: ["Visa"],
        minimum_amount: 0,
        maximum_amount: 68607706,
        recurring_enabled: false,
        installment_payment_enabled: true,
      },
    ],
  },
  {
    payment_method: "bank_redirect",
    payment_method_types: [
      {
        payment_method_type: "ideal",
        payment_experience: null,
        card_networks: null,
        accepted_countries: null,
        minimum_amount: 0,
        maximum_amount: 68607706,
        recurring_enabled: true,
        installment_payment_enabled: false,
      },
    ],
  },
];

export const create_payment_body_with_currency_country = (
  currency,
  billingCountry
) => ({
  currency: currency,
  amount: 6500,
  authentication_type: "three_ds",
  description: "Joseph First Crypto",
  email: "hyperswitch_sdk_demo_id@gmail.com",
  setup_future_usage: "",
  metadata: {
    udf1: "value1",
    new_customer: "true",
    login_date: "2019-09-10T10:11:12Z",
  },
  business_label: "default",
  billing: {
    address: {
      line1: "1946",
      line2: "Gandhi Nagar",
      line3: "Ramnagar",
      city: "Ranchi",
      state: "Jharkhand",
      zip: "827013",
      country: billingCountry, // Billing country from parameter
      first_name: "Joseph",
      last_name: "Doe",
    },
    phone: {
      number: "8056594427",
      country_code: "+91",
    },
    email: "example@example.com",
  },
});

export const create_payment_body_with_currency = (currency) => ({
  currency: currency,
  amount: 6500,
  authentication_type: "three_ds",
  description: "Joseph First Crypto",
  email: "hyperswitch_sdk_demo_id@gmail.com",
  setup_future_usage: "",
  metadata: {
    udf1: "value1",
    new_customer: "true",
    login_date: "2019-09-10T10:11:12Z",
  },
  business_label: "default",
});

export const create_payment_body_in_USD = {
  currency: "USD",
  amount: 6500,
  authentication_type: "three_ds",
  description: "Joseph First Crypto",
  email: "hyperswitch_sdk_demo_id@gmail.com",
  setup_future_usage: "",
  metadata: {
    udf1: "value1",
    new_customer: "true",
    login_date: "2019-09-10T10:11:12Z",
  },
  business_label: "default",
};
