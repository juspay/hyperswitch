import State from "../../../utils/State";
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
    noon: "Noon",
    paybox: "Paybox",
    paypal: "Paypal",
    wellsfargo: "Wellsfargo",
    fiuu: "Fiuu",
    // Add more known exceptions here
  };

  if (typeof input !== "string") {
    const specName = Cypress.spec.name;

    if (specName.includes("-")) {
      const parts = specName.split("-");

      if (parts.length > 1 && parts[1].includes(".")) {
        return parts[1].split(".")[0];
      }
    }

    // Fallback
    return `${specName}`;
  }

  const lowerCaseInput = input.toLowerCase();
  return exceptions[lowerCaseInput] || input;
}

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

const getUnsupportedExchange = () => ({
  Request: {},
  Response: {
    status: 400,
    body: {
      error: {
        type: "invalid_request",
        message: `Payment method type not supported`,
        code: "IR_19",
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
    ...(overrides.ResponseCustom
      ? { ResponseCustom: overrides.ResponseCustom }
      : {}),
  };
};

// Function to update the default status code
export const updateDefaultStatusCode = () => {
  return getUnsupportedExchange().Response;
};
