import { customerAcceptance } from "./Commons";
import { getCustomExchange } from "./Modifiers";

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

// Mastercard no-3DS card from the permitted test cards, used without
// `payment_method_type` so the attempt can only be typed by the BIN lookup.
const untypedNo3DSCardDetails = {
  card_number: "5454545454545454",
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

const billingAddressAT = {
  ...billingAddress,
  address: {
    ...billingAddress.address,
    country: "AT",
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
    // The same payment without `payment_method_type`, as the web SDK sends it.
    // With no `cards_info` row for the card's BIN the attempt stays untyped
    // (asserted below), and its refund used to fail with IR_04 (PT-4386).
    No3DSManualCaptureNoPmType: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: untypedNo3DSCardDetails,
        },
        currency: "ZAR",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          payment_method_type: null,
        },
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
    // Retrieved after the 3DS challenge completes, so the authentication has actually
    // happened by the time these are asserted. Two regressions are pinned here:
    // - ACI parses `threeDSecure` but used to drop it on the floor, leaving
    //   `payment_method_data.card.authentication_data` null for a demonstrably
    //   authenticated payment (PT-3899);
    // - the force_sync this retrieve triggers is a second connector response, and ACI need
    //   not repeat the 3DS block in it. A sync that overwrites rather than preserves wipes
    //   the evidence the authorize established, so asserting after the sync is the point.
    "3DSAutoCaptureRetrieve": {
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
        assertNotNull: [
          "connector_transaction_id",
          "payment_method_data.card.authentication_data",
        ],
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
    // ACI forwards the statement descriptor and its suffix, space-joined, as OPPWA's
    // `descriptor` (AN127) — the only descriptor field in the Basic Payment parameter
    // set. The remaining billing_descriptor keys have no Basic Payment target and are
    // not forwarded; they are still echoed by the Hyperswitch response asserted below.
    PaymentIntentWithBillingDescriptor: {
      Request: {
        currency: "ZAR",
        billing_descriptor: {
          name: "Peach Payments",
          city: "Cape Town",
          phone: "0211234567",
          statement_descriptor: "PEACH STORE",
          statement_descriptor_suffix: "ORDER99",
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
        currency: "ZAR",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          billing_descriptor: {
            name: "Peach Payments",
            city: "Cape Town",
            phone: "0211234567",
            statement_descriptor: "PEACH STORE",
            statement_descriptor_suffix: "ORDER99",
            reference: "ref-qa-001",
          },
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
    // The 3DS mandate_data CITs carried TRIGGER_SKIP from the upstream ACI
    // configs, written when the "3DS" card was 4111... and the plain 3DS CIT
    // was skipped too. 5386... is challenged (see "3DSAutoCapture"), so these
    // assert the redirect like every other 3DS CIT here; the outcome is
    // checked on the retrieve after the challenge.
    MandateSingleUse3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "ZAR",
        authentication_type: "three_ds",
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
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "ZAR",
        authentication_type: "three_ds",
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
        amount: 6000,
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
        amount: 6000,
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
        amount: 6000,
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
        // registrationId). The CITI (`network_transaction_id`) is asserted on
        // the CIT by 25-ConnectorAgnosticNTID, and its replay on the MITs by
        // 20-MandatesUsingPMID.
        assertNotNull: [
          "mandate_id",
          "connector_mandate_id",
          "payment_method_data.card.auth_code",
        ],
      },
    },
    MandateMultiUseNo3DSManualCapture: {
      Request: {
        amount: 6000,
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
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "ZAR",
        authentication_type: "three_ds",
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
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "ZAR",
        authentication_type: "three_ds",
        mandate_data: multiUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    ZeroAuthMandate: {
      Request: {
        amount: 0,
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
        amount: 0,
        currency: "EUR",
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
    // Un-skipped with the 3DS mandate_data CITs above, for the same reason.
    SaveCardUse3DSAutoCaptureOffSession: {
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        // The sandbox entity is ZAR-only; the USD default is rejected with
        // "not configured for this currency".
        currency: "ZAR",
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
    // The same CIT without `payment_method_type`, as the web SDK and the
    // playground send it. The type then comes only from the BIN lookup, and
    // `cards_info` has no row for the test card, so core used to drop
    // `setup_future_usage` to on_session and ACI got no `createRegistration`.
    SaveCardUse3DSAutoCaptureOffSessionNoPmType: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "ZAR",
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          setup_future_usage: "off_session",
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
        amount: 6000,
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
        // A MIT on this payment_method_id succeeding proves no linkage: ACI's
        // test environment approves a REPEATED charge with or without
        // standingInstruction.initialTransactionId/agreementId. The replay is
        // asserted on the MIT's wire body in 20-MandatesUsingPMID (mitm record
        // mode, cy.assertAciMitStandingInstruction).
        assertNotNull: [
          "connector_transaction_id",
          "payment_method_data.card.auth_code",
        ],
      },
    },
    PaymentMethodIdMandateNo3DSManualCapture: {
      Request: {
        amount: 6000,
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
        amount: 6000,
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
    // Declares the series RECURRING on the CIT; ACI records the type it sent
    // and every MIT of the series must repeat it.
    PaymentMethodIdMandate3DSAutoCaptureRecurring: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "ZAR",
        mandate_data: null,
        authentication_type: "three_ds",
        customer_acceptance: customerAcceptance,
        mit_category: "recurring",
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
        amount: 6000,
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
        amount: 6000,
        currency: "ZAR",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    // setup_future_usage=off_session on a MIT is what lets core copy a sync's
    // network_transaction_id onto the payment method; the PSync of this MIT
    // while it is still requires_capture must not re-anchor the series.
    MITManualCaptureOffSession: {
      Request: {
        currency: "ZAR",
        setup_future_usage: "off_session",
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
        amount: 6000,
        currency: "ZAR",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    MITAutoCaptureWithCustomerAcceptance: {
      Request: {
        amount: 6000,
        currency: "ZAR",
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
    Giropay: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "giropay",
        payment_method_data: {
          bank_redirect: {
            giropay: {
              bank_name: "",
              bank_account_bic: "",
              bank_account_iban: "",
              preferred_language: "en",
              country: "DE",
            },
          },
        },
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_code: "800.900.300",
          error_message: "invalid authentication information",
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
        billing: billingAddressAT,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_code: "800.900.300",
          error_message: "invalid authentication information",
        },
      },
    },
    Sofort: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "sofort",
        payment_method_data: {
          bank_redirect: {
            sofort: {
              country: "DE",
              preferred_language: "en",
            },
          },
        },
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_message: "invalid authentication information",
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
    Interac: {
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "interac",
        payment_method_data: {
          bank_redirect: {
            interac: {
              bank_name: "ing",
            },
          },
        },
        billing: {
          ...billingAddress,
          email: "guest@juspay.in",
        },
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_code: "600.200.500",
          error_message: "invalid authentication information",
        },
      },
    },
    Eft: {
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "eft",
        payment_method_data: {
          bank_redirect: {
            eft: {
              provider: "ozow",
            },
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_code: "200.300.404",
          error_message: "invalid or missing parameter",
        },
      },
    },
  },
  pay_later_pm: {
    PaymentIntent: getCustomExchange({
      Request: {
        currency: "EUR",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
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
        billing: billingAddress,
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
    Capture: getCustomExchange({
      Request: {
        amount_to_capture: 6000,
      },
    }),
  },
};
