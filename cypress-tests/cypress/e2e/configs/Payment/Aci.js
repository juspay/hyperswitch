import { customerAcceptance } from "./Commons";

const successfulNo3DSCardDetails = {
  card_number: "4242424242424242",
  card_exp_month: "01",
  card_exp_year: "2045",
  card_holder_name: "morino",
  card_cvc: "737",
};

const successfulThreeDSTestCardDetails = {
  card_number: "5386024192625914",
  card_exp_month: "01",
  card_exp_year: "2045",
  card_holder_name: "morino",
  card_cvc: "737",
};

// This card details will fail because of card expiryYear
const failedNo3DSCardDetails = {
  card_number: "4012001037461114",
  card_exp_month: "01",
  card_exp_year: "35",
  card_holder_name: "joseph Doe",
  card_cvc: "737",
};

const singleUseMandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    single_use: {
      amount: 8000,
      currency: "ZAR",
    },
  },
};

const multiUseMandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    multi_use: {
      amount: 8000,
      currency: "ZAR",
    },
  },
};

const billingAddress = {
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
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "ZAR",
        customer_acceptance: null,
        setup_future_usage: "on_session",
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
        amount: 6000,
        authentication_type: "no_three_ds",
        currency: "ZAR",
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
    No3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "ZAR",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method: "card",
          attempt_count: 1,
        },
        // Verify ACI mapping populates the hyperswitch response fields:
        // - auth_code: from resultDetails.AuthCode
        // - payment_checks: from parsed ConnectorTxID (STAN, originalTransactionId, acquirerResponse)
        assertNotNull: [
          "connector_transaction_id",
          "payment_method_data.card.auth_code",
          "payment_method_data.card.payment_checks",
        ],
      },
    },
    No3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "ZAR",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          payment_method: "card",
          attempt_count: 1,
        },
        assertNotNull: [
          "connector_transaction_id",
          "payment_method_data.card.auth_code",
          "payment_method_data.card.payment_checks",
        ],
      },
    },
    "3DSAutoCapture": {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "ZAR",
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
    "3DSManualCapture": {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "ZAR",
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
    PaymentIntentWithShippingCost: {
      Request: {
        currency: "ZAR",
        shipping_cost: 50,
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    PaymentConfirmWithShippingCost: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "ZAR",
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
        // ACI refund response populates connector_refund_id from the new
        // AciRefundResponse.id — verifies response mapping survived the
        // references[] enrichment.
        assertNotNull: ["connector_refund_id"],
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
        assertNotNull: ["connector_refund_id"],
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
    Capture: {
      Request: {
        amount_to_capture: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
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
    MandateSingleUse3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "ZAR",
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
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "ZAR",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    MandateSingleUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "ZAR",
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
        currency: "ZAR",
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
        currency: "ZAR",
        mandate_data: multiUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
        // Mandate CIT must return mandate_id + connector_mandate_id (ACI
        // registrationId). `network_transaction_id` (CITI) is acquirer-
        // specific (Nedbank pipe format), so not asserted here.
        assertNotNull: [
          "mandate_id",
          "connector_mandate_id",
          "payment_method_data.card.auth_code",
        ],
      },
    },
    MandateMultiUseNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "ZAR",
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
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "ZAR",
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
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "ZAR",
        mandate_data: multiUseMandateData,
      },
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
        currency: "ZAR",
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
        currency: "ZAR",
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
        },
      },
    },
    SaveCardUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "ZAR",
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
    SaveCardUseNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "ZAR",
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
        // Off-session CIT proves `auth_code` surfacing in the save-card path.
        // `connector_mandate_id` is verified by the MandateMultiUse specs
        // (asserting it here would fail because this config is also reused
        // on retrieves after the subsequent token-based MIT, where the
        // response doesn't echo a new mandate id).
        assertNotNull: [
          "connector_transaction_id",
          "payment_method_data.card.auth_code",
        ],
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
    SaveCardUse3DSAutoCaptureOffSession: {
      Configs: {
        TRIGGER_SKIP: true,
      },
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
    PaymentMethodIdMandateNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "ZAR",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
        // MIT roundtrip: if the subsequent payment succeeds with this
        // stored payment_method_id, the standingInstruction.initialTransactionId
        // (CITI) and agreementId (when Mastercard) were correctly replayed
        // from mandate_metadata.
        assertNotNull: [
          "connector_transaction_id",
          "payment_method_data.card.auth_code",
        ],
      },
    },
    PaymentMethodIdMandateNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "ZAR",
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
        currency: "ZAR",
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
        currency: "ZAR",
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
    MITManualCapture: {
      Request: {
        currency: "ZAR",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    MITAutoCapture: {
      Request: {
        currency: "ZAR",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    No3DSFailPayment: {
      Configs: {
        TRIGGER_SKIP: true,
      },
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
          error_code: "100.390.112",
          error_message: "Technical Error in 3D system",
          unified_code: "UE_9000",
          unified_message: "Something went wrong",
        },
      },
    },
    // External 3DS passthrough — merchant has already completed 3DS and is
    // forwarding the results (eci/cavv/dsTransactionId) for ACI to pass to
    // the acquirer. Skipped in CI because the CAVV must come from a real
    // authentication run; included here to document the expected payload
    // shape and ACI response behaviour.
    ExternalThreeDsPassthroughAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "ZAR",
        authentication_type: "no_three_ds",
        three_ds_data: {
          eci: "05",
          authentication_cryptogram: {
            cavv: {
              authentication_cryptogram: "AAABCSIIAAAAAAAAAAAAAAAAAAo=",
            },
          },
          ds_trans_id: "a8e95050-e7a1-4e67-a25a-example00001",
          version: "2.1.0",
          transaction_status: "Y",
        },
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
  },
  wallet_pm: {
    // Verify ACI returns apple_pay and google_pay session tokens
    SessionToken: {
      Request: {
        currency: "ZAR",
      },
      Response: {
        status: 200,
        body: {
          session_token: [
            { wallet_name: "apple_pay", connector: "aci" },
            { wallet_name: "google_pay", connector: "aci" },
          ],
        },
      },
    },
    // Wallet payment flows require real device tokens — skipped in CI
    ApplePayAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "wallet",
        payment_method_type: "apple_pay",
        payment_method_data: {
          wallet: { apple_pay_redirect: {} },
        },
        currency: "ZAR",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    GooglePayAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "wallet",
        payment_method_type: "google_pay",
        payment_method_data: {
          wallet: { google_pay_redirect: {} },
        },
        currency: "ZAR",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    SamsungPayAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "wallet",
        payment_method_type: "samsung_pay",
        payment_method_data: {
          wallet: { samsung_pay: { token: "test_token" } },
        },
        currency: "ZAR",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
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
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    Przelewy24: {
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "przelewy24",
        payment_method_data: {
          bank_redirect: {
            przelewy24: {
              bank_name: "citi",
              billing_details: {
                email: "guest@juspay.in",
              },
            },
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_code: "200.100.103",
          error_message:
            "invalid Request Message. The request contains structural errors",
        },
      },
    },
  },
};
