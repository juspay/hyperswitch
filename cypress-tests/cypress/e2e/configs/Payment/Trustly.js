import { getCurrency, getCustomExchange } from "./Modifiers";

const billingAddressSE = {
  address: {
    line1: "1 Main St",
    city: "Stockholm",
    zip: "11122",
    country: "SE",
    first_name: "John",
    last_name: "Doe",
  },
  email: "test@example.com",
  phone: {
    number: "9123456789",
    country_code: "+46",
  },
};

const unsupportedBankRedirect = getCustomExchange({
  Configs: {
    TRIGGER_SKIP: true,
  },
});

export const connectorDetails = {
  card_pm: {
    Configs: {
      TRIGGER_SKIP_ALL: true,
    },
  },
  bank_redirect_pm: {
    PaymentIntent: (paymentMethodType) =>
      getCustomExchange({
        Request: {
          currency: getCurrency(paymentMethodType),
          customer_acceptance: null,
        },
        Response: {
          status: 200,
          body: {
            status: "requires_payment_method",
          },
        },
      }),
    Blik: unsupportedBankRedirect,
    Eps: unsupportedBankRedirect,
    Giropay: unsupportedBankRedirect,
    Ideal: unsupportedBankRedirect,
    Sofort: unsupportedBankRedirect,
    Przelewy24: unsupportedBankRedirect,
    OpenBankingUk: unsupportedBankRedirect,
    OnlineBankingFpx: unsupportedBankRedirect,
    Interac: unsupportedBankRedirect,
    Trustly: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "trustly",
        payment_method_data: {
          bank_redirect: {
            trustly: {
              country: "SE",
            },
          },
        },
        currency: "EUR",
        billing: billingAddressSE,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method_type: "trustly",
          connector: "trustly",
        },
      },
      Configs: {
        TRIGGER_SKIP: false,
        skipPaymentMethodStatusAssertion: true,
      },
    }),
    Eft: unsupportedBankRedirect,
    BancontactCard: unsupportedBankRedirect,
  },
};
