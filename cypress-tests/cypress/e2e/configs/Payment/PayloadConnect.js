import {
  connectorDetails as commonConnectorDetails,
  customerAcceptance,
  multiUseMandateData,
  singleUseMandateData,
} from "./Commons";
import { getCustomExchange } from "./Modifiers";

const DUPLICATION_TIMEOUT = 30000; // 30 seconds

const successfulNo3DSCardDetails = {
  card_number: "4242424242424242",
  card_exp_month: "12",
  card_exp_year: "30",
  card_holder_name: "John Doe",
  card_cvc: "123",
};

const successfulThreeDSTestCardDetails = {
  ...successfulNo3DSCardDetails,
};

const billingDescriptor = {
  name: "Test Business",
  city: "San Francisco",
  phone: "1234567890",
  statement_descriptor: "Test Descriptor",
  statement_descriptor_suffix: "Suffix",
  reference: "REF123",
};

const threeDSNotSupportedError = {
  type: "invalid_request",
  message: "3DS authentication is not supported by Payload",
  code: "IR_00",
};

// Payload Connect Split Payments Configuration
const payloadSplitPaymentData = {
  payload_split_payment: {
    ledger: [
      {
        receiver_id: "acct_3eoxafCHioIB3jNMKJev4",
        amount: 5000,
      },
      {
        receiver_id: "acct_3epxuNyAtNd77zShIaaL1",
        amount: 1000,
      },
    ],
  },
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        split_payments: payloadSplitPaymentData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          setup_future_usage: "on_session",
        },
      },
    },
    PaymentIntentOffSession: {
      Request: {
        currency: "USD",
        customer_acceptance: null,
        amount: 6000,
        authentication_type: "no_three_ds",
        setup_future_usage: "off_session",
        split_payments: payloadSplitPaymentData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          setup_future_usage: "off_session",
        },
      },
    },
    // NOTE: split_payments is intentionally omitted here -- the split
    // ledger (payloadSplitPaymentData) sums to 6000, which does not cover
    // the full 6050 order total once shipping_cost is added, and combining
    // the two produced a live "amount" error_message from payload. Revisit
    // if/when a shipping-cost-aware ledger is verified to work.
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
    PaymentIntentWithBillingDescriptor: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: DUPLICATION_TIMEOUT,
        },
      },
      Request: {
        currency: "USD",
        billing_descriptor: billingDescriptor,
        split_payments: payloadSplitPaymentData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    PaymentConfirmWithBillingDescriptor: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: DUPLICATION_TIMEOUT,
        },
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        billing_descriptor: billingDescriptor,
        split_payments: payloadSplitPaymentData,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          amount_received: 6000,
          split_payments: payloadSplitPaymentData,
        },
      },
    },
    "3DSManualCapture": {
      Configs: {
        TRIGGER_SKIP: true,
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
        status: 400,
        body: {
          error: threeDSNotSupportedError,
        },
      },
    },
    "3DSAutoCapture": {
      Configs: {
        TRIGGER_SKIP: true,
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
        status: 400,
        body: {
          error: threeDSNotSupportedError,
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
        split_payments: payloadSplitPaymentData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture", // Manual capture should require explicit capture
          payment_method: "card",
          attempt_count: 1,
          split_payments: payloadSplitPaymentData,
        },
      },
    },
    No3DSAutoCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: DUPLICATION_TIMEOUT,
        },
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        split_payments: payloadSplitPaymentData,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method: "card",
          attempt_count: 1,
          split_payments: payloadSplitPaymentData,
        },
      },
    },
    No3DSFailPayment: {
      // DELAY is required here (missing on the base Payload.js key too) --
      // without a cooldown, this reuses the same card+amount as other
      // No3DS* keys closely enough in a full-suite run to trip Payload's
      // duplicate-transaction detection.
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: DUPLICATION_TIMEOUT,
        },
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails, //payload doesnt support failed cards
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
        split_payments: payloadSplitPaymentData,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method: "card",
          attempt_count: 1,
          split_payments: payloadSplitPaymentData,
        },
      },
    },
    // NOTE: split_payments is intentionally not asserted in Response.body
    // here -- captureCallTest (commands.js) compares resData.body fields
    // with plain `.to.equal`, which fails on nested objects even when
    // their contents match (strict reference equality), unlike
    // createConfirmPaymentTest which uses `.to.deep.equal`. Whether the
    // /capture response actually preserves split_payments is unverified.
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
    // payloadconnect splits the full payment amount to receivers via the
    // ledger, leaving nothing in the merchant's transaction balance for a
    // standard refund. Payload's connector integration has no split-aware
    // refund mechanism (unlike stripeconnect's split_refunds), so every
    // refund attempt against a split payment fails -- verified live against
    // the real API: HTTP 200 with a business-level "failed" refund status.
    Refund: {
      Request: {
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_code: "InvalidAttributes",
          error_message:
            '{"ledger":[{"amount":"Amount is above transaction balance"}]}',
          unified_code: "UE_9000",
          unified_message: "Something went wrong",
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
          error_code: "InvalidAttributes",
          error_message:
            '{"ledger":[{"amount":"Amount is above transaction balance"}]}',
          unified_code: "UE_9000",
          unified_message: "Something went wrong",
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
          error_code: "InvalidAttributes",
          error_message:
            '{"ledger":[{"amount":"Amount is above transaction balance"}]}',
          unified_code: "UE_9000",
          unified_message: "Something went wrong",
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
          error_code: "InvalidAttributes",
          error_message:
            '{"ledger":[{"amount":"Amount is above transaction balance"}]}',
          unified_code: "UE_9000",
          unified_message: "Something went wrong",
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
    SaveCardUseNo3DSAutoCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: DUPLICATION_TIMEOUT,
        },
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
        split_payments: payloadSplitPaymentData,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          split_payments: payloadSplitPaymentData,
        },
      },
    },
    SaveCardUseNo3DSManualCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: DUPLICATION_TIMEOUT / 2, // 15 seconds
        },
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
        split_payments: payloadSplitPaymentData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture", // Keep this as requires_capture for manual flows
          split_payments: payloadSplitPaymentData,
        },
      },
    },
    PaymentMethodIdMandateNo3DSAutoCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: DUPLICATION_TIMEOUT / 2, // 15 seconds
        },
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
    SaveCardUseNo3DSAutoCaptureOffSession: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: DUPLICATION_TIMEOUT,
        },
      },
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
        split_payments: payloadSplitPaymentData,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          split_payments: payloadSplitPaymentData,
        },
      },
    },
    SaveCardUse3DSAutoCaptureOffSession: {
      Configs: {
        TRIGGER_SKIP: true,
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
        status: 400,
        body: {
          error: threeDSNotSupportedError,
        },
      },
    },
    SaveCardUseNo3DSManualCaptureOffSession: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: DUPLICATION_TIMEOUT / 2, // 15 seconds
        },
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
        split_payments: payloadSplitPaymentData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          split_payments: payloadSplitPaymentData,
        },
      },
    },
    SaveCardConfirmAutoCaptureOffSession: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: DUPLICATION_TIMEOUT / 2, // 15 seconds
        },
      },
      Request: {
        setup_future_usage: "off_session",
        split_payments: payloadSplitPaymentData,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          split_payments: payloadSplitPaymentData,
        },
      },
    },
    SaveCardConfirmManualCaptureOffSession: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: DUPLICATION_TIMEOUT / 2, // 15 seconds
        },
      },
      Request: {
        setup_future_usage: "off_session",
        split_payments: payloadSplitPaymentData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          split_payments: payloadSplitPaymentData,
        },
      },
    },
    SaveCardConfirmAutoCaptureOffSessionWithoutBilling: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: DUPLICATION_TIMEOUT,
        },
      },
      Request: {
        setup_future_usage: "off_session",
        billing: null,
        split_payments: payloadSplitPaymentData,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          split_payments: payloadSplitPaymentData,
        },
      },
    },
    PaymentMethodIdMandateNo3DSManualCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: DUPLICATION_TIMEOUT,
        },
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
    MandateSingleUseNo3DSAutoCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: DUPLICATION_TIMEOUT,
        },
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
    // NOTE: split_payments is intentionally omitted from all
    // Mandate*/MIT* keys below -- payload is excluded from every mandate
    // spec in this suite (MANDATE_ID_TEST exclusion list, plus
    // PaymentMethodIdMandateNo3DSAutoCapture's TRIGGER_SKIP cascades a
    // skip through 20-MandatesUsingPMID.cy.js), so this config is never
    // actually exercised. Revisit if payload mandate support is ever
    // un-excluded.
    MandateSingleUseNo3DSManualCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: DUPLICATION_TIMEOUT,
        },
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
        DELAY: {
          STATUS: true,
          TIMEOUT: DUPLICATION_TIMEOUT,
        },
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
        DELAY: {
          STATUS: true,
          TIMEOUT: DUPLICATION_TIMEOUT,
        },
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
    ZeroAuthPaymentIntent: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: DUPLICATION_TIMEOUT,
        },
      },
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
          amount: 0,
        },
      },
    },
    ZeroAuthConfirmPayment: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: DUPLICATION_TIMEOUT,
        },
      },
      Request: {
        payment_type: "setup_mandate",
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        setup_future_usage: "off_session",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          amount: 0,
          setup_future_usage: "off_session",
          payment_method_type: "credit",
        },
      },
    },
    ZeroAuthMandate: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: DUPLICATION_TIMEOUT,
        },
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          amount: 0,
        },
      },
    },
    MITAutoCapture: getCustomExchange({
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: DUPLICATION_TIMEOUT,
        },
      },
      ...commonConnectorDetails.card_pm.MITAutoCapture,
    }),
    MITManualCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: DUPLICATION_TIMEOUT,
        },
      },
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
  },
  bank_debit_pm: {
    Ach: getCustomExchange({
      Request: {
        payment_method: "bank_debit",
        payment_method_type: "ach",
        payment_method_data: {
          bank_debit: {
            ach_bank_debit: {
              account_number: "000123456789",
              routing_number: "110000000",
              bank_account_holder_name: "John Doe",
              bank_type: "checking",
            },
          },
        },
        billing: {
          address: {
            first_name: "John",
            last_name: "Doe",
            line1: "123 Main St",
            city: "San Francisco",
            state: "California",
            zip: "94122",
            country: "US",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),
    MandateSingleUseAch: getCustomExchange({
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: DUPLICATION_TIMEOUT,
        },
        LOCAL_VAULT_REQUIRED: true,
      },
      Request: {
        amount: 6540,
        payment_method: "bank_debit",
        payment_method_type: "ach",
        currency: "USD",
        payment_method_data: {
          bank_debit: {
            ach_bank_debit: {
              account_number: "000123456789",
              routing_number: "110000000",
              bank_account_holder_name: "John Doe",
              bank_type: "checking",
            },
          },
        },
        mandate_data: {
          customer_acceptance: customerAcceptance,
          mandate_type: {
            multi_use: {
              amount: 8000,
              currency: "USD",
            },
          },
        },
        setup_future_usage: "off_session",
        billing: {
          address: {
            first_name: "John",
            last_name: "Doe",
            line1: "123 Main St",
            city: "San Francisco",
            state: "California",
            zip: "94122",
            country: "US",
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
    }),
    MITAutoCaptureAch: getCustomExchange({
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: DUPLICATION_TIMEOUT,
        },
      },
      Request: {
        amount: 6540,
        off_session: true,
        confirm: true,
        currency: "USD",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),
  },
  webhook: {
    TransactionIdConfig: {
      path: "triggered_on.id",
      type: "string",
    },
  },
};
