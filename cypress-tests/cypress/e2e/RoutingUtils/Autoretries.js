const card_1142 = {
  card_number: "4111111145551142",
  card_exp_month: "03",
  card_exp_year: "30",
  card_holder_name: "Borino",
  card_cvc: "737",
};
const card_9299 = {
  card_number: "4263982640269299",
  card_exp_month: "02",
  card_exp_year: "26",
  card_holder_name: "Borino",
  card_cvc: "837",
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    StripeConfirm: {
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: card_1142,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          connector: "stripe",
        },
      },
    },
    AdyenConfirm: {
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: card_1142,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          connector: "adyen",
        },
      },
    },
    BluesnapConfirm: {
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: card_9299,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          connector: "bluesnap",
        },
      },
    },
  },
};
