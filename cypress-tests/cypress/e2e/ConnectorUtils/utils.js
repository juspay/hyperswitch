import { connectorDetails as adyenConnectorDetails } from "./Adyen.js";
import { connectorDetails as bankOfAmericaConnectorDetails } from "./BankOfAmerica.js";
import { connectorDetails as bluesnapConnectorDetails } from "./Bluesnap.js";
import { connectorDetails as cybersourceConnectorDetails } from "./Cybersource.js";
import { connectorDetails as nmiConnectorDetails } from "./Nmi.js";
import { connectorDetails as paypalConnectorDetails } from "./Paypal.js";
import { connectorDetails as stripeConnectorDetails } from "./Stripe.js";
import { connectorDetails as trustpayConnectorDetails } from "./Trustpay.js";

import globalState from "../../utils/State.js";

const connectorDetails = {
  "adyen": adyenConnectorDetails,
  "bankofamerica": bankOfAmericaConnectorDetails,
  "bluesnap": bluesnapConnectorDetails,
  "cybersource": cybersourceConnectorDetails,
  "nmi": nmiConnectorDetails,
  "paypal": paypalConnectorDetails,
  "stripe": stripeConnectorDetails,
  "trustpay": trustpayConnectorDetails


}


export default function getConnectorDetails(connectorId) {
  let x = getValueByKey(connectorDetails, connectorId);
  return x;
}

function getValueByKey(jsonObject, key) {
  const data = typeof jsonObject === 'string' ? JSON.parse(jsonObject) : jsonObject;

  if (data && typeof data === 'object' && key in data) {
    return data[key];
  } else {
    return null; 
  }
}