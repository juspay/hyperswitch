import { connectorDetails as CommonConnectorDetails } from "./Commons.js";
import { connectorDetails as noonConnectorDetails } from "./Noon.js";

const connectorDetails = {
  commons: CommonConnectorDetails,
  noon: noonConnectorDetails,
};

export default function getConnectorDetails(connectorId) {
  const x = mergeDetails(connectorId);
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
    const value = data[key];

    // Check if the value has connector_account_details
    if (value && typeof value === "object" && value.connector_account_details) {
      return value;
    }

    // Check if it has nested structure like connector_1, connector_2
    if (value && typeof value === "object") {
      // Default to connector_1 if it exists
      if (value.connector_1 && value.connector_1.connector_account_details) {
        return value.connector_1;
      }
      // Fallback to first key that has connector_account_details
      const keys = Object.keys(value);
      for (const nestedKey of keys) {
        if (value[nestedKey] && value[nestedKey].connector_account_details) {
          return value[nestedKey];
        }
      }
    }

    return value;
  } else {
    return null;
  }
}

export const should_continue_further = (res_data) => {
  if (res_data.trigger_skip !== undefined) {
    return !res_data.trigger_skip;
  }

  if (
    res_data.Response.body.error !== undefined ||
    res_data.Response.body.error_code !== undefined ||
    res_data.Response.body.error_message !== undefined
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
      // Check if the error message is a Json deserialize error
      const apiResponseContent = response.body.error[key];
      const expectedContent = response_data.body.error[key];
      if (
        typeof apiResponseContent === "string" &&
        apiResponseContent.includes("Json deserialize error")
      ) {
        expect(apiResponseContent).to.include(expectedContent);
      } else {
        expect(apiResponseContent).to.equal(expectedContent);
      }
    }
  }
}
