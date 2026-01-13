import stripePayment from "./stripe_payment_webhook.json";
import noonPayment from "./noon_payment_webhook.json";
import authorizedotnetPayment from "./authorizedotnet_payment_webhook.json"

export const webhookBodies = {
  stripe: {
    "payment" : stripePayment,
  },
  noon:{
    "payment" : noonPayment,
  },
  authorizedotnet:{
    "payment" : authorizedotnetPayment,
  }
};