import { execConfig, validateConfig } from "../../../utils/featureFlags.js";

import { updateDefaultStatusCode } from "./Modifiers.js";

import { connectorDetails as aciConnectorDetails } from "./Aci.js";
import { connectorDetails as adyenConnectorDetails } from "./Adyen.js";
import { connectorDetails as airwallexConnectorDetails } from "./Airwallex.js";
import { connectorDetails as archipelConnectorDetails } from "./Archipel.js";
import { connectorDetails as authipayConnectorDetails } from "./Authipay.js";
import { connectorDetails as authorizedotnetConnectorDetails } from "./Authorizedotnet.js";
import { connectorDetails as bamboraConnectorDetails } from "./Bambora.js";
import { connectorDetails as bamboraapacConnectorDetails } from "./Bamboraapac.js";
import { connectorDetails as bankOfAmericaConnectorDetails } from "./BankOfAmerica.js";
import { connectorDetails as barclaycardConnectorDetails } from "./Barclaycard.js";
import { connectorDetails as billwerkConnectorDetails } from "./Billwerk.js";
import { connectorDetails as bluesnapConnectorDetails } from "./Bluesnap.js";
import { connectorDetails as braintreeConnectorDetails } from "./Braintree.js";
import { connectorDetails as celeroConnectorDetails } from "./Celero.js";
import { connectorDetails as checkbookConnectorDetails } from "./Checkbook.js";
import { connectorDetails as checkoutConnectorDetails } from "./Checkout.js";
import { connectorDetails as commonConnectorDetails } from "./Commons.js";
import { connectorDetails as cybersourceConnectorDetails } from "./Cybersource.js";
import { connectorDetails as datatransConnectorDetails } from "./Datatrans.js";
import { connectorDetails as deutschebankConnectorDetails } from "./Deutschebank.js";
import { connectorDetails as dlocalConnectorDetails } from "./Dlocal.js";
import { connectorDetails as elavonConnectorDetails } from "./Elavon.js";
import { connectorDetails as facilitapayConnectorDetails } from "./Facilitapay.js";
import { connectorDetails as finixConnectorDetails } from "./Finix.js";
import { connectorDetails as fiservConnectorDetails } from "./Fiserv.js";
import { connectorDetails as fiservemeaConnectorDetails } from "./Fiservemea.js";
import { connectorDetails as fiuuConnectorDetails } from "./Fiuu.js";
import { connectorDetails as forteConnectorDetails } from "./Forte.js";
import { connectorDetails as getnetConnectorDetails } from "./Getnet.js";
import { connectorDetails as globalpayConnectorDetails } from "./Globalpay.js";
import { connectorDetails as hipayConnectorDetails } from "./Hipay.js";
import { connectorDetails as iatapayConnectorDetails } from "./Iatapay.js";
import { connectorDetails as itaubankConnectorDetails } from "./ItauBank.js";
import { connectorDetails as jpmorganConnectorDetails } from "./Jpmorgan.js";
import { connectorDetails as monerisConnectorDetails } from "./Moneris.js";
import { connectorDetails as multisafepayConnectorDetails } from "./Multisafepay.js";
import { connectorDetails as nexinetsConnectorDetails } from "./Nexinets.js";
import { connectorDetails as nexixpayConnectorDetails } from "./Nexixpay.js";
import { connectorDetails as nmiConnectorDetails } from "./Nmi.js";
import { connectorDetails as noonConnectorDetails } from "./Noon.js";
import { connectorDetails as novalnetConnectorDetails } from "./Novalnet.js";
import { connectorDetails as nuveiConnectorDetails } from "./Nuvei.js";
import { connectorDetails as payboxConnectorDetails } from "./Paybox.js";
import { connectorDetails as payloadConnectorDetails } from "./Payload.js";
import { connectorDetails as paypalConnectorDetails } from "./Paypal.js";
import { connectorDetails as paysafeConnectorDetails } from "./Paysafe.js";
import { connectorDetails as payuConnectorDetails } from "./Payu.js";
import { connectorDetails as peachpaymentsConnectorDetails } from "./Peachpayments.js";
import { connectorDetails as powertranzConnectorDetails } from "./PowerTranz.js";
import { connectorDetails as redsysConnectorDetails } from "./Redsys.js";
import { connectorDetails as shift4ConnectorDetails } from "./Shift4.js";
import { connectorDetails as silverflowConnectorDetails } from "./Silverflow.js";
import { connectorDetails as squareConnectorDetails } from "./Square.js";
import { connectorDetails as staxConnectorDetails } from "./Stax.js";
import { connectorDetails as stripeConnectorDetails } from "./Stripe.js";
import { connectorDetails as tesouroConnectorDetails } from "./Tesouro.js";
import { connectorDetails as trustpayConnectorDetails } from "./Trustpay.js";
import { connectorDetails as trustpaymentsConnectorDetails } from "./TrustPayments.js";
import { connectorDetails as tsysConnectorDetails } from "./Tsys.js";
import { connectorDetails as wellsfargoConnectorDetails } from "./WellsFargo.js";
import { connectorDetails as worldpayConnectorDetails } from "./WorldPay.js";
import { connectorDetails as worldpayvantivConnectorDetails } from "./Worldpayvantiv.js";
import { connectorDetails as worldpayxmlConnectorDetails } from "./Worldpayxml.js";
import { connectorDetails as xenditConnectorDetails } from "./Xendit.js";
const connectorDetails = {
  aci: aciConnectorDetails,
  adyen: adyenConnectorDetails,
  airwallex: airwallexConnectorDetails,
  archipel: archipelConnectorDetails,
  authipay: authipayConnectorDetails,
  authorizedotnet: authorizedotnetConnectorDetails,
  bambora: bamboraConnectorDetails,
  bamboraapac: bamboraapacConnectorDetails,
  barclaycard: barclaycardConnectorDetails,
  bankofamerica: bankOfAmericaConnectorDetails,
  billwerk: billwerkConnectorDetails,
  bluesnap: bluesnapConnectorDetails,
  braintree: braintreeConnectorDetails,
  celero: celeroConnectorDetails,
  checkout: checkoutConnectorDetails,
  checkbook: checkbookConnectorDetails,
  commons: commonConnectorDetails,
  cybersource: cybersourceConnectorDetails,
  dlocal: dlocalConnectorDetails,
  datatrans: datatransConnectorDetails,
  deutschebank: deutschebankConnectorDetails,
  elavon: elavonConnectorDetails,
  facilitapay: facilitapayConnectorDetails,
  fiserv: fiservConnectorDetails,
  fiservemea: fiservemeaConnectorDetails,
  fiuu: fiuuConnectorDetails,
  finix: finixConnectorDetails,
  forte: forteConnectorDetails,
  getnet: getnetConnectorDetails,
  globalpay: globalpayConnectorDetails,
  hipay: hipayConnectorDetails,
  iatapay: iatapayConnectorDetails,
  itaubank: itaubankConnectorDetails,
  jpmorgan: jpmorganConnectorDetails,
  moneris: monerisConnectorDetails,
  multisafepay: multisafepayConnectorDetails,
  nexinets: nexinetsConnectorDetails,
  nexixpay: nexixpayConnectorDetails,
  nmi: nmiConnectorDetails,
  noon: noonConnectorDetails,
  novalnet: novalnetConnectorDetails,
  nuvei: nuveiConnectorDetails,
  paybox: payboxConnectorDetails,
  payload: payloadConnectorDetails,
  paypal: paypalConnectorDetails,
  paysafe: paysafeConnectorDetails,
  payu: payuConnectorDetails,
  peachpayments: peachpaymentsConnectorDetails,
  powertranz: powertranzConnectorDetails,
  redsys: redsysConnectorDetails,
  shift4: shift4ConnectorDetails,
  silverflow: silverflowConnectorDetails,
  square: squareConnectorDetails,
  stax: staxConnectorDetails,
  stripe: stripeConnectorDetails,
  trustpay: trustpayConnectorDetails,
  tesouro: tesouroConnectorDetails,
  trustpayments: trustpaymentsConnectorDetails,
  tsys: tsysConnectorDetails,
  wellsfargo: wellsfargoConnectorDetails,
  worldpay: worldpayConnectorDetails,
  worldpayvantiv: worldpayvantivConnectorDetails,
  worldpayxml: worldpayxmlConnectorDetails,
  xendit: xenditConnectorDetails,
};

export default function getConnectorDetails(connectorId) {
  const x = mergeDetails(connectorId);
  return x;
}

export function getConnectorFlowDetails(connectorData, commonData, key) {
  const data =
    connectorData[key] === undefined ? commonData[key] : connectorData[key];
  return data;
}

function mergeDetails(connectorId) {
  const connectorData = getValueByKey(
    connectorDetails,
    connectorId
  ).authDetails;
  const fallbackData = getValueByKey(connectorDetails, "commons").authDetails;
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

export function handleMultipleConnectors(keys) {
  return {
    MULTIPLE_CONNECTORS: {
      status: true,
      count: keys.length,
    },
  };
}

export function getValueByKey(jsonObject, key, keyNumber = 0) {
  const data =
    typeof jsonObject === "string" ? JSON.parse(jsonObject) : jsonObject;

  if (data && typeof data === "object" && key in data) {
    // Connector object has multiple keys
    if (typeof data[key].connector_account_details === "undefined") {
      const keys = Object.keys(data[key]);

      for (let i = keyNumber; i < keys.length; i++) {
        const currentItem = data[key][keys[i]];

        if (
          Object.prototype.hasOwnProperty.call(
            currentItem,
            "connector_account_details"
          )
        ) {
          // Return state update instead of setting directly
          return {
            authDetails: currentItem,
            stateUpdate: handleMultipleConnectors(keys),
          };
        }
      }
    }
    return {
      authDetails: data[key],
      stateUpdate: null,
    };
  }
  return {
    authDetails: null,
    stateUpdate: null,
  };
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

export function extractIntegerAtEnd(str) {
  // Match one or more digits at the end of the string
  const match = str.match(/(\d+)$/);
  return match ? parseInt(match[0], 10) : 0;
}

// Common helper function to check if operation should proceed
function shouldProceedWithOperation(multipleConnector, multipleConnectors) {
  return !(
    multipleConnector?.nextConnector === true &&
    (multipleConnectors?.status === false ||
      typeof multipleConnectors === "undefined")
  );
}

// Helper to get connector configuration
function getConnectorConfig(
  globalState,
  multipleConnector = { nextConnector: false }
) {
  const multipleConnectors = globalState.get("MULTIPLE_CONNECTORS");
  const mcaConfig = getConnectorDetails(globalState.get("connectorId"));

  return {
    CONNECTOR_CREDENTIAL:
      multipleConnector?.nextConnector && multipleConnectors?.status
        ? multipleConnector
        : mcaConfig?.multi_credential_config || multipleConnector,
    multipleConnectors,
  };
}

// Simplified createBusinessProfile
export function createBusinessProfile(
  createBusinessProfileBody,
  globalState,
  multipleConnector = { nextConnector: false }
) {
  const config = getConnectorConfig(globalState, multipleConnector);
  const { profilePrefix } = execConfig(config);

  if (
    shouldProceedWithOperation(multipleConnector, config.multipleConnectors)
  ) {
    cy.createBusinessProfileTest(
      createBusinessProfileBody,
      globalState,
      profilePrefix
    );
  }
}

// Simplified createMerchantConnectorAccount
export function createMerchantConnectorAccount(
  paymentType,
  createMerchantConnectorAccountBody,
  globalState,
  paymentMethodsEnabled,
  multipleConnector = { nextConnector: false }
) {
  const config = getConnectorConfig(globalState, multipleConnector);
  const { profilePrefix, merchantConnectorPrefix } = execConfig(config);

  if (
    shouldProceedWithOperation(multipleConnector, config.multipleConnectors)
  ) {
    cy.createConnectorCallTest(
      paymentType,
      createMerchantConnectorAccountBody,
      paymentMethodsEnabled,
      globalState,
      profilePrefix,
      merchantConnectorPrefix
    );
  }
}

export function updateBusinessProfile(
  updateBusinessProfileBody,
  is_connector_agnostic_enabled,
  collect_billing_address_from_wallet_connector,
  collect_shipping_address_from_wallet_connector,
  always_collect_billing_address_from_wallet_connector,
  always_collect_shipping_address_from_wallet_connector,
  globalState
) {
  const multipleConnectors = globalState.get("MULTIPLE_CONNECTORS");
  cy.log(`MULTIPLE_CONNECTORS: ${JSON.stringify(multipleConnectors)}`);

  // Get MCA config
  const mcaConfig = getConnectorDetails(globalState.get("connectorId"));
  const { profilePrefix } = execConfig({
    CONNECTOR_CREDENTIAL: mcaConfig?.multi_credential_config,
  });

  cy.UpdateBusinessProfileTest(
    updateBusinessProfileBody,
    is_connector_agnostic_enabled,
    collect_billing_address_from_wallet_connector,
    collect_shipping_address_from_wallet_connector,
    always_collect_billing_address_from_wallet_connector,
    always_collect_shipping_address_from_wallet_connector,
    globalState,
    profilePrefix
  );
}

export const CONNECTOR_LISTS = {
  // Exclusion lists (skip these connectors)
  EXCLUDE: {
    CONNECTOR_AGNOSTIC_NTID: [
      "bamboraapac",
      "bankofamerica",
      "billwerk",
      "braintree",
      "facilitapay",
      "fiserv",
      "fiuu",
      "forte",
      "globalpay",
      "jpmorgan",
      "nexinets",
      "novalnet",
      "payload",
      "paypal",
      "stax",
      "wellsfargo",
      "worldpayxml",
      "finix",
    ],
    // Add more exclusion lists
  },

  // Inclusion lists (only run for these connectors)
  INCLUDE: {
    MANDATES_USING_NTID_PROXY: ["cybersource", "checkout"],
    INCREMENTAL_AUTH: [
      "archipel",
      // "cybersource",    // issues with MULTIPLE_CONNECTORS handling
      "paypal",
      // "stripe",
    ],
    DDC_RACE_CONDITION: ["worldpay"],
    // ucs connectors
    UCS_CONNECTORS: ["authorizedotnet"],
    OVERCAPTURE: ["adyen"],
    MANUAL_RETRY: ["cybersource"],
    // Add more inclusion lists
  },
};

// Helper functions
export const shouldExcludeConnector = (connectorId, list) => {
  return list.includes(connectorId);
};

export const shouldIncludeConnector = (connectorId, list) => {
  return !list.includes(connectorId);
};
