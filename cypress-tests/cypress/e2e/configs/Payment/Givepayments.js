import { generateRandomEmail } from "../../../utils/RequestBodyUtils";
import { multiUseMandateData, singleUseMandateData } from "./Commons";

const successfulNo3DSCardDetails = {
  card_number: "4111111111111111",
  card_exp_month: "12",
  card_exp_year: "2029",
  card_holder_name: "John Doe",
  card_cvc: "123",
  card_network: "Visa",
};

const successfulDebitCardDetails = {
  card_number: "4000056655665556",
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
        email: generateRandomEmail(),
        billing: billingWithEmail(generateRandomEmail()),
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
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        email: generateRandomEmail(),
        billing: billingWithEmail(generateRandomEmail()),
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
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
        email: generateRandomEmail(),
        billing: billingWithEmail(generateRandomEmail()),
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    DebitNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulDebitCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        email: generateRandomEmail(),
        billing: billingWithEmail(generateRandomEmail()),
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    DebitNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulDebitCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        email: generateRandomEmail(),
        billing: billingWithEmail(generateRandomEmail()),
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
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
    SyncRefund: {
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
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
        email: generateRandomEmail(),
        billing: billingWithEmail(generateRandomEmail()),
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
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
        email: generateRandomEmail(),
        billing: billingWithEmail(generateRandomEmail()),
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
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: multiUseMandateData,
        email: generateRandomEmail(),
        billing: billingWithEmail(generateRandomEmail()),
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
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: multiUseMandateData,
        email: generateRandomEmail(),
        billing: billingWithEmail(generateRandomEmail()),
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    DebitMandateSingleUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulDebitCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
        email: generateRandomEmail(),
        billing: billingWithEmail(generateRandomEmail()),
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    DebitMandateMultiUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulDebitCardDetails,
        },
        currency: "USD",
        mandate_data: multiUseMandateData,
        email: generateRandomEmail(),
        billing: billingWithEmail(generateRandomEmail()),
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
        },
      },
    },
    MITManualCapture: {
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
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
