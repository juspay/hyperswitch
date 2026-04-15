import businessProfile from "./business-profile.json";
import captureBody from "./capture-flow-body.json";
import configs from "./configs.json";
import confirmBody from "./confirm-body.json";
import apiKeyCreateBody from "./create-api-key-body.json";
import rawCreateConfirmPaymentBody from "./create-confirm-body.json";
import createConnectorBody from "./create-connector-body.json";
import customerCreateBody from "./create-customer-body.json";
import rawCitConfirmBody from "./create-mandate-cit.json";
import rawMitConfirmBody from "./create-mandate-mit.json";
import rawCreatePaymentBody from "./create-payment-body.json";
import createPayoutBody from "./create-payout-confirm-body.json";
import rawPmIdConfirmBody from "./create-pm-id-mit.json";
import gsmBody from "./gsm-body.json";
import listRefundCall from "./list-refund-call-body.json";
import merchantCreateBody from "./merchant-create-body.json";
import merchantUpdateBody from "./merchant-update-body.json";
import refundBody from "./refund-flow-body.json";
import routingConfigBody from "./routing-config-body.json";
import saveCardConfirmBody from "./save-card-confirm-body.json";
import sessionTokenBody from "./session-token.json";
import apiKeyUpdateBody from "./update-api-key-body.json";
import updateConnectorBody from "./update-connector-body.json";
import customerUpdateBody from "./update-customer-body.json";
import voidBody from "./void-payment-body.json";
import rawNtidConfirmBody from "./create-ntid-mit.json";
import blocklistCreateBody from "./blocklist-create-body.json";
import eligibilityCheckBody from "./eligibility-check-body.json";
import * as IncomingWebhookBody from "./webhooks/import";
import customerCreate from "./modularPmService/modularPmServiceCustomerCreate.json";
import paymentMethodCreate from "./modularPmService/modular-pm-service-pm-create.json";
import paymentMethodUpdate from "./modularPmService/modular-pm-service-pm-update.json";
import paymentMethodSessionCreate from "./modularPmService/modular-pm-service-pms-create.json";
import paymentMethodSessionUpdate from "./modularPmService/modular-pm-service-update-pms-saved-pm.json";
import paymentMethodSessionConfirm from "./modularPmService/modular-pm-service-pms-confim.json";
import rawModularPmServicePaymentsCall from "./modularPmService/modular-pm-service-payments-call.json";

// Read default currency once from Cypress env. Falls back to USD when unset
// or when loaded outside a Cypress runtime (e.g. static tooling).
const DEFAULT_CURRENCY =
  (typeof Cypress !== "undefined" && Cypress.env("DEFAULT_CURRENCY")) || "USD";

// Recursively override every `currency` field on a cloned fixture so nested
// mandate_type blocks (and similar) pick up the run's default.
const withDefaultCurrency = (body) => {
  const clone = JSON.parse(JSON.stringify(body));
  const visit = (node) => {
    if (Array.isArray(node)) {
      node.forEach(visit);
      return;
    }
    if (node && typeof node === "object") {
      if (typeof node.currency === "string") {
        node.currency = DEFAULT_CURRENCY;
      }
      Object.values(node).forEach(visit);
    }
  };
  visit(clone);
  return clone;
};

const createConfirmPaymentBody = withDefaultCurrency(rawCreateConfirmPaymentBody);
const citConfirmBody = withDefaultCurrency(rawCitConfirmBody);
const mitConfirmBody = withDefaultCurrency(rawMitConfirmBody);
const createPaymentBody = withDefaultCurrency(rawCreatePaymentBody);
const pmIdConfirmBody = withDefaultCurrency(rawPmIdConfirmBody);
const ntidConfirmBody = withDefaultCurrency(rawNtidConfirmBody);
const modularPmServicePaymentsCall = withDefaultCurrency(
  rawModularPmServicePaymentsCall
);
export {
  apiKeyCreateBody,
  apiKeyUpdateBody,
  blocklistCreateBody,
  businessProfile,
  captureBody,
  citConfirmBody,
  configs,
  confirmBody,
  createConfirmPaymentBody,
  createConnectorBody,
  createPaymentBody,
  createPayoutBody,
  customerCreateBody,
  customerUpdateBody,
  eligibilityCheckBody,
  gsmBody,
  listRefundCall,
  merchantCreateBody,
  merchantUpdateBody,
  mitConfirmBody,
  ntidConfirmBody,
  pmIdConfirmBody,
  refundBody,
  routingConfigBody,
  saveCardConfirmBody,
  sessionTokenBody,
  updateConnectorBody,
  voidBody,
  IncomingWebhookBody,
  customerCreate,
  paymentMethodCreate,
  paymentMethodUpdate,
  paymentMethodSessionCreate,
  paymentMethodSessionUpdate,
  paymentMethodSessionConfirm,
  modularPmServicePaymentsCall,
};
