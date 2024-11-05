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
  let x = getValueByKey(connectorDetails, connectorId);
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
