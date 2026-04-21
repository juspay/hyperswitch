import stripePayment from "./stripe_payment_webhook.json";
import noonPayment from "./noon_payment_webhook.json";
import authorizedotnetPayment from "./authorizedotnet_payment_webhook.json";
import airwallexPayment from "./airwallex_payment_webhook.json";
import finixPayment from "./finix_payment_webhook.json";
import fiuuPayment from "./fiuu_payment_webhook.json";
import molliePayment from "./mollie_payment_webhook.json";
import nmiPayment from "./nmi_payment_webhook.json";
import novalnetPayment from "./novalnet_payment_webhook.json";
import payloadPayment from "./payload_payment_webhook.json";
import paypalPayment from "./paypal_payment_webhook.json";
import trustpayPayment from "./trustpay_payment_webhook.json";
import airwallexRefund from "./airwallex_refund_webhook.json";
import finixRefund from "./finix_refund_webhook.json";
import fiuuRefund from "./fiuu_refund_webhook.json";
import nmiRefund from "./nmi_refund_webhook.json";
import novalnetRefund from "./novalnet_refund_webhook.json";
import paypalRefund from "./paypal_refund_webhook.json";
import stripeRefund from "./stripe_refund_webhook.json";

export const webhookBodies = {
  stripe: {
    payment: stripePayment,
    refund: stripeRefund,
  },
  noon: {
    payment: noonPayment,
  },
  authorizedotnet: {
    payment: authorizedotnetPayment,
  },
  airwallex: {
    payment: airwallexPayment,
    refund: airwallexRefund,
  },
  finix: {
    payment: finixPayment,
    refund: finixRefund,
  },
  fiuu: {
    payment: fiuuPayment,
    refund: fiuuRefund,
  },
  mollie: {
    payment: molliePayment,
  },
  nmi: {
    payment: nmiPayment,
    refund: nmiRefund,
  },
  novalnet: {
    payment: novalnetPayment,
    refund: novalnetRefund,
  },
  payload: {
    payment: payloadPayment,
  },
  paypal: {
    payment: paypalPayment,
    refund: paypalRefund,
  },
  trustpay: {
    payment: trustpayPayment,
  },
};
