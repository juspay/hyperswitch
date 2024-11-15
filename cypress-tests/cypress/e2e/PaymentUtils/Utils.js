import { connectorDetails as adyenConnectorDetails } from "./Adyen.js";
import { connectorDetails as bankOfAmericaConnectorDetails } from "./BankOfAmerica.js";
import { connectorDetails as bluesnapConnectorDetails } from "./Bluesnap.js";
import {
  connectorDetails as CommonConnectorDetails,
  updateDefaultStatusCode,
} from "./Commons.js";
import { connectorDetails as cybersourceConnectorDetails } from "./Cybersource.js";
import { connectorDetails as datatransConnectorDetails } from "./Datatrans.js";
import { connectorDetails as fiservemeaConnectorDetails } from "./Fiservemea.js";
import { connectorDetails as iatapayConnectorDetails } from "./Iatapay.js";
import { connectorDetails as itaubankConnectorDetails } from "./ItauBank.js";
import { connectorDetails as nexixpayConnectorDetails } from "./Nexixpay.js";
import { connectorDetails as nmiConnectorDetails } from "./Nmi.js";
import { connectorDetails as noonConnectorDetails } from "./Noon.js";
import { connectorDetails as novalnetConnectorDetails } from "./Novalnet.js";
import { connectorDetails as payboxConnectorDetails } from "./Paybox.js";
import { connectorDetails as paypalConnectorDetails } from "./Paypal.js";
import { connectorDetails as stripeConnectorDetails } from "./Stripe.js";
import { connectorDetails as trustpayConnectorDetails } from "./Trustpay.js";
import { connectorDetails as wellsfargoConnectorDetails } from "./WellsFargo.js";
import { connectorDetails as fiuuConnectorDetails } from "./Fiuu.js";
import { connectorDetails as worldpayConnectorDetails } from "./WorldPay.js";
import { connectorDetails as checkoutConnectorDetails } from "./Checkout.js";
import { connectorDetails as elavonConnectorDetails } from "./Elavon.js";

const connectorDetails = {
  adyen: adyenConnectorDetails,
  bankofamerica: bankOfAmericaConnectorDetails,
  bluesnap: bluesnapConnectorDetails,
  checkout: checkoutConnectorDetails,
  commons: CommonConnectorDetails,
  cybersource: cybersourceConnectorDetails,
  fiservemea: fiservemeaConnectorDetails,
  iatapay: iatapayConnectorDetails,
  itaubank: itaubankConnectorDetails,
  nexixpay: nexixpayConnectorDetails,
  nmi: nmiConnectorDetails,
  novalnet: novalnetConnectorDetails,
  paybox: payboxConnectorDetails,
  paypal: paypalConnectorDetails,
  stripe: stripeConnectorDetails,
  elavon: elavonConnectorDetails,
  trustpay: trustpayConnectorDetails,
  datatrans: datatransConnectorDetails,
  wellsfargo: wellsfargoConnectorDetails,
  fiuu: fiuuConnectorDetails,
  worldpay: worldpayConnectorDetails,
  noon: noonConnectorDetails,
};

export default function getConnectorDetails(connectorId) {
  let x = mergeDetails(connectorId);
  return x;
}

export function getConnectorFlowDetails(connectorData, commonData, key) {
  let data = connectorData[key] === undefined ? commonData[key] : connectorData[key];
  return data;
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

export function getValueByKey(jsonObject, key, increment = false) {
  const data =
    typeof jsonObject === "string" ? JSON.parse(jsonObject) : jsonObject;

  if (data && typeof data === "object" && key in data) {
    // Connector object has multiple keys
    if (typeof data[key].connector_account_details === "undefined") {
      const keys = Object.keys(data[key]);

      for (let i = 0; i < keys.length; i++) {
        const currentItem = data[key][keys[i]];

        if (currentItem.hasOwnProperty("connector_account_details")) {
          if (increment) {
            i++;
          }

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

export const should_continue_further = (res_data, config_data) => {
  if (config_data?.trigger_skip !== undefined) {
    return !config_data.trigger_skip;
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
      // Check if the error message is a Json deserialize error
      let apiResponseContent = response.body.error[key];
      let expectedContent = response_data.body.error[key];
      if (typeof apiResponseContent === "string" && apiResponseContent.includes("Json deserialize error")) {
        expect(apiResponseContent).to.include(expectedContent);
      } else {
        expect(apiResponseContent).to.equal(expectedContent);
      }
    }
  }
}
