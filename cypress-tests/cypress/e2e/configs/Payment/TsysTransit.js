import { customerAcceptance } from "./Commons";

// TSYS TransIT test card numbers
// Visa
const successfulNo3DSCardDetailsVisa = {
  card_number: "4012000033330026",
  card_exp_month: "12",
  card_exp_year: "30",
  card_holder_name: "Joseph Doe",
  card_cvc: "123",
};

// Mastercard
const successfulNo3DSCardDetailsMastercard = {
  card_number: "5424000000000015",
  card_exp_month: "12",
  card_exp_year: "30",
  card_holder_name: "Joseph Doe",
  card_cvc: "123",
};

// American Express
const successfulNo3DSCardDetailsAmex = {
  card_number: "371449635398431",
  card_exp_month: "12",
  card_exp_year: "30",
  card_holder_name: "Joseph Doe",
  card_cvc: "1234",
};

const failedCardDetails = {
  ...successfulNo3DSCardDetailsVisa,
  card_number: "4012000099990026",
};

const billingAddress = {
  address: {
    line1: "1467 Harrison Street",
    line2: null,
    line3: null,
    city: "San Francisco",
    state: "CA",
    zip: "94122",
    country: "NA",
    first_name: "Joseph",
    last_name: "Doe",
  },
  phone: {
    number: "9123456789",
    country_code: "+1",
  },
  email: "joseph.doe@example.com",
};

const billingUS = {
  address: {
    line1: "1467 Harrison Street",
    line2: null,
    line3: null,
    city: "San Francisco",
    state: "CA",
    zip: "94122",
    country: "US",
    first_name: "Joseph",
    last_name: "Doe",
  },
  phone: {
    number: "9123456789",
    country_code: "+1",
  },
  email: "joseph.doe@example.com",
};

const payment_method_data_visa = {
  card: {
    last4: "0026",
    card_type: "DEBIT",
    card_network: "Visa",
    card_issuer: "VISA PRODUCTION SUPPORT CLIENT BID 1",
    card_issuing_country: "UNITEDSTATES",
    card_isin: "401200",
    card_extended_bin: null,
    card_exp_month: "12",
    card_exp_year: "30",
    card_holder_name: "Joseph Doe",
    payment_checks: null,
    authentication_data: null,
    auth_code: null,
  },
  billing: billingUS,
};

const payment_method_data_mastercard = {
  card: {
    last4: "0015",
    card_type: "CREDIT",
    card_network: "Mastercard",
    card_issuer: "MASTERCARD TEST",
    card_issuing_country: "UNITEDSTATES",
    card_isin: "542400",
    card_extended_bin: null,
    card_exp_month: "12",
    card_exp_year: "30",
    card_holder_name: "Joseph Doe",
    payment_checks: null,
    authentication_data: null,
    auth_code: null,
  },
  billing: billingUS,
};

const payment_method_data_amex = {
  card: {
    last4: "8431",
    card_type: "CREDIT",
    card_network: "AmericanExpress",
    card_issuer: "AMERICAN EXPRESS TEST",
    card_issuing_country: "UNITEDSTATES",
    card_isin: "371449",
    card_extended_bin: null,
    card_exp_month: "12",
    card_exp_year: "30",
    card_holder_name: "Joseph Doe",
    payment_checks: null,
    authentication_data: null,
    auth_code: null,
  },
  billing: billingUS,
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
          setup_future_usage: "on_session",
        },
      },
    },

    // ── Basic card flows ──────────────────────────────────────────

    No3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetailsVisa,
        },
        billing: billingAddress,
        currency: "USD",
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

    No3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetailsVisa,
        },
        billing: billingAddress,
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
          error_code: "card_declined",
          error_message: "The card has been declined",
          unified_code: "UE_9000",
          unified_message: "Something went wrong",
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

    SyncPayment: {
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
    // Visa variant (primary — used by 25-ConnectorAgnosticNTID.cy.js)

    SaveCardUseNo3DSAutoCapture: {
      Request: {
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
      Request: {
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

    // Moto CIT — Mastercard, off_session
    SaveCardUseNo3DSAutoCaptureOffSessionMastercard: {
      Request: {
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

    // Moto CIT — Amex, off_session
    SaveCardUseNo3DSAutoCaptureOffSessionAmex: {
      Request: {
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

    SaveCardUseNo3DSManualCaptureOffSession: {
      Request: {
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
      Request: {
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

    PaymentIntentOffSession: {
      Request: {
        currency: "USD",
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
      Request: {
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
      Request: {
        setup_future_usage: "off_session",
        payment_method_data: {
          card: successfulNo3DSCardDetailsVisa,
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },

    SaveCardConfirmAutoCaptureOffSessionWithoutBilling: {
      Request: {
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
    // Profile 2 uses NTID from Profile 1's CIT to authorise the MIT.
    MITAutoCapture: {
      Request: {},
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

    MITManualCapture: {
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          mandate_id: null,
          payment_method: "card",
          payment_method_data: payment_method_data_visa,
          connector: "tsys_transit",
        },
      },
    },

    MITWithoutBillingAddress: {
      Request: {
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
      Request: {
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
      Request: {
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

    // ── Mandate flows (single-use / multi-use) ────────────────────

    MandateSingleUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetailsVisa,
        },
        currency: "USD",
        customer_acceptance: customerAcceptance,
        mandate_data: {
          customer_acceptance: customerAcceptance,
          mandate_type: {
            single_use: {
              amount: 8000,
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

    MandateSingleUseNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetailsVisa,
        },
        currency: "USD",
        customer_acceptance: customerAcceptance,
        mandate_data: {
          customer_acceptance: customerAcceptance,
          mandate_type: {
            single_use: {
              amount: 8000,
              currency: "USD",
            },
          },
        },
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
          card: successfulNo3DSCardDetailsVisa,
        },
        currency: "USD",
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
          card: successfulNo3DSCardDetailsVisa,
        },
        currency: "USD",
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
      Request: {
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
      Request: {
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
      Request: {
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

    // ── 3DS (not supported) ───────────────────────────────────────

    "3DSAutoCapture": {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "3DS not supported for TsysTransit",
            code: "IR_00",
          },
        },
      },
    },

    "3DSManualCapture": {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "3DS not supported for TsysTransit",
            code: "IR_00",
          },
        },
      },
    },

    MandateSingleUse3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "3DS not supported for TsysTransit",
            code: "IR_00",
          },
        },
      },
    },

    MandateSingleUse3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "3DS not supported for TsysTransit",
            code: "IR_00",
          },
        },
      },
    },

    MandateMultiUse3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "3DS not supported for TsysTransit",
            code: "IR_00",
          },
        },
      },
    },

    MandateMultiUse3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "3DS not supported for TsysTransit",
            code: "IR_00",
          },
        },
      },
    },

    PaymentMethodIdMandate3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "3DS not supported for TsysTransit",
            code: "IR_00",
          },
        },
      },
    },

    PaymentMethodIdMandate3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "3DS not supported for TsysTransit",
            code: "IR_00",
          },
        },
      },
    },

    SaveCardUse3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "3DS not supported for TsysTransit",
            code: "IR_00",
          },
        },
      },
    },

    SaveCardUse3DSAutoCaptureOnSession: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "3DS not supported for TsysTransit",
            code: "IR_00",
          },
        },
      },
    },

    SaveCardUse3DSAutoCaptureOffSession: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "3DS not supported for TsysTransit",
            code: "IR_00",
          },
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
  },
};
