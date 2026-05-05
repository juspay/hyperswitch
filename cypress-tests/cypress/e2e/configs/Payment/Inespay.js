import { getCustomExchange } from "./Modifiers";

// Inespay uses SEPA bank debit for payments with Spanish IBAN
const sepaBankDebitData = {
  iban: "ES9121000418450200051332",
  bank_account_holder_name: "John Doe",
};

export const connectorDetails = {
  bank_debit_pm: {
    PaymentIntent: getCustomExchange({
      Request: {
        currency: "EUR",
        amount: 6540,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),

    // Auto-capture SEPA bank debit payment
    No3DSAutoCapture: getCustomExchange({
      Request: {
        payment_method: "bank_debit",
        payment_method_type: "sepa",
        payment_method_data: {
          bank_debit: {
            sepa: sepaBankDebitData,
          },
        },
        currency: "EUR",
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            city: "San Francisco",
            state: "California",
            zip: "94122",
            country: "ES",
            first_name: "john",
            last_name: "doe",
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

    // Full refund
    Refund: getCustomExchange({
      Request: {
        amount: 6540,
        reason: "Customer requested refund",
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
        },
      },
    }),

    // Partial refund
    PartialRefund: getCustomExchange({
      Request: {
        amount: 3270,
        reason: "Partial refund",
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
        },
      },
    }),

    // Sync refund status
    SyncRefund: getCustomExchange({
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),
  },

  // Card pm section for compatibility (Inespay does not support cards)
  card_pm: {
    PaymentIntent: getCustomExchange({
      Request: {
        currency: "EUR",
        amount: 6540,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
    No3DSAutoCapture: getCustomExchange({
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Card payments not supported by Inespay",
          },
        },
      },
    }),
  },
};
