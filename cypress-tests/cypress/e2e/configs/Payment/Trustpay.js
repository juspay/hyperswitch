import {
  customerAcceptance,
  connectorDetails as commonConnectorDetails,
} from "./Commons";
import { getCustomExchange } from "./Modifiers";

const successfulNo3DSCardDetails = {
  card_number: "4200000000000000",
  card_exp_month: "10",
  card_exp_year: "30",
  card_holder_name: "joseph Doe",
  card_cvc: "123",
};

const successfulThreeDSTestCardDetails = {
  card_number: "4200000000000067",
  card_exp_month: "03",
  card_exp_year: "30",
  card_holder_name: "John Doe",
  card_cvc: "737",
};

const singleUseMandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    single_use: {
      amount: 8000,
      currency: "USD",
    },
  },
};

const multiUseMandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    multi_use: {
      amount: 8000,
      currency: "USD",
    },
  },
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: getCustomExchange({
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
    }),
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
          card: successfulNo3DSCardDetails,
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
    No3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
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
    Capture: {
      Request: {
        amount_to_capture: 6000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "This Payment could not be captured because it has a payment.status of requires_payment_method. The expected state is requires_capture, partially_captured_and_capturable, processing",
            code: "IR_14",
          },
        },
      },
    },
    PartialCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 15000,
        },
      },
      Request: {
        amount_to_capture: 2000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "This Payment could not be captured because it has a payment.status of requires_payment_method. The expected state is requires_capture, partially_captured_and_capturable, processing",
            code: "IR_14",
          },
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
        DELAY: {
          STATUS: true,
          TIMEOUT: 15000,
        },
      },
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
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 15000,
        },
      },
      Request: {
        amount: 2000,
      },
      Response: {
        status: 200,
        body: {
          reason: "FRAUD",
          status: "succeeded",
        },
      },
    },
    SyncRefund: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 15000,
        },
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
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
          status: "succeeded",
        },
      },
    },
    ZeroAuthMandate: {
      Response: {
        status: 501,
        body: {
          error: {
            type: "invalid_request",
            message: "Setup Mandate flow for Trustpay is not implemented",
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
          card: successfulNo3DSCardDetails,
        },
      },
      Response: {
        status: 501,
        body: {
          error: {
            type: "invalid_request",
            message: "Setup Mandate flow for Trustpay is not implemented",
            code: "IR_00",
          },
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
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    MandateSingleUseNo3DSAutoCapture: {
      //Skipping this test as Trustpay does not support mandates and however setup future usage is downgraded to on_session
      Configs: {
        TRIGGER_SKIP: true,
      },
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
        body: {
          status: "succeeded",
        },
      },
    },
    MandateSingleUseNo3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
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
        body: {
          status: "requires_capture",
        },
      },
    },
    MandateMultiUseNo3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
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
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: multiUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    MandateMultiUse3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true,
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
          status: "requires_capture",
        },
      },
    },
    SaveCardUseNo3DSAutoCaptureOffSession: {
      Configs: {
        TRIGGER_SKIP: true,
      },
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
    SaveCardUseNo3DSManualCaptureOffSession: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    PaymentMethodIdMandateNo3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    PaymentMethodIdMandateNo3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    PaymentMethodIdMandate3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
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
        TRIGGER_SKIP: true,
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
    SessionToken: {
      Response: {
        status: 200,
        body: {
          session_token: [
            {
              wallet_name: "apple_pay",
              connector: "trustpay",
            },
            {
              wallet_name: "google_pay",
              connector: "trustpay",
            },
          ],
        },
      },
    },
  },
  bank_redirect_pm: {
    Ideal: {
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "ideal",
        payment_method_data: {
          bank_redirect: {
            ideal: {
              bank_name: "ing",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "San Fransico",
            state: "California",
            zip: "94122",
            country: "NL",
            first_name: "joseph",
            last_name: "Doe",
          },
          phone: {
            number: "9123456789",
            country_code: "+91",
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
    Giropay: {
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "giropay",
        payment_method_data: {
          bank_redirect: {
            giropay: {
              bank_name: "",
              bank_account_bic: "",
              bank_account_iban: "",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "San Fransico",
            state: "California",
            zip: "94122",
            country: "DE",
            first_name: "joseph",
            last_name: "Doe",
          },
          phone: {
            number: "9123456789",
            country_code: "+91",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_code: "1133001",
          error_message:
            "Giropay payments are not enabled in Project 4107608031",
        },
      },
    },
    Sofort: {
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "sofort",
        payment_method_data: {
          bank_redirect: {
            sofort: {},
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "San Fransico",
            state: "California",
            zip: "94122",
            country: "DE",
            first_name: "joseph",
            last_name: "Doe",
          },
          phone: {
            number: "9123456789",
            country_code: "+91",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_code: "1133001",
        },
      },
    },
    Eps: {
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "eps",
        payment_method_data: {
          bank_redirect: {
            eps: {
              bank_name: "ing",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "San Fransico",
            state: "California",
            zip: "94122",
            country: "AT",
            first_name: "joseph",
            last_name: "Doe",
          },
          phone: {
            number: "9123456789",
            country_code: "+91",
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
    Blik: {
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "blik",
        payment_method_data: {
          bank_redirect: {
            blik: {
              name: "John Doe",
              email: "example@email.com",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "San Fransico",
            state: "California",
            zip: "94122",
            country: "PL",
            first_name: "john",
            last_name: "doe",
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
  bank_transfer_pm: {
    InstantBankTransferFinland: getCustomExchange(
      {
        Response: {
          status: 200,
          body: {
            status: "requires_customer_action",
          },
        },
      },
      commonConnectorDetails.bank_transfer_pm.InstantBankTransferFinland
    ),
    InstantBankTransferPoland: getCustomExchange(
      {
        Response: {
          status: 200,
          body: {
            status: "requires_customer_action",
          },
        },
      },
      commonConnectorDetails.bank_transfer_pm.InstantBankTransferPoland
    ),
  },
};
