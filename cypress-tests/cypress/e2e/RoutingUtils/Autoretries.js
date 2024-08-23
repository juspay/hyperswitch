const card_4242 = {
  card_number: "4242424242424242",
  card_exp_month: "03",
  card_exp_year: "30",
  card_holder_name: "Borino",
  card_cvc: "737",
};
const card_1142 = {
  card_number: "4111111145551142",
  card_exp_month: "03",
  card_exp_year: "30",
  card_holder_name: "Borino",
  card_cvc: "737",
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: card_4242,
        },
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
    StripeConfirmMAR1: {
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
    StripeConfirmMAR0: {
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
          card: card_4242,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          connector: "stripe",
        },
      },
    },
  },
};
