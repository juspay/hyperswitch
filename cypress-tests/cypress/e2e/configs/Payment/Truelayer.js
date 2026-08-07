import { getCurrency, getCustomExchange } from "./Modifiers";

const billingAddressGB = {
  address: {
    line1: "1 Principal Place",
    city: "London",
    zip: "EC2A 2FA",
    country: "GB",
    first_name: "John",
    last_name: "Doe",
  },
  email: "test@example.com",
  phone: {
    number: "7123456789",
    country_code: "+44",
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
          currency:
            paymentMethodType === "Truelayer"
              ? "GBP"
              : getCurrency(paymentMethodType),
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
    Truelayer: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "open_banking",
        payment_method_data: {
          bank_redirect: {
            open_banking: {},
          },
        },
        currency: "GBP",
        billing: billingAddressGB,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method_type: "open_banking",
          connector: "truelayer",
        },
      },
      Configs: {
        skipPaymentMethodStatusAssertion: true,
      },
    }),
    OnlineBankingFpx: unsupportedBankRedirect,
    Interac: unsupportedBankRedirect,
    Trustly: unsupportedBankRedirect,
    Eft: unsupportedBankRedirect,
    BancontactCard: unsupportedBankRedirect,
    TruelayerNoBilling: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "open_banking",
        payment_method_data: {
          bank_redirect: {
            open_banking: {},
          },
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            code: "IR_04",
          },
        },
      },
    }),
  },
};
