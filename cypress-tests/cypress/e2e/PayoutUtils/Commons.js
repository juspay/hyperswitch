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
    adyenplatform: "Adyenplatform",
    wise: "Wise",
    // Add more known exceptions here
  };

  if (exceptions[input.toLowerCase()]) {
    return exceptions[input.toLowerCase()];
  } else {
    return input;
  }
}

const card_data = {
  card_number: "4111111111111111",
  expiry_month: "3",
  expiry_year: "2030",
  card_holder_name: "John Smith",
};

const payment_card_data = {
  card_number: "4111111111111111",
  card_exp_month: "03",
  card_exp_year: "2030",
  card_holder_name: "John Doe",
};

const billing = {
  address: {
    line1: "Raadhuisplein",
    line2: "92",
    city: "Hoogeveen",
    state: "FL",
    zip: "7901 BW",
    country: "NL",
    first_name: "John",
    last_name: "Doe",
  },
  phone: {
    number: "0650242319",
    country_code: "+31",
  },
};

/*
`getDefaultExchange` contains the default Request and Response to be considered if none provided.
`getCustomExchange` takes in 2 optional fields named as Request and Response.
with `getCustomExchange`, if 501 response is expected, there is no need to pass Response as it considers default values.
*/

// Const to get default PaymentExchange object
const getDefaultExchange = () => ({
  Request: {},
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

export const connectorDetails = {
  card_pm: {
    Create: getCustomExchange({
      Request: {
        payout_type: "card",
        payout_method_data: {
          card: card_data,
        },
        currency: "EUR",
        payout_type: "card",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_creation",
          payout_type: "card",
        },
      },
    }),
    Confirm: getCustomExchange({
      Request: {
        payout_type: "card",
        payout_method_data: {
          card: card_data,
        },
        currency: "EUR",
        payout_type: "card",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_fulfillment",
          payout_type: "card",
        },
      },
    }),
    Fulfill: getCustomExchange({
      Request: {
        payout_type: "card",
        payout_method_data: {
          card: card_data,
        },
        currency: "EUR",
        payout_type: "card",
      },
    }),
    SavePayoutMethod: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        card: payment_card_data,
        metadata: {
          city: "NY",
          unit: "245",
        },
      },
      Response: {
        status: 200,
      },
    }),
    Token: getCustomExchange({
      Request: {
        payout_token: "token",
        payout_type: "card",
      },
    }),
  },
  bank_transfer_pm: {
    Create: getCustomExchange({
      Request: {
        payout_type: "bank",
        priority: "regular",
        payout_method_data: {
          bank: {
            iban: "NL57INGB4654188101",
          },
        },
        billing: billing,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_creation",
          payout_type: "bank",
        },
      },
    }),
    Confirm: getCustomExchange({
      Request: {
        payout_type: "bank",
        priority: "regular",
        payout_method_data: {
          bank: {
            iban: "NL57INGB4654188101",
          },
        },
        billing: billing,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_fulfillment",
          payout_type: "bank",
        },
      },
    }),
    Fulfill: getCustomExchange({
      Request: {
        payout_type: "bank",
        priority: "regular",
        payout_method_data: {
          bank: {
            iban: "NL57INGB4654188101",
          },
        },
        billing: billing,
      },
    }),
    Token: getCustomExchange({
      Request: {
        payout_token: "token",
        payout_type: "card",
      },
    }),
  },
};
