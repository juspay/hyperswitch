import State from "../../utils/State";
import {
  connectorDetails as CommonConnectorDetails,
  updateDefaultStatusCode,
} from "./Commons.js";
import { connectorDetails as stripeConnectorDetails } from "./Stripe.js";

const connectorDetails = {
  commons: CommonConnectorDetails,
  stripe: stripeConnectorDetails,
};


export default function getConnectorDetails(connectorId) {
  let x = mergeDetails(connectorId);
  return x;
}

function mergeDetails(connectorId) {
  const connectorData = getValueByKey(connectorDetails, connectorId);
  const fallbackData = getValueByKey(connectorDetails, "commons");
  // Merge data, prioritizing connectorData and filling missing data from fallbackData
  const mergedDetails = mergeConnectorDetails(connectorData, fallbackData);
  return mergedDetails;
}

function mergeConnectorDetails(source, fallback) {
  const merged = {};

  // Loop through each key in the source object
  for (const key in source) {
    merged[key] = { ...source[key] }; // Copy properties from source

    // Check if fallback has the same key and properties are missing in source
    if (fallback[key]) {
      for (const subKey in fallback[key]) {
        if (!merged[key][subKey]) {
          merged[key][subKey] = fallback[key][subKey];
        }
      }
    }
  }

  // Add missing keys from fallback that are not present in source
  for (const key in fallback) {
    if (!merged[key]) {
      merged[key] = fallback[key];
    }
  }

  return merged;
}

export function getValueByKey(jsonObject, key) {
  const data =
    typeof jsonObject === "string" ? JSON.parse(jsonObject) : jsonObject;

  if (data && typeof data === "object" && key in data) {
    return data[key];
  } else {
    return null;
  }
}

export const should_continue_further = (res_data) => {
  if (res_data.trigger_skip !== undefined) {
    return !res_data.trigger_skip;
  }

  if (
    res_data.body.error !== undefined ||
    res_data.body.error_code !== undefined ||
    res_data.body.error_message !== undefined
  ) {
    return false;
  } else {
    return true;
  }
};

export function defaultErrorHandler(response, response_data) {
  if (
    response.status === 400 &&
    response.body.error.message === "Payment method type not supported"
  ) {
    // Update the default status from 501 to 400 as `unsupported payment method` error is the next common error after `not implemented` error
    response_data = updateDefaultStatusCode();
  }

  if (response_data.status === 200) {
    throw new Error("Expecting valid response but got an error response");
  }

  expect(response.body).to.have.property("error");

  if (typeof response.body.error === "object") {
    for (const key in response_data.body.error) {
      expect(response_data.body.error[key]).to.equal(response.body.error[key]);
    }
  } else if (typeof response.body.error === "string") {
    expect(response.body.error).to.include(response_data.body.error);
  }
}

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
    paybox: "Paybox",
    paypal: "Paypal",
    wellsfargo: "Wellsfargo",
    // Add more known exceptions here
  };

  if (typeof input !== "string") {
    const spec_name = Cypress.spec.name.split("-")[1].split(".")[0];
    return `${spec_name}`;
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

const getUnsupportedExchange = () => ({
  Request: {
    currency: "EUR",
  },
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
