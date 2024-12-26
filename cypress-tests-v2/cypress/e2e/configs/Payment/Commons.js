// This file is the default. To override, add to connector.js

import { getCustomExchange } from "./_Reusable";

const successfulNo3DSCardDetails = {
  card_number: "4111111111111111",
  card_exp_month: "08",
  card_exp_year: "25",
  card_holder_name: "joseph Doe",
  card_cvc: "999",
};

const successfulThreeDSTestCardDetails = {
  card_number: "4111111111111111",
  card_exp_month: "10",
  card_exp_year: "25",
  card_holder_name: "morino",
  card_cvc: "999",
};

const singleUseMandateData = {
  customer_acceptance: {
    acceptance_type: "offline",
    accepted_at: "1963-05-03T04:07:52.723Z",
    online: {
      ip_address: "125.0.0.1",
      user_agent: "amet irure esse",
    },
  },
  mandate_type: {
    single_use: {
      amount: 8000,
      currency: "USD",
    },
  },
};

const multiUseMandateData = {
  customer_acceptance: {
    acceptance_type: "offline",
    accepted_at: "1963-05-03T04:07:52.723Z",
    online: {
      ip_address: "125.0.0.1",
      user_agent: "amet irure esse",
    },
  },
  mandate_type: {
    multi_use: {
      amount: 8000,
      currency: "USD",
    },
  },
};

export const payment_methods_enabled = [
  {
    payment_method: "bank_debit",
    payment_method_types: [
      {
        payment_method_type: "ach",
        payment_experience: null,
        card_networks: null,
        accepted_currencies: null,
        accepted_countries: null,
        minimum_amount: -1,
        maximum_amount: 68607706,
        recurring_enabled: true,
        installment_payment_enabled: true,
      },
      {
        payment_method_type: "bacs",
        payment_experience: null,
        card_networks: null,
        accepted_currencies: null,
        accepted_countries: null,
        minimum_amount: -1,
        maximum_amount: 68607706,
        recurring_enabled: true,
        installment_payment_enabled: true,
      },
      {
        payment_method_type: "becs",
        payment_experience: null,
        card_networks: null,
        accepted_currencies: null,
        accepted_countries: null,
        minimum_amount: -1,
        maximum_amount: 68607706,
        recurring_enabled: true,
        installment_payment_enabled: true,
      },
      {
        payment_method_type: "sepa",
        payment_experience: null,
        card_networks: null,
        accepted_currencies: null,
        accepted_countries: null,
        minimum_amount: -1,
        maximum_amount: 68607706,
        recurring_enabled: true,
        installment_payment_enabled: true,
      },
    ],
  },
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
    ],
  },
  {
    payment_method: "bank_transfer",
    payment_method_types: [
      {
        payment_method_type: "ach",
        payment_experience: "redirect_to_url",
        card_networks: null,
        accepted_currencies: null,
        accepted_countries: null,
        minimum_amount: -1,
        maximum_amount: 68607706,
        recurring_enabled: true,
        installment_payment_enabled: true,
      },
      {
        payment_method_type: "bacs",
        payment_experience: "redirect_to_url",
        card_networks: null,
        accepted_currencies: null,
        accepted_countries: null,
        minimum_amount: -1,
        maximum_amount: 68607706,
        recurring_enabled: true,
        installment_payment_enabled: true,
      },
      {
        payment_method_type: "pix",
        minimum_amount: -1,
        maximum_amount: 68607706,
        recurring_enabled: false,
        installment_payment_enabled: true,
      },
      {
        payment_method_type: "sepa",
        payment_experience: "redirect_to_url",
        card_networks: null,
        accepted_currencies: null,
        accepted_countries: null,
        minimum_amount: -1,
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
        minimum_amount: -1,
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
        minimum_amount: -1,
        maximum_amount: 68607706,
        recurring_enabled: false,
        installment_payment_enabled: true,
      },
    ],
  },
  {
    payment_method: "card_redirect",
    payment_method_types: [
      {
        payment_method_type: "card_redirect",
        payment_experience: "redirect_to_url",
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
];

export const connectorDetails = {
  bank_transfer_pm: {
    PaymentIntent: getCustomExchange({
      Request: {
        currency: "BRL",
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
  },
  bank_redirect_pm: {
    PaymentIntent: getCustomExchange({
      Request: {
        currency: "EUR",
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
    BlikPaymentIntent: getCustomExchange({
      Request: {
        currency: "PLN",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
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
    Capture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
      },
    }),
    PartialCapture: getCustomExchange({
      Request: {},
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
    Refund: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
      },
      ResponseCustom: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "The refund amount exceeds the amount captured",
            code: "IR_13",
          },
        },
      },
    }),
    PartialRefund: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
      },
    }),
    SyncRefund: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
      },
    }),
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
    SaveCardUseNo3DSAutoCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        setup_future_usage: "on_session",
        customer_acceptance: {
          acceptance_type: "offline",
          accepted_at: "1963-05-03T04:07:52.723Z",
          online: {
            ip_address: "127.0.0.1",
            user_agent: "amet irure esse",
          },
        },
      },
    }),
    SaveCardUseNo3DSAutoCaptureOffSession: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        setup_future_usage: "off_session",
        customer_acceptance: {
          acceptance_type: "offline",
          accepted_at: "1963-05-03T04:07:52.723Z",
          online: {
            ip_address: "127.0.0.1",
            user_agent: "amet irure esse",
          },
        },
      },
    }),
    SaveCardUseNo3DSManualCaptureOffSession: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        setup_future_usage: "off_session",
        customer_acceptance: {
          acceptance_type: "offline",
          accepted_at: "1963-05-03T04:07:52.723Z",
          online: {
            ip_address: "127.0.0.1",
            user_agent: "amet irure esse",
          },
        },
      },
    }),
    SaveCardConfirmAutoCaptureOffSession: getCustomExchange({
      Request: {
        setup_future_usage: "off_session",
      },
    }),
    SaveCardConfirmManualCaptureOffSession: getCustomExchange({
      Request: {
        setup_future_usage: "off_session",
      },
    }),
    SaveCardUseNo3DSManualCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        setup_future_usage: "on_session",
        customer_acceptance: {
          acceptance_type: "offline",
          accepted_at: "1963-05-03T04:07:52.723Z",
          online: {
            ip_address: "127.0.0.1",
            user_agent: "amet irure esse",
          },
        },
      },
    }),
    PaymentMethodIdMandateNo3DSAutoCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        customer_acceptance: {
          acceptance_type: "offline",
          accepted_at: "1963-05-03T04:07:52.723Z",
          online: {
            ip_address: "125.0.0.1",
            user_agent: "amet irure esse",
          },
        },
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
        customer_acceptance: {
          acceptance_type: "offline",
          accepted_at: "1963-05-03T04:07:52.723Z",
          online: {
            ip_address: "125.0.0.1",
            user_agent: "amet irure esse",
          },
        },
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
        customer_acceptance: {
          acceptance_type: "offline",
          accepted_at: "1963-05-03T04:07:52.723Z",
          online: {
            ip_address: "125.0.0.1",
            user_agent: "amet irure esse",
          },
        },
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
        customer_acceptance: {
          acceptance_type: "offline",
          accepted_at: "1963-05-03T04:07:52.723Z",
          online: {
            ip_address: "125.0.0.1",
            user_agent: "amet irure esse",
          },
        },
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
            code: "IR_06"
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
            message: "Json deserialize error: unknown variant `United`, expected one of `AED`, `AFN`, `ALL`, `AMD`, `ANG`, `AOA`, `ARS`, `AUD`, `AWG`, `AZN`, `BAM`, `BBD`, `BDT`, `BGN`, `BHD`, `BIF`, `BMD`, `BND`, `BOB`, `BRL`, `BSD`, `BTN`, `BWP`, `BYN`, `BZD`, `CAD`, `CDF`, `CHF`, `CLP`, `CNY`, `COP`, `CRC`, `CUP`, `CVE`, `CZK`, `DJF`, `DKK`, `DOP`, `DZD`, `EGP`, `ERN`, `ETB`, `EUR`, `FJD`, `FKP`, `GBP`, `GEL`, `GHS`, `GIP`, `GMD`, `GNF`, `GTQ`, `GYD`, `HKD`, `HNL`, `HRK`, `HTG`, `HUF`, `IDR`, `ILS`, `INR`, `IQD`, `IRR`, `ISK`, `JMD`, `JOD`, `JPY`, `KES`, `KGS`, `KHR`, `KMF`, `KPW`, `KRW`, `KWD`, `KYD`, `KZT`, `LAK`, `LBP`, `LKR`, `LRD`, `LSL`, `LYD`, `MAD`, `MDL`, `MGA`, `MKD`, `MMK`, `MNT`, `MOP`, `MRU`, `MUR`, `MVR`, `MWK`, `MXN`, `MYR`, `MZN`, `NAD`, `NGN`, `NIO`, `NOK`, `NPR`, `NZD`, `OMR`, `PAB`, `PEN`, `PGK`, `PHP`, `PKR`, `PLN`, `PYG`, `QAR`, `RON`, `RSD`, `RUB`, `RWF`, `SAR`, `SBD`, `SCR`, `SDG`, `SEK`, `SGD`, `SHP`, `SLE`, `SLL`, `SOS`, `SRD`, `SSP`, `STN`, `SVC`, `SYP`, `SZL`, `THB`, `TJS`, `TMT`, `TND`, `TOP`, `TRY`, `TTD`, `TWD`, `TZS`, `UAH`, `UGX`, `USD`, `UYU`, `UZS`, `VES`, `VND`, `VUV`, `WST`, `XAF`, `XCD`, `XOF`, `XPF`, `YER`, `ZAR`, `ZMW`, `ZWL`",
            code: "IR_06"
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
            message: "Json deserialize error: unknown variant `auto`, expected one of `automatic`, `manual`, `manual_multiple`, `scheduled`",
            code: "IR_06"
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
            message: "Json deserialize error: unknown variant `this_supposed_to_be_a_card`, expected one of `card`, `card_redirect`, `pay_later`, `wallet`, `bank_redirect`, `bank_transfer`, `crypto`, `bank_debit`, `reward`, `real_time_payment`, `upi`, `voucher`, `gift_card`, `open_banking`, `mobile_payment`",
            code: "IR_06"
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
            card_exp_year: "2026",
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
            card_exp_year: "2026",
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
            message: "A payment token or payment method data or ctp service details is required",
            code: "IR_06",
          },
        },
      },
    },
    CaptureGreaterAmount: {
      Request: {
        Request: {
          payment_method: "card",
          payment_method_data: {
            card: successfulNo3DSCardDetails,
          },
          currency: "USD",
          customer_acceptance: null,
        },
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
          payment_method: "card",
          payment_method_data: {
            card: successfulNo3DSCardDetails,
          },
          currency: "USD",
          customer_acceptance: null,
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
};
