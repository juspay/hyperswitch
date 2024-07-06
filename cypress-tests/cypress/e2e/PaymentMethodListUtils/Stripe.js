const successfulNo3DSCardDetails = {
  card_number: "4242424242424242",
  card_exp_month: "10",
  card_exp_year: "25",
  card_holder_name: "morino",
  card_cvc: "737",
};

export const connectorDetails = {
  pm_list: {
    PaymentIntent: {
      RequestCurrencyUSD: {
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "off_session",
        authentication_type: "no_three_ds",
      },
      RequestCurrencyEUR: {
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "EUR",
        customer_acceptance: null,
        setup_future_usage: "off_session",
        authentication_type: "no_three_ds",
      },
      RequestCurrencyINR: {
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
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
    },
    PmListResponse: {
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
    },
  },
};
