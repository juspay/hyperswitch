import { execConfig, validateConfig } from "../../../utils/featureFlags.js";

import { updateDefaultStatusCode } from "./Modifiers.js";

import { connectorDetails as signifydConnectorDetails } from "../FRM/Signifyd.js";
import { connectorDetails as aciConnectorDetails } from "./Aci.js";
import { connectorDetails as adyenConnectorDetails } from "./Adyen.js";
import { connectorDetails as affirmConnectorDetails } from "./Affirm.js";
import { connectorDetails as airwallexConnectorDetails } from "./Airwallex.js";
import { connectorDetails as archipelConnectorDetails } from "./Archipel.js";
import { connectorDetails as authipayConnectorDetails } from "./Authipay.js";
import { connectorDetails as authorizedotnetConnectorDetails } from "./Authorizedotnet.js";
import { connectorDetails as bamboraConnectorDetails } from "./Bambora.js";
import { connectorDetails as bamboraapacConnectorDetails } from "./Bamboraapac.js";
import { connectorDetails as bankOfAmericaConnectorDetails } from "./BankOfAmerica.js";
import { connectorDetails as barclaycardConnectorDetails } from "./Barclaycard.js";
import { connectorDetails as billwerkConnectorDetails } from "./Billwerk.js";
import { connectorDetails as bitpayConnectorDetails } from "./Bitpay.js";
import { connectorDetails as bluesnapConnectorDetails } from "./Bluesnap.js";
import { connectorDetails as braintreeConnectorDetails } from "./Braintree.js";
import { connectorDetails as calidaConnectorDetails } from "./Calida.js";
import { connectorDetails as cashtocodeConnectorDetails } from "./Cashtocode.js";
import { connectorDetails as celeroConnectorDetails } from "./Celero.js";
import { connectorDetails as checkbookConnectorDetails } from "./Checkbook.js";
import { connectorDetails as checkoutConnectorDetails } from "./Checkout.js";
import { connectorDetails as coingateConnectorDetails } from "./Coingate.js";
import { connectorDetails as commonConnectorDetails } from "./Commons.js";
import { connectorDetails as cryptopayConnectorDetails } from "./Cryptopay.js";
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
import { connectorDetails as gigadatConnectorDetails } from "./Gigadat.js";
import { connectorDetails as globalpayConnectorDetails } from "./Globalpay.js";
import { connectorDetails as globepayConnectorDetails } from "./Globepay.js";
import { connectorDetails as helcimConnectorDetails } from "./Helcim.js";
import { connectorDetails as hipayConnectorDetails } from "./Hipay.js";
import { connectorDetails as iatapayConnectorDetails } from "./Iatapay.js";
import { connectorDetails as inespayConnectorDetails } from "./Inespay.js";
import { connectorDetails as itaubankConnectorDetails } from "./ItauBank.js";
import { connectorDetails as jpmorganConnectorDetails } from "./Jpmorgan.js";
import { connectorDetails as klarnaConnectorDetails } from "./Klarna.js";
import { connectorDetails as loonioConnectorDetails } from "./Loonio.js";
import { connectorDetails as mifinityConnectorDetails } from "./Mifinity.js";
import { connectorDetails as mollieConnectorDetails } from "./Mollie.js";
import { connectorDetails as monerisConnectorDetails } from "./Moneris.js";
import { connectorDetails as multisafepayConnectorDetails } from "./Multisafepay.js";
import { connectorDetails as nexinetsConnectorDetails } from "./Nexinets.js";
import { connectorDetails as nexixpayConnectorDetails } from "./Nexixpay.js";
import { connectorDetails as nmiConnectorDetails } from "./Nmi.js";
import { connectorDetails as noonConnectorDetails } from "./Noon.js";
import { connectorDetails as novalnetConnectorDetails } from "./Novalnet.js";
import { connectorDetails as nuveiConnectorDetails } from "./Nuvei.js";
import { connectorDetails as payboxConnectorDetails } from "./Paybox.js";
import { connectorDetails as payjustnowConnectorDetails } from "./Payjustnow.js";
import { connectorDetails as payjustnowinstoreConnectorDetails } from "./Payjustnowinstore.js";
import { connectorDetails as payloadConnectorDetails } from "./Payload.js";
import { connectorDetails as paypalConnectorDetails } from "./Paypal.js";
import { connectorDetails as paysafeConnectorDetails } from "./Paysafe.js";
import { connectorDetails as paystackConnectorDetails } from "./Paystack.js";
import { connectorDetails as payuConnectorDetails } from "./Payu.js";
import { connectorDetails as peachpaymentsConnectorDetails } from "./Peachpayments.js";
import { connectorDetails as placetopayConnectorDetails } from "./Placetopay.js";
import { connectorDetails as plaidConnectorDetails } from "./Plaid.js";
import { connectorDetails as powertranzConnectorDetails } from "./PowerTranz.js";
import { connectorDetails as prophetpayConnectorDetails } from "./Prophetpay.js";
import { connectorDetails as rapydConnectorDetails } from "./Rapyd.js";
import { connectorDetails as redsysConnectorDetails } from "./Redsys.js";
import { connectorDetails as santanderConnectorDetails } from "./Santander.js";
import { connectorDetails as shift4ConnectorDetails } from "./Shift4.js";
import { connectorDetails as silverflowConnectorDetails } from "./Silverflow.js";
import { connectorDetails as squareConnectorDetails } from "./Square.js";
import { connectorDetails as staxConnectorDetails } from "./Stax.js";
import { connectorDetails as stripeConnectorDetails } from "./Stripe.js";
import { connectorDetails as stripeconnectConnectorDetails } from "./StripeConnect.js";
import { connectorDetails as tesouroConnectorDetails } from "./Tesouro.js";
import { connectorDetails as trustpayConnectorDetails } from "./Trustpay.js";
import { connectorDetails as trustpaymentsConnectorDetails } from "./TrustPayments.js";
import { connectorDetails as tsysConnectorDetails } from "./Tsys.js";
import { connectorDetails as voltConnectorDetails } from "./Volt.js";
import { connectorDetails as wellsfargoConnectorDetails } from "./WellsFargo.js";
import { connectorDetails as worldpayConnectorDetails } from "./WorldPay.js";
import { connectorDetails as worldpayvantivConnectorDetails } from "./Worldpayvantiv.js";
import { connectorDetails as worldpayxmlConnectorDetails } from "./Worldpayxml.js";
import { connectorDetails as xenditConnectorDetails } from "./Xendit.js";
import { connectorDetails as ziftConnectorDetails } from "./Zift.js";
const connectorDetails = {
  aci: aciConnectorDetails,
  adyen: adyenConnectorDetails,
  affirm: affirmConnectorDetails,
  airwallex: airwallexConnectorDetails,
  archipel: archipelConnectorDetails,
  authipay: authipayConnectorDetails,
  authorizedotnet: authorizedotnetConnectorDetails,
  bambora: bamboraConnectorDetails,
  bamboraapac: bamboraapacConnectorDetails,
  barclaycard: barclaycardConnectorDetails,
  bankofamerica: bankOfAmericaConnectorDetails,
  bitpay: bitpayConnectorDetails,
  billwerk: billwerkConnectorDetails,
  bluesnap: bluesnapConnectorDetails,
  braintree: braintreeConnectorDetails,
  calida: calidaConnectorDetails,
  cashtocode: cashtocodeConnectorDetails,
  celero: celeroConnectorDetails,
  checkout: checkoutConnectorDetails,
  checkbook: checkbookConnectorDetails,
  commons: commonConnectorDetails,
  coingate: coingateConnectorDetails,
  cryptopay: cryptopayConnectorDetails,
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
  gigadat: gigadatConnectorDetails,
  globalpay: globalpayConnectorDetails,
  globepay: globepayConnectorDetails,
  helcim: helcimConnectorDetails,
  hipay: hipayConnectorDetails,
  iatapay: iatapayConnectorDetails,
  inespay: inespayConnectorDetails,
  itaubank: itaubankConnectorDetails,
  jpmorgan: jpmorganConnectorDetails,
  klarna: klarnaConnectorDetails,
  mollie: mollieConnectorDetails,
  moneris: monerisConnectorDetails,
  multisafepay: multisafepayConnectorDetails,
  nexinets: nexinetsConnectorDetails,
  nexixpay: nexixpayConnectorDetails,
  nmi: nmiConnectorDetails,
  noon: noonConnectorDetails,
  novalnet: novalnetConnectorDetails,
  nuvei: nuveiConnectorDetails,
  paybox: payboxConnectorDetails,
  payjustnow: payjustnowConnectorDetails,
  payjustnowinstore: payjustnowinstoreConnectorDetails,
  payload: payloadConnectorDetails,
  paypal: paypalConnectorDetails,
  paysafe: paysafeConnectorDetails,
  paystack: paystackConnectorDetails,
  placetopay: placetopayConnectorDetails,
  plaid: plaidConnectorDetails,
  payu: payuConnectorDetails,
  peachpayments: peachpaymentsConnectorDetails,
  powertranz: powertranzConnectorDetails,
  prophetpay: prophetpayConnectorDetails,
  rapyd: rapydConnectorDetails,
  redsys: redsysConnectorDetails,
  santander: santanderConnectorDetails,
  shift4: shift4ConnectorDetails,
  signifyd: signifydConnectorDetails,
  silverflow: silverflowConnectorDetails,
  square: squareConnectorDetails,
  stax: staxConnectorDetails,
  stripe: stripeConnectorDetails,
  stripeconnect: stripeconnectConnectorDetails,
  trustpay: trustpayConnectorDetails,
  tesouro: tesouroConnectorDetails,
  trustpayments: trustpaymentsConnectorDetails,
  tsys: tsysConnectorDetails,
  volt: voltConnectorDetails,
  wellsfargo: wellsfargoConnectorDetails,
  worldpay: worldpayConnectorDetails,
  worldpayvantiv: worldpayvantivConnectorDetails,
  worldpayxml: worldpayxmlConnectorDetails,
  xendit: xenditConnectorDetails,
  zift: ziftConnectorDetails,
  loonio: loonioConnectorDetails,
  mifinity: mifinityConnectorDetails,
};

/**
 * Get the backend connector name for a given connector ID
 * Maps stripeconnect -> stripe for backend API calls
 * @param {string} connectorId - The test connector ID
 * @returns {string} - The backend connector name
 */
export function getOriginalConnectorName(connectorId) {
  return connectorId === "stripeconnect" ? "stripe" : connectorId;
}

export default function getConnectorDetails(connectorId) {
  return mergeDetails(connectorId);
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
      connectorCredentials: keys,
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
  if (!data) {
    return false;
  }
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
  // Use originalConnectorId if available, otherwise fallback to connectorId
  // This ensures correct config is loaded for stripeconnect tests
  const connectorId =
    globalState.get("originalConnectorId") || globalState.get("connectorId");
  const mcaConfig = getConnectorDetails(connectorId);

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

export function createBusinessProfilesAndMerchantConnectorAccounts(
  paymentType,
  createMerchantConnectorAccountBody,
  createBusinessProfileBody,
  globalState,
  paymentMethodsEnabled
) {
  const connectorCount = globalState.get("MULTIPLE_CONNECTORS")?.count || 0;

  if (connectorCount <= 1) {
    cy.task(
      "cli_log",
      "Skipping multiple connector account setup; no multiple connector credentials configured."
    );
    return;
  }

  for (
    let connectorIndex = 2;
    connectorIndex <= connectorCount;
    connectorIndex++
  ) {
    const multipleConnector = {
      nextConnector: true,
      value: `connector_${connectorIndex}`,
    };

    createBusinessProfile(
      structuredClone(createBusinessProfileBody),
      globalState,
      multipleConnector
    );

    createMerchantConnectorAccount(
      paymentType,
      structuredClone(createMerchantConnectorAccountBody),
      globalState,
      paymentMethodsEnabled,
      multipleConnector
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
      "bluesnap",
      "braintree",
      "calida",
      "cashtocode",
      "facilitapay",
      "fiserv",
      "fiuu",
      "forte",
      "globalpay",
      "gigadat",
      "jpmorgan",
      "loonio",
      "mifinity",
      "nexinets",
      "nmi",
      "noon",
      "novalnet",
      "payload",
      "paypal",
      "stax",
      "stripeconnect",
      "wellsfargo",
      "worldpayxml",
      "finix",
      "mollie",
      "zift",
    ],
    MANDATE_ID_TEST: [
      "airwallex",
      "calida",
      "payload",
      "gigadat",
      "loonio",
      "redsys",
      "worldpayxml",
      "mifinity",
    ],
    SAVE_CARD: ["helcim"],
    // Add more exclusion lists
    // Note: mitUsingPMId/mitForMandatesCallTest/listMandateCallTest use
    // per-config TRIGGER_SKIP or globalState checks instead of a static
    // list here.
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
    CONNECTOR_TESTING_DATA: ["adyen", "airwallex", "braintree", "noon"],
    // ucs connectors
    UCS_CONNECTORS: ["authorizedotnet"],
    OVERCAPTURE: ["adyen"],
    IFRAME_REDIRECTION: [
      "adyen",
      "cybersource",
      "barclaycard",
      "paypal",
      "bluesnap",
      "braintree",
      "nmi",
      "nexixpay",
      "deutschebank",
    ],
    MANUAL_RETRY: [
      "cybersource",
      "checkout",
      "stripe",
      "adyen",
      "airwallex",
      "authorizedotnet",
      "bankofamerica",
      "datatrans",
      "finix",
      "fiuu",
      "globalpay",
      "nexinets",
      "nuvei",
      "paypal",
      "powertranz",
      "shift4",
      "trustpay",
      "worldpay",
      "worldpayvantiv",
      "worldpayxml",
    ],
    PAYMENTS_WEBHOOK: [
      "noon",
      "stripe",
      "authorizedotnet",
      "airwallex",
      "finix",
      "fiuu",
      "mollie",
      "nmi",
      "novalnet",
      "payload",
      "paypal",
      "trustpay",
      "worldpay",
    ],
    REFUNDS_WEBHOOK: [
      "airwallex",
      "finix",
      "fiuu",
      "nmi",
      "novalnet",
      "paypal",
      "stripe",
    ],
    BANK_DEBIT: [
      "adyen",
      "inespay",
      "novalnet",
      "payload",
      "stax",
      "stripe",
      "wellsfargo",
    ], // payload verified as working
    BANK_REDIRECT_BANCONTACT: ["adyen", "stripe"],
    BANK_REDIRECT_MANDATE: ["adyen", "stripe"],
    CARD_REDIRECT: ["prophetpay"],
    BLUECODE_WALLET: ["calida"],
    ALIPAY_HK_WALLET: [""],
    PAYPAL_WALLET: [
      "airwallex",
      "globalpay",
      "multisafepay",
      "novalnet",
      "paypal",
    ],
    MIFINITY_WALLET: ["mifinity"],
    ALIPAY_WALLET: ["globepay", "stripe", "multisafepay"],
    WECHATPAY_WALLET: ["globepay", "stripe", "multisafepay"],
    MBWAY_WALLET: ["multisafepay"],
    SKRILL_WALLET: ["paysafe"],
    PAYSAFECARD_GIFT_CARD: ["paysafe"],
    PAYPAL_MANDATE: ["globalpay", "novalnet", "paypal"],
    PAYPAL_WALLET_MANDATE: ["adyen"],
    KAKAO_PAY_WALLET_MANDATE: ["adyen"],
    GCASH_WALLET_MANDATE: ["adyen"],
    TWINT_WALLET_MANDATE: ["adyen"],
    DANA_WALLET_MANDATE: ["adyen"],
    GOPAY_WALLET_MANDATE: ["adyen"],
    CARD_INSTALLMENTS: ["adyen"],
    BILLING_DESCRIPTOR: [
      "adyen",
      "checkout",
      "stripe",
      "nuvei",
      "trustpay",
      "finix",
      "payload",
    ],
    BILLING_DESCRIPTOR_INVALID_PHONE: ["nuvei"],
    FEATURE_METADATA: ["bankofamerica"],
    AUTO_RETRY: [
      "cybersource",
      "checkout",
      "stripe",
      "adyen",
      "airwallex",
      "authorizedotnet",
      "bankofamerica",
      "datatrans",
      "finix",
      "fiuu",
      "globalpay",
      "nexinets",
      "nmi",
      "paypal",
      "powertranz",
      "shift4",
      "trustpay",
      "worldpay",
      "worldpayvantiv",
    ],
    EXTERNAL_THREE_DS: ["stripe", "finix"],
    BOLETO: ["santander"],
    PIX_AUTOMATICO: ["santander"],
    PARTNER_MERCHANT_IDENTIFIER: ["adyen", "checkout"],
    AFFIRM_PAY_LATER: ["affirm"],
    AFTERPAY_CLEARPAY: ["adyen", "stripe"],
    ALMA: ["adyen"],
    WALLEY: ["adyen"],
    EXTEND_AUTHORIZATION: ["adyen", "paypal"],
    GIFT_CARD: ["adyen"],
    VOUCHER: ["adyen", "dlocal"],
    RELAY_OPERATIONS: ["bankofamerica"],
    AMAZONPAY_WALLET: ["stripe"],
    CASHAPP_WALLET: ["stripe"],
    REVOLUTPAY_WALLET: ["stripe"],
    PAY_LATER: [
      "klarna",
      "adyen",
      "aci",
      "stripe",
      "airwallex",
      "mollie",
      "affirm",
      "payjustnow",
      "payjustnowinstore",
    ],
    PAY_LATER_KLARNA_MANDATE: ["adyen"],
    AFFIRM: ["stripe"],
    ATOME: ["adyen"],
    PAYJUSTNOW: ["payjustnow"],
    PAYJUSTNOWINSTORE: ["payjustnowinstore"],
    AUTH_SERVICE_ELIGIBILITY: ["stripe", "cybersource"],
    STEP_UP_AUTH: ["cybersource"],
    PARTIAL_AUTH: ["nuvei", "checkout", "worldpayvantiv"],
    PAYMENT_RESPONSE_HASH: ["stripe"],
    MULTIPLE_CAPTURE: ["adyen", "checkout"],
    USE_BILLING_AS_PAYMENT_METHOD_BILLING: ["bankofamerica"],
    MIT_WITH_LIMITED_CARD_DATA: ["peachpayments"],
    EXTENDED_CARD_INFO: ["stripe"],
    PAYMENT_LINK_CARD: ["stripe"],
    ORDER_DETAILS: [
      "stripe",
      "cybersource",
      "checkout",
      "airwallex",
      "braintree",
      "bankofamerica",
      "paypal",
      "trustpay",
    ],
    CARD_TESTING_GUARD: ["bankofamerica"],
    CLEAR_PAN_RETRY: ["bankofamerica"],
    L2L3DATA: ["checkout", "nuvei", "worldpayvantiv"],
    REFUND_MANUAL_UPDATE: ["bankofamerica", "cybersource"],
    REFUND_TYPE: ["stripe", "adyen", "checkout"],
    MANUAL_PAYMENT_UPDATE: ["stripe"],
    CRYPTO_PAYMENT: ["bitpay", "coingate", "cryptopay"],
    STEP_UP_RETRY: [
      "cybersource",
      "checkout",
      "stripe",
      "adyen",
      "airwallex",
      "authorizedotnet",
      "bankofamerica",
      "datatrans",
      "fiuu",
      "globalpay",
      "nexinets",
      "nmi",
      "nuvei",
      "paypal",
      "powertranz",
      "shift4",
      "trustpay",
      "worldpay",
      "worldpayvantiv",
    ],
    POLL_CONFIG: ["stripe"],
    FRM: ["stripe"],
    PAYOUT_PRIORITY: ["adyenplatform"],
    DELAYED_SESSION_TOKEN: ["trustpay", "payme"],
    OPEN_BANKING_PIS: ["plaid"],
    CLIENT_SESSION_VALIDATION: ["stripe"],
    WEBHOOK_CONFIG: ["stripe"],
    REQUIRES_CVV: ["bankofamerica"],
    // Add more inclusion lists
  },
};

// Helper functions
export const shouldExcludeConnector = (connectorId, list) => {
  return Array.isArray(list) && list.includes(connectorId);
};

export const shouldIncludeConnector = (connectorId, list) => {
  if (!Array.isArray(list)) return true;
  return !list.includes(connectorId);
};

export function setNormalizedValue(
  webhookBody,
  config,
  connectorTransactionID
) {
  if (!config?.path) {
    throw new Error("Invalid config: missing path");
  }
  // Split the dot-separated path into individual keys
  const keys = config.path.split(".");
  let target = webhookBody;

  // Traverse the object until the parent of the final key
  for (const key of keys.slice(0, -1)) {
    if (!Object.prototype.hasOwnProperty.call(target, key)) {
      throw new Error(`Path does not exist: ${config.path}`);
    }
    target = target[key];
  }
  // The final key where the normalized value will be assigned
  const finalKey = keys[keys.length - 1];

  // Coerce value based on expected type
  const normalizedconnectorTransactionID = coerceValue(
    connectorTransactionID,
    config.type
  );

  target[finalKey] = normalizedconnectorTransactionID;
}

function coerceValue(value, type) {
  switch (type) {
    case "string":
      return String(value);

    case "number": {
      const num = Number(value);
      if (!Number.isFinite(num)) {
        throw new Error(`Cannot coerce "${value}" to number`);
      }
      return num;
    }

    default:
      return value;
  }
}

export function stampPaymentMethodType(scenarios, paymentMethodType) {
  const cloned = JSON.parse(JSON.stringify(scenarios));
  for (const scenario of Object.values(cloned)) {
    if (scenario.Request && typeof scenario.Request === "object") {
      scenario.Request.payment_method_type = paymentMethodType;
    }
  }
  return cloned;
}

// Rotate cards to avoid Helcim's duplicate-decline window.
// Helcim has strict idempotency rules which include identifying card number,
// cardholder name, etc. If they are found in any retry within 5 minutes of a
// previous transaction it will be marked as a duplicate. Therefore we rotate
// the cards and try to deny retries.
const helcimTestCards = [
  "4111111111111111",
  "4000000000000002",
  "4242424242424242",
  "4012888888881881",
  "4000056655665556",
  "4532015112830366",
  "4000000000000127",
  "4000000000000119",
  "4111111111111129",
  "4111111111111137",
  "4111111111111145",
  "4111111111111152",
  "4000000000000259",
  "4000000000003238",
  "5555555555554444",
  "5105105105105100",
  "5200828282828210",
  "5100000000000008",
  "4111111111111160",
  "4000000000000340",
];

export function injectHelcimTestCard(body, globalState) {
  if (globalState.get("connectorId") !== "helcim") return;
  if (!body.payment_method_data?.card) return;

  const testOffset = globalState.get("helcimCardIndex") ?? 0;
  const timeOffset = Math.floor(Date.now() / 1000) % helcimTestCards.length;
  const idx = (timeOffset + testOffset) % helcimTestCards.length;
  globalState.set("helcimCardIndex", testOffset + 1);

  const ts = Date.now();
  const rnd = Math.floor(Math.random() * 100000);
  const uniqueSuffix = `${ts.toString(36)}_${rnd}`;
  body.payment_method_data.card.card_number = helcimTestCards[idx];
  body.payment_method_data.card.card_holder_name = `HelcimTest ${uniqueSuffix}`;
}
