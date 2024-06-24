// This file is the default. To override, add to connector.js
import State from "../../utils/State";

const globalState = new State({
  connectorId: Cypress.env("CONNECTOR"),
  baseUrl: Cypress.env("BASEURL"),
  adminApiKey: Cypress.env("ADMINAPIKEY"),
  connectorAuthFilePath: Cypress.env("CONNECTOR_AUTH_FILE_PATH"),
});

const connectorName = normalise(globalState.get("connectorId"));

function normalise(input) {
  const exceptions = {
    bankofamerica: "Bank of America",
    cybersource: "Cybersource",
    paypal: "Paypal",
    // Add more known exceptions here
  };

  if (exceptions[input.toLowerCase()]) {
    return exceptions[input.toLowerCase()];
  } else {
    return input;
  }
}

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
        message: `Selected payment method through ${connectorName} is not implemented`,
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

export const connectorDetails = {
  card_pm: {
    PaymentIntent: getCustomExchange({
      Request: {
        card: successfulNo3DSCardDetails,
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
    "3DSManualCapture": {
      Request: {
        card: successfulThreeDSTestCardDetails,
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    "3DSAutoCapture": {
      Request: {
        card: successfulThreeDSTestCardDetails,
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    No3DSManualCapture: {
      Request: {
        card: successfulNo3DSCardDetails,
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    No3DSAutoCapture: {
      Request: {
        card: successfulNo3DSCardDetails,
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    Capture: {
      Request: {
        card: successfulNo3DSCardDetails,
        currency: "USD",
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
          amount: 6500,
          amount_capturable: 6500,
        },
      },
    },
    PartialCapture: {
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "processing",
          amount: 6500,
          amount_capturable: 6500,
        },
      },
    },
    Void: {
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            code: "IR_16",
            message:
              "You cannot cancel this payment because it has status processing",
            type: "invalid_request",
          },
        },
      },
    },
    Refund: {
      Request: {
        card: successfulNo3DSCardDetails,
        currency: "USD",
        customer_acceptance: null,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "This Payment could not be refund because it has a status of processing. The expected state is succeeded, partially_captured",
            code: "IR_14",
          },
        },
      },
    },
    PartialRefund: {
      Request: {
        card: successfulNo3DSCardDetails,
        currency: "USD",
        customer_acceptance: null,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "This Payment could not be refund because it has a status of processing. The expected state is succeeded, partially_captured",
            code: "IR_14",
          },
        },
      },
    },
    SyncRefund: {
      Request: {
        card: successfulNo3DSCardDetails,
        currency: "USD",
        customer_acceptance: null,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "This Payment could not be refund because it has a status of processing. The expected state is succeeded, partially_captured",
            code: "IR_14",
          },
        },
      },
    },
    MandateSingleUse3DSAutoCapture: getCustomExchange({
      Request: {
        card: successfulThreeDSTestCardDetails,
        currency: "USD",
        mandate_type: {
          single_use: {
            amount: 8000,
            currency: "USD",
          },
        },
      },
    }),
    MandateSingleUse3DSManualCapture: getCustomExchange({
      Request: {
        card: successfulThreeDSTestCardDetails,
        currency: "USD",
        mandate_type: {
          single_use: {
            amount: 8000,
            currency: "USD",
          },
        },
      },
    }),
    MandateSingleUseNo3DSAutoCapture: getCustomExchange({
      Request: {
        card: successfulNo3DSCardDetails,
        currency: "USD",
        mandate_type: {
          single_use: {
            amount: 8000,
            currency: "USD",
          },
        },
      },
    }),
    MandateSingleUseNo3DSManualCapture: getCustomExchange({
      Request: {
        card: successfulNo3DSCardDetails,
        currency: "USD",
        mandate_type: {
          single_use: {
            amount: 8000,
            currency: "USD",
          },
        },
      },
    }),
    MandateMultiUseNo3DSAutoCapture: getCustomExchange({
      Request: {
        card: successfulNo3DSCardDetails,
        currency: "USD",
        mandate_type: {
          single_use: {
            amount: 8000,
            currency: "USD",
          },
        },
      },
    }),
    MandateMultiUseNo3DSManualCapture: getCustomExchange({
      Request: {
        card: successfulNo3DSCardDetails,
        currency: "USD",
        mandate_type: {
          multi_use: {
            amount: 8000,
            currency: "USD",
          },
        },
      },
    }),
    MandateMultiUse3DSAutoCapture: getCustomExchange({
      Request: {
        card: successfulThreeDSTestCardDetails,
        currency: "USD",
        mandate_type: {
          multi_use: {
            amount: 8000,
            currency: "USD",
          },
        },
      },
    }),
    MandateMultiUse3DSManualCapture: getCustomExchange({
      Request: {
        card: successfulThreeDSTestCardDetails,
        currency: "USD",
        mandate_type: {
          multi_use: {
            amount: 8000,
            currency: "USD",
          },
        },
      },
    }),
    ZeroAuthMandate: getCustomExchange({
      Request: {
        card: successfulNo3DSCardDetails,
        currency: "USD",
        mandate_type: {
          single_use: {
            amount: 8000,
            currency: "USD",
          },
        },
      },
    }),
    SaveCardUseNo3DSAutoCapture: getCustomExchange({
      Request: {
        card: successfulNo3DSCardDetails,
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
    SaveCardUseNo3DSManualCapture: getCustomExchange({
      Request: {
        card: successfulNo3DSCardDetails,
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
  },
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
    ideal: getCustomExchange({
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
    giropay: getCustomExchange({
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
    sofort: getCustomExchange({
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
    eps: getCustomExchange({
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
    przelewy24: getCustomExchange({
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
    blikPaymentIntent: getCustomExchange({
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
    blik: getCustomExchange({
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
};
