import {
  customerAcceptance,
  multiUseMandateData,
  singleUseMandateData,
  standardBillingAddress,
} from "./Commons";

// givepayments' sandbox does real deliverability verification, not just
// pattern matching — even human-looking fabricated addresses (e.g.
// jane.smith860@gmail.com) get rejected as "disposable or unreachable".
// Use a single real, monitored inbox everywhere.
const generateGivepaymentsEmail = () => "venkatakarthik.m@juspay.in";

const successfulNo3DSCardDetails = {
  card_number: "4111111111111111",
  card_exp_month: "12",
  card_exp_year: "2029",
  card_holder_name: "John Doe",
  card_cvc: "123",
  card_network: "Visa",
};

const billingWithEmail = (email) => ({
  address: {
    line1: "1467",
    line2: "Harrison Street",
    city: "San Francisco",
    state: "California",
    zip: "94122",
    country: "US",
    first_name: "John",
    last_name: "Doe",
  },
  email,
});

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
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
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
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
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
      },
      // Confirm returns "processing" synchronously — givepayments settles
      // async, shortly after — confirmed live.
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    No3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
      },
      // Confirm returns "processing" synchronously — givepayments settles
      // async, shortly after — confirmed live.
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    // givepayments doesn't implement 3DS at all (connector-service
    // field_probe: authenticate/pre_authenticate/post_authenticate are all
    // "not_supported", redirection_data is hardcoded None on every
    // Authorize response), so a "3DS" card just processes as a normal
    // frictionless payment with no challenge — confirmed live via the
    // off-session 3DS save-card flow, which returns "succeeded" directly
    // rather than requires_customer_action or an error.
    "3DSAutoCapture": {
      // Confirm returns "processing" synchronously — givepayments settles
      // async, shortly after — confirmed live. There's no real redirect
      // (givepayments never sets next_action), so the subsequent "handle
      // redirection" step in 05/16 can never complete; confirmCallTest
      // doesn't honor TRIGGER_SKIP so this Response is still asserted for
      // real, but TRIGGER_SKIP makes should_continue_further correctly
      // skip the doomed redirection step afterward (same mechanism
      // 3DSManualCapture gets for free via its real body.error).
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    // manual capture is rejected regardless of 3DS (see No3DSManualCapture)
    // — this hits the same capture_method restriction, not a 3DS-specific
    // rejection.
    // givepayments only supports automatic capture — confirmCallTest and
    // createConfirmPaymentTest don't honor TRIGGER_SKIP, so this asserts
    // the connector's real, confirmed rejection instead of skipping (same
    // pattern Helcim uses for its always-failing refunds). Note: a
    // separate confirmCallTest-only run (create-then-confirm, not
    // combined) observed a 501 "Capture method not supported..." instead
    // — the two flows apparently hit different validation paths in UCS.
    // This uses the combined create+confirm result since that's the more
    // common flow shape in this file.
    "3DSManualCapture": {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
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
    No3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
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
    // givepayments 400s a refund attempted while the payment is still
    // "processing" — poll for settlement first, then refund. The refund
    // itself still settles asynchronously (never observed reaching a
    // terminal state within the test's lifetime), so its own response is
    // asserted as "pending", not "succeeded".
    Refund: {
      Configs: {
        POLL_BEFORE: true,
      },
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
      Configs: {
        POLL_BEFORE: true,
      },
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
      Configs: {
        POLL_BEFORE: true,
      },
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
    manualPaymentPartialRefund: {
      Configs: {
        POLL_BEFORE: true,
      },
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
    SyncRefund: {
      Response: {
        status: 200,
        body: {
          status: "pending",
        },
      },
    },
    // Confirm returns "processing" immediately (async settlement) — no
    // immediate status assertion here; POLL_AFTER polls until it actually
    // reaches "succeeded" and asserts that explicitly, same as
    // PaymentMethodIdMandateNo3DSAutoCapture.
    MandateSingleUseNo3DSAutoCapture: {
      Configs: {
        POLL_AFTER: true,
      },
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
      },
      Response: {
        status: 200,
        body: {},
      },
    },
    MandateSingleUseNo3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true, // givepayments only supports automatic capture
      },
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
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
        POLL_AFTER: true,
      },
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: multiUseMandateData,
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
      },
      Response: {
        status: 200,
        body: {},
      },
    },
    MandateMultiUseNo3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true, // givepayments only supports automatic capture
      },
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: multiUseMandateData,
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    MITAutoCapture: {
      // wait for the CIT to settle before attempting the repeat charge —
      // the connector_mandate_id needed for MIT isn't reliably resolvable
      // while the CIT is still "processing". OMIT_AMOUNT matches a known
      // working manual (Postman) recipe from dev that sends no "amount"
      // field on the recurring_details/payment_method_id MIT request —
      // testing whether that's the actual differentiator.
      Configs: {
        POLL_BEFORE: true,
        OMIT_AMOUNT: true,
      },
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    MITWithoutBillingAddress: {
      Configs: {
        POLL_BEFORE: true,
        OMIT_AMOUNT: true,
      },
      Request: { billing: null },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    MITManualCapture: {
      Configs: {
        TRIGGER_SKIP: true, // givepayments only supports automatic capture
      },
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    PaymentMethodIdMandateNo3DSAutoCapture: {
      // Confirm returns "processing" immediately (async settlement) — no
      // immediate status assertion here; POLL_AFTER polls until it actually
      // reaches "succeeded" and asserts that explicitly, before letting the
      // subsequent MIT step run. A still-"processing" CIT hasn't reliably
      // persisted connector_mandate_id onto the payment_method yet.
      Configs: {
        POLL_AFTER: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
      },
      Response: {
        status: 200,
        body: {},
      },
    },
    PaymentMethodIdMandateNo3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true, // givepayments only supports automatic capture
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    // givepayments does not support 3DS at all (connector metadata:
    // "three_ds": "not_supported"); see REDIRECT_THREE_DS exclude list.
    PaymentMethodIdMandate3DSAutoCapture: {
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
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
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
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
  },
  // givepayments doesn't support any bank redirect method either;
  // confirmBankRedirectCallTest honors TRIGGER_SKIP, but real evidence is
  // available (identical 400 CE_01 across every method below), so this
  // asserts that real rejection instead of skipping — Trustly is excluded
  // because Commons.js already sets TRIGGER_SKIP: true on it globally.
  bank_redirect_pm: {
    Ideal: {
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "ideal",
        payment_method_data: {
          bank_redirect: {
            ideal: {
              bank_name: "ing",
              country: "NL",
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
            first_name: "john",
            last_name: "doe",
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
    OpenBankingUk: {
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "open_banking_uk",
        payment_method_data: {
          bank_redirect: {
            open_banking_uk: {
              issuer: "citi",
              country: "GB",
            },
          },
        },
        billing: standardBillingAddress,
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
    OnlineBankingFpx: {
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "online_banking_fpx",
        payment_method_data: {
          bank_redirect: {
            online_banking_fpx: {
              issuer: "affin_bank",
            },
          },
        },
        billing: standardBillingAddress,
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
              preferred_language: "en",
              country: "DE",
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
            first_name: "john",
            last_name: "doe",
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
    Sofort: {
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
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "San Fransico",
            state: "California",
            zip: "94122",
            country: "DE",
            first_name: "john",
            last_name: "doe",
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
            first_name: "john",
            last_name: "doe",
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
    Blik: {
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "blik",
        payment_method_data: {
          bank_redirect: {
            blik: {
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
          ...standardBillingAddress,
          address: {
            ...standardBillingAddress.address,
            country: "CA",
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
  // givepayments doesn't support UPI; confirmUpiCall doesn't honor
  // TRIGGER_SKIP, so this asserts the connector's real, confirmed
  // rejection — a 400 CE_01 "Payment failed during authorization with
  // connector. Retry payment" — confirmed live for both UPI methods.
  upi_pm: {
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
  // givepayments doesn't support reward/cashtocode payment methods;
  // confirmRewardCallTest doesn't honor TRIGGER_SKIP, so this asserts the
  // connector's real, confirmed rejection — a 400 CE_01 "Payment failed
  // during authorization with connector. Retry payment" — confirmed live
  // for both reward methods.
  reward_pm: {
    Evoucher: {
      Request: {
        payment_method: "reward",
        payment_method_type: "evoucher",
        payment_method_data: "reward",
        billing: standardBillingAddress,
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
        billing: standardBillingAddress,
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
  // givepayments doesn't support any bank transfer method (connector-service
  // field_probe: Pix/Ach/InstantBankTransferFinland/InstantBankTransferPoland
  // are all "not_implemented"); confirmBankTransferCallTest doesn't honor
  // TRIGGER_SKIP, so these assert the connector's real, confirmed rejection
  // instead of the generic Commons.js 501 default — a 400 CE_01 "Payment
  // failed during authorization with connector. Retry payment" — confirmed
  // live for all four methods.
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
  webhook: {
    TransactionIdConfig: {
      // Defines how to locate and parse the payment reference ID from connector-specific webhook payloads
      path: "payload.id",
      type: "string",
    },
  },
};
