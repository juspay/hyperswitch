import {
  customerAcceptance,
  multiUseMandateData,
  singleUseMandateData,
} from "./Commons";
import { generateRandomEmail } from "../../../utils/RequestBodyUtils";

const successfulNo3DSCardDetails = {
  card_number: "4111111111111111",
  card_exp_month: "12",
  card_exp_year: "2029",
  card_holder_name: "John Doe",
  card_cvc: "123",
  card_network: "Visa",
};

const successfulThreeDSTestCardDetails = {
  card_number: "4111111111111111",  // Visa test card (approved in Authorize.Net sandbox)
  card_exp_month: "12",
  card_exp_year: "2029",
  card_holder_name: "John Doe",
  card_cvc: "123",
  card_network: "Visa",
};

const failedNo3DSCardDetails = {
  card_number: "4000000000000127",
  card_exp_month: "12",
  card_exp_year: "2029",
  card_holder_name: "John Doe",
  card_cvc: "123",
};

const paymentMethodData = {
  card: {
    last4: "1111",
    card_type: "CREDIT",
    card_network: "Visa",
    card_issuer: "JP Morgan",
    card_issuing_country: "INDIA",
    card_isin: "411111",
    card_extended_bin: null,
    card_exp_month: "12",
    card_exp_year: "2029",
    card_holder_name: "John Doe",
    payment_checks: {
      description: "The street address and postal code matched.",
      avs_result_code: "Y",
    },
    authentication_data: null,
  },
  billing: null,
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        email: generateRandomEmail(),
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
        email: generateRandomEmail(),
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
        email: generateRandomEmail(),
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
          card: successfulNo3DSCardDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
        email: generateRandomEmail(),
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
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        email: generateRandomEmail(),
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method_data: paymentMethodData,
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
        email: generateRandomEmail(),
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method_data: paymentMethodData,
        },
      },
    },
    No3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        email: generateRandomEmail(),
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          payment_method_data: paymentMethodData,
        },
      },
    },
    No3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        email: generateRandomEmail(),
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method_data: paymentMethodData,
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
        email: generateRandomEmail(),
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded", // No Test card for failed payment in Authorizedotnet
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
      Configs: {
        TRIGGER_SKIP: true, // Refund will happen only after the payment is settled.
      },
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
      Configs: {
        TRIGGER_SKIP: true, // Refund will happen only after the payment is settled.
      },
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
      Configs: {
        TRIGGER_SKIP: true, // Refund will happen only after the payment is settled.
      },
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
      Configs: {
        TRIGGER_SKIP: true, // Refund will happen only after the payment is settled.
      },
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
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
        email: generateRandomEmail(),
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method_data: paymentMethodData,
        },
      },
    },
    MandateSingleUse3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
        email: generateRandomEmail(),
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method_data: paymentMethodData,
        },
      },
    },
    MandateSingleUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
        email: generateRandomEmail(),
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method_data: paymentMethodData,
        },
      },
    },
    MandateSingleUseNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
        email: generateRandomEmail(),
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    MandateMultiUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: multiUseMandateData,
        email: generateRandomEmail(),
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method_data: paymentMethodData,
        },
      },
    },
    MandateMultiUseNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: multiUseMandateData,
        email: generateRandomEmail(),
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          payment_method_data: paymentMethodData,
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
        email: generateRandomEmail(),
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method_data: paymentMethodData,
        },
      },
    },
    MandateMultiUse3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        mandate_data: multiUseMandateData,
        email: generateRandomEmail(),
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method_data: paymentMethodData,
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
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
        email: generateRandomEmail(),
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
          card: successfulNo3DSCardDetails,
        },
        email: generateRandomEmail(),
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          setup_future_usage: "off_session",
        },
      },
    },
    SaveCardUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
        email: generateRandomEmail(),
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
          card: successfulNo3DSCardDetails,
        },
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
        email: generateRandomEmail(),
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
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
        email: generateRandomEmail(),
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
          card: successfulNo3DSCardDetails,
        },
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
        email: generateRandomEmail(),
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    SaveCardConfirmAutoCaptureOffSession: {
      Request: {
        setup_future_usage: "off_session",
        email: generateRandomEmail(),
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
        email: generateRandomEmail(),
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    SaveCardUseNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
        email: generateRandomEmail(),
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    PaymentMethodIdMandateNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
        email: generateRandomEmail(),
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    PaymentMethodIdMandateNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
        email: generateRandomEmail(),
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
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
        email: generateRandomEmail(),
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    PaymentMethodIdMandate3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        mandate_data: null,
        authentication_type: "three_ds",
        customer_acceptance: customerAcceptance,
        email: generateRandomEmail(),
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    UCSZeroAuthMandate: {
      Request: {
        amount: 0,
        confirm: false,
        currency: "USD",
        customer_id: "Customer123_UCS",
        setup_future_usage: "off_session"
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method"
        }
      }
    },

    UCSConfirmMandate: {
      Request: {
        confirm: true,
        customer_id: "Customer123_UCS",
        payment_type: "setup_mandate",
        customer_acceptance: {
          acceptance_type: "online",
          accepted_at: "1963-05-03T04:07:52.723Z",
          online: {
            ip_address: "127.0.0.1",
            user_agent: "amet irure esse"
          }
        },
        payment_method: "card",
        payment_method_type: "credit",
        email: generateRandomEmail(),
        payment_method_data: {
          card: {
            card_number: "4349940199004549", // Visa from real mandate example
            card_exp_month: "12",
            card_exp_year: "30",
            card_holder_name: "joseph Doe",
            card_cvc: "396",
            card_network: "VISA"
          },
          billing: {
            address: {
              line1: "1467",
              line2: "Harrison Street",
              line3: "Harrison Street",
              city: "San Fransico",
              state: "California", 
              zip: "94122",
              country: "IT",
              first_name: "joseph",
              last_name: "Doe"
            },
            email: generateRandomEmail(),
            phone: {
              number: "8056594427",
              country_code: "+91"
            }
          }
        },
        all_keys_required: true
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded"
        }
      }
    },

    UCSRecurringPayment: {
      Request: {
        amount: 100,
        currency: "USD",
        confirm: true,
        capture_method: "automatic",
        customer_id: "Customer123_UCS",
        off_session: true,
        recurring_details: {
          type: "payment_method_id",
          data: "pm_placeholder" // Will be dynamically set in tests
        },
        all_keys_required: true,
        metadata: {
          ucs_test: "recurring_payment",
          payment_sequence: "recurring"
        }
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded"
        }
      }
    },
  },
};
