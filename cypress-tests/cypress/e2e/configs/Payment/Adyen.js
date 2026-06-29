import { customerAcceptance, multiUseMandateData } from "./Commons";
import {
  getCurrency,
  getCustomExchange,
  getIframeRedirectionConfig,
} from "./Modifiers";

const successfulNo3DSCardDetails = {
  card_number: "4111111111111111",
  card_exp_month: "03",
  card_exp_year: "30",
  card_holder_name: "John Doe",
  card_cvc: "737",
};

const successfulThreeDSTestCardDetails = {
  card_number: "4917610000000000",
  card_exp_month: "03",
  card_exp_year: "30",
  card_holder_name: "Joseph Doe",
  card_cvc: "737",
};

const failedNo3DSCardDetails = {
  card_number: "4242424242424242",
  card_exp_month: "01",
  card_exp_year: "35",
  card_holder_name: "joseph Doe",
  card_cvc: "123",
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

const voucherCurrencyMap = {
  Boleto: "BRL",
  Oxxo: "MXN",
  Alfamart: "IDR",
  Indomaret: "IDR",
  SevenEleven: "JPY",
  Lawson: "JPY",
  MiniStop: "JPY",
  FamilyMart: "JPY",
  Seicomart: "JPY",
  PayEasy: "JPY",
};

const mandateBrowserInfo = {
  user_agent:
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/70.0.3538.110 Safari/537.36",
  accept_header:
    "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8",
  language: "nl-NL",
  color_depth: 24,
  screen_height: 723,
  screen_width: 1536,
  time_zone: 0,
  java_enabled: true,
  java_script_enabled: true,
  ip_address: "127.0.0.1",
};

const getMandateData = (currency) => ({
  customer_acceptance: {
    acceptance_type: "online",
    accepted_at: "2025-01-01T00:00:00.000Z",
    online: {
      ip_address: "127.0.0.1",
      user_agent: "Mozilla/5.0",
    },
  },
  mandate_type: {
    multi_use: {
      amount: 6540,
      currency,
    },
  },
});

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
    "3DSManualCapture": {
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
    ...getIframeRedirectionConfig({
      cardDetails: successfulThreeDSTestCardDetails,
    }),
    No3DSManualCapture: {
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
          status: "requires_capture",
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
          error_code: "2",
          error_message: "Refused",
          unified_code: "UE_3000",
          unified_message: "Technical issue with PSP",
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
          status: "processing",
          amount: 6000,
          amount_capturable: 6000,
          amount_received: null,
        },
      },
    },
    Overcapture: {
      Request: {
        amount_to_capture: 7000,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
          amount: 6000,
          amount_capturable: 6000,
          amount_received: null, // Amount is updated via webhooks
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
          status: "processing",
          amount: 6000,
          amount_capturable: 6000,
          amount_received: null,
        },
      },
    },
    MultipleCapture: {
      Request: {
        amount_to_capture: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    MultipleCapturePartial: {
      Request: {
        amount_to_capture: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    MultipleCaptureFinal: {
      Request: {
        amount_to_capture: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    MultipleCaptureRetrieve: {
      Response: {
        status: 200,
        body: {
          status: "processing",
          amount: 6000,
          amount_capturable: 0,
          amount_received: 6000,
        },
      },
    },
    MultipleCaptureOvercapture: {
      Request: {
        amount_to_capture: 7000,
      },
      Response: {
        status: 422,
        body: {
          error: {
            type: "invalid_request",
            message: "amount_to_capture is greater than amount",
            code: "IR_06",
          },
        },
      },
    },
    VoidAfterConfirm: getCustomExchange({
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
      ResponseCustom: {
        status: 200,
        body: {
          status: "cancelled",
        },
      },
    }),
    Refund: {
      Request: {
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
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
          status: "pending",
        },
      },
    },
    manualPaymentRefund: {
      Request: {
        amount: 6000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "This Payment could not be refund because it has a status of processing. The expected state is succeeded, partially_captured",
            code: "IR_14",
          },
        },
      },
    },
    manualPaymentPartialRefund: {
      Request: {
        amount: 2000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "This Payment could not be refund because it has a status of processing. The expected state is succeeded, partially_captured",
            code: "IR_14",
          },
        },
      },
    },
    SyncRefund: {
      Response: {
        status: 200,
        body: {
          status: "pending",
        },
      },
    },
    SyncRefundScheduled: {
      Response: {
        status: 200,
        body: {
          status: "pending",
        },
      },
    },
    RefundInstant: {
      Request: {
        amount: 6000,
        refund_type: "instant",
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
        },
      },
    },
    RefundScheduled: {
      Request: {
        amount: 6000,
        refund_type: "scheduled",
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
        },
      },
    },
    ExtendAuthorizationNo3DSManual: {
      Request: {
        extended_authorization_days: 7,
      },
      Response: {
        status: 200,
        body: {
          status: "processing", // Adyen: Extend Authorization is async, returns processing
          amount: 6000,
          amount_capturable: 6000,
          amount_received: null,
          request_extended_authorization: true,
        },
      },
      // Adyen: Extend Authorization is async (processing), capture is skipped
    },
    ExtendAuthorizationInvalidStatus: {
      Request: {
        extended_authorization_days: 7,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "You cannot extend authorization this payment because it has status succeeded",
            code: "IR_16",
          },
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
          status: "succeeded",
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
          status: "requires_capture",
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
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
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
        mandate_data: null,
        customer_acceptance: customerAcceptance,
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
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    ManualRetryPaymentDisabled: {
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
        status: 400,
        body: {
          type: "invalid_request",
          message:
            "You cannot confirm this payment because it has status failed, you can enable `manual_retry` in profile to try this payment again",
          code: "IR_16",
        },
      },
    },
    ManualRetryPaymentEnabled: {
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
          payment_method: "card",
          attempt_count: 2,
        },
      },
    },
    ManualRetryPaymentCutoffExpired: {
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
        status: 400,
        body: {
          type: "invalid_request",
          message:
            "You cannot confirm this payment using `manual_retry` because the allowed duration has expired",
          code: "IR_16",
        },
      },
    },
    PaymentIntentWithInstallments: {
      Request: {
        amount: 6000,
        currency: "BRL",
        installment_options: [
          {
            payment_method: "card",
            installments: [
              {
                number_of_installments: [3, 6, 12],
                billing_frequency: "month",
                interest_rate: 5.0,
              },
            ],
          },
        ],
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          amount: 6000,
          currency: "BRL",
        },
      },
    },
    CardInstallmentConfirm: {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        installment_data: {
          number_of_installments: 3,
          billing_frequency: "month",
        },
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          net_amount: 6610,
        },
      },
    },
    PaymentIntentWithInstallmentsAndConfirmTrue: {
      Request: {
        currency: "BRL",
        confirm: true,
        installment_options: [
          {
            payment_method: "card",
            installments: [
              {
                number_of_installments: [3, 6, 12],
                billing_frequency: "month",
                interest_rate: 5.0,
              },
            ],
          },
        ],
      },
      Response: {
        status: 422,
        body: {
          error: {
            type: "invalid_request",
            message:
              "installment_options and installment_data are not supported when confirm is true.",
            code: "IR_06",
          },
        },
      },
    },
    PaymentIntentWithBillingDescriptor: {
      Request: {
        currency: "USD",
        billing_descriptor: {
          name: "Juspay",
          city: "San Francisco",
          phone: "8056594427",
          statement_descriptor: "QA-BillingDesc",
          statement_descriptor_suffix: "SUFFIX1",
          reference: "ref-qa-001",
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    PaymentConfirmWithBillingDescriptor: {
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
          billing_descriptor: {
            name: "Juspay",
            city: "San Francisco",
            phone: "8056594427",
            statement_descriptor: "QA-BillingDesc",
            statement_descriptor_suffix: "SUFFIX1",
            reference: "ref-qa-001",
          },
        },
      },
    },
    PartnerMerchantIdentifier: {
      Request: {
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "San Francisco",
            state: "California",
            zip: "94122",
            country: "US",
            first_name: "joseph",
            last_name: "Doe",
          },
        },
        partner_merchant_identifier_details: {
          partner_details: {
            name: "TestPartner",
            version: "1.0.0",
            integrator: "TestIntegrator123",
          },
          merchant_details: {
            name: "TestMerchantApp",
            version: "2.0.0",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          partner_merchant_identifier_details: {
            partner_details: {
              name: "TestPartner",
              version: "1.0.0",
              integrator: "TestIntegrator123",
            },
            merchant_details: {
              name: "TestMerchantApp",
              version: "2.0.0",
            },
          },
        },
      },
    },
    PartnerMerchantIdentifierConfirm: {
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
        },
      },
    },
    ConnectorTestingData: {
      Request: {
        currency: "USD",
        connector_metadata: {
          adyen: {
            testing: {
              holder_name: "Test Holder Name Override",
            },
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    ConnectorTestingDataConfirm: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: {
            card_number: "4111111111111111",
            card_exp_month: "12",
            card_exp_year: "2030",
            card_cvc: "123",
            card_holder_name: "Original Card Holder",
          },
        },
        connector_metadata: {
          adyen: {
            testing: {
              holder_name: "Test Holder Name Override",
            },
          },
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
        },
      },
    },
  },
  bank_transfer_pm: {
    Pix: {
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "pix",
        payment_method_data: {
          bank_transfer: {
            pix: {},
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
            country: "BR",
            first_name: "joseph",
            last_name: "Doe",
          },
          phone: {
            number: "9123456789",
            country_code: "+91",
          },
        },
        currency: "BRL",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
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
            country_code: "+31",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
      MandateSingleUseAutoCapture: {
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
          browser_info: mandateBrowserInfo,
          currency: "EUR",
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
              country_code: "+31",
            },
          },
          mandate_data: getMandateData("EUR"),
          payment_type: "new_mandate",
          setup_future_usage: "off_session",
        },
        Response: {
          status: 200,
          body: {
            status: "requires_customer_action",
          },
        },
      },
    },
    BancontactCard: {
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "bancontact_card",
        payment_method_data: {
          bank_redirect: {
            bancontact_card: {
              card_number: "6703444444444449",
              card_exp_month: "03",
              card_exp_year: "2030",
            },
          },
        },
        currency: "EUR",
        billing: {
          address: {
            line1: "1 Main St",
            line2: "Apt 4",
            city: "Brussels",
            zip: "1000",
            country: "BE",
            first_name: "John",
            last_name: "Doe",
          },
          email: "test@example.com",
          phone: {
            number: "9123456789",
            country_code: "+32",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
      MandateSingleUseAutoCapture: {
        Request: {
          payment_method: "bank_redirect",
          payment_method_type: "bancontact_card",
          payment_method_data: {
            bank_redirect: {
              bancontact_card: {
                card_number: "6703444444444449",
                card_exp_month: "03",
                card_exp_year: "2030",
              },
            },
          },
          browser_info: mandateBrowserInfo,
          currency: "EUR",
          billing: {
            address: {
              line1: "1 Main St",
              line2: "Apt 4",
              city: "Brussels",
              zip: "1000",
              country: "BE",
              first_name: "John",
              last_name: "Doe",
            },
            email: "test@example.com",
            phone: {
              number: "9123456789",
              country_code: "+32",
            },
          },
          mandate_data: getMandateData("EUR"),
          payment_type: "new_mandate",
          setup_future_usage: "off_session",
        },
        Response: {
          status: 200,
          body: {
            status: "requires_customer_action",
          },
        },
      },
    },
    OpenBankingUk: {
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "open_banking_uk",
        payment_method_data: {
          bank_redirect: {
            open_banking_uk: {
              issuer: "lloyds",
            },
          },
        },
        currency: "GBP",
        billing: {
          address: {
            line1: "1 Main St",
            city: "London",
            zip: "SW1A 1AA",
            country: "GB",
            first_name: "John",
            last_name: "Doe",
          },
          email: "test@example.com",
          phone: {
            number: "9123456789",
            country_code: "+44",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
      MandateSingleUseAutoCapture: {
        Request: {
          payment_method: "bank_redirect",
          payment_method_type: "open_banking_uk",
          payment_method_data: {
            bank_redirect: {
              open_banking_uk: {
                issuer: "lloyds",
              },
            },
          },
          browser_info: mandateBrowserInfo,
          currency: "GBP",
          billing: {
            address: {
              line1: "1 Main St",
              city: "London",
              zip: "SW1A 1AA",
              country: "GB",
              first_name: "John",
              last_name: "Doe",
            },
            email: "test@example.com",
            phone: {
              number: "9123456789",
              country_code: "+44",
            },
          },
          mandate_data: getMandateData("GBP"),
          payment_type: "new_mandate",
          setup_future_usage: "off_session",
        },
        Response: {
          status: 200,
          body: {
            status: "requires_customer_action",
          },
        },
      },
    },
    Trustly: {
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "trustly",
        payment_method_data: {
          bank_redirect: {
            trustly: {
              country: "SE",
            },
          },
        },
        currency: "EUR",
        billing: {
          address: {
            line1: "1 Main St",
            city: "Stockholm",
            zip: "11122",
            country: "SE",
            first_name: "John",
            last_name: "Doe",
          },
          email: "test@example.com",
          phone: {
            number: "9123456789",
            country_code: "+46",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
      MandateSingleUseAutoCapture: {
        Request: {
          payment_method: "bank_redirect",
          payment_method_type: "trustly",
          payment_method_data: {
            bank_redirect: {
              trustly: {
                country: "SE",
              },
            },
          },
          browser_info: mandateBrowserInfo,
          currency: "EUR",
          billing: {
            address: {
              line1: "1 Main St",
              city: "Stockholm",
              zip: "11122",
              country: "SE",
              first_name: "John",
              last_name: "Doe",
            },
            email: "test@example.com",
            phone: {
              number: "9123456789",
              country_code: "+46",
            },
          },
          mandate_data: getMandateData("EUR"),
          payment_type: "new_mandate",
          setup_future_usage: "off_session",
        },
        Response: {
          status: 200,
          body: {
            status: "requires_customer_action",
          },
        },
        Configs: {
          TRIGGER_SKIP: true,
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
              blik_code: "777987",
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
          status: "processing",
        },
      },
    },
  },

  upi_pm: {
    PaymentIntent: {
      Request: {
        currency: "INR",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    UpiCollect: {
      Request: {
        payment_method: "upi",
        payment_method_type: "upi_collect",
        payment_method_data: {
          upi: {
            upi_collect: {
              vpa_id: "successtest@iata",
            },
          },
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "HE_03",
            reason: "automatic for upi_collect is not supported by adyen",
          },
        },
      },
    },
    UpiIntent: {
      Request: {
        payment_method: "upi",
        payment_method_type: "upi_intent",
        payment_method_data: {
          upi: {
            upi_intent: {},
          },
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "HE_03",
            reason: "automatic for upi_intent is not supported by adyen",
          },
        },
      },
    },
  },
  wallet_pm: {
    PaymentIntent: (paymentMethodType) =>
      getCustomExchange({
        Request: {
          currency: getCurrency(paymentMethodType),
        },
        Response: {
          status: 200,
          body: {
            status: "requires_payment_method",
          },
        },
      }),
    AliPayHk: getCustomExchange({
      Request: {
        payment_method: "wallet",
        payment_method_type: "ali_pay_hk",
        payment_method_data: {
          wallet: {
            ali_pay_hk_redirect: {},
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "Hong Kong",
            state: "HK",
            zip: "999077",
            country: "HK",
            first_name: "joseph",
            last_name: "Doe",
          },
          phone: {
            number: "9123456789",
            country_code: "+852",
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
    PaypalWalletMandateCIT: getCustomExchange({
      Request: {
        payment_method: "wallet",
        payment_method_type: "paypal",
        payment_method_data: {
          wallet: {
            paypal_redirect: {},
          },
        },
        customer_acceptance: customerAcceptance,
        mandate_data: getMandateData("USD"),
        setup_future_usage: "off_session",
        currency: "USD",
        return_url: "https://example.com",
        billing: {
          email: "test@example.com",
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method_status: null,
        },
      },
    }),
    KakaoPayWalletMandateCIT: getCustomExchange({
      Configs: {
        // KakaoPay redirects to the KakaoPay mobile app; cannot be completed in a headless CI browser
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "wallet",
        payment_method_type: "kakao_pay",
        payment_method_data: {
          wallet: {
            kakao_pay_redirect: {},
          },
        },
        customer_acceptance: customerAcceptance,
        mandate_data: getMandateData("KRW"),
        setup_future_usage: "off_session",
        currency: "KRW",
        return_url: "https://example.com",
        billing: {
          email: "test@example.com",
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method_status: null,
        },
      },
    }),
    GcashWalletMandateCIT: getCustomExchange({
      Request: {
        payment_method: "wallet",
        payment_method_type: "gcash",
        payment_method_data: {
          wallet: {
            gcash_redirect: {},
          },
        },
        customer_acceptance: customerAcceptance,
        mandate_data: getMandateData("PHP"),
        setup_future_usage: "off_session",
        currency: "PHP",
        return_url: "https://example.com",
        billing: {
          email: "test@example.com",
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method_status: null,
        },
      },
    }),
    TwintWalletMandateCIT: getCustomExchange({
      Configs: {
        // Twint uses a QR-code scanned via a Swiss banking app; cannot be automated in a headless CI browser
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "wallet",
        payment_method_type: "twint",
        payment_method_data: {
          wallet: {
            twint_redirect: {},
          },
        },
        customer_acceptance: customerAcceptance,
        mandate_data: getMandateData("CHF"),
        setup_future_usage: "off_session",
        currency: "CHF",
        return_url: "https://example.com",
        billing: {
          email: "test@example.com",
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method_status: null,
        },
      },
    }),
    DanaWalletMandateCIT: getCustomExchange({
      Request: {
        payment_method: "wallet",
        payment_method_type: "dana",
        payment_method_data: {
          wallet: {
            dana_redirect: {},
          },
        },
        customer_acceptance: customerAcceptance,
        mandate_data: getMandateData("IDR"),
        setup_future_usage: "off_session",
        currency: "IDR",
        return_url: "https://example.com",
        billing: {
          email: "test@example.com",
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method_status: null,
        },
      },
    }),
    GoPayWalletMandateCIT: getCustomExchange({
      Request: {
        payment_method: "wallet",
        payment_method_type: "go_pay",
        payment_method_data: {
          wallet: {
            go_pay_redirect: {},
          },
        },
        browser_info: mandateBrowserInfo,
        customer_acceptance: customerAcceptance,
        mandate_data: getMandateData("IDR"),
        setup_future_usage: "off_session",
        currency: "IDR",
        return_url: "https://example.com",
        billing: {
          email: "test@example.com",
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method_status: null,
        },
      },
    }),
  },

  gift_card_pm: {
    GivexGiftCard: getCustomExchange({
      Request: {
        payment_method: "gift_card",
        payment_method_type: "givex",
        payment_method_data: {
          gift_card: {
            givex: {
              number: "6006490000000000",
              cvc: "737",
            },
          },
        },
        amount: 1100,
        currency: "EUR",
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),
    GivexGiftCardInsufficientBalance: getCustomExchange({
      Request: {
        payment_method: "gift_card",
        payment_method_type: "givex",
        payment_method_data: {
          gift_card: {
            givex: {
              number: "6006490000000000",
              cvc: "737",
            },
          },
        },
        amount: 14100,
        currency: "EUR",
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_message: "Insufficient balance in the payment method",
        },
      },
    }),
    GivexGiftCardCurrencyMismatch: getCustomExchange({
      Request: {
        payment_method: "gift_card",
        payment_method_type: "givex",
        payment_method_data: {
          gift_card: {
            givex: {
              number: "6006490000000000",
              cvc: "737",
            },
          },
        },
        currency: "USD",
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_message:
            "Payment Method currency does not match the payment currency",
        },
      },
    }),
    PaymentIntent: () => {
      return getCustomExchange({
        Request: {
          currency: "EUR",
        },
        Response: {
          status: 200,
          body: {
            status: "requires_payment_method",
          },
        },
      });
    },
    PaySafeCardGiftCard: getCustomExchange({
      Request: {
        payment_method: "gift_card",
        payment_method_type: "pay_safe_card",
        payment_method_data: {
          gift_card: {
            pay_safe_card: {},
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "San Francisco",
            state: "California",
            zip: "94122",
            country: "US",
            first_name: "joseph",
            last_name: "Doe",
          },
        },
        return_url: "https://example.com",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
    GivexGiftCardRefund: {
      Request: {
        amount: 1000,
        reason: "Test refund",
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
        },
      },
    },
    GivexGiftCardSyncRefund: {
      Response: {
        status: 200,
        body: {
          status: "pending",
        },
      },
    },
  },

  pay_later_pm: {
    Klarna: getCustomExchange({
      Request: {
        payment_method: "pay_later",
        payment_method_type: "klarna",
        payment_experience: "redirect_to_url",
        payment_method_data: {
          pay_later: {
            klarna_redirect: {
              billing_email: "guest@juspay.in",
              billing_country: "DE",
            },
          },
        },
        billing: {
          email: "guest@juspay.in",
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
        },
        order_details: [
          {
            product_name: "Test Product",
            quantity: 1,
            amount: 6000,
          },
        ],
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
    AutoCapture: getCustomExchange({
      Request: {
        currency: "EUR",
        capture_method: "automatic",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
    ManualCapture: getCustomExchange({
      Request: {
        currency: "EUR",
        capture_method: "manual",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
    PaymentIntent: getCustomExchange({
      Request: {
        currency: "USD",
        capture_method: "manual",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
    Refund: getCustomExchange({
      Request: {
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
        },
      },
    }),
    PartialRefund: getCustomExchange({
      Request: {
        amount: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
        },
      },
    }),
    SyncRefund: getCustomExchange({}),
    Capture: getCustomExchange({
      Request: {
        amount_to_capture: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
          amount: 6000,
          amount_capturable: 6000,
          amount_received: null,
        },
      },
    }),
    AtomeAutoCapture: getCustomExchange({
      Request: {
        currency: "SGD",
        capture_method: "automatic",
        customer_acceptance: {
          acceptance_type: "online",
        },
        order_details: [
          {
            product_name: "Test Product",
            quantity: 1,
            amount: 6000,
          },
        ],
        billing: {
          address: {
            line1: "123 Test Street",
            line2: "Unit 4",
            city: "Singapore",
            state: "Singapore",
            zip: "018956",
            country: "SG",
            first_name: "John",
            last_name: "Doe",
          },
          phone: {
            number: "91234567",
            country_code: "+65",
          },
          email: "test@test.com",
        },
        shipping: {
          address: {
            line1: "123 Test Street",
            line2: "Unit 4",
            city: "Singapore",
            state: "Singapore",
            zip: "018956",
            country: "SG",
            first_name: "John",
            last_name: "Doe",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
    Atome: getCustomExchange({
      Request: {
        payment_method: "pay_later",
        payment_method_type: "atome",
        payment_experience: "redirect_to_url",
        payment_method_data: {
          pay_later: {
            atome_redirect: {},
          },
        },
        customer_acceptance: {
          acceptance_type: "online",
        },
        billing: {
          address: {
            line1: "123 Test Street",
            line2: "Unit 4",
            city: "Singapore",
            state: "Singapore",
            zip: "018956",
            country: "SG",
            first_name: "John",
            last_name: "Doe",
          },
          phone: {
            number: "91234567",
            country_code: "+65",
          },
          email: "test@test.com",
        },
        shipping: {
          address: {
            line1: "123 Test Street",
            line2: "Unit 4",
            city: "Singapore",
            state: "Singapore",
            zip: "018956",
            country: "SG",
            first_name: "John",
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
    CaptureOnWrongStatus: getCustomExchange({
      Request: {
        amount_to_capture: 6000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "This Payment could not be captured because it has a payment.status of requires_customer_action. The expected state is requires_capture, partially_captured_and_capturable, processing",
            code: "IR_14",
          },
        },
      },
    }),
    ConfirmWithoutPmData: getCustomExchange({
      Request: {
        payment_method: undefined,
        payment_method_type: undefined,
        payment_experience: undefined,
        payment_method_data: undefined,
        order_details: undefined,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            code: "IR_06",
          },
        },
      },
    }),
  },

  voucher_pm: {
    PaymentIntent: (paymentMethodType) => {
      return {
        Request: {
          currency: voucherCurrencyMap[paymentMethodType] || "USD",
        },
        Response: {
          status: 200,
          body: {
            status: "requires_payment_method",
          },
        },
      };
    },
    Boleto: getCustomExchange({
      Request: {
        payment_method: "voucher",
        payment_method_type: "boleto",
        payment_method_data: {
          voucher: {
            boleto: {
              social_security_number: "12345678909",
              document_type: "cpf",
            },
          },
        },
        billing: {
          address: {
            line1: "Rua Test 123",
            city: "Sao Paulo",
            state: "SP",
            zip: "01310100",
            country: "BR",
            first_name: "Test",
            last_name: "User",
          },
          phone: {
            number: "11987654321",
            country_code: "+55",
          },
          email: "test@example.com",
        },
        currency: voucherCurrencyMap.Boleto,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method: "voucher",
          payment_method_type: "boleto",
        },
      },
    }),
    Oxxo: getCustomExchange({
      Request: {
        payment_method: "voucher",
        payment_method_type: "oxxo",
        payment_method_data: {
          voucher: {
            oxxo: null,
          },
        },
        billing: {
          address: {
            line1: "123 Test St",
            city: "Mexico City",
            state: "Mexico",
            zip: "06600",
            country: "MX",
            first_name: "Test",
            last_name: "User",
          },
          email: "test@example.com",
        },
        currency: voucherCurrencyMap.Oxxo,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method: "voucher",
          payment_method_type: "oxxo",
        },
      },
    }),
    Alfamart: getCustomExchange({
      Request: {
        payment_method: "voucher",
        payment_method_type: "alfamart",
        payment_method_data: {
          voucher: {
            alfamart: {
              first_name: "Test",
              last_name: "User",
              email: "test@example.com",
            },
          },
        },
        billing: {
          address: {
            line1: "Jl Test 123",
            city: "Jakarta",
            state: "DKI Jakarta",
            zip: "10110",
            country: "ID",
            first_name: "Test",
            last_name: "User",
          },
          email: "test@example.com",
        },
        currency: voucherCurrencyMap.Alfamart,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method: "voucher",
          payment_method_type: "alfamart",
        },
      },
    }),
    Indomaret: getCustomExchange({
      Request: {
        payment_method: "voucher",
        payment_method_type: "indomaret",
        payment_method_data: {
          voucher: {
            indomaret: {
              first_name: "Test",
              last_name: "User",
              email: "test@example.com",
            },
          },
        },
        billing: {
          address: {
            line1: "Jl Test 123",
            city: "Jakarta",
            state: "DKI Jakarta",
            zip: "10110",
            country: "ID",
            first_name: "Test",
            last_name: "User",
          },
          email: "test@example.com",
        },
        currency: voucherCurrencyMap.Indomaret,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method: "voucher",
          payment_method_type: "indomaret",
        },
      },
    }),
    SevenEleven: getCustomExchange({
      Request: {
        payment_method: "voucher",
        payment_method_type: "seven_eleven",
        payment_method_data: {
          voucher: {
            seven_eleven: {
              first_name: "Test",
              last_name: "User",
              email: "test@example.com",
            },
          },
        },
        billing: {
          address: {
            line1: "1-1 Test",
            city: "Tokyo",
            state: "Tokyo",
            zip: "1000001",
            country: "JP",
            first_name: "Test",
            last_name: "User",
          },
          phone: {
            number: "312345678",
            country_code: "+81",
          },
          email: "test@example.com",
        },
        currency: voucherCurrencyMap.SevenEleven,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method: "voucher",
          payment_method_type: "seven_eleven",
        },
      },
    }),
    Lawson: getCustomExchange({
      Request: {
        payment_method: "voucher",
        payment_method_type: "lawson",
        payment_method_data: {
          voucher: {
            lawson: {
              first_name: "Test",
              last_name: "User",
              email: "test@example.com",
            },
          },
        },
        billing: {
          address: {
            line1: "1-1 Test",
            city: "Tokyo",
            state: "Tokyo",
            zip: "1000001",
            country: "JP",
            first_name: "Test",
            last_name: "User",
          },
          phone: {
            number: "312345678",
            country_code: "+81",
          },
          email: "test@example.com",
        },
        currency: voucherCurrencyMap.Lawson,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method: "voucher",
          payment_method_type: "lawson",
        },
      },
    }),
    MiniStop: getCustomExchange({
      Request: {
        payment_method: "voucher",
        payment_method_type: "mini_stop",
        payment_method_data: {
          voucher: {
            mini_stop: {
              first_name: "Test",
              last_name: "User",
              email: "test@example.com",
            },
          },
        },
        billing: {
          address: {
            line1: "1-1 Test",
            city: "Tokyo",
            state: "Tokyo",
            zip: "1000001",
            country: "JP",
            first_name: "Test",
            last_name: "User",
          },
          phone: {
            number: "312345678",
            country_code: "+81",
          },
          email: "test@example.com",
        },
        currency: voucherCurrencyMap.MiniStop,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method: "voucher",
          payment_method_type: "mini_stop",
        },
      },
    }),
    FamilyMart: getCustomExchange({
      Request: {
        payment_method: "voucher",
        payment_method_type: "family_mart",
        payment_method_data: {
          voucher: {
            family_mart: {
              first_name: "Test",
              last_name: "User",
              email: "test@example.com",
            },
          },
        },
        billing: {
          address: {
            line1: "1-1 Test",
            city: "Tokyo",
            state: "Tokyo",
            zip: "1000001",
            country: "JP",
            first_name: "Test",
            last_name: "User",
          },
          phone: {
            number: "312345678",
            country_code: "+81",
          },
          email: "test@example.com",
        },
        currency: voucherCurrencyMap.FamilyMart,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method: "voucher",
          payment_method_type: "family_mart",
        },
      },
    }),
    Seicomart: getCustomExchange({
      Request: {
        payment_method: "voucher",
        payment_method_type: "seicomart",
        payment_method_data: {
          voucher: {
            seicomart: {
              first_name: "Test",
              last_name: "User",
              email: "test@example.com",
            },
          },
        },
        billing: {
          address: {
            line1: "1-1 Test",
            city: "Tokyo",
            state: "Tokyo",
            zip: "1000001",
            country: "JP",
            first_name: "Test",
            last_name: "User",
          },
          phone: {
            number: "312345678",
            country_code: "+81",
          },
          email: "test@example.com",
        },
        currency: voucherCurrencyMap.Seicomart,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method: "voucher",
          payment_method_type: "seicomart",
        },
      },
    }),
    PayEasy: getCustomExchange({
      Request: {
        payment_method: "voucher",
        payment_method_type: "pay_easy",
        payment_method_data: {
          voucher: {
            pay_easy: {
              first_name: "Test",
              last_name: "User",
              email: "test@example.com",
            },
          },
        },
        billing: {
          address: {
            line1: "1-1 Test",
            city: "Tokyo",
            state: "Tokyo",
            zip: "1000001",
            country: "JP",
            first_name: "Test",
            last_name: "User",
          },
          phone: {
            number: "312345678",
            country_code: "+81",
          },
          email: "test@example.com",
        },
        currency: voucherCurrencyMap.PayEasy,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method: "voucher",
          payment_method_type: "pay_easy",
        },
      },
    }),
    OxxoInvalidFormat: getCustomExchange({
      Request: {
        payment_method: "voucher",
        payment_method_type: "oxxo",
        payment_method_data: {
          voucher: {
            oxxo: "oxxo",
          },
        },
        billing: {
          address: {
            line1: "123 Test St",
            city: "Mexico City",
            state: "Mexico",
            zip: "06600",
            country: "MX",
            first_name: "Test",
            last_name: "User",
          },
          email: "test@example.com",
        },
        currency: voucherCurrencyMap.Oxxo,
      },
      Response: {
        status: 400,
        body: {
          error: {
            message: 'invalid type: string "oxxo", expected unit',
            code: "IR_06",
          },
        },
      },
    }),
  },

  pm_list: {
    PmListResponse: {
      PmListNull: {
        payment_methods: [],
      },
      PmListWithInstallmentsNull: {
        intent_data: {
          status: "requires_payment_method",
          amount: 6000,
          currency: "USD",
          installment_options: null,
        },
      },
      PmListWithInstallmentsBRL: {
        intent_data: {
          status: "requires_payment_method",
          amount: 6000,
          currency: "BRL",
          installment_options: [
            {
              payment_method: "card",
              available_plans: [
                {
                  number_of_installments: 3,
                  billing_frequency: "month",
                  interest_rate: 5,
                  amount_details: {
                    amount_per_installment: 22.04,
                    total_amount: 66.1,
                  },
                },
                {
                  number_of_installments: 6,
                  billing_frequency: "month",
                  interest_rate: 5,
                  amount_details: {
                    amount_per_installment: 11.83,
                    total_amount: 70.93,
                  },
                },
                {
                  number_of_installments: 12,
                  billing_frequency: "month",
                  interest_rate: 5,
                  amount_details: {
                    amount_per_installment: 6.77,
                    total_amount: 81.24,
                  },
                },
              ],
            },
          ],
        },
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
                    eligible_connectors: ["adyen"],
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
                    eligible_connectors: ["adyen"],
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
                    eligible_connectors: ["adyen"],
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
                    eligible_connectors: ["adyen"],
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

  bank_debit_pm: {
    PaymentIntent: (paymentMethodType) => {
      if (paymentMethodType === "Ach") {
        return {
          Configs: {
            TRIGGER_SKIP: true,
          },
          Request: {
            currency: "USD",
            setup_future_usage: "off_session",
          },
          Response: {
            status: 200,
            body: {
              status: "requires_payment_method",
            },
          },
        };
      }
      if (paymentMethodType === "Sepa") {
        return {
          Request: {
            currency: "EUR",
          },
          Response: {
            status: 200,
            body: {
              status: "requires_payment_method",
            },
          },
        };
      }
      const currencyMap = {
        Becs: "AUD",
        Bacs: "GBP",
      };
      return {
        Request: {
          currency: currencyMap[paymentMethodType] || "USD",
          setup_future_usage: "off_session",
        },
        Response: {
          status: 200,
          body: {
            status: "requires_payment_method",
          },
        },
      };
    },
    Sepa: {
      Request: {
        payment_method: "bank_debit",
        payment_method_type: "sepa",
        payment_method_data: {
          bank_debit: {
            sepa_bank_debit: {
              iban: "DE89370400440532013000",
              bank_account_holder_name: "John Doe",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "Amsterdam",
            state: "North Holland",
            zip: "1012",
            country: "NL",
            first_name: "John",
            last_name: "Doe",
          },
        },
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    Ach: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "bank_debit",
        payment_method_type: "ach",
        payment_method_data: {
          bank_debit: {
            ach_bank_debit: {
              account_number: "000123456789",
              routing_number: "121000358",
              bank_type: "checking",
              bank_account_holder_name: "John Doe",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "San Francisco",
            state: "California",
            zip: "94122",
            country: "US",
            first_name: "John",
            last_name: "Doe",
          },
        },
        customer_acceptance: customerAcceptance,
        mandate_data: {
          customer_acceptance: customerAcceptance,
          mandate_type: {
            multi_use: {
              amount: 8000,
              currency: "USD",
            },
          },
        },
        payment_type: "new_mandate",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    Bacs: {
      Request: {
        payment_method: "bank_debit",
        payment_method_type: "bacs",
        payment_method_data: {
          bank_debit: {
            bacs_bank_debit: {
              account_number: "09083055",
              sort_code: "560036",
              bank_account_holder_name: "David Archer",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "London",
            state: "England",
            zip: "SW1A 1AA",
            country: "GB",
            first_name: "John",
            last_name: "Doe",
          },
        },
        customer_acceptance: customerAcceptance,
        mandate_data: {
          customer_acceptance: customerAcceptance,
          mandate_type: {
            multi_use: {
              amount: 8000,
              currency: "GBP",
            },
          },
        },
        payment_type: "new_mandate",
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    Becs: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "bank_debit",
        payment_method_type: "becs",
        payment_method_data: {
          bank_debit: {
            becs_bank_debit: {
              account_number: "000123456",
              bsb_number: "000000",
              bank_account_holder_name: "John Doe",
            },
          },
        },
        currency: "AUD",
        customer_acceptance: customerAcceptance,
        setup_future_usage: "off_session",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Selected payment method through Adyen is not implemented",
            code: "IR_00",
          },
        },
      },
    },
  },
};
