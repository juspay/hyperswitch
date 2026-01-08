// Gigadat connector config for Cypress tests

export const connectorDetails = {
  bank_redirect_pm: {
    PaymentIntent: (methodType = "Interac") => ({
      Request: {
        currency: "CAD",
        payment_method: "bank_redirect",
        payment_method_type: methodType.toLowerCase(),
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
    Interac: {
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "interac",
        currency: "CAD",
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
