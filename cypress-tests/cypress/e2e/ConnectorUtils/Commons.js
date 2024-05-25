// This file is the default. To override, add to connector.js
import State from "../../utils/State";

const globalState = new State({
  connectorId: Cypress.env("CONNECTOR"),
  baseUrl: Cypress.env("BASEURL"),
  adminApiKey: Cypress.env("ADMINAPIKEY"),
  connectorAuthFilePath: Cypress.env("CONNECTOR_AUTH_FILE_PATH"),
});

const connectorId = globalState.get("connectorId");

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

// Function to get default PaymentIntent object
const getDefaultIntent = () => ({
  Request: {
    currency: "EUR",
  },
  Response: {
    status: 501,
    body: {
      error: {
        type: "invalid_request",
        message: `Selected payment method through ${connectorId} is not implemented`,
        code: "IR_00",
      },
    },
  },
});

// Function to get PaymentIntent with overridden properties
export const getCustomIntent = (overrides) => {
  const defaultIntent = getDefaultIntent();

  return {
    ...defaultIntent,
    Request: {
      ...defaultIntent.Request,
      ...(overrides.Request || {}),
    },
    Response: {
      ...defaultIntent.Response,
      ...(overrides.Response || {}),
    },
  };
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: getCustomIntent({
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
    MandateSingleUse3DSAutoCapture: getCustomIntent({
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
    MandateSingleUse3DSManualCapture: getCustomIntent({
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
    MandateSingleUseNo3DSAutoCapture: getCustomIntent({
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
    MandateSingleUseNo3DSManualCapture: getCustomIntent({
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
    MandateMultiUseNo3DSAutoCapture: getCustomIntent({
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
    MandateMultiUseNo3DSManualCapture: getCustomIntent({
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
    MandateMultiUse3DSAutoCapture: getCustomIntent({
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
    MandateMultiUse3DSManualCapture: getCustomIntent({
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
    ZeroAuthMandate: getCustomIntent({
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
    SaveCardUseNo3DSAutoCapture: getCustomIntent({
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
    SaveCardUseNo3DSManualCapture: getCustomIntent({
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
  },
  bank_transfer_pm: {
    PaymentIntent: getCustomIntent({
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
    Pix: getCustomIntent({
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "pix",
        bank_transfer: {
          pix: {},
        },
        currency: "USD",
      },
    }),
  },
  bank_redirect_pm: {
    PaymentIntent: getCustomIntent({
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
    ideal: getCustomIntent({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "ideal",
        bank_redirect: {
          ideal: {
            bank_name: "ing",
            country: "NL",
          },
        },
      },
    }),
    giropay: getCustomIntent({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "giropay",
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
    }),
    sofort: getCustomIntent({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "sofort",
        bank_redirect: {
          sofort: {
            country: "DE",
            preferred_language: "en",
          },
        },
      },
    }),
    eps: getCustomIntent({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "eps",
        bank_redirect: {
          eps: {
            bank_name: "ing",
          },
        },
      },
    }),
    blikPaymentIntent: getCustomIntent({
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
    blik: getCustomIntent({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "blik",
        bank_redirect: {
          blik: {
            name: "John Doe",
            email: "example@email.com",
          },
        },
        billing: {
          address: {
            country: "PL",
          },
        },
      },
    }),
  },
};
