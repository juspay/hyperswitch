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
        body: {},
      },
    }),
    CryptoCurrency: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
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
    }),
    CryptoCurrencyManualCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
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
    }),
  },
};
