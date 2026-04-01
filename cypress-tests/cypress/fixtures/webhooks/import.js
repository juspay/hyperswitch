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
import worldpayPayment from "./worldpay_payment_webhook.json";

export const webhookBodies = {
  stripe: {
    payment: stripePayment,
  },
  noon: {
    payment: noonPayment,
  },
  authorizedotnet: {
    payment: authorizedotnetPayment,
  },
  airwallex: {
    payment: airwallexPayment,
  },
  finix: {
    payment: finixPayment,
  },
  fiuu: {
    payment: fiuuPayment,
  },
  mollie: {
    payment: molliePayment,
  },
  nmi: {
    payment: nmiPayment,
  },
  novalnet: {
    payment: novalnetPayment,
  },
  payload: {
    payment: payloadPayment,
  },
  paypal: {
    payment: paypalPayment,
  },
  trustpay: {
    payment: trustpayPayment,
  },
  worldpay: {
    payment: worldpayPayment,
  },
};
