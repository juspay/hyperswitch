const successfulThreeDSTestCardDetails = {
  card_number: "4000000000001091",
  card_exp_month: "12",
  card_exp_year: "50",
  card_holder_name: "Max Mustermann",
  card_cvc: "123",
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "EUR",
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
    "3DSManualCapture": {
      Request: {
        payment_method: "card",
        billing: {
          address: {
            line1: "1467",
            line2: "CA",
            line3: "CA",
            city: "Musterhausen",
            state: "California",
            zip: "12345",
            country: "DE",
            first_name: "Max",
            last_name: "Mustermann",
          },
          email: "test@novalnet.de",
          phone: {
            number: "9123456789",
            country_code: "+91",
          },
        },
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    "3DSAutoCapture": {
      Request: {
        payment_method: "card",
        billing: {
          address: {
            line1: "1467",
            line2: "CA",
            line3: "CA",
            city: "Musterhausen",
            state: "California",
            zip: "12345",
            country: "DE",
            first_name: "Max",
            last_name: "Mustermann",
          },
          email: "test@novalnet.de",
          phone: {
            number: "9123456789",
            country_code: "+91",
          },
        },
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    //TODO: Add No3DSManualCapture, No3DSAutoCapture
    // No3DSManualCapture: {
    //   Request: {
    //     payment_method: "card",
    //     payment_method_data: {
    //       card: successfulNo3DSCardDetails,
    //     },
    //     customer_acceptance: null,
    //     setup_future_usage: "on_session",
    //   },
    //   Response: {
    //     status: 200,
    //     body: {
    //       status: "requires_capture",
    //     },
    //   },
    // },
    // No3DSAutoCapture: {
    //   Request: {
    //     payment_method: "card",
    //     payment_method_data: {
    //       card: successfulNo3DSCardDetails,
    //     },
    //     customer_acceptance: null,
    //     setup_future_usage: "on_session",
    //   },
    //   Response: {
    //     status: 200,
    //     body: {
    //       status: "succeeded",
    //     },
    //   },
    // },
    Capture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          amount: 6500,
          amount_capturable: 0,
          amount_received: 6500,
        },
      },
    },
    PartialCapture: {
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "partially_captured",
          amount: 6500,
          amount_capturable: 0,
          amount_received: 100,
        },
      },
    },
    Void: {
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "cancelled",
        },
      },
    },
    Refund: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    PartialRefund: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    SyncRefund: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
  },
  pm_list: {
    PmListResponse: {
      PmListNull: {
        payment_methods: [],
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
                    eligible_connectors: ["novalnet"],
                  },
                ],
                required_fields: {
                  "billing.address.first_name": {
                    required_field:
                      "payment_method_data.billing.address.first_name",
                    display_name: "first_name",
                    field_type: "user_full_name",
                    value: null,
                  },
                  "billing.address.last_name": {
                    required_field:
                      "payment_method_data.billing.address.last_name",
                    display_name: "last_name",
                    field_type: "user_full_name",
                    value: null,
                  },
                  "billing.email": {
                    required_field: "payment_method_data.billing.email",
                    display_name: "email_address",
                    field_type: "user_email_address",
                    value: "hyperswitch_sdk_demo_id@gmail.com",
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
                    eligible_connectors: ["novalnet"],
                  },
                ],
                required_fields: {
                  "billing.address.first_name": {
                    required_field:
                      "payment_method_data.billing.address.first_name",
                    display_name: "first_name",
                    field_type: "user_full_name",
                    value: "joseph",
                  },
                  "billing.address.last_name": {
                    required_field:
                      "payment_method_data.billing.address.last_name",
                    display_name: "last_name",
                    field_type: "user_full_name",
                    value: "Doe",
                  },
                  "billing.email": {
                    required_field: "payment_method_data.billing.email",
                    display_name: "email_address",
                    field_type: "user_email_address",
                    value: "hyperswitch.example@gmail.com",
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
                    eligible_connectors: ["novalnet"],
                  },
                ],
                required_fields: {
                  "billing.address.first_name": {
                    required_field:
                      "payment_method_data.billing.address.first_name",
                    display_name: "first_name",
                    field_type: "user_full_name",
                    value: "joseph",
                  },
                  "billing.address.last_name": {
                    required_field:
                      "payment_method_data.billing.address.last_name",
                    display_name: "last_name",
                    field_type: "user_full_name",
                    value: "Doe",
                  },
                  "billing.email": {
                    required_field: "payment_method_data.billing.email",
                    display_name: "email_address",
                    field_type: "user_email_address",
                    value: "hyperswitch.example@gmail.com",
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
                    eligible_connectors: ["novalnet"],
                  },
                ],
                required_fields: {
                  "billing.address.first_name": {
                    required_field:
                      "payment_method_data.billing.address.first_name",
                    display_name: "first_name",
                    field_type: "user_full_name",
                    value: "joseph",
                  },
                  "billing.address.last_name": {
                    required_field:
                      "payment_method_data.billing.address.last_name",
                    display_name: "last_name",
                    field_type: "user_full_name",
                    value: "Doe",
                  },
                  "billing.email": {
                    required_field: "payment_method_data.billing.email",
                    display_name: "email_address",
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
