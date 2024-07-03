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

export const create_payment_body_in_EUR = {
  currency: "EUR",
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

export const create_payment_body_in_EUR_US = {
  currency: "EUR",
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
      line1: "1467",
      line2: "Harrison Street",
      line3: "Harrison Street",
      city: "San Fransico",
      state: "California",
      zip: "94122",
      country: "US",
      first_name: "joseph",
      last_name: "Doe",
    },
    phone: {
      number: "8056594427",
      country_code: "+91",
    },
    email: "example@example.com",
  },
};

export const create_payment_body_in_USD_IN = {
  currency: "USD",
  amount: 6500,
  authentication_type: "three_ds",
  description: "Ramesh First Crypto",
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
      country: "IN",
      first_name: "Ramesh",
      last_name: "Kumar",
    },
    phone: {
      number: "8056594427",
      country_code: "+91",
    },
    email: "example@example.com",
  },
};

export const create_payment_body_in_INR = {
  currency: "INR",
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

/*
`getDefaultExchange` contains the default Request and Response to be considered if none provided.
`getCustomExchange` takes in 2 optional fields named as Request and Response.
with `getCustomExchange`, if 501 response is expected, there is no need to pass Response as it considers default values.
*/

// Const to get default PaymentExchange object
const getDefaultExchange = () => ({
  Request: {
    currency: "EUR",
  },
  Response: {
    status: 501,
    body: {
      error: {
        type: "invalid_request",
        message: `Selected payment method is not implemented`,
        code: "IR_00",
      },
    },
  },
});

// Const to get PaymentExchange with overridden properties
export const getCustomExchange = (overrides) => {
  const defaultExchange = getDefaultExchange();

  return {
    ...defaultExchange,
    Request: {
      ...defaultExchange.Request,
      ...(overrides.Request || {}),
    },
    Response: {
      ...defaultExchange.Response,
      ...(overrides.Response || {}),
    },
  };
};

export const payment_methods_enabled = [
  {
    payment_method: "card",
    payment_method_types: [
      {
        payment_method_type: "credit",
        card_networks: [
          "AmericanExpress",
          "Discover",
          "Interac",
          "JCB",
          "Mastercard",
          "Visa",
          "DinersClub",
          "UnionPay",
          "RuPay",
        ],
        minimum_amount: 0,
        maximum_amount: 68607706,
        recurring_enabled: false,
        installment_payment_enabled: true,
      },
      {
        payment_method_type: "debit",
        card_networks: [
          "AmericanExpress",
          "Discover",
          "Interac",
          "JCB",
          "Mastercard",
          "Visa",
          "DinersClub",
          "UnionPay",
          "RuPay",
        ],
        minimum_amount: 0,
        maximum_amount: 68607706,
        recurring_enabled: false,
        installment_payment_enabled: true,
      },
    ],
  },
  {
    payment_method: "bank_transfer",
    payment_method_types: [
      {
        payment_method_type: "pix",
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
        accepted_currencies: null,
        accepted_countries: null,
        minimum_amount: 1,
        maximum_amount: 68607706,
        recurring_enabled: true,
        installment_payment_enabled: true,
      },
      {
        payment_method_type: "giropay",
        payment_experience: null,
        card_networks: null,
        accepted_currencies: null,
        accepted_countries: null,
        minimum_amount: 1,
        maximum_amount: 68607706,
        recurring_enabled: true,
        installment_payment_enabled: true,
      },
      {
        payment_method_type: "sofort",
        payment_experience: null,
        card_networks: null,
        accepted_currencies: null,
        accepted_countries: null,
        minimum_amount: 1,
        maximum_amount: 68607706,
        recurring_enabled: true,
        installment_payment_enabled: true,
      },
      {
        payment_method_type: "eps",
        payment_experience: null,
        card_networks: null,
        accepted_currencies: null,
        accepted_countries: null,
        minimum_amount: 1,
        maximum_amount: 68607706,
        recurring_enabled: true,
        installment_payment_enabled: true,
      },
      {
        payment_method_type: "blik",
        payment_experience: null,
        card_networks: null,
        accepted_currencies: null,
        accepted_countries: null,
        minimum_amount: 1,
        maximum_amount: 68607706,
        recurring_enabled: true,
        installment_payment_enabled: true,
      },
      {
        payment_method_type: "przelewy24",
        payment_experience: null,
        card_networks: null,
        accepted_currencies: null,
        accepted_countries: null,
        minimum_amount: 1,
        maximum_amount: 68607706,
        recurring_enabled: true,
        installment_payment_enabled: true,
      },
    ],
  },
];
