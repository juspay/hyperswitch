export const connectorDetails = {
  pm_list: {
    PaymentIntent: {
      RequestCurrencyUSD: {
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "off_session",
        authentication_type: "no_three_ds",
      },
      RequestCurrencyEUR: {
        currency: "EUR",
        customer_acceptance: null,
        setup_future_usage: "off_session",
        authentication_type: "no_three_ds",
      },
      RequestCurrencyINR: {
        currency: "INR",
        customer_acceptance: null,
        setup_future_usage: "off_session",
        authentication_type: "no_three_ds",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
      ResponseAllowedPaymentMethodTypesIdeal: {
        status: 200,
        body: {
          status: "requires_payment_method",
          allowed_payment_method_types: ["ideal"],
        },
      },
      ResponseAllowedPaymentMethodTypesCredit: {
        status: 200,
        body: {
          status: "requires_payment_method",
          allowed_payment_method_types: ["credit"],
        },
      },
      ResponseAllowedPaymentMethodTypesCreditAndPaypal: {
        status: 200,
        body: {
          status: "requires_payment_method",
          allowed_payment_method_types: ["credit", "paypal"],
        },
      },
      ResponseIncorrectPaymentMethodConfiguration: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "No eligible connector was found for the current payment method configuration",
          },
        },
      },
    },
    PmListResponse: {
      PmListCustomerAcceptanceSupportCardCredit: {
        customer_acceptance_support: [
          {
            paymentMethod: "card",
            paymentMethodType: "credit",
            expected: "supported",
          },
        ],
      },
      PmListCustomerAcceptanceSupportOpenBanking: {
        customer_acceptance_support: [
          {
            paymentMethod: "bank_redirect",
            paymentMethodType: "open_banking",
            expected: "partially_supported",
          },
        ],
      },
      PmListNull: {
        payment_methods: [],
      },
      PmListWithStripeForIdeal: {
        status: "requires_payment_method",
        payment_methods: [
          {
            payment_method: "bank_redirect",
            payment_method_types: [
              {
                payment_method_type: "ideal",
                bank_names: [
                  {
                    eligible_connectors: ["stripe"],
                  },
                ],
              },
            ],
          },
        ],
      },
      PmListWithCreditOneConnector: {
        payment_methods: [
          {
            payment_method: "card",
            payment_method_types: [
              {
                payment_method_type: "credit",
              },
            ],
          },
        ],
      },
      PmListWithIdealOneConnector: {
        payment_methods: [
          {
            payment_method: "bank_redirect",
            payment_method_types: [
              {
                payment_method_type: "ideal",
              },
            ],
          },
        ],
      },
      PmListWithCreditTwoConnector: {
        payment_methods: [
          {
            payment_method: "card",
            payment_method_types: [
              {
                payment_method_type: "credit",
                card_networks: [
                  {
                    eligible_connectors: ["stripe", "cybersource"],
                  },
                ],
              },
            ],
          },
        ],
      },
      PmListWithCreditAdyenAuthorizedotnet: {
        payment_methods: [
          {
            payment_method: "card",
            payment_method_types: [
              {
                payment_method_type: "credit",
                card_networks: [
                  {
                    eligible_connectors: ["adyen", "authorizedotnet"],
                  },
                ],
              },
            ],
          },
        ],
      },
    },
  },
};
