import {
  customerAcceptance,
  multiUseMandateData,
  singleUseMandateData,
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

// givepayments' real, confirmed rejection for flows it doesn't support at
// all (manual capture, unsupported payment methods) — a generic UCS_400
// "Payment failed during authorization with connector. Retry payment".
const authorizationFailedError = {
  error: {
    type: "invalid_request",
    message:
      "Payment failed during authorization with connector. Retry payment",
    code: "CE_01",
  },
};

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
          amount: 6000,
          shipping_cost: 50,
          net_amount: 6050,
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
          amount: 6000,
          net_amount: 6000,
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
          amount: 6000,
          net_amount: 6000,
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
        body: authorizationFailedError,
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
        body: authorizationFailedError,
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
          amount: 6000,
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
          amount: 2000,
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
          amount: 6000,
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
          amount: 2000,
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
      // amount is known upfront regardless of settlement state; status is
      // deliberately omitted here — it's asserted only after polling (see
      // POLL_AFTER above), not on this immediate response.
      Response: {
        status: 200,
        body: {
          amount: 6000,
        },
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
      // see MandateSingleUseNo3DSAutoCapture — status intentionally omitted
      Response: {
        status: 200,
        body: {
          amount: 6000,
        },
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
      // while the CIT is still "processing".
      Configs: {
        POLL_BEFORE: true,
      },
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          amount: 6000,
          net_amount: 6000,
        },
      },
    },
    MITWithoutBillingAddress: {
      Configs: {
        POLL_BEFORE: true,
      },
      Request: { billing: null },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          amount: 6000,
          net_amount: 6000,
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
        body: {
          amount: 6000,
        },
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
};
