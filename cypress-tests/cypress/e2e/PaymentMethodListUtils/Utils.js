import { connectorDetails as CommonConnectorDetails } from "./Commons.js";
import { connectorDetails as ConnectorDetails } from "./Connector.js";

const connectorDetails = {
  commons: CommonConnectorDetails,
  connector: ConnectorDetails,
};

export default function getConnectorDetails(connectorId) {
  const x = mergeDetails(connectorId);
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

export const should_continue_further = (resData) => {
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
