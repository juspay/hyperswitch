export const cardCreditEnabled = [
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

export const cardCreditEnabledInUsd = [
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

export const cardCreditEnabledInUs = [
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

export const cardCreditEnabledInEur = [
  {
    payment_method: "card",
    payment_method_types: [
      {
        payment_method_type: "credit",
        card_networks: ["Visa"],
        minimum_amount: 0,
        accepted_currencies: {
          type: "enable_only",
          list: ["EUR"],
        },
        maximum_amount: 68607706,
        recurring_enabled: false,
        installment_payment_enabled: true,
      },
    ],
  },
];

export const bankRedirectIdealEnabled = [
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

export const bankRedirectIdealAndCreditEnabled = [
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

export const createPaymentBodyWithCurrencyCountry = (
  currency,
  billingCountry,
  shippingCountry
) => ({
  currency: currency,
  amount: 6000,
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
      number: "9123456789",
      country_code: "+91",
    },
    email: "example@example.com",
  },
  shipping: {
    address: {
      line1: "130",
      line2: "Koramangala",
      line3: "Ramnagar",
      city: "Bengaluru",
      state: "Karnataka",
      zip: "560011",
      country: shippingCountry, // Shipping country from parameter
      first_name: "John",
      last_name: "Joseph",
    },
    phone: {
      number: "9123456789",
      country_code: "+91",
    },
    email: "example@example.com",
  },
});

export const createPaymentBodyWithCurrency = (currency) => ({
  currency: currency,
  amount: 6000,
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
