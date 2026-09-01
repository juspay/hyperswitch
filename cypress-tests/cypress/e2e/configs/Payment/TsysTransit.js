import { customerAcceptance } from "./Commons";

// TSYS TransIT echoes the merchant order reference back to the connector, so a
// fresh value is generated for every payment intent rather than once per run.
const randomMerchantOrderReferenceId = () =>
  `tsys_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 10)}`;

const successfulNo3DSCardDetailsVisa = {
  card_number: "4012000098765439",
  card_exp_month: "12",
  card_exp_year: "28",
  card_holder_name: "Joseph Doe",
  card_cvc: "999",
};

const successfulNo3DSCardDetailsMastercard = {
  card_number: "5146315000000055",
  card_exp_month: "12",
  card_exp_year: "28",
  card_holder_name: "Joseph Doe",
  card_cvc: "998",
};

// 15 digit PAN, 4 digit CID
const successfulNo3DSCardDetailsAmex = {
  card_number: "371449635392376",
  card_exp_month: "12",
  card_exp_year: "28",
  card_holder_name: "Joseph Doe",
  card_cvc: "9997",
};

// TSYS TransIT test card numbers — 3DS
const successfulThreeDSCardDetailsVisa = {
  card_number: "4012000033330026",
  card_exp_month: "12",
  card_exp_year: "30",
  card_holder_name: "Joseph Doe",
  card_cvc: "999",
};

const successfulThreeDSCardDetailsMastercard =
  successfulNo3DSCardDetailsMastercard;

const successfulThreeDSCardDetailsAmex = successfulNo3DSCardDetailsAmex;

const failedCardDetails = {
  ...successfulNo3DSCardDetailsVisa,
  card_cvc: "123",
};

const payment_method_data_visa = {
  card: {
    last4: "5439",
    card_type: "DEBIT",
    card_network: "Visa",
    card_issuer: "VISA PRODUCTION SUPPORT CLIENT BID 1",
    card_issuing_country: "UNITEDSTATES",
    card_isin: "401200",
    card_extended_bin: null,
    card_exp_month: "12",
    card_exp_year: "28",
    card_holder_name: "Joseph Doe",
    payment_checks: null,
    authentication_data: null,
    auth_code: null,
  },
  billing: null,
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        payment_channel: "telephone_order",
        get merchant_order_reference_id() {
          return randomMerchantOrderReferenceId();
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          setup_future_usage: "on_session",
        },
      },
    },

    // ── No-3DS card flows ─────────────────────────────────────────

    No3DSAutoCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        payment_channel: "telephone_order",
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetailsVisa,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        get merchant_order_reference_id() {
          return randomMerchantOrderReferenceId();
        },
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method: "card",
          payment_method_data: payment_method_data_visa,
        },
      },
    },

    No3DSManualCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        payment_channel: "telephone_order",
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetailsVisa,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          payment_method: "card",
          payment_method_data: payment_method_data_visa,
        },
      },
    },

    No3DSFailPayment: {
      Request: {
        payment_channel: "telephone_order",
        payment_method: "card",
        payment_method_data: {
          card: failedCardDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_code: "D2020",
          error_message: "CVV2 verification failed",
        },
      },
    },

    // ── 3DS card flows — Visa ─────────────────────────────────────

    "3DSAutoCapture": {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_channel: "telephone_order",
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetailsVisa,
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

    "3DSManualCapture": {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_channel: "telephone_order",
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSCardDetailsVisa,
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

    // 3DS — Mastercard variant
    "3DSAutoCaptureMastercard": {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_channel: "telephone_order",
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSCardDetailsMastercard,
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

    // 3DS — Amex variant
    "3DSAutoCaptureAmex": {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_channel: "telephone_order",
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSCardDetailsAmex,
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

    // ── Capture / Void / Refund ───────────────────────────────────

    Capture: {
      Request: {
        amount_to_capture: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          amount: 6000,
          amount_capturable: 6000,
          amount_received: null,
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
          status: "succeeded",
          amount: 6000,
          amount_capturable: 6000,
          amount_received: null,
        },
      },
    },

    Void: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        cancellation_reason: "VOID",
      },
      Response: {
        status: 200,
        body: {
          status: "cancelled",
        },
      },
    },

    VoidAfterConfirm: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        cancellation_reason: "VOID",
      },
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
          amount: 6000,
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
          amount: 2000,
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
          amount: 2000,
        },
      },
    },

    SyncPayment: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method: "card",
          payment_method_data: payment_method_data_visa,
        },
      },
    },

    // ── Moto CIT — connector agnostic token, off_session ─────────
    // Profile must have is_connector_agnostic_mit_enabled = true.

    SaveCardUseNo3DSAutoCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        payment_channel: "telephone_order",
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetailsVisa,
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

    // Moto CIT — Visa, off_session (primary CIT for connector-agnostic MIT)
    SaveCardUseNo3DSAutoCaptureOffSession: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_channel: "telephone_order",
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetailsVisa,
        },
        currency: "USD",
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
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_channel: "telephone_order",
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetailsVisa,
        },
        currency: "USD",
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

    SaveCardUseNo3DSManualCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        payment_channel: "telephone_order",
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetailsVisa,
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

    // 3DS SaveCard flows
    SaveCardUse3DSAutoCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_channel: "telephone_order",
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSCardDetailsVisa,
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

    SaveCardUse3DSAutoCaptureOnSession: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_channel: "telephone_order",
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSCardDetailsVisa,
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

    SaveCardUse3DSAutoCaptureOffSession: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_channel: "telephone_order",
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSCardDetailsVisa,
        },
        currency: "USD",
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

    PaymentIntentOffSession: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        payment_channel: "telephone_order",
        currency: "USD",
        setup_future_usage: "off_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },

    // Moto MIT — using payment token of CIT (setup_future_usage: off_session)
    SaveCardConfirmAutoCaptureOffSession: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_channel: "telephone_order",
        setup_future_usage: "off_session",
        payment_method_data: {
          card: successfulNo3DSCardDetailsVisa,
        },
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },

    SaveCardConfirmManualCaptureOffSession: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_channel: "telephone_order",
        setup_future_usage: "off_session",
        payment_method_data: {
          card: successfulNo3DSCardDetailsVisa,
        },
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },

    SaveCardConfirmAutoCaptureOffSessionWithoutBilling: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_channel: "telephone_order",
        setup_future_usage: "off_session",
        billing: null,
        payment_method_data: {
          card: successfulNo3DSCardDetailsVisa,
        },
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          billing: null,
        },
      },
    },

    // Moto MIT — with payment_method_id (NTID flow; psp token not present)
    MITAutoCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        payment_channel: "telephone_order",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          mandate_id: null,
          payment_method: "card",
          connector: "tsys_transit",
        },
      },
    },

    MITManualCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        payment_channel: "telephone_order",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          mandate_id: null,
          payment_method: "card",
          payment_method_data: payment_method_data_visa,
          connector: "tsys_transit",
        },
      },
    },

    MITWithoutBillingAddress: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        payment_channel: "telephone_order",
        billing: null,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          billing: null,
        },
      },
    },

    // ── PaymentMethodId mandate flows ─────────────────────────────

    PaymentMethodIdMandateNo3DSAutoCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_channel: "telephone_order",
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetailsVisa,
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
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_channel: "telephone_order",
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetailsVisa,
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
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_channel: "telephone_order",
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSCardDetailsVisa,
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

    PaymentMethodIdMandate3DSManualCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_channel: "telephone_order",
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSCardDetailsVisa,
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

    // ── ZeroAuth ──────────────────────────────────────────────────

    ZeroAuthMandate: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        payment_channel: "telephone_order",
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetailsVisa,
        },
        currency: "USD",
        mandate_data: {
          customer_acceptance: customerAcceptance,
          mandate_type: {
            single_use: {
              amount: 0,
              currency: "USD",
            },
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

    ZeroAuthPaymentIntent: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        payment_channel: "telephone_order",
        currency: "USD",
        amount: 0,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },

    ZeroAuthConfirmPayment: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        payment_channel: "telephone_order",
        payment_type: "setup_mandate",
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetailsVisa,
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
    // ── Misc ──────────────────────────────────────────────────────

    PaymentIntentWithShippingCost: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        payment_channel: "telephone_order",
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
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        payment_channel: "telephone_order",
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetailsVisa,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method: "card",
          payment_method_data: payment_method_data_visa,
        },
      },
    },

    // ══ 57-TsysTransitMandates.cy.js ══════════════════════════════
    // Everything below belongs to the tagged TSYS TransIT spec only; the
    // shared specs (04, 06, 09, 14, ...) never read these keys.
    //
    // `payment_method_data` is deliberately not asserted on the Mastercard and
    // Amex flows: the issuer metadata returned for those BINs is not stable, so
    // asserting it would fail on card bin data refreshes rather than on
    // connector regressions.

    // ── Auto capture, one card per network ────────────────────────

    No3DSAutoCaptureVisa: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        payment_channel: "telephone_order",
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetailsVisa,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        get merchant_order_reference_id() {
          return randomMerchantOrderReferenceId();
        },
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method: "card",
          payment_method_data: payment_method_data_visa,
        },
      },
    },

    No3DSAutoCaptureMastercard: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        payment_channel: "telephone_order",
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetailsMastercard,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        get merchant_order_reference_id() {
          return randomMerchantOrderReferenceId();
        },
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method: "card",
        },
      },
    },

    No3DSAutoCaptureAmex: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        payment_channel: "telephone_order",
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetailsAmex,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        get merchant_order_reference_id() {
          return randomMerchantOrderReferenceId();
        },
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method: "card",
        },
      },
    },

    // ── Off session CIT, one card per network ─────────────────────
    // Runs on the connector agnostic profile the spec creates. No
    // `connector_mandate_id` is returned by TSYS TransIT, the assertion on it
    // is skipped for this connector.

    SaveCardUseNo3DSAutoCaptureOffSessionVisa: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        payment_channel: "telephone_order",
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetailsVisa,
        },
        currency: "USD",
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

    SaveCardUseNo3DSAutoCaptureOffSessionMastercard: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        payment_channel: "telephone_order",
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetailsMastercard,
        },
        currency: "USD",
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

    SaveCardUseNo3DSAutoCaptureOffSessionAmex: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        payment_channel: "telephone_order",
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetailsAmex,
        },
        currency: "USD",
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

    // ── Mandate flows on the connector agnostic profile ───────────
    // The payment intents of these flows stay on the shared
    // `PaymentIntentOffSession`; only the confirms are spec owned.

    // CIT that saves the card, MIT then reuses the payment token
    MandateCitOffSession: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        payment_channel: "telephone_order",
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetailsVisa,
        },
        currency: "USD",
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

    MandateMitUsingToken: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        amount: 6000,
        payment_channel: "telephone_order",
        setup_future_usage: "off_session",
        payment_method_data: {
          card: successfulNo3DSCardDetailsVisa,
        },
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },

    // CIT that registers the mandate, MIT then reuses the payment_method_id
    MandatePmIdCit: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        payment_channel: "telephone_order",
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetailsVisa,
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

    MandatePmIdMit: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        payment_channel: "telephone_order",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          mandate_id: null,
          payment_method: "card",
          connector: "tsys_transit",
        },
      },
    },
  },
};
