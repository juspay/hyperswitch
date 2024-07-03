import { connectorDetails as CommonConnectorDetails } from "./Common.js";
import { connectorDetails as cybersourceConnectorDetails } from "./Cybersource.js";
import { connectorDetails as stripeConnectorDetails } from "./Stripe.js";

const connectorDetails = {
  commons: CommonConnectorDetails,
  cybersource: cybersourceConnectorDetails,
  stripe: stripeConnectorDetails,
};

export default function getConnectorDetails(connectorId) {
  let x = mergeDetails(connectorId);
  return x;
}

function mergeDetails(connectorId) {
  const connectorData = getValueByKey(connectorDetails, connectorId);

  return connectorData;
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
