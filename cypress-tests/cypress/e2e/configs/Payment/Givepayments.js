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

// givepayments only supports automatic capture — confirmed live via
// 06-NoThreeDSManualCapture.cy.js: router rejects manual capture before
// ever reaching the connector (ConnectorError::NotImplemented -> IR_00),
// not a connector-side authorization failure.
const captureMethodNotSupportedError = {
  error: {
    type: "invalid_request",
    message:
      "Capture method not supported. Givepayments connector supports Automatic capture method. is not implemented",
    code: "IR_00",
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
    // givepayments doesn't implement 3DS at all, so a "3DS" card just processes as a normal frictionless payment with no challenge — confirmed live.
    "3DSAutoCapture": {
      // TRIGGER_SKIP lets should_continue_further skip the doomed redirection step, since givepayments never sets next_action — confirmed live.
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
    // manual capture is rejected regardless of 3DS — confirmed live, same as No3DSManualCapture.
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
        status: 501,
        body: captureMethodNotSupportedError,
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
        status: 501,
        body: captureMethodNotSupportedError,
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
    // manual capture is rejected regardless of mandate — confirmed live, same as No3DSManualCapture.
    MandateSingleUseNo3DSManualCapture: {
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
        status: 501,
        body: captureMethodNotSupportedError,
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
    // manual capture is rejected regardless of mandate — confirmed live, same as No3DSManualCapture.
    MandateMultiUseNo3DSManualCapture: {
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
        status: 501,
        body: captureMethodNotSupportedError,
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
    // manual capture is rejected regardless of MIT — confirmed live, same as No3DSManualCapture.
    MITManualCapture: {
      Request: {},
      Response: {
        status: 501,
        body: captureMethodNotSupportedError,
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
    // manual capture is rejected regardless of mandate — confirmed live, same as No3DSManualCapture.
    PaymentMethodIdMandateNo3DSManualCapture: {
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
        status: 501,
        body: captureMethodNotSupportedError,
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
    // NOT independently confirmed live (unlike the rest of this file) — modeled on the async-settlement
    // pattern the other auto-capture confirms in this file exhibit (see No3DSAutoCapture). Verify against
    // the sandbox before relying on this for CI; adjust `status` here if it turns out to differ.
    SaveCardUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
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
    // NOT independently confirmed live — see SaveCardUseNo3DSAutoCapture.
    SaveCardUseNo3DSAutoCaptureOffSession: {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
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
  },
};
