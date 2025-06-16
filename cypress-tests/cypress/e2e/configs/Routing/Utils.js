import { connectorDetails as adyenConnectorDetails } from "./Adyen.js";
import { connectorDetails as autoretryConnectorDetails } from "./Autoretries.js";
import { connectorDetails as commonConnectorDetails } from "./Commons.js";
import { connectorDetails as stripeConnectorDetails } from "./Stripe.js";

const connectorDetails = {
  adyen: adyenConnectorDetails,
  autoretries: autoretryConnectorDetails,
  common: commonConnectorDetails,
  stripe: stripeConnectorDetails,
};

export const getConnectorDetails = (connectorId) => {
  const x = getValueByKey(connectorDetails, connectorId);
  return x;
};

function getValueByKey(jsonObject, key) {
  const data =
    typeof jsonObject === "string" ? JSON.parse(jsonObject) : jsonObject;

  if (data && typeof data === "object" && key in data) {
    return data[key];
  } else {
    return null;
  }
}

export const should_continue_further = (data) => {
  const resData = data.Response || {};

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
