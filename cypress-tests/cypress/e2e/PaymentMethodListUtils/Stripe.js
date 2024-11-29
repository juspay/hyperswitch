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
      RequestCurrencyUSDWithBilling: {
        currency: "USD",
        setup_future_usage: "off_session",
        billing: {
          address: {
            line1: "1467",
            line2: "CA",
            line3: "Harrison Street",
            city: "San Fransico",
            state: "CA",
            zip: "94122",
            country: "PL",
            first_name: "joseph",
            last_name: "Doe",
          },
          phone: {
            number: "9111222333",
            country_code: "+91",
          },
        },
        email: "hyperswitch.example@gmail.com",
      },
      RequestWithNameField: {
        currency: "USD",
        setup_future_usage: "off_session",
        billing: {
          address: {
            first_name: "joseph",
            last_name: "Doe",
          },
          phone: {
            number: "9111222333",
            country_code: "+91",
          },
        },
      },
      RequestWithBillingEmail: {
        currency: "USD",
        setup_future_usage: "off_session",
        email: "hyperswitch_sdk_demo_id1@gmail.com",
        billing: {
          address: {
            first_name: "joseph",
            last_name: "Doe",
          },
          phone: {
            number: "9111222333",
            country_code: "+91",
          },
          email: "hyperswitch.example@gmail.com",
        },
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
      pmListDynamicFieldWithoutBilling: {
        payment_methods: [
          {
            payment_method: "card",
            payment_method_types: [
              {
                payment_method_type: "credit",
                card_networks: [
                  {
                    eligible_connectors: ["cybersource"],
                  },
                ],
                required_fields: {
                  "billing.address.first_name": {
                    required_field:
                      "payment_method_data.billing.address.first_name",
                    display_name: "card_holder_name",
                    field_type: "user_full_name",
                    value: null,
                  },
                  "payment_method_data.card.card_number": {
                    required_field: "payment_method_data.card.card_number",
                    display_name: "card_number",
                    field_type: "user_card_number",
                    value: null,
                  },
                  "payment_method_data.card.card_cvc": {
                    required_field: "payment_method_data.card.card_cvc",
                    display_name: "card_cvc",
                    field_type: "user_card_cvc",
                    value: null,
                  },

                  "payment_method_data.card.card_exp_year": {
                    required_field: "payment_method_data.card.card_exp_year",
                    display_name: "card_exp_year",
                    field_type: "user_card_expiry_year",
                    value: null,
                  },
                  "billing.address.last_name": {
                    required_field:
                      "payment_method_data.billing.address.last_name",
                    display_name: "card_holder_name",
                    field_type: "user_full_name",
                    value: null,
                  },
                  "billing.address.state": {
                    required_field: "payment_method_data.billing.address.state",
                    display_name: "state",
                    field_type: "user_address_state",
                    value: null,
                  },
                  "billing.email": {
                    required_field: "payment_method_data.billing.email",
                    display_name: "email",
                    field_type: "user_email_address",
                    value: "hyperswitch_sdk_demo_id@gmail.com",
                  },
                  "billing.address.zip": {
                    required_field: "payment_method_data.billing.address.zip",
                    display_name: "zip",
                    field_type: "user_address_pincode",
                    value: null,
                  },
                  "payment_method_data.card.card_exp_month": {
                    required_field: "payment_method_data.card.card_exp_month",
                    display_name: "card_exp_month",
                    field_type: "user_card_expiry_month",
                    value: null,
                  },
                  "billing.address.line1": {
                    required_field: "payment_method_data.billing.address.line1",
                    display_name: "line1",
                    field_type: "user_address_line1",
                    value: null,
                  },
                  "billing.address.city": {
                    required_field: "payment_method_data.billing.address.city",
                    display_name: "city",
                    field_type: "user_address_city",
                    value: null,
                  },
                },
              },
            ],
          },
        ],
      },
      pmListDynamicFieldWithBilling: {
        payment_methods: [
          {
            payment_method: "card",
            payment_method_types: [
              {
                payment_method_type: "credit",
                card_networks: [
                  {
                    eligible_connectors: ["cybersource"],
                  },
                ],
                required_fields: {
                  "billing.address.city": {
                    required_field: "payment_method_data.billing.address.city",
                    display_name: "city",
                    field_type: "user_address_city",
                    value: "San Fransico",
                  },
                  "billing.address.state": {
                    required_field: "payment_method_data.billing.address.state",
                    display_name: "state",
                    field_type: "user_address_state",
                    value: "CA",
                  },
                  "billing.address.zip": {
                    required_field: "payment_method_data.billing.address.zip",
                    display_name: "zip",
                    field_type: "user_address_pincode",
                    value: "94122",
                  },
                  "billing.address.country": {
                    required_field:
                      "payment_method_data.billing.address.country",
                    display_name: "country",
                    field_type: {
                      user_address_country: {
                        options: ["ALL"],
                      },
                    },
                    value: "PL",
                  },
                  "billing.address.first_name": {
                    required_field:
                      "payment_method_data.billing.address.first_name",
                    display_name: "card_holder_name",
                    field_type: "user_full_name",
                    value: "joseph",
                  },
                  "billing.address.last_name": {
                    required_field:
                      "payment_method_data.billing.address.last_name",
                    display_name: "card_holder_name",
                    field_type: "user_full_name",
                    value: "Doe",
                  },
                  "billing.email": {
                    required_field: "payment_method_data.billing.email",
                    display_name: "email",
                    field_type: "user_email_address",
                    value: "hyperswitch.example@gmail.com",
                  },
                  "payment_method_data.card.card_cvc": {
                    required_field: "payment_method_data.card.card_cvc",
                    display_name: "card_cvc",
                    field_type: "user_card_cvc",
                    value: null,
                  },
                  "billing.address.line1": {
                    required_field: "payment_method_data.billing.address.line1",
                    display_name: "line1",
                    field_type: "user_address_line1",
                    value: "1467",
                  },
                  "payment_method_data.card.card_exp_month": {
                    required_field: "payment_method_data.card.card_exp_month",
                    display_name: "card_exp_month",
                    field_type: "user_card_expiry_month",
                    value: null,
                  },
                  "payment_method_data.card.card_number": {
                    required_field: "payment_method_data.card.card_number",
                    display_name: "card_number",
                    field_type: "user_card_number",
                    value: null,
                  },
                  "payment_method_data.card.card_exp_year": {
                    required_field: "payment_method_data.card.card_exp_year",
                    display_name: "card_exp_year",
                    field_type: "user_card_expiry_year",
                    value: null,
                  },
                },
              },
            ],
          },
        ],
      },
      pmListDynamicFieldWithNames: {
        payment_methods: [
          {
            payment_method: "card",
            payment_method_types: [
              {
                payment_method_type: "credit",
                card_networks: [
                  {
                    eligible_connectors: ["cybersource"],
                  },
                ],
                required_fields: {
                  "billing.address.last_name": {
                    required_field:
                      "payment_method_data.billing.address.last_name",
                    display_name: "card_holder_name",
                    field_type: "user_full_name",
                    value: "Doe",
                  },
                  "billing.address.first_name": {
                    required_field:
                      "payment_method_data.billing.address.first_name",
                    display_name: "card_holder_name",
                    field_type: "user_full_name",
                    value: "joseph",
                  },
                },
              },
            ],
          },
        ],
      },
      pmListDynamicFieldWithEmail: {
        payment_methods: [
          {
            payment_method: "card",
            payment_method_types: [
              {
                payment_method_type: "credit",
                card_networks: [
                  {
                    eligible_connectors: ["cybersource"],
                  },
                ],
                required_fields: {
                  "billing.email": {
                    required_field: "payment_method_data.billing.email",
                    display_name: "email",
                    field_type: "user_email_address",
                    value: "hyperswitch.example@gmail.com",
                  },
                },
              },
            ],
          },
        ],
      },
    },
  },
};
