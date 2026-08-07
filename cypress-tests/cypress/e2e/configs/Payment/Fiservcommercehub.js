import { cardRequiredField, customerAcceptance } from "./Commons";

// Commerce Hub cert-environment test cards
const successfulNo3DSCardDetails = {
  card_number: "4111111111111111",
  card_exp_month: "12",
  card_exp_year: "2030",
  card_holder_name: "Joseph Doe",
  card_cvc: "123",
};

const successfulThreeDSCardDetails = {
  card_number: "4005519200000004",
  card_exp_month: "12",
  card_exp_year: "2030",
  card_holder_name: "Joseph Doe",
  card_cvc: "123",
};

const failedCardDetails = {
  card_number: "4000000000000002",
  card_exp_month: "12",
  card_exp_year: "30",
  card_holder_name: "Joseph Doe",
  card_cvc: "123",
};

// CIT/MIT (mandate) flows must use this card — it's the only one confirmed
// to work for Setup Mandate / CIT / MIT against Fiserv Commerce Hub cert env.
const citMitCardDetails = {
  card_number: "4000000000001091",
  card_exp_month: "12",
  card_exp_year: "2030",
  card_holder_name: "Joseph Doe",
  card_cvc: "123",
};

const billingAddress = {
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
};

const singleUseMandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    single_use: {
      amount: 6000,
      currency: "USD",
    },
  },
};

const multiUseMandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    multi_use: {
      amount: 6000,
      currency: "USD",
    },
  },
};

// payment_method_data is intentionally left out of the Response bodies
// below (only keys present in the expected Response.body get asserted) —
// the Fiserv Commerce Hub cert environment returns a per-transaction
// auth_code that can never be hardcoded to match deterministically across
// runs, and this BIN's card_type/issuer/country data also isn't stable
// enough to assert reliably.

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
    PaymentIntentOffSession: {
      Request: {
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "off_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          setup_future_usage: "off_session",
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
    No3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method: "card",
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
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          payment_method: "card",
        },
      },
    },
    "3DSAutoCapture": {
      // fiservcommercehub doesn't implement 3DS redirection (Authenticate/
      // PreAuthenticate/PostAuthenticate are not_supported; redirection_data
      // is hardcoded None on Authorize), so a "3DS" test card actually
      // processes as a normal payment with no challenge — confirmed live.
      // TRIGGER_SKIP is a no-op here: every spec reading this key (05, 09,
      // 16) confirms via confirmCallTest, which doesn't honor it — so the
      // Response below must match reality rather than assume the step
      // never actually runs.
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: billingAddress,
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
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    PaymentConfirmWithShippingCost: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        billing: billingAddress,
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method: "card",
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
        },
      },
    },
    // Fiserv Commerce Hub's cert environment does not decline this card —
    // it processes it as a normal successful authorization instead of
    // simulating a failure, so this asserts the connector's real observed
    // behavior rather than a generic decline.
    No3DSFailPayment: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: failedCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    MandateSingleUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: citMitCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
        billing: billingAddress,
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
          card: citMitCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
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
          card: citMitCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    MandateMultiUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: citMitCardDetails,
        },
        currency: "USD",
        mandate_data: multiUseMandateData,
        billing: billingAddress,
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
          card: citMitCardDetails,
        },
        currency: "USD",
        mandate_data: multiUseMandateData,
        billing: billingAddress,
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
          card: citMitCardDetails,
        },
        currency: "USD",
        mandate_data: multiUseMandateData,
        billing: billingAddress,
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
          card: citMitCardDetails,
        },
        currency: "USD",
        mandate_data: multiUseMandateData,
        billing: billingAddress,
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
          card: citMitCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
        billing: billingAddress,
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
          card: citMitCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
        billing: billingAddress,
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
          card: citMitCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
        billing: billingAddress,
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
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: citMitCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
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
        billing: billingAddress,
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
        currency: "USD",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
        billing: billingAddress,
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
        payment_method_data: {
          card: citMitCardDetails,
        },
        currency: "USD",
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
        billing: billingAddress,
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
          card: citMitCardDetails,
        },
        currency: "USD",
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    // The payment itself does complete as "succeeded" off-session (no
    // redirect data returned), but 24-PaymentMethods.cy.js's "Handle
    // redirection" step still runs unconditionally afterward and hangs
    // waiting for a redirect that never comes — same REDIRECT_THREE_DS gap
    // as the on-session 3DS specs, so skip here too.
    SaveCardUse3DSAutoCaptureOffSession: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: citMitCardDetails,
        },
        currency: "USD",
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    // saveCardConfirmCallTest always confirms with a stored payment_token
    // (generic vault-token replay) and strips payment_method_data from the
    // request entirely — so, like the CONNECTOR_AGNOSTIC_NTID flows, this
    // hits fiservcommercehub's real gap: it relies on its own TransArmor
    // token for repeat charges, not hyperswitch's generic token replay.
    // Router rejects it before ever reaching the connector: IR_39 "no
    // eligible connector found for token-based MIT payment".
    SaveCardConfirmAutoCaptureOffSession: {
      Request: {
        setup_future_usage: "off_session",
        payment_method_data: {
          card: citMitCardDetails,
        },
        billing: billingAddress,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "No eligible connector was found for the current payment method configuration",
            code: "IR_39",
          },
        },
      },
    },
    SaveCardConfirmManualCaptureOffSession: {
      Request: {
        setup_future_usage: "off_session",
        payment_method_data: {
          card: citMitCardDetails,
        },
        billing: billingAddress,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "No eligible connector was found for the current payment method configuration",
            code: "IR_39",
          },
        },
      },
    },
    SaveCardConfirmAutoCaptureOffSessionWithoutBilling: {
      Request: {
        setup_future_usage: "off_session",
        billing: null,
        payment_method_data: {
          card: citMitCardDetails,
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "No eligible connector was found for the current payment method configuration",
            code: "IR_39",
          },
        },
      },
    },
    ZeroAuthMandate: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: citMitCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
        billing: billingAddress,
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
          card: citMitCardDetails,
        },
        mandate_data: null,
        customer_acceptance: customerAcceptance,
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    MITAutoCapture: {
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method: "card",
          connector: "fiservcommercehub",
        },
      },
    },
    MITManualCapture: {
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          payment_method: "card",
          connector: "fiservcommercehub",
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
  },
  bank_transfer_pm: {
    // fiservcommercehub is cards-only — no bank transfer support (Pix, ACH,
    // Instant Bank Transfer). confirmBankTransferCallTest doesn't honor
    // TRIGGER_SKIP, so — same as upi_pm below — this asserts the
    // connector's real, confirmed rejection instead of the generic
    // Commons.js 501 default: UCS surfaces this as a 400 CE_01 "Payment
    // failed during authorization with connector. Retry payment."
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
            first_name: "john",
            last_name: "doe",
          },
        },
        currency: "BRL",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "Payment failed during authorization with connector. Retry payment",
            code: "CE_01",
          },
        },
      },
    },
    Ach: {
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "ach",
        payment_method_data: {
          bank_transfer: {
            ach_bank_transfer: {},
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
            first_name: "john",
            last_name: "doe",
          },
        },
        currency: "BRL",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "Payment failed during authorization with connector. Retry payment",
            code: "CE_01",
          },
        },
      },
    },
    InstantBankTransferFinland: {
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "instant_bank_transfer_finland",
        payment_method_data: {
          bank_transfer: {
            instant_bank_transfer_finland: {},
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
            country: "FI",
            first_name: "john",
            last_name: "doe",
          },
        },
        currency: "EUR",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "Payment failed during authorization with connector. Retry payment",
            code: "CE_01",
          },
        },
      },
    },
    InstantBankTransferPoland: {
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "instant_bank_transfer_poland",
        payment_method_data: {
          bank_transfer: {
            instant_bank_transfer_poland: {},
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
        currency: "PLN",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "Payment failed during authorization with connector. Retry payment",
            code: "CE_01",
          },
        },
      },
    },
  },
  bank_redirect_pm: {
    // fiservcommercehub is cards-only — no bank redirect support. Unlike
    // bank_transfer_pm/upi_pm above, confirmBankRedirectCallTest and
    // citForMandatesCallTest both honor TRIGGER_SKIP, so these are skipped
    // outright rather than asserting a real rejection.
    Blik: { Configs: { TRIGGER_SKIP: true } },
    Eps: { Configs: { TRIGGER_SKIP: true } },
    Giropay: { Configs: { TRIGGER_SKIP: true } },
    Ideal: {
      Configs: { TRIGGER_SKIP: true },
      MandateSingleUseAutoCapture: { Configs: { TRIGGER_SKIP: true } },
    },
    Sofort: { Configs: { TRIGGER_SKIP: true } },
    Przelewy24: { Configs: { TRIGGER_SKIP: true } },
    OpenBankingUk: {
      Configs: { TRIGGER_SKIP: true },
      MandateSingleUseAutoCapture: { Configs: { TRIGGER_SKIP: true } },
    },
    OnlineBankingFpx: { Configs: { TRIGGER_SKIP: true } },
    Interac: { Configs: { TRIGGER_SKIP: true } },
    Trustly: {
      Configs: { TRIGGER_SKIP: true },
      MandateSingleUseAutoCapture: { Configs: { TRIGGER_SKIP: true } },
    },
    Eft: { Configs: { TRIGGER_SKIP: true } },
    BancontactCard: {
      MandateSingleUseAutoCapture: { Configs: { TRIGGER_SKIP: true } },
    },
  },
  upi_pm: {
    // fiservcommercehub does not support UPI; confirmUpiCall doesn't honor
    // TRIGGER_SKIP, so this asserts the connector's real, confirmed
    // rejection instead of the generic Commons.js 501 default: UCS_501
    // surfaces to the merchant as a 400 CE_01 "Payment failed during
    // authorization with connector. Retry payment."
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
            message:
              "Payment failed during authorization with connector. Retry payment",
            code: "CE_01",
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
            message:
              "Payment failed during authorization with connector. Retry payment",
            code: "CE_01",
          },
        },
      },
    },
  },
  reward_pm: {
    // fiservcommercehub does not support reward/cashtocode payment methods;
    // confirmRewardCallTest doesn't honor TRIGGER_SKIP, so this asserts the
    // connector's real, confirmed rejection instead of the generic
    // Commons.js 501 default: UCS_501 surfaces to the merchant as a 400
    // CE_01 "Payment failed during authorization with connector. Retry
    // payment."
    Evoucher: {
      Request: {
        payment_method: "reward",
        payment_method_type: "evoucher",
        payment_method_data: "reward",
        billing: billingAddress,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "Payment failed during authorization with connector. Retry payment",
            code: "CE_01",
          },
        },
      },
    },
    Classic: {
      Request: {
        payment_method: "reward",
        payment_method_type: "classic",
        payment_method_data: "reward",
        billing: billingAddress,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "Payment failed during authorization with connector. Retry payment",
            code: "CE_01",
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
                    eligible_connectors: ["fiservcommercehub"],
                  },
                ],
                required_fields: cardRequiredField,
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
                    eligible_connectors: ["fiservcommercehub"],
                  },
                ],
                required_fields: cardRequiredField,
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
                    eligible_connectors: ["fiservcommercehub"],
                  },
                ],
                required_fields: cardRequiredField,
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
                    eligible_connectors: ["fiservcommercehub"],
                  },
                ],
                required_fields: cardRequiredField,
              },
            ],
          },
        ],
      },
    },
  },
};
