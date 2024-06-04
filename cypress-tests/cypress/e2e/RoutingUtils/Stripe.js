const card_data = {
  card_number: "4242424242424242",
  card_exp_month: "03",
  card_exp_year: "30",
  card_holder_name: "morino",
  card_cvc: "737",
};

export const connectorDetails = {
  routing: {
    Request: {
      name: "stripe config",
      description: "some desc",
      algorithm: {
        type: "priority",
        data: [],
      },
      profile_id: "{{profile_id}}",
    },
    Response: {
      status: 200,
      body: {},
    },
  },
  card_pm: {
    Confirm: {
      Request: {
        card: card_data,
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
