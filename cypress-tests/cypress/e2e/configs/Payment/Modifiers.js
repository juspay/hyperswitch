import State from "../../../utils/State";

const globalState = new State({
  connectorId: Cypress.env("CONNECTOR"),
  baseUrl: Cypress.env("BASEURL"),
  adminApiKey: Cypress.env("ADMINAPIKEY"),
  connectorAuthFilePath: Cypress.env("CONNECTOR_AUTH_FILE_PATH"),
});

const connectorName = normalize(globalState.get("connectorId"));

function normalize(input) {
  const exceptions = {
    adyen: "Adyen",
    archipel: "Archipel",
    bankofamerica: "Bank of America",
    cybersource: "Cybersource",
    datatrans: "Datatrans",
    facilitapay: "Facilitapay",
    noon: "Noon",
    paybox: "Paybox",
    paypal: "Paypal",
    stax: "Stax",
    wellsfargo: "Wellsfargo",
    nmi: "Nmi",
    stripeconnect: "stripe",
    // Add more known exceptions here
  };

  if (typeof input !== "string") {
    const specName = Cypress.spec.name;

    if (specName.includes("-")) {
      const parts = specName.split("-");

      if (parts.length > 1 && parts[1].includes(".")) {
        return parts[1].split(".")[0];
      }
    }

    // Fallback
    return `${specName}`;
  }

  const lowerCaseInput = input.toLowerCase();
  return exceptions[lowerCaseInput] || input;
}

/*
`getDefaultExchange` contains the default Request and Response to be considered if none provided.
`getCustomExchange` takes in 2 optional fields named as Request and Response.
with `getCustomExchange`, if 501 response is expected, there is no need to pass Response as it considers default values.
*/

// Const to get default PaymentExchange object
const getDefaultExchange = () => ({
  Request: {},
  Response: {
    status: 501,
    body: {
      error: {
        type: "invalid_request",
        message: `Selected payment method through ${connectorName} is not implemented`,
        code: "IR_00",
      },
    },
  },
});

const getUnsupportedExchange = () => ({
  Request: {
    currency: "EUR",
  },
  Response: {
    status: 400,
    body: {
      error: {
        type: "invalid_request",
        message: `Payment method type not supported`,
        code: "IR_19",
      },
    },
  },
});

// Const to get PaymentExchange with overridden properties
export const getCustomExchange = (overrides, inheritFrom = null) => {
  const defaultExchange = getDefaultExchange();
  const baseExchange = inheritFrom || defaultExchange;

  return {
    ...baseExchange,
    ...(overrides.Configs ? { Configs: overrides.Configs } : {}),
    Request: {
      ...baseExchange.Request,
      ...(overrides.Request || {}),
    },
    Response: {
      ...baseExchange.Response,
      ...(overrides.Response || {}),
    },
    ...(overrides.ResponseCustom
      ? { ResponseCustom: overrides.ResponseCustom }
      : {}),
  };
};

// Function to update the default status code
export const updateDefaultStatusCode = () => {
  return getUnsupportedExchange().Response;
};

export const getIframeRedirectionConfig = (opts = {}) => {
  const {
    cardDetails,
    currency = "USD",
    amount,
    payment_method_type,
    payment_method_data_3ds,
    configs,
  } = opts;

  const iframeRedirection = {
    Request: {
      payment_method: "card",
      payment_method_data: { card: cardDetails },
      currency,
      customer_acceptance: null,
      setup_future_usage: "on_session",
      is_iframe_redirection_enabled: true,
    },
    Response: {
      status: 200,
      body: {
        status: "requires_customer_action",
        setup_future_usage: "on_session",
      },
    },
  };

  if (amount !== undefined) {
    iframeRedirection.Request.amount = amount;
  }

  if (payment_method_type) {
    iframeRedirection.Request.payment_method_type = payment_method_type;
  }

  if (payment_method_data_3ds) {
    iframeRedirection.Response.body.payment_method_data =
      payment_method_data_3ds;
  }

  if (configs && Object.keys(configs).length > 0) {
    iframeRedirection.Configs = configs;
  }

  return {
    IframeRedirectionCreate: {
      Request: {
        amount: amount ?? 6000,
        currency,
        is_iframe_redirection_enabled: true,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    IframeRedirection: iframeRedirection,
  };
};

// Currency map with logical grouping
const CURRENCY_MAP = {
  // Polish payment methods
  Blik: "PLN",
  InstantBankTransferPoland: "PLN",

  // Brazilian payment methods
  Pix: "BRL",

  // European payment methods (EUR)
  Bluecode: "EUR",
  Eps: "EUR",
  Giropay: "EUR",
  Ideal: "EUR",
  InstantBankTransferFinland: "EUR",
  Klarna: "EUR",
  Przelewy24: "EUR",
  Sofort: "EUR",
  Trustly: "EUR",
  BancontactCard: "EUR",
  OpenBankingUk: "GBP", // Great British Pound payment method
  OnlineBankingFpx: "MYR", // Malaysian payment methods
  Interac: "CAD", // Canadian payment method
  AliPayHk: "HKD", // Hong Kong payment method
  Payjustnow: "ZAR", // South African BNPL
  Affirm: "USD", // US BNPL payment method
  AliPay: "CNY", // Default ISO-4217 currency; MultiSafepay sandbox overrides to EUR (see Multisafepay.js::wallet_pm.PaymentIntent)
  WeChatPay: "CNY", // Default ISO-4217 currency; MultiSafepay sandbox overrides to EUR (see Multisafepay.js::wallet_pm.PaymentIntent)
  Paypal: "EUR",
  MbWay: "EUR",
  Mifinity: "EUR", // Mifinity wallet payment method
  Alma: "EUR", // French pay_later
  Atome: "SGD", // Singapore pay_later
  Walley: "SEK", // Swedish pay_later

  // Voucher payment methods
  Boleto: "BRL",
  Oxxo: "MXN",
  Alfamart: "IDR",
  Indomaret: "IDR",
  SevenEleven: "JPY",
  Lawson: "JPY",
  MiniStop: "JPY",
  FamilyMart: "JPY",
  Seicomart: "JPY",
  PayEasy: "JPY",
  Skrill: "USD", // Skrill wallet payment method
  PaySafeCard: "USD", // PaySafeCard gift card payment method

  // Wallet redirect and mandate payment methods
  PaypalRedirect: "USD",

  // Wallet mandate payment methods
  KakaoPay: "KRW",
  Gcash: "PHP",
  Momo: "VND",
  Twint: "CHF",
  Vipps: "NOK",
  Dana: "IDR",
  GoPay: "IDR",
};

export const getCurrency = (paymentMethodType) => {
  return CURRENCY_MAP[paymentMethodType] || "USD";
};
