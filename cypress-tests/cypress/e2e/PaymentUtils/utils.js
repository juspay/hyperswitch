import { connectorDetails as adyenConnectorDetails } from "./Adyen.js";
import { connectorDetails as bankOfAmericaConnectorDetails } from "./BankOfAmerica.js";
import { connectorDetails as bluesnapConnectorDetails } from "./Bluesnap.js";
import { connectorDetails as CommonConnectorDetails } from "./Commons.js";
import { connectorDetails as cybersourceConnectorDetails } from "./Cybersource.js";
import { connectorDetails as nmiConnectorDetails } from "./Nmi.js";
import { connectorDetails as paypalConnectorDetails } from "./Paypal.js";
import { connectorDetails as stripeConnectorDetails } from "./Stripe.js";
import { connectorDetails as trustpayConnectorDetails } from "./Trustpay.js";

const connectorDetails = {
  adyen: adyenConnectorDetails,
  bankofamerica: bankOfAmericaConnectorDetails,
  bluesnap: bluesnapConnectorDetails,
  commons: CommonConnectorDetails,
  cybersource: cybersourceConnectorDetails,
  nmi: nmiConnectorDetails,
  paypal: paypalConnectorDetails,
  stripe: stripeConnectorDetails,
  trustpay: trustpayConnectorDetails,
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

function getValueByKey(jsonObject, key) {
  const data =
    typeof jsonObject === "string" ? JSON.parse(jsonObject) : jsonObject;

  if (data && typeof data === "object" && key in data) {
    return data[key];
  } else {
    return null;
  }
}

export const should_continue_further = (res_data) => {
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
