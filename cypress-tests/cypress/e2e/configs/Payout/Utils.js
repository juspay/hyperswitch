import { validateConfig } from "../../../utils/featureFlags.js";

import { connectorDetails as adyenConnectorDetails } from "./Adyen.js";
import { connectorDetails as adyenPlatformConnectorDetails } from "./AdyenPlatform.js";
import { connectorDetails as CommonConnectorDetails } from "./Commons.js";
import { connectorDetails as gotymeSanlamConnectorDetails } from "./GotymeSanlam.js";
import { connectorDetails as wiseConnectorDetails } from "./Wise.js";
import { connectorDetails as nomupayConnectorDetails } from "./Nomupay.js";
import { connectorDetails as truelayerConnectorDetails } from "./Truelayer.js";

const connectorDetails = {
  adyen: adyenConnectorDetails,
  adyenplatform: adyenPlatformConnectorDetails,
  commons: CommonConnectorDetails,
  gotyme_sanlam: gotymeSanlamConnectorDetails,
  nomupay: nomupayConnectorDetails,
  truelayer: truelayerConnectorDetails,
  wise: wiseConnectorDetails,
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

// Connector inclusion/exclusion lists for feature gates
export const CONNECTOR_LISTS = {
  // Exclusion lists (skip these connectors)
  EXCLUDE: {
    // gotyme_sanlam only supports bank transfer payouts (payshap /
    // payshap_proxy) and has no card payout method, so it is skipped for
    // the card payout tests in 00003-CardTest.cy.js
    CARD_TEST: ["gotyme_sanlam"],
  },
  INCLUDE: {
    ENTITY_TYPE: ["wise"],
    // Payout recurring feature - only verified connectors
    PAYOUT_RECURRING: ["adyenplatform"],
    PAYOUT_LINK: ["wise"],
    BANK_TRANSFER_OPEN_BANKING: ["truelayer"],
    BANK_TRANSFER_OPEN_BANKING_INVALID_REFERENCE_FULFILL: [],
    BANK_TRANSFER_PAYSHAP: ["gotyme_sanlam"],
    BANK_TRANSFER_PAYSHAP_PROXY: ["gotyme_sanlam"],
    BANK_TRANSFER_SEPA: ["adyen", "adyenplatform", "nomupay", "wise"],
    SAVED_CARD: ["adyen", "adyenplatform", "nomupay", "wise"],
    SAVED_BANK_TRANSFER_SEPA: ["adyen", "adyenplatform", "nomupay", "wise"],
  },
};

// Helper functions
export const shouldExcludeConnector = (connectorId, list) => {
  return Array.isArray(list) && list.includes(connectorId);
};

export const ENTITY_TYPE_LIST = [
  { key: "EntityTypeIndividual", name: "Individual" },
  { key: "EntityTypeCompany", name: "Company" },
  { key: "EntityTypeNonProfit", name: "NonProfit" },
  { key: "EntityTypePublicSector", name: "PublicSector" },
  { key: "EntityTypeNaturalPerson", name: "NaturalPerson" },
  { key: "EntityTypePersonal", name: "Personal" },
];

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

/*
 * Injects sensitive payout bank transfer details (bank_account_number,
 * account_holder_name, bank_name, shap_id) from the gitignored `creds.json`
 * (`<connector>_payout` -> `payout_bank_transfer`, stashed into globalState by
 * `createPayoutConnectorCallTest`) into the bank_transfer payout_method_data
 * of a payout create/confirm request body. Connector configs only declare the
 * payout_method_type; the sensitive values never live in committed code.
 * `creds.json` may group multiple account variants under one payout method
 * type (e.g. `payshap` -> `intrabank` / `interbank`), but the payout API
 * accepts only the flat bank fields, so a variant group is flattened to a
 * single variant (interbank preferred: full field set) before injection.
 * No-op for every connector other than `gotyme_sanlam`.
 */
export const injectGotymePayoutBankTransfer = (body, globalState) => {
  if (globalState.get("connectorId") !== "gotyme_sanlam") {
    return body;
  }

  const payoutBankTransferDetails = globalState.get(
    "payoutBankTransferDetails"
  );
  const bankTransferData = body?.payout_method_data?.bank_transfer;

  if (!payoutBankTransferDetails || !bankTransferData) {
    return body;
  }

  let credsForType =
    payoutBankTransferDetails[bankTransferData.payout_method_type];

  // A variant group (e.g. payshap -> {intrabank: {...}, interbank: {...}})
  // holds only object values, unlike flat creds (e.g. {shap_id: "..."}).
  // Select one variant so its fields are spread flat into the request below.
  const isVariantGroup =
    credsForType &&
    Object.keys(credsForType).length > 0 &&
    Object.values(credsForType).every(
      (variant) => variant && typeof variant === "object"
    );

  if (isVariantGroup) {
    credsForType = credsForType.interbank || credsForType.intrabank;
  }

  if (credsForType) {
    body.payout_method_data.bank_transfer = {
      ...bankTransferData,
      ...credsForType,
    };
  }

  return body;
};
