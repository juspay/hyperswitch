import { validateConfig } from "../../../utils/featureFlags.js";

import { connectorDetails as adyenConnectorDetails } from "./Adyen.js";
import { connectorDetails as adyenPlatformConnectorDetails } from "./AdyenPlatform.js";
import { connectorDetails as CommonConnectorDetails } from "./Commons.js";
import { connectorDetails as wiseConnectorDetails } from "./Wise.js";
import { connectorDetails as nomupayConnectorDetails } from "./Nomupay.js";

const connectorDetails = {
  adyen: adyenConnectorDetails,
  adyenplatform: adyenPlatformConnectorDetails,
  commons: CommonConnectorDetails,
  wise: wiseConnectorDetails,
  nomupay: nomupayConnectorDetails,
};

export function getConnectorDetails(connectorId) {
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
    // Connector object has multiple keys
    if (typeof data[key].connector_account_details === "undefined") {
      const keys = Object.keys(data[key]);

      for (let i = 0; i < keys.length; i++) {
        const currentItem = data[key][keys[i]];

        if (
          Object.prototype.hasOwnProperty.call(
            currentItem,
            "connector_account_details"
          )
        ) {
          Cypress.env("MULTIPLE_CONNECTORS", {
            status: true,
            count: keys.length,
          });

          return currentItem;
        }
      }
    }

    return data[key];
  } else {
    return null;
  }
}

export const should_continue_further = (data) => {
  const resData = data.Response || {};
  const configData = validateConfig(data.Configs) || {};

  if (typeof configData?.TRIGGER_SKIP !== "undefined") {
    return !configData.TRIGGER_SKIP;
  }

  if (
    typeof resData.body.error !== "undefined" ||
    typeof resData.body.error_code !== "undefined" ||
    typeof resData.body.error_message !== "undefined"
  ) {
    return false;
  } else {
    return true;
  }
};
