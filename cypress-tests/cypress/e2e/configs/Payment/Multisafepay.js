import {
  customerAcceptance,
  multiUseMandateData,
  singleUseMandateData,
} from "./Commons";

const successfulThreeDSTestCardDetails = {
  card_number: "5500000000000004",
  card_exp_month: "03",
  card_exp_year: "30",
  card_holder_name: "Joseph Doe",
  card_cvc: "123",
};

const failedNo3DSCardDetails = {
  card_number: "4242424242424242",
  card_exp_month: "01",
  card_exp_year: "35",
  card_holder_name: "Joseph Doe",
  card_cvc: "123",
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
    PaymentIntentOffSession: {
      Request: {
        amount: 6000,
        authentication_type: "no_three_ds",
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "off_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    PaymentIntentWithShippingCost: {
      Request: {
        currency: "USD",
        shipping_cost: 50,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          shipping_cost: 50,
          amount: 6000,
        },
      },
    },
    PaymentConfirmWithShippingCost: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          shipping_cost: 50,
          amount_received: 6050,
          amount: 6000,
          net_amount: 6050,
        },
      },
    },
    "3DSManualCapture": {
      Configs: {
        TRIGGER_SKIP: true, // Multisafepay does not support manual capture
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
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
    "3DSAutoCapture": {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
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
    No3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true, // Multisafepay does not support manual capture
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
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
    No3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
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
    No3DSFailPayment: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: failedNo3DSCardDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_code: "1024",
          error_message: "declined",
        },
      },
    },
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
          amount_capturable: 4000,
          amount_received: 2000,
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
    manualPaymentRefund: {
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
    manualPaymentPartialRefund: {
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
    MandateSingleUse3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    MandateSingleUse3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true, // Multisafepay does not support manual capture
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    MandateSingleUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    MandateSingleUseNo3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true, // Multisafepay does not support manual capture
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    MandateMultiUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        mandate_data: multiUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    MandateMultiUseNo3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true, // Multisafepay does not support manual capture
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        mandate_data: multiUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    MandateMultiUse3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        mandate_data: multiUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    MandateMultiUse3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true, // Multisafepay does not support manual capture
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        mandate_data: multiUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
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
    ZeroAuthMandate: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 501,
        body: {
          error: {
            type: "invalid_request",
            message: "Setup Mandate flow for Multisafepay is not implemented",
            code: "IR_00",
          },
        },
      },
    },
    ZeroAuthPaymentIntent: {
      Request: {
        amount: 0,
        setup_future_usage: "off_session",
        currency: "USD",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          setup_future_usage: "off_session",
        },
      },
    },
    ZeroAuthConfirmPayment: {
      Request: {
        payment_type: "setup_mandate",
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
      },
      Response: {
        status: 501,
        body: {
          error: {
            type: "invalid_request",
            message: "Setup Mandate flow for Multisafepay is not implemented",
            code: "IR_00",
          },
        },
      },
    },
    SaveCardUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    SaveCardUseNo3DSAutoCaptureOffSession: {
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    SaveCardUse3DSAutoCaptureOffSession: {
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    SaveCardUseNo3DSManualCaptureOffSession: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    SaveCardConfirmAutoCaptureOffSession: {
      Request: {
        setup_future_usage: "off_session",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    SaveCardConfirmManualCaptureOffSession: {
      Request: {
        setup_future_usage: "off_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    SaveCardUseNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    PaymentMethodIdMandateNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    PaymentMethodIdMandateNo3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true, // Multisafepay does not support manual capture
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    PaymentMethodIdMandate3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        authentication_type: "three_ds",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    PaymentMethodIdMandate3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true, // Multisafepay does not support manual capture
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        mandate_data: null,
        authentication_type: "three_ds",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
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
                    eligible_connectors: ["multisafepay"],
                  },
                ],
                required_fields: {
                  "payment_method_data.card.card_number": {
                    required_field: "payment_method_data.card.card_number",
                    display_name: "card_number",
                    field_type: "user_card_number",
                    value: null,
                  },
                  "payment_method_data.card.card_exp_month": {
                    required_field: "payment_method_data.card.card_exp_month",
                    display_name: "card_exp_month",
                    field_type: "user_card_expiry_month",
                    value: null,
                  },
                  "payment_method_data.card.card_exp_year": {
                    required_field: "payment_method_data.card.card_exp_year",
                    display_name: "card_exp_year",
                    field_type: "user_card_expiry_year",
                    value: null,
                  },
                  "payment_method_data.card.card_cvc": {
                    required_field: "payment_method_data.card.card_cvc",
                    display_name: "card_cvc",
                    field_type: "user_card_cvc",
                    value: null,
                  },
                  "billing.address.first_name": {
                    required_field:
                      "payment_method_data.billing.address.first_name",
                    display_name: "card_holder_name",
                    field_type: "user_full_name",
                    value: null,
                  },
                  "billing.address.last_name": {
                    required_field:
                      "payment_method_data.billing.address.last_name",
                    display_name: "card_holder_name",
                    field_type: "user_full_name",
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
                    eligible_connectors: ["multisafepay"],
                  },
                ],
                required_fields: {
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
                    eligible_connectors: ["multisafepay"],
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
                    eligible_connectors: ["multisafepay"],
                  },
                ],
                required_fields: {
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
                },
              },
            ],
          },
        ],
      },
    },
  },
  bank_redirect_pm: {
    Eps: {
      Request: {
        amount: 333,
        currency: "EUR",
        confirm: true,
        capture_method: "automatic",
        payment_method: "bank_redirect",
        payment_method_type: "eps",
        authentication_type: "no_three_ds",
        description: "hello world",
        billing: {
          address: {
            zip: "560095",
            first_name: "John",
            last_name: "Doe",
            line1: "Harry Street",
            country: "AT",
            line2: "Downbridge",
            city: "London",
          },
        },
        payment_method_data: {
          bank_redirect: {
            eps: {},
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    Sofort: {
      Request: {
        amount: 333,
        currency: "EUR",
        confirm: true,
        capture_method: "automatic",
        payment_method: "bank_redirect",
        payment_method_type: "sofort",
        authentication_type: "no_three_ds",
        description: "hello world",
        billing: {
          address: {
            zip: "560095",
            first_name: "John",
            last_name: "Doe",
            line1: "Harry Street",
            country: "AT",
            line2: "line2",
            city: "London",
          },
        },
        payment_method_data: {
          bank_redirect: {
            sofort: {},
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    Mbway: {
      Request: {
        amount: 333,
        currency: "EUR",
        confirm: true,
        capture_method: "automatic",
        payment_method: "wallet",
        payment_method_type: "mb_way",
        authentication_type: "no_three_ds",
        description: "hello world",
        billing: {
          address: {
            zip: "560095",
            first_name: "John",
            last_name: "Doe",
            line1: "Main street",
            country: "PT",
            line2: "sub street",
            city: "city",
          },
        },
        payment_method_data: {
          wallet: {
            mb_way_redirect: {},
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
  },
};
