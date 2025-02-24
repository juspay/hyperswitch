const successfulThreeDSTestCardDetails = {
  card_number: "4000000000001091",
  card_exp_month: "12",
  card_exp_year: "50",
  card_holder_name: "Max Mustermann",
  card_cvc: "123",
};

const successfulNo3DSCardDetails = {
  card_number: "4200000000000000",
  card_exp_month: "03",
  card_exp_year: "30",
  card_holder_name: "joseph Doe",
  card_cvc: "123",
};

const customerAcceptance = {
  acceptance_type: "offline",
  accepted_at: "1963-05-03T04:07:52.723Z",
  online: {
    ip_address: "127.0.0.1",
    user_agent: "amet irure esse",
  },
};

const singleUseMandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    single_use: {
      amount: 8000,
      currency: "EUR",
    },
  },
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
        amount_to_capture: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          amount: 6000,
          amount_capturable: 0,
          amount_received: 6000,
        },
      },
    },
    PartialCapture: {
      Request: {
        amount_to_capture: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "partially_captured",
          amount: 6000,
          amount_capturable: 0,
          amount_received: 2000,
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
        amount: 6000,
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
        amount: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    SyncRefund: {
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    SaveCardConfirmAutoCaptureOffSession: {
      Request: {
        setup_future_usage: "off_session",
      },
      Response: {
        status: 200,
        trigger_skip: true,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    PaymentIntentOffSession: {
      Request: {
        currency: "EUR",
        amount: 6000,
        authentication_type: "no_three_ds",
        customer_acceptance: null,
        setup_future_usage: "off_session",
      },
      Response: {
        status: 200,
        trigger_skip: true,
        body: {
          status: "requires_payment_method",
          setup_future_usage: "off_session",
        },
      },
    },
    MITAutoCapture: {
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    MITManualCapture: {
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    SaveCardUseNo3DSAutoCaptureOffSession: {
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    ZeroAuthPaymentIntent: {
      Request: {
        amount: 0,
        setup_future_usage: "off_session",
        currency: "EUR",
      },
      Response: {
        status: 200,
        trigger_skip: true,
        body: {
          status: "requires_payment_method",
          setup_future_usage: "off_session",
        },
      },
    },
    ZeroAuthMandate: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 200,
        trigger_skip: true,
        body: {
          status: "succeeded",
        },
      },
    },
    ZeroAuthConfirmPayment: {
      Request: {
        payment_type: "setup_mandate",
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
      },
      Response: {
        status: 501,
        body: {
          error: {
            type: "invalid_request",
            message: "Setup Mandate flow for Novalnet is not implemented",
            code: "IR_00",
          },
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
