import { getCustomExchange } from "./Modifiers";
import { standardBillingAddress } from "./Commons";

const nlBillingAddress = {
  ...standardBillingAddress,
  address: {
    ...standardBillingAddress.address,
    line1: "Damrak 1",
    line2: "",
    line3: "",
    city: "Amsterdam",
    state: "NH",
    zip: "1012 LG",
    country: "NL",
    first_name: "Jan",
    last_name: "Jansen",
  },
  phone: {
    number: "612345678",
    country_code: "+31",
  },
};

export const connectorDetails = {
  crypto_pm: {
    PaymentIntent: getCustomExchange({
      Request: {
        currency: "EUR",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
    CryptoCurrency: getCustomExchange({
      Request: {
        payment_method: "crypto",
        payment_method_type: "crypto_currency",
        payment_method_data: {
          crypto: {
            network: "bitcoin",
            pay_currency: "BTC",
          },
        },
        billing: nlBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
    // Coingate does not support manual capture for crypto payments.
    // The IR_19 error response causes should_continue_further() to return false
    // (because Response.body contains an `error` object), which skips the
    // redirect-handling and retrieve-payment steps in the spec.
    CryptoCurrencyManualCapture: getCustomExchange({
      Request: {
        payment_method: "crypto",
        payment_method_type: "crypto_currency",
        payment_method_data: {
          crypto: {
            network: "bitcoin",
            pay_currency: "BTC",
          },
        },
        billing: nlBillingAddress,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "No eligible connector was found for the current payment method configuration",
            code: "IR_19",
          },
        },
      },
    }),
  },
};
