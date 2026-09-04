import {
  customerAcceptance,
  multiUseMandateData,
  singleUseMandateData,
  standardBillingAddress,
} from "./Commons";
import { getCurrency, getCustomExchange } from "./Modifiers";

// Triggers a 3DS challenge (customer-verification redirect) in MultiSafepay's
// test environment. Used by flows that expect `requires_customer_action`.
const successfulThreeDSTestCardDetails = {
  card_number: "4761340000000019",
  card_exp_month: "03",
  card_exp_year: "30",
  card_holder_name: "Joseph Doe",
  card_cvc: "737",
};

// MultiSafepay's test environment cannot authorize direct card orders after a
// 3DS challenge — the order is always declined with reason_code 5999
// "Internal System Error" (reproducible even with MultiSafepay's own
// documented example request). This Amex card authenticates frictionlessly
// (no challenge) and completes instantly, so every flow that needs a terminal
// `succeeded` payment (sync, refund, capture, shipping-cost, mandates, MIT)
// must use it.
const successfulNoThreeDSTestCardDetails = {
  card_number: "374500000000015",
  card_exp_month: "03",
  card_exp_year: "30",
  card_holder_name: "Joseph Doe",
  card_cvc: "7373",
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
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNoThreeDSTestCardDetails,
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
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
            reason: "manual is not supported by multisafepay",
          },
        },
      },
    },
    "3DSAutoCapture": {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
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
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNoThreeDSTestCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
            reason: "manual is not supported by multisafepay",
          },
        },
      },
    },
    No3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNoThreeDSTestCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    No3DSFailPayment: {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
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
          status: "failed",
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
          status: "failed",
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
          status: "failed",
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
          status: "failed",
        },
      },
    },
    SyncRefund: {
      Response: {
        status: 200,
        body: {
          status: "failed",
        },
      },
    },
    MandateSingleUse3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNoThreeDSTestCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    MandateSingleUse3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true, // Multisafepay does not support manual capture
      },
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNoThreeDSTestCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
            reason: "manual is not supported by multisafepay",
          },
        },
      },
    },
    MandateSingleUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNoThreeDSTestCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    MandateSingleUseNo3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true, // Multisafepay does not support manual capture
      },
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNoThreeDSTestCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
            reason: "manual is not supported by multisafepay",
          },
        },
      },
    },
    MandateMultiUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNoThreeDSTestCardDetails,
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
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNoThreeDSTestCardDetails,
        },
        currency: "USD",
        mandate_data: multiUseMandateData,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
            reason: "manual is not supported by multisafepay",
          },
        },
      },
    },
    MandateMultiUse3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNoThreeDSTestCardDetails,
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
    MandateMultiUse3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true, // Multisafepay does not support manual capture
      },
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNoThreeDSTestCardDetails,
        },
        currency: "USD",
        mandate_data: multiUseMandateData,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
            reason: "manual is not supported by multisafepay",
          },
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
    MITAutoCaptureWithCustomerAcceptance: {
      Request: {
        customer_acceptance: {
          acceptance_type: "offline",
          accepted_at: "1963-05-03T04:07:52.723Z",
          online: {
            ip_address: "127.0.0.1",
            user_agent: "amet irure esse",
          },
        },
      },
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
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
            reason: "manual is not supported by multisafepay",
          },
        },
      },
    },
    ZeroAuthMandate: {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNoThreeDSTestCardDetails,
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
          card: successfulNoThreeDSTestCardDetails,
        },
        mandate_data: null,
        customer_acceptance: customerAcceptance,
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
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNoThreeDSTestCardDetails,
        },
        currency: "USD",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    SaveCardUseNo3DSAutoCaptureOffSession: {
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulNoThreeDSTestCardDetails,
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
    SaveCardUse3DSAutoCaptureOffSession: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulNoThreeDSTestCardDetails,
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
    SaveCardUseNo3DSManualCaptureOffSession: {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNoThreeDSTestCardDetails,
        },
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
            reason: "manual is not supported by multisafepay",
          },
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
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
            reason: "manual is not supported by multisafepay",
          },
        },
      },
    },
    SaveCardUseNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNoThreeDSTestCardDetails,
        },
        currency: "USD",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
            reason: "manual is not supported by multisafepay",
          },
        },
      },
    },
    PaymentMethodIdMandateNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNoThreeDSTestCardDetails,
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
        payment_method_type: "credit",
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
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
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
        payment_method_type: "credit",
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
  wallet_pm: {
    PaymentIntent: (paymentMethodType) =>
      getCustomExchange({
        Request: {
          currency: ["AliPay", "WeChatPay"].includes(paymentMethodType)
            ? "EUR"
            : getCurrency(paymentMethodType),
        },
        Response: {
          status: 200,
          body: {
            status: "requires_payment_method",
          },
        },
      }),

    AliPay: getCustomExchange({
      Request: {
        payment_method: "wallet",
        payment_method_type: "ali_pay",
        authentication_type: "no_three_ds",
        payment_method_data: {
          wallet: {
            ali_pay_redirect: {},
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "Shanghai",
            state: "Shanghai",
            zip: "200000",
            country: "CN",
            first_name: "joseph",
            last_name: "Doe",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),

    PayPal: getCustomExchange({
      // NOTE: MultiSafepay PayPal sandbox enforces CSRF protection, so the
      // redirection handler skips the click-through (returns verifyUrl=false).
      // See redirectionHandler.js::bankRedirectRedirection > multisafepay.
      Request: {
        payment_method: "wallet",
        payment_method_type: "paypal",
        authentication_type: "no_three_ds",
        payment_method_data: {
          wallet: {
            paypal_redirect: {},
          },
        },
        billing: standardBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),

    WeChatPay: getCustomExchange({
      Request: {
        payment_method: "wallet",
        payment_method_type: "we_chat_pay",
        authentication_type: "no_three_ds",
        payment_method_data: {
          wallet: {
            we_chat_pay_redirect: {},
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "Shanghai",
            state: "Shanghai",
            zip: "200000",
            country: "CN",
            first_name: "joseph",
            last_name: "Doe",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),

    MbWay: getCustomExchange({
      Request: {
        payment_method: "wallet",
        payment_method_type: "mb_way",
        authentication_type: "no_three_ds",
        payment_method_data: {
          wallet: {
            mb_way_redirect: {},
          },
        },
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
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
  },
};
