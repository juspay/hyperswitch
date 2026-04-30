// This file is the default. To override, add to connector.js
import { getCurrency, getCustomExchange } from "./Modifiers";

export const blockedPaymentErrorBodyForIssuingCountry = {
  status: 200,
  expectBlockedPayment: true,
  body: {
    error: {
      type: "blocked",
      message:
        "Cards issued in your region aren't supported for this transaction, please try a different card",
      code: "HE_03",
      reason: "Blocked",
    },
  },
};

export const blockedPaymentErrorBodyForDebitCard = {
  status: 200,
  expectBlockedPayment: true,
  body: {
    error: {
      type: "blocked",
      message:
        "Debit cards are not accepted for this transaction, please try a different card",
      code: "HE_03",
      reason: "Blocked",
    },
  },
};

export const blockedPaymentErrorBodyForCardSubtype = {
  status: 200,
  expectBlockedPayment: true,
  body: {
    error: {
      type: "blocked",
      message:
        "This card is not accepted for this transaction, please try a different card",
      code: "HE_03",
      reason: "Blocked",
    },
  },
};

export const blockedPaymentErrorBodyForBinUnavailable = {
  status: 200,
  expectBlockedPayment: true,
  body: {
    error: {
      type: "blocked",
      message:
        "We're unable to accept this card, please try another card or a different payment method",
      code: "HE_03",
      reason: "Blocked",
    },
  },
};

export const customerAcceptance = {
  acceptance_type: "offline",
  accepted_at: "1963-05-03T04:07:52.723Z",
  online: {
    ip_address: "127.0.0.1",
    user_agent:
      "Mozilla/5.0 (iPhone; CPU iPhone OS 18_5 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/22F76 [FBAN/FBIOS;FBAV/520.0.0.38.101;FBBV/756351453;FBDV/iPhone14,7;FBMD/iPhone;FBSN/iOS;FBSV/18.5;FBSS/3;FBID/phone;FBLC/fr_FR;FBOP/5;FBRV/760683563;IABMV/1]",
  },
};

export const cardCreditEnabled = [
  {
    payment_method: "card",
    payment_method_types: [
      {
        payment_method_type: "credit",
        card_networks: [
          "Visa",
          "Mastercard",
          "AmericanExpress",
          "Discover",
          "JCB",
          "DinersClub",
          "UnionPay",
          "RuPay",
          "Interac",
        ],
        minimum_amount: 0,
        maximum_amount: 68607706,
        recurring_enabled: false,
        installment_payment_enabled: true,
      },
    ],
  },
];

const successfulNo3DSCardDetails = {
  card_number: "4111111111111111",
  card_exp_month: "08",
  card_exp_year: "30",
  card_holder_name: "joseph Doe",
  card_cvc: "999",
};

const blocklistedCardDetails = {
  card_number: "4242424242424242",
  card_exp_month: "01",
  card_exp_year: "2050",
  card_holder_name: "John Smith",
  card_cvc: "349",
  card_network: "Visa",
};

const successfulThreeDSTestCardDetails = {
  card_number: "4111111111111111",
  card_exp_month: "10",
  card_exp_year: "30",
  card_holder_name: "morino",
  card_cvc: "999",
};

const PaymentMethodCardDetails = {
  card_number: "4111111145551142",
  card_exp_month: "03",
  card_exp_year: "30",
  card_holder_name: "Joseph Doe",
};

export const singleUseMandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    single_use: {
      amount: 8000,
      currency: "USD",
    },
  },
};

export const multiUseMandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    multi_use: {
      amount: 8000,
      currency: "USD",
    },
  },
};

export const standardBillingAddress = {
  address: {
    line1: "1467",
    line2: "Harrison Street",
    line3: "Harrison Street",
    city: "San Fransico",
    state: "CA",
    zip: "94122",
    country: "US",
    first_name: "John",
    last_name: "Doe",
  },
  phone: {
    number: "8056594427",
    country_code: "+91",
  },
};

export const cardRequiredField = {
  "payment_method_data.card.card_number": {
    required_field: "payment_method_data.card.card_number",
    display_name: "card_number",
    field_type: "user_card_number",
    value: null,
  },
  "payment_method_data.card.card_exp_year": {
    required_field: "payment_method_data.card.card_exp_year",
    display_name: "card_exp_year",
    field_type: "user_card_expiry_year",
    value: null,
  },
  "payment_method_data.card.card_cvc": {
    required_field: "payment_method_data.card.card_cvc",
    display_name: "card_cvc",
    field_type: "user_card_cvc",
    value: null,
  },
  "payment_method_data.card.card_exp_month": {
    required_field: "payment_method_data.card.card_exp_month",
    display_name: "card_exp_month",
    field_type: "user_card_expiry_month",
    value: null,
  },
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

export const payment_methods_enabled = [
  {
    payment_method: "bank_redirect",
    payment_method_types: [
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
        payment_method_type: "local_bank_redirect",
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
      {
        payment_method_type: "open_banking_uk",
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
        payment_method_type: "online_banking_fpx",
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
        payment_method_type: "interac",
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
  {
    payment_method: "bank_debit",
    payment_method_types: [
      {
        payment_method_type: "sepa",
        minimum_amount: 1,
        maximum_amount: 68607706,
        recurring_enabled: true,
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
      {
        payment_method_type: "ach",
        minimum_amount: 0,
        maximum_amount: 68607706,
        recurring_enabled: false,
      },
      {
        payment_method_type: "instant_bank_transfer_finland",
        minimum_amount: 1,
        maximum_amount: 68607706,
        recurring_enabled: true,
        installment_payment_enabled: true,
      },
      {
        payment_method_type: "instant_bank_transfer_poland",
        minimum_amount: 1,
        maximum_amount: 68607706,
        recurring_enabled: true,
        installment_payment_enabled: true,
      },
    ],
  },
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
    payment_method: "real_time_payment",
    payment_method_types: [
      {
        payment_method_type: "duit_now",
        minimum_amount: 1,
        maximum_amount: 68607706,
        recurring_enabled: true,
        installment_payment_enabled: true,
        payment_experience: "redirect_to_url",
      },
      {
        payment_method_type: "fps",
        minimum_amount: 1,
        maximum_amount: 68607706,
        recurring_enabled: true,
        installment_payment_enabled: true,
        payment_experience: "redirect_to_url",
      },
      {
        payment_method_type: "prompt_pay",
        minimum_amount: 1,
        maximum_amount: 68607706,
        recurring_enabled: true,
        installment_payment_enabled: true,
        payment_experience: "redirect_to_url",
      },
      {
        payment_method_type: "viet_qr",
        minimum_amount: 1,
        maximum_amount: 68607706,
        recurring_enabled: true,
        installment_payment_enabled: true,
        payment_experience: "redirect_to_url",
      },
    ],
  },
  {
    payment_method: "upi",
    payment_method_types: [
      {
        payment_method_type: "upi_collect",
        minimum_amount: 1,
        maximum_amount: 68607706,
        recurring_enabled: true,
        installment_payment_enabled: true,
        payment_experience: "redirect_to_url",
      },
      {
        payment_method_type: "upi_intent",
        minimum_amount: 1,
        maximum_amount: 68607706,
        recurring_enabled: true,
        installment_payment_enabled: true,
        payment_experience: "redirect_to_url",
      },
    ],
  },
  {
    payment_method: "wallet",
    payment_method_types: [
      {
        payment_method_type: "apple_pay",
        minimum_amount: 1,
        maximum_amount: 68607706,
        recurring_enabled: true,
        installment_payment_enabled: true,
        payment_experience: "invoke_sdk_client",
      },
      {
        payment_method_type: "google_pay",
        minimum_amount: 1,
        maximum_amount: 68607706,
        recurring_enabled: true,
        installment_payment_enabled: true,
        payment_experience: "invoke_sdk_client",
      },
      {
        payment_method_type: "bluecode",
        payment_experience: "redirect_to_url",
        minimum_amount: 1,
        maximum_amount: 68607706,
        recurring_enabled: false,
        installment_payment_enabled: false,
      },
      {
        payment_method_type: "skrill",
        payment_experience: "redirect_to_url",
        minimum_amount: 1,
        maximum_amount: 68607706,
        recurring_enabled: false,
        installment_payment_enabled: false,
      },
      {
        payment_method_type: "ali_pay_hk",
        payment_experience: "redirect_to_url",
        minimum_amount: 1,
        maximum_amount: 68607706,
        recurring_enabled: false,
        installment_payment_enabled: false,
      },
    ],
  },
  {
    payment_method: "reward",
    payment_method_types: [
      {
        payment_method_type: "evoucher",
        payment_experience: "redirect_to_url",
        card_networks: null,
        accepted_currencies: null,
        accepted_countries: null,
        minimum_amount: 0,
        maximum_amount: 68607706,
        recurring_enabled: true,
        installment_payment_enabled: false,
      },
      {
        payment_method_type: "classic",
        payment_experience: "redirect_to_url",
        card_networks: null,
        accepted_currencies: null,
        accepted_countries: null,
        minimum_amount: 0,
        maximum_amount: 68607706,
        recurring_enabled: true,
        installment_payment_enabled: false,
      },
    ],
  },
  {
    payment_method: "gift_card",
    payment_method_types: [
      {
        payment_method_type: "givex",
        payment_experience: null,
        card_networks: null,
        accepted_currencies: null,
        accepted_countries: null,
        minimum_amount: 1,
        maximum_amount: 68607706,
        recurring_enabled: false,
        installment_payment_enabled: false,
      },
    ],
  },
  {
    payment_method: "crypto",
    payment_method_types: [
      {
        payment_method_type: "crypto_currency",
        payment_experience: "redirect_to_url",
        card_networks: null,
        accepted_currencies: null,
        accepted_countries: null,
        minimum_amount: 0,
        maximum_amount: 68607706,
        recurring_enabled: true,
        installment_payment_enabled: false,
      },
    ],
  },
];

export const connectorDetails = {
  bank_transfer_pm: {
    PaymentIntent: (paymentMethodType) =>
      getCustomExchange({
        Request: {
          currency: getCurrency(paymentMethodType),
        },
        Response: {
          status: 200,
          body: {
            status: "requires_payment_method",
          },
        },
      }),

    Pix: getCustomExchange({
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "pix",
        payment_method_data: {
          bank_transfer: {
            pix: {},
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "San Fransico",
            state: "California",
            zip: "94122",
            country: "BR",
            first_name: "john",
            last_name: "doe",
          },
        },
        currency: "BRL",
      },
    }),
    Ach: getCustomExchange({
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "ach",
        payment_method_data: {
          bank_transfer: {
            ach_bank_transfer: {},
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "San Fransico",
            state: "California",
            zip: "94122",
            country: "BR",
            first_name: "john",
            last_name: "doe",
          },
        },
        currency: "BRL",
      },
    }),
    InstantBankTransferFinland: getCustomExchange({
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "instant_bank_transfer_finland",
        payment_method_data: {
          bank_transfer: {
            instant_bank_transfer_finland: {},
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "San Fransico",
            state: "California",
            zip: "94122",
            country: "FI",
            first_name: "john",
            last_name: "doe",
          },
        },
        currency: "EUR",
      },
    }),
    InstantBankTransferPoland: getCustomExchange({
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "instant_bank_transfer_poland",
        payment_method_data: {
          bank_transfer: {
            instant_bank_transfer_poland: {},
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "San Fransico",
            state: "California",
            zip: "94122",
            country: "PL",
            first_name: "john",
            last_name: "doe",
          },
        },
        currency: "PLN",
      },
    }),
  },
  bank_redirect_pm: {
    PaymentIntent: (paymentMethodType) =>
      getCustomExchange({
        Request: {
          currency: getCurrency(paymentMethodType),
        },
        Response: {
          status: 200,
          body: {
            status: "requires_payment_method",
          },
        },
      }),
    Ideal: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "ideal",
        payment_method_data: {
          bank_redirect: {
            ideal: {
              bank_name: "ing",
              country: "NL",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "San Fransico",
            state: "California",
            zip: "94122",
            country: "NL",
            first_name: "john",
            last_name: "doe",
          },
        },
      },
    }),
    OpenBankingUk: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "open_banking_uk",
        payment_method_data: {
          bank_redirect: {
            open_banking_uk: {
              issuer: "citi",
              country: "GB",
            },
          },
        },
        billing: standardBillingAddress,
      },
    }),
    OnlineBankingFpx: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "online_banking_fpx",
        payment_method_data: {
          bank_redirect: {
            online_banking_fpx: {
              issuer: "affin_bank",
            },
          },
        },
        billing: standardBillingAddress,
      },
    }),
    Giropay: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "giropay",
        payment_method_data: {
          bank_redirect: {
            giropay: {
              bank_name: "",
              bank_account_bic: "",
              bank_account_iban: "",
              preferred_language: "en",
              country: "DE",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "San Fransico",
            state: "California",
            zip: "94122",
            country: "DE",
            first_name: "john",
            last_name: "doe",
          },
        },
      },
    }),
    Sofort: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "sofort",
        payment_method_data: {
          bank_redirect: {
            sofort: {
              country: "DE",
              preferred_language: "en",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "San Fransico",
            state: "California",
            zip: "94122",
            country: "DE",
            first_name: "john",
            last_name: "doe",
          },
        },
      },
    }),
    Eps: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "eps",
        payment_method_data: {
          bank_redirect: {
            eps: {
              bank_name: "ing",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "San Fransico",
            state: "California",
            zip: "94122",
            country: "AT",
            first_name: "john",
            last_name: "doe",
          },
        },
      },
    }),
    Przelewy24: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "przelewy24",
        payment_method_data: {
          bank_redirect: {
            przelewy24: {
              bank_name: "citi",
              billing_details: {
                email: "guest@juspay.in",
              },
            },
          },
        },
      },
    }),
    Blik: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "blik",
        payment_method_data: {
          bank_redirect: {
            blik: {
              blik_code: "777987",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "San Fransico",
            state: "California",
            zip: "94122",
            country: "PL",
            first_name: "john",
            last_name: "doe",
          },
        },
      },
    }),
    Interac: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "interac",
        payment_method_data: {
          bank_redirect: {
            interac: {
              bank_name: "ing",
            },
          },
        },
        billing: {
          ...standardBillingAddress,
          address: {
            ...standardBillingAddress.address,
            country: "CA",
          },
        },
      },
    }),
  },
  bank_debit_pm: {
    PaymentIntent: (paymentMethodType) => {
      const currencyMap = { Sepa: "EUR", Ach: "USD", Becs: "AUD", Bacs: "GBP" };
      return getCustomExchange({
        Request: {
          currency: currencyMap[paymentMethodType] || "USD",
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
              bank_account_holder_name: "Test Account",
            },
          },
        },
        billing: {
          address: {
            country: "DE",
          },
          email: "test@example.com",
        },
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
          },
          email: "test@example.com",
        },
      },
    }),
    Becs: getCustomExchange({
      Request: {
        payment_method: "bank_debit",
        payment_method_type: "becs",
        payment_method_data: {
          bank_debit: {
            becs_bank_debit: {
              account_number: "000123456",
              bsb_number: "000000",
              bank_account_holder_name: "Test Account",
            },
          },
        },
        billing: {
          address: {
            country: "AU",
          },
          email: "test@example.com",
        },
      },
    }),
    Bacs: getCustomExchange({
      Request: {
        payment_method: "bank_debit",
        payment_method_type: "bacs",
        payment_method_data: {
          bank_debit: {
            bacs_bank_debit: {
              account_number: "00012345",
              sort_code: "108800",
              bank_account_holder_name: "Test Account",
            },
          },
        },
        billing: {
          address: {
            country: "GB",
          },
          email: "test@example.com",
        },
      },
    }),
  },
  wallet_pm: {
    PaymentIntent: (paymentMethodType) =>
      getCustomExchange({
        Request: {
          currency: getCurrency(paymentMethodType),
        },
        Response: {
          status: 200,
          body: {
            status: "requires_payment_method",
          },
        },
      }),
    Bluecode: getCustomExchange({
      Request: {
        payment_method: "wallet",
        payment_method_type: "bluecode",
        payment_method_data: {
          wallet: {
            bluecode_redirect: {},
          },
        },
        billing: {
          ...standardBillingAddress,
          address: {
            ...standardBillingAddress.address,
            country: "AT",
          },
        },
      },
    }),
    AliPayHk: getCustomExchange({
      Request: {
        payment_method: "wallet",
        payment_method_type: "ali_pay_hk",
        payment_method_data: {
          wallet: {
            ali_pay_hk_redirect: {},
          },
        },
        billing: {
          ...standardBillingAddress,
          address: {
            ...standardBillingAddress.address,
            country: "HK",
          },
        },
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
  },
  real_time_payment_pm: {
    PaymentIntent: getCustomExchange({
      Request: {
        currency: "MYR",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: standardBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
    DuitNow: getCustomExchange({
      Request: {
        payment_method: "real_time_payment",
        payment_method_type: "duit_now",
        payment_method_data: {
          real_time_payment: {
            duit_now: {},
          },
        },
        billing: standardBillingAddress,
      },
    }),
  },
  card_pm: {
    PaymentIntent: getCustomExchange({
      Request: {
        currency: "USD",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
    PaymentIntentOffSession: getCustomExchange({
      Request: {
        currency: "USD",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
    PaymentIntentWithShippingCost: getCustomExchange({
      Request: {
        currency: "USD",
        shipping_cost: 50,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          shipping_cost: 50,
          amount: 6000,
        },
      },
    }),
    PaymentConfirmWithShippingCost: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
    }),
    "3DSManualCapture": getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
    }),
    "3DSAutoCapture": getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
    }),
    SessionToken: {
      Response: {
        status: 200,
        body: {
          session_token: [],
        },
      },
    },
    No3DSManualCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
    }),
    No3DSAutoCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
    }),
    No3DSFailPayment: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
    }),
    ManualRetryPaymentDisabled: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 400,
        body: {
          type: "invalid_request",
          message:
            "You cannot confirm this payment because it has status failed, you can enable `manual_retry` in profile to try this payment again",
          code: "IR_16",
        },
      },
    }),
    ManualRetryPaymentEnabled: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method: "card",
          attempt_count: 2,
        },
      },
    }),
    ManualRetryPaymentCutoffExpired: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 400,
        body: {
          type: "invalid_request",
          message:
            "You cannot confirm this payment using `manual_retry` because the allowed duration has expired",
          code: "IR_16",
        },
      },
    }),
    Capture: getCustomExchange({
      Request: {
        amount_to_capture: 6000,
      },
    }),
    PartialCapture: getCustomExchange({
      Request: {
        amount_to_capture: 2000,
      },
    }),
    Void: getCustomExchange({
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "cancelled",
          capture_method: "manual",
        },
      },
      ResponseCustom: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "You cannot cancel this payment because it has status succeeded",
            code: "IR_16",
          },
        },
      },
    }),
    VoidAfterConfirm: getCustomExchange({
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "cancelled",
          capture_method: "manual",
        },
      },
      ResponseCustom: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "You cannot cancel this payment because it has status succeeded",
            code: "IR_16",
          },
        },
      },
    }),
    Refund: getCustomExchange({
      Request: {
        amount: 6000,
      },
    }),
    manualPaymentRefund: getCustomExchange({
      Request: {
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
        },
      },
    }),
    manualPaymentPartialRefund: getCustomExchange({
      Request: {
        amount: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
        },
      },
    }),
    PartialRefund: getCustomExchange({
      Request: {
        amount: 2000,
      },
    }),
    SyncRefund: getCustomExchange({}),
    MandateSingleUse3DSAutoCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
      },
    }),
    MandateSingleUse3DSManualCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
      },
    }),
    MandateSingleUseNo3DSAutoCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
      },
    }),
    MandateSingleUseNo3DSManualCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
      },
    }),
    MandateMultiUseNo3DSAutoCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: multiUseMandateData,
      },
    }),
    MandateMultiUseNo3DSManualCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: multiUseMandateData,
      },
    }),
    MandateMultiUse3DSAutoCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        mandate_data: multiUseMandateData,
      },
    }),
    MandateMultiUse3DSManualCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        mandate_data: multiUseMandateData,
      },
    }),
    ZeroAuthMandate: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
      },
    }),
    ZeroAuthPaymentIntent: getCustomExchange({
      Request: {
        amount: 0,
        setup_future_usage: "off_session",
        currency: "USD",
        payment_type: "setup_mandate",
      },
    }),
    ZeroAuthConfirmPayment: getCustomExchange({
      Request: {
        payment_type: "setup_mandate",
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        mandate_data: null,
        customer_acceptance: customerAcceptance,
      },
    }),
    SaveCardUseNo3DSAutoCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
      },
    }),
    SaveCardUseNo3DSAutoCaptureOffSession: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
      },
    }),
    SaveCardUse3DSAutoCaptureOffSession: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
      },
    }),
    SaveCardUseNo3DSManualCaptureOffSession: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
      },
    }),
    SaveCardConfirmAutoCaptureOffSession: getCustomExchange({
      Request: {
        setup_future_usage: "off_session",
      },
      ResponseCustom: {
        status: 400,
        body: {
          error: {
            message:
              "No eligible connector was found for the current payment method configuration",
            type: "invalid_request",
          },
        },
      },
    }),
    SaveCardConfirmManualCaptureOffSession: getCustomExchange({
      Request: {
        setup_future_usage: "off_session",
      },
    }),
    SaveCardConfirmAutoCaptureOffSessionWithoutBilling: {
      Request: {
        setup_future_usage: "off_session",
        billing: null,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          billing: null,
        },
      },
    },
    SaveCardUseNo3DSManualCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
      },
    }),
    PaymentMethod: {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_issuer: "Gpay",
        payment_method_issuer_code: "jp_hdfc",
        card: PaymentMethodCardDetails,
      },
      Response: {
        status: 200,
        body: {},
      },
    },
    PaymentMethodIdMandateNo3DSAutoCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
      },
    }),
    PaymentMethodIdMandateNo3DSManualCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
      },
    }),
    PaymentMethodIdMandate3DSAutoCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        authentication_type: "three_ds",
        customer_acceptance: customerAcceptance,
      },
    }),
    PaymentMethodIdMandate3DSManualCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        mandate_data: null,
        authentication_type: "three_ds",
        customer_acceptance: customerAcceptance,
      },
    }),
    InvalidCardNumber: {
      Request: {
        currency: "USD",
        payment_method: "card",
        payment_method_type: "debit",
        setup_future_usage: "on_session",
        payment_method_data: {
          card: {
            card_number: "123456",
            card_exp_month: "10",
            card_exp_year: "25",
            card_holder_name: "joseph Doe",
            card_cvc: "123",
          },
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            error_type: "invalid_request",
            message: "Json deserialize error: invalid card number length",
            code: "IR_06",
          },
        },
      },
    },
    InvalidExpiryMonth: {
      Request: {
        currency: "USD",
        payment_method: "card",
        payment_method_type: "debit",
        setup_future_usage: "on_session",
        payment_method_data: {
          card: {
            card_number: "4242424242424242",
            card_exp_month: "00",
            card_exp_year: "2023",
            card_holder_name: "joseph Doe",
            card_cvc: "123",
          },
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Invalid Expiry Month",
            code: "IR_16",
          },
        },
      },
    },
    InvalidExpiryYear: {
      Request: {
        currency: "USD",
        payment_method: "card",
        payment_method_type: "debit",
        setup_future_usage: "on_session",
        payment_method_data: {
          card: {
            card_number: "4242424242424242",
            card_exp_month: "01",
            card_exp_year: "2023",
            card_holder_name: "joseph Doe",
            card_cvc: "123",
          },
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Invalid Expiry Year",
            code: "IR_16",
          },
        },
      },
    },
    InvalidCardCvv: {
      Request: {
        currency: "USD",
        payment_method: "card",
        payment_method_type: "debit",
        setup_future_usage: "on_session",
        payment_method_data: {
          card: {
            card_number: "4242424242424242",
            card_exp_month: "01",
            card_exp_year: "2023",
            card_holder_name: "joseph Doe",
            card_cvc: "123456",
          },
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Invalid card_cvc length",
            code: "IR_16",
          },
        },
      },
    },
    InvalidCurrency: {
      Request: {
        currency: "United",
        payment_method: "card",
        payment_method_type: "debit",
        setup_future_usage: "on_session",
        payment_method_data: {
          card: {
            card_number: "4242424242424242",
            card_exp_month: "01",
            card_exp_year: "2023",
            card_holder_name: "joseph Doe",
            card_cvc: "123456",
          },
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            error_type: "invalid_request",
            message:
              "Json deserialize error: unknown variant `United`, expected one of `AED`, `AFN`, `ALL`, `AMD`, `ANG`, `AOA`, `ARS`, `AUD`, `AWG`, `AZN`, `BAM`, `BBD`, `BDT`, `BGN`, `BHD`, `BIF`, `BMD`, `BND`, `BOB`, `BRL`, `BSD`, `BTN`, `BWP`, `BYN`, `BZD`, `CAD`, `CDF`, `CHF`, `CLF`, `CLP`, `CNY`, `COP`, `CRC`, `CUC`, `CUP`, `CVE`, `CZK`, `DJF`, `DKK`, `DOP`, `DZD`, `EGP`, `ERN`, `ETB`, `EUR`, `FJD`, `FKP`, `GBP`, `GEL`, `GHS`, `GIP`, `GMD`, `GNF`, `GTQ`, `GYD`, `HKD`, `HNL`, `HRK`, `HTG`, `HUF`, `IDR`, `ILS`, `INR`, `IQD`, `IRR`, `ISK`, `JMD`, `JOD`, `JPY`, `KES`, `KGS`, `KHR`, `KMF`, `KPW`, `KRW`, `KWD`, `KYD`, `KZT`, `LAK`, `LBP`, `LKR`, `LRD`, `LSL`, `LYD`, `MAD`, `MDL`, `MGA`, `MKD`, `MMK`, `MNT`, `MOP`, `MRU`, `MUR`, `MVR`, `MWK`, `MXN`, `MYR`, `MZN`, `NAD`, `NGN`, `NIO`, `NOK`, `NPR`, `NZD`, `OMR`, `PAB`, `PEN`, `PGK`, `PHP`, `PKR`, `PLN`, `PYG`, `QAR`, `RON`, `RSD`, `RUB`, `RWF`, `SAR`, `SBD`, `SCR`, `SDG`, `SEK`, `SGD`, `SHP`, `SLE`, `SLL`, `SOS`, `SRD`, `SSP`, `STD`, `STN`, `SVC`, `SYP`, `SZL`, `THB`, `TJS`, `TMT`, `TND`, `TOP`, `TRY`, `TTD`, `TWD`, `TZS`, `UAH`, `UGX`, `USD`, `UYU`, `UZS`, `VES`, `VND`, `VUV`, `WST`, `XAF`, `XCD`, `XOF`, `XPF`, `YER`, `ZAR`, `ZMW`, `ZWL`",
            code: "IR_06",
          },
        },
      },
    },
    InvalidCaptureMethod: {
      Request: {
        currency: "USD",
        capture_method: "auto",
        payment_method: "card",
        payment_method_type: "debit",
        setup_future_usage: "on_session",
        payment_method_data: {
          card: {
            card_number: "4242424242424242",
            card_exp_month: "01",
            card_exp_year: "2023",
            card_holder_name: "joseph Doe",
            card_cvc: "123456",
          },
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            error_type: "invalid_request",
            message:
              "Json deserialize error: unknown variant `auto`, expected one of `automatic`, `manual`, `manual_multiple`, `scheduled`",
            code: "IR_06",
          },
        },
      },
    },
    InvalidPaymentMethod: {
      Request: {
        currency: "USD",
        payment_method: "this_supposed_to_be_a_card",
        payment_method_type: "debit",
        setup_future_usage: "on_session",
        payment_method_data: {
          card: {
            card_number: "4242424242424242",
            card_exp_month: "01",
            card_exp_year: "2023",
            card_holder_name: "joseph Doe",
            card_cvc: "123456",
          },
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            error_type: "invalid_request",
            message:
              "Json deserialize error: unknown variant `this_supposed_to_be_a_card`, expected one of `card`, `card_redirect`, `pay_later`, `wallet`, `bank_redirect`, `bank_transfer`, `crypto`, `bank_debit`, `reward`, `real_time_payment`, `upi`, `voucher`, `gift_card`, `open_banking`, `mobile_payment`",
            code: "IR_06",
          },
        },
      },
    },
    InvalidAmountToCapture: {
      Request: {
        currency: "USD",
        amount_to_capture: 10000,
        payment_method: "card",
        payment_method_type: "debit",
        setup_future_usage: "on_session",
        payment_method_data: {
          card: {
            card_number: "4242424242424242",
            card_exp_month: "01",
            card_exp_year: "2030",
            card_holder_name: "joseph Doe",
            card_cvc: "123",
          },
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "amount_to_capture contains invalid data. Expected format is amount_to_capture lesser than amount",
            code: "IR_05",
          },
        },
      },
    },
    MissingRequiredParam: {
      Request: {
        currency: "USD",
        payment_method_type: "debit",
        setup_future_usage: "on_session",
        payment_method_data: {
          card: {
            card_number: "4242424242424242",
            card_exp_month: "01",
            card_exp_year: "2030",
            card_holder_name: "joseph Doe",
            card_cvc: "123",
          },
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Missing required param: payment_method",
            code: "IR_04",
          },
        },
      },
    },
    PaymentIntentErrored: {
      Request: {
        currency: "USD",
      },
      Response: {
        status: 422,
        body: {
          error: {
            type: "invalid_request",
            message:
              "A payment token or payment method data or ctp service details is required",
            code: "IR_06",
          },
        },
      },
    },
    CaptureGreaterAmount: {
      Request: {
        amount_to_capture: 6000000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "amount_to_capture is greater than amount",
            code: "IR_06",
          },
        },
      },
    },
    CaptureCapturedAmount: getCustomExchange({
      Request: {
        Request: {
          amount_to_capture: 6000,
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "This Payment could not be captured because it has a payment.status of succeeded. The expected state is requires_capture, partially_captured_and_capturable, processing",
            code: "IR_14",
          },
        },
      },
    }),
    ConfirmSuccessfulPayment: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "You cannot confirm this payment because it has status succeeded",
            code: "IR_16",
          },
        },
      },
    }),
    RefundGreaterAmount: {
      Request: {
        amount: 6000000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "The refund amount exceeds the amount captured",
            code: "IR_13",
          },
        },
      },
    },
    MITAutoCapture: getCustomExchange({
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
      ResponseCustom: {
        status: 400,
        body: {
          error: {
            message:
              "No eligible connector was found for the current payment method configuration",
            type: "invalid_request",
          },
        },
      },
    }),
    MITWithoutBillingAddress: getCustomExchange({
      Request: {
        billing: null,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),
    PartnerMerchantIdentifier: getCustomExchange({
      Request: {
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "San Francisco",
            state: "California",
            zip: "94122",
            country: "US",
            first_name: "joseph",
            last_name: "Doe",
          },
        },
        partner_merchant_identifier_details: {
          partner_details: {
            name: "TestPartner",
            version: "1.0.0",
            integrator: "TestIntegrator123",
          },
          merchant_details: {
            name: "TestMerchantApp",
            version: "2.0.0",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
    PaymentWithoutBilling: {
      Request: {
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        authentication_type: "no_three_ds",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    PaymentWithBilling: {
      Request: {
        currency: "USD",
        setup_future_usage: "on_session",
        billing: {
          address: {
            line1: "1467",
            line2: "CA",
            line3: "Harrison Street",
            city: "San Fransico",
            state: "CA",
            zip: "94122",
            country: "US",
            first_name: "joseph",
            last_name: "Doe",
          },
          phone: {
            number: "9111222333",
            country_code: "+91",
          },
        },
        email: "hyperswitch.example@gmail.com",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    PaymentWithFullName: {
      Request: {
        currency: "USD",
        setup_future_usage: "on_session",
        billing: {
          address: {
            first_name: "joseph",
            last_name: "Doe",
          },
          phone: {
            number: "9111222333",
            country_code: "+91",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    PaymentWithBillingEmail: {
      Request: {
        currency: "USD",
        setup_future_usage: "on_session",
        email: "hyperswitch_sdk_demo_id1@gmail.com",
        billing: {
          address: {
            first_name: "joseph",
            last_name: "Doe",
          },
          phone: {
            number: "9111222333",
            country_code: "+91",
          },
          email: "hyperswitch.example@gmail.com",
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    DuplicatePaymentID: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "The payment with the specified payment_id already exists in our records",
            code: "HE_01",
          },
        },
      },
    },
    DuplicateRefundID: {
      Request: {
        amount: 2000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "Duplicate refund request. Refund already attempted with the refund ID",
            code: "HE_01",
          },
        },
      },
    },
    InvalidPublishableKey: {
      Request: {},
      Response: {
        status: 401,
        body: {
          error: {
            type: "invalid_request",
            message: "API key not provided or invalid API key used",
            code: "IR_01",
          },
        },
      },
    },
    DDCRaceConditionServerSide: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
      DDCConfig: {
        completeUrlPath: "/redirect/complete/default",
        collectionReferenceParam: "collectionReference",
        firstSubmissionValue: "",
        secondSubmissionValue: "race_condition_test_ddc_123",
        expectedError: {
          status: 400,
          body: {
            error: {
              code: "IR_07",
              type: "invalid_request",
              message:
                "Invalid value provided: collection_reference not allowed in AuthenticationPending state",
            },
          },
        },
      },
    }),
    DDCRaceConditionClientSide: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
      DDCConfig: {
        redirectUrlPath: "/payments/redirect",
        collectionReferenceParam: "collectionReference",
        delayBeforeSubmission: 2000,
        raceConditionScript: `
          <script>
            console.log("INJECTING_RACE_CONDITION_TEST");
            
            // Track submission attempts and ddcProcessed flag behavior
            window.testResults = {
              submissionAttempts: 0,
              actualSubmissions: 0,
              blockedSubmissions: 0
            };
            
            // Override the submitCollectionReference function to test race conditions
            var originalSubmit = window.submitCollectionReference;
            
            window.submitCollectionReference = function(collectionReference) {
              window.testResults.submissionAttempts++;
              console.log("SUBMISSION_ATTEMPT_" + window.testResults.submissionAttempts + ": " + collectionReference);
              
              // Check if ddcProcessed flag would block this
              if (window.ddcProcessed) {
                window.testResults.blockedSubmissions++;
                console.log("SUBMISSION_BLOCKED_BY_DDC_PROCESSED_FLAG");
                return;
              }
              
              window.testResults.actualSubmissions++;
              console.log("SUBMISSION_PROCEEDING: " + collectionReference);
              
              if (originalSubmit) {
                return originalSubmit(collectionReference);
              }
            };
            
            // Submit first value at configured timing
            setTimeout(function() {
              console.log("FIRST_SUBMISSION_TRIGGERED_AT_100MS");
              window.submitCollectionReference("");
            }, 100);
            
            // Submit second value at configured timing (should be blocked)
            setTimeout(function() {
              console.log("SECOND_SUBMISSION_ATTEMPTED_AT_200MS");
              window.submitCollectionReference("test_ddc_123");
            }, 200);
          </script>
        `,
      },
    }),
    PaymentIntentWithInstallments: getCustomExchange({
      Request: {
        currency: "BRL",
        installment_options: [
          {
            payment_method: "card",
            installments: [
              {
                number_of_installments: [3, 6, 12],
                billing_frequency: "month",
                interest_rate: 5.0,
              },
            ],
          },
        ],
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
    CardInstallmentConfirm: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        installment_data: {
          number_of_installments: 3,
          billing_frequency: "month",
        },
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),
    PaymentIntentWithInstallmentsAndConfirmTrue: getCustomExchange({
      Request: {
        currency: "BRL",
        confirm: true,
        installment_options: [
          {
            payment_method: "card",
            installments: [
              {
                number_of_installments: [3, 6, 12],
                billing_frequency: "month",
                interest_rate: 5.0,
              },
            ],
          },
        ],
      },
      Response: {
        status: 422,
        body: {
          error: {
            type: "invalid_request",
            message:
              "installment_options and installment_data are not supported when confirm is true.",
            code: "IR_06",
          },
        },
      },
    }),
    external_three_ds: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        authentication_type: "three_ds",
        request_external_three_ds_authentication: true,
        three_ds_data: {
          authentication_cryptogram: {
            cavv: {
              authentication_cryptogram: "3q2+78r+ur7erb7vyv66vv////8=",
            },
          },
          ds_trans_id: "c4e59ceb-a382-4d6a-bc87-385d591fa09d",
          version: "2.1.0",
          eci: "05",
          transaction_status: "Y",
          exemption_indicator: "low_value",
        },
      },
    }),
  },
  upi_pm: {
    PaymentIntent: getCustomExchange({
      Request: {
        currency: "INR",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
    UpiCollect: getCustomExchange({
      Request: {
        payment_method: "upi",
        payment_method_type: "upi_collect",
        payment_method_data: {
          upi: {
            upi_collect: {
              vpa_id: "successtest@iata",
            },
          },
        },
      },
    }),
    UpiIntent: getCustomExchange({
      Request: {
        payment_method: "upi",
        payment_method_type: "upi_intent",
        payment_method_data: {
          upi: {
            upi_intent: {},
          },
        },
      },
    }),
  },
  reward_pm: {
    PaymentIntentUSD: getCustomExchange({
      Request: {
        currency: "USD",
        amount: 6000,
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: standardBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
    PaymentIntentEUR: getCustomExchange({
      Request: {
        currency: "EUR",
        amount: 6000,
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: standardBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
    Evoucher: getCustomExchange({
      Request: {
        payment_method: "reward",
        payment_method_type: "evoucher",
        payment_method_data: "reward",
        billing: standardBillingAddress,
      },
    }),
    Classic: getCustomExchange({
      Request: {
        payment_method: "reward",
        payment_method_type: "classic",
        payment_method_data: "reward",
        billing: standardBillingAddress,
      },
    }),
  },
  crypto_pm: {
    PaymentIntent: getCustomExchange({
      Request: {
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
    CryptoCurrency: getCustomExchange({
      Request: {
        payment_method: "crypto",
        payment_method_type: "crypto_currency",
        payment_method_data: {
          crypto: {
            network: "bitcoin",
            pay_currency: "BTC",
          },
        },
        billing: standardBillingAddress,
      },
    }),
    CryptoCurrencyManualCapture: getCustomExchange({
      Request: {
        payment_method: "crypto",
        payment_method_type: "crypto_currency",
        payment_method_data: {
          crypto: {
            network: "bitcoin",
            pay_currency: "BTC",
          },
        },
        billing: standardBillingAddress,
      },
    }),
  },
  pm_list: {
    PmListResponse: {
      PmListNull: {
        payment_methods: [],
      },
      pmListDynamicFieldWithoutBilling: {
        payment_methods: [
          {
            payment_method: "card",
            payment_method_types: [
              {
                payment_method_type: "credit",
                card_networks: [],
                required_fields: {},
              },
            ],
          },
        ],
      },
      pmListDynamicFieldWithBilling: {
        payment_methods: [
          {
            payment_method: "card",
            payment_method_types: [
              {
                payment_method_type: "credit",
                card_networks: [],
                required_fields: {},
              },
            ],
          },
        ],
      },
      pmListDynamicFieldWithNames: {
        payment_methods: [
          {
            payment_method: "card",
            payment_method_types: [
              {
                payment_method_type: "credit",
                card_networks: [],
                required_fields: {},
              },
            ],
          },
        ],
      },
      pmListDynamicFieldWithEmail: {
        payment_methods: [
          {
            payment_method: "card",
            payment_method_types: [
              {
                payment_method_type: "credit",
                card_networks: [],
                required_fields: {},
              },
            ],
          },
        ],
      },
    },
  },
  return_url_variations: {
    return_url_too_long: getCustomExchange({
      Request: {
        customer_id: "customer_1234567890",
        return_url: "http://example.com/" + "a".repeat(2031),
      },
      Response: {
        status: 400,
        body: {
          error: {
            message:
              "return_url must be at most 2048 characters long. Received 2050 characters",
            code: "IR_06",
            type: "invalid_request",
          },
        },
      },
    }),
    return_url_invalid_format: getCustomExchange({
      Request: {
        return_url: "not_a_valid_url",
      },
      Response: {
        status: 400,
        body: {
          error: {
            message:
              'Json deserialize error: relative URL without a base: "not_a_valid_url" at line 1 column 357',
            code: "IR_06",
            error_type: "invalid_request",
          },
        },
      },
    }),
  },
  eligibility_api: {
    PaymentIntentForBlocklist: getCustomExchange({
      Request: {
        currency: "USD",
        amount: 6500,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
    BlocklistedCardDenied: getCustomExchange({
      Request: {
        payment_method_type: "card",
        payment_method_data: {
          card: blocklistedCardDetails,
          billing: standardBillingAddress,
        },
      },
      Response: {
        status: 200,
        body: {
          sdk_next_action: {
            next_action: {
              deny: {
                message:
                  "We're unable to accept this card, please try another card or a different payment method",
              },
            },
          },
        },
      },
    }),
    NonBlocklistedCardAllowed: getCustomExchange({
      Request: {
        payment_method_type: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: standardBillingAddress,
        },
      },
      Response: {
        status: 200,
        body: {
          // Should not have deny action for non-blocklisted cards
        },
      },
    }),
  },
  auth_service_eligibility: {
    OrgEnabledMerchantEnabled: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        amount: 6500,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          authentication_type: "three_ds",
        },
      },
    }),
    OrgEnabledMerchantDisabled: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        amount: 6500,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          authentication_type: "three_ds",
        },
      },
    }),
    OrgDisabledMerchantEnabled: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        amount: 6500,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          authentication_type: "no_three_ds",
        },
      },
    }),
    OrgDisabledMerchantDisabled: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        amount: 6500,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          authentication_type: "no_three_ds",
        },
      },
    }),
    MerchantOnlyEnabled: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        amount: 6500,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          authentication_type: "three_ds",
        },
      },
    }),
    MerchantOnlyDisabled: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        amount: 6500,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          authentication_type: "no_three_ds",
        },
      },
    }),
    NoConfigDefault: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        amount: 6500,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          authentication_type: "three_ds",
        },
      },
    }),
  },
  Dispute: {
    ListDisputes: {
      Response: {
        status: 200,
      },
    },
    ListDisputesWithConnectorFilter: {
      Response: {
        status: 200,
      },
    },
    ListDisputesWithTimeRange: {
      Response: {
        status: 200,
      },
    },
    ListDisputesWithInvalidStatusFilter: {
      Response: {
        status: 400,
      },
    },
    ListDisputesWithInvalidStageFilter: {
      Response: {
        status: 400,
      },
    },
    ListDisputesWithLimit: {
      Response: {
        status: 200,
      },
    },
    ListDisputesWithLargeTimeRange: {
      Response: {
        status: 200,
      },
    },
    RetrieveDispute: {
      Response: {
        status: 200,
      },
    },
    RetrieveNonExistentDispute: {
      Response: {
        status: 404,
        body: {
          error: {
            code: "HE_04",
          },
        },
      },
    },
    AcceptDispute: {
      Response: {
        status: 200,
      },
    },
    AcceptNonExistentDispute: {
      Response: {
        status: 404,
        body: {
          error: {
            code: "HE_04",
          },
        },
      },
    },
    SubmitEvidence: {
      Response: {
        status: 200,
      },
    },
    SubmitEvidenceNonExistentDispute: {
      Response: {
        status: 404,
        body: {
          error: {
            code: "HE_04",
          },
        },
      },
    },
    SubmitEvidenceEmptyBody: {
      Response: {
        status: 400,
      },
    },
    RetrieveEvidence: {
      Response: {
        status: 200,
      },
    },
    AttachEvidenceFileMissingType: {
      Response: {
        status: 400,
        body: {
          error: {
            code: "IR_04",
          },
        },
      },
    },
    FetchDisputes: {
      Response: {},
    },
    FetchDisputesWithTimeRange: {
      Response: {},
    },
    FetchDisputesMissingParams: {
      Response: {
        status: 400,
      },
    },
    FetchDisputesInvalidConnector: {
      Response: {
        status: 400,
      },
    },
  },
  mandate_id_too_long: getCustomExchange({
    Request: {
      mandate_id: "mnd_" + "a".repeat(63),
      off_session: true,
    },
    Response: {
      status: 400,
      body: {
        error: {
          message:
            "mandate_id must be at most 64 characters long. Received 67 characters",
          code: "IR_06",
          type: "invalid_request",
        },
      },
    },
  }),
  payment_method_blocking_pm: {
    BlockIssuingCountry: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: {
            card_number: "4000000000000002",
            card_exp_month: "03",
            card_exp_year: "30",
            card_holder_name: "joseph Doeeee",
            card_cvc: "737",
            card_network: "Visa",
          },
        },
      },
      Response: blockedPaymentErrorBodyForIssuingCountry,
    }),
    BlockCardType: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: {
            card_number: "4111111111111111",
            card_exp_month: "03",
            card_exp_year: "30",
            card_holder_name: "joseph Doeeee",
            card_cvc: "737",
            card_network: "Visa",
          },
        },
      },
      Response: blockedPaymentErrorBodyForDebitCard,
    }),
    BlockCardSubtype: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: {
            card_number: "378282246310005",
            card_exp_month: "03",
            card_exp_year: "30",
            card_holder_name: "joseph Doeeee",
            card_cvc: "737",
            card_network: "Visa",
          },
        },
      },
      Response: blockedPaymentErrorBodyForCardSubtype,
    }),
    BlockIfBinInfoUnavailable: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: {
            card_number: "6304000000000000",
            card_exp_month: "03",
            card_exp_year: "30",
            card_holder_name: "joseph Doeeee",
            card_cvc: "737",
            card_network: "Visa",
          },
        },
      },
      Response: blockedPaymentErrorBodyForBinUnavailable,
    }),
  },
  gift_card_pm: {
    GivexGiftCard: getCustomExchange({
      Request: {
        payment_method: "gift_card",
        payment_method_type: "givex",
        payment_method_data: {
          gift_card: {
            givex: {
              number: "6036280000000000000",
              cvc: "122222",
            },
          },
        },
        currency: "EUR",
        customer_acceptance: null,
      },
    }),
    GivexGiftCardInsufficientBalance: getCustomExchange({
      Request: {
        payment_method: "gift_card",
        payment_method_type: "givex",
        payment_method_data: {
          gift_card: {
            givex: {
              number: "6036280000000000000",
              cvc: "122222",
            },
          },
        },
        currency: "EUR",
        customer_acceptance: null,
      },
    }),
    GivexGiftCardCurrencyMismatch: getCustomExchange({
      Request: {
        payment_method: "gift_card",
        payment_method_type: "givex",
        payment_method_data: {
          gift_card: {
            givex: {
              number: "6036280000000000000",
              cvc: "122222",
            },
          },
        },
        currency: "USD",
        customer_acceptance: null,
      },
    }),
  },
};
