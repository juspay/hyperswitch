import { standardBillingAddress } from "./Commons";

const toSnakeCase = (value = "") =>
  value.replace(/([a-z0-9])([A-Z])/g, "$1_$2").toLowerCase();

const BANK_REDIRECT_DEFAULTS = {
  online_banking_fpx: {
    issuer: "affin_bank",
  },
};

export const connectorDetails = {
  bank_redirect_pm: {
    PaymentIntent: (methodType = "Interac") => {
      const normalizedMethodType = toSnakeCase(methodType);
      return {
        Request: {
          currency: "CAD",
          payment_method: "bank_redirect",
          payment_method_type: normalizedMethodType,
          payment_method_data: {
            billing: standardBillingAddress,
            bank_redirect: {
              [normalizedMethodType]:
                BANK_REDIRECT_DEFAULTS[normalizedMethodType] || {},
            },
          },
        },
        Response: {
          status: 200,
          body: {
            status: "requires_confirmation",
          },
        },
      };
    },
    Interac: {
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "interac",
        currency: "CAD",
        payment_method_data: {
          billing: standardBillingAddress,
          bank_redirect: {
            interac: {},
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
  },
  pm_list: {
    PmListResponse: {
      payment_methods: [
        {
          payment_method: "bank_redirect",
          payment_method_types: [
            {
              payment_method_type: "interac",
              supported_currencies: ["CAD"],
            },
          ],
        },
      ],
    },
  },
};
