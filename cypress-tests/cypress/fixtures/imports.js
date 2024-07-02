import captureBody from "./capture-flow-body.json";
import confirmBody from "./confirm-body.json";
import apiKeyCreateBody from "./create-api-key-body.json";
import createConfirmPaymentBody from "./create-confirm-body.json";
import createConnectorBody from "./create-connector-body.json";
import customerCreateBody from "./create-customer-body.json";
import citConfirmBody from "./create-mandate-cit.json";
import mitConfirmBody from "./create-mandate-mit.json";
import createPaymentBody from "./create-payment-body.json";
import {
  default as createPayoutBody,
  default as initialCreatePayoutBody,
} from "./create-payout-confirm-body.json";
import pmIdConfirmBody from "./create-pm-id-mit.json";
import listRefundCall from "./list-refund-call-body.json";
import merchantCreateBody from "./merchant-create-body.json";
import refundBody from "./refund-flow-body.json";
import routingConfigBody from "./routing-config-body.json";
import SaveCardConfirmBody from "./save-card-confirm-body.json";
import voidBody from "./void-payment-body.json";

export {
  SaveCardConfirmBody,
  apiKeyCreateBody,
  captureBody,
  citConfirmBody,
  confirmBody,
  createConfirmPaymentBody,
  createConnectorBody,
  createPaymentBody,
  createPayoutBody,
  customerCreateBody,
  initialCreatePayoutBody,
  listRefundCall,
  merchantCreateBody,
  mitConfirmBody,
  pmIdConfirmBody,
  refundBody,
  routingConfigBody,
  voidBody,
};
