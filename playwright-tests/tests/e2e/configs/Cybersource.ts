/**
 * Cybersource Connector Configuration
 *
 * Test data and expected responses for Cybersource connector
 * Ported from Cypress Cybersource.js - Simplified for POC
 */

import {
  customerAcceptance,
  cardRequiredField,
  successfulNo3DSCardDetails,
  successfulThreeDSTestCardDetails,
  singleUseMandateData,
  multiUseMandateData,
} from './Commons';
import type { ConnectorConfig } from './ConnectorTypes';

// Cybersource-specific card details
const cybersourceNo3DSCardDetails = {
  card_number: '4111111111111111',
  card_exp_month: '03',
  card_exp_year: '30',
  card_holder_name: 'John Doe',
  card_cvc: '123',
};

const cybersourceThreeDSCardDetails = {
  card_number: '4000000000001091',
  card_exp_month: '03',
  card_exp_year: '30',
  card_holder_name: 'John Doe',
  card_cvc: '123',
};

const cybersourceFailedCardDetails = {
  card_number: '4000000000000101',
  card_exp_month: '03',
  card_exp_year: '30',
  card_holder_name: 'John Doe',
  card_cvc: '123',
};

export const connectorDetails: ConnectorConfig = {
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: 'USD',
        customer_acceptance: null,
        setup_future_usage: 'on_session',
      },
      Response: {
        status: 200,
        body: {
          status: 'requires_payment_method',
          setup_future_usage: 'on_session',
        },
      },
    },

    No3DSAutoCapture: {
      Request: {
        payment_method: 'card',
        payment_method_data: {
          card: cybersourceNo3DSCardDetails,
        },
        currency: 'USD',
        customer_acceptance: null,
        setup_future_usage: 'on_session',
      },
      Response: {
        status: 200,
        body: {
          status: 'succeeded',
          payment_method: 'card',
          attempt_count: 1,
        },
      },
    },

    No3DSManualCapture: {
      Request: {
        payment_method: 'card',
        payment_method_data: {
          card: cybersourceNo3DSCardDetails,
        },
        currency: 'USD',
        customer_acceptance: null,
        setup_future_usage: 'on_session',
      },
      Response: {
        status: 200,
        body: {
          status: 'requires_capture',
          payment_method: 'card',
          attempt_count: 1,
        },
      },
    },

    '3DSAutoCapture': {
      Request: {
        payment_method: 'card',
        payment_method_data: {
          card: cybersourceThreeDSCardDetails,
        },
        currency: 'USD',
        customer_acceptance: null,
        setup_future_usage: 'on_session',
      },
      Response: {
        status: 200,
        body: {
          status: 'requires_customer_action',
          setup_future_usage: 'on_session',
        },
      },
    },

    '3DSManualCapture': {
      Request: {
        payment_method: 'card',
        payment_method_data: {
          card: cybersourceThreeDSCardDetails,
        },
        currency: 'USD',
        customer_acceptance: null,
        setup_future_usage: 'on_session',
      },
      Response: {
        status: 200,
        body: {
          status: 'requires_customer_action',
          setup_future_usage: 'on_session',
        },
      },
    },

    No3DSFailPayment: {
      Request: {
        payment_method: 'card',
        payment_method_data: {
          card: cybersourceFailedCardDetails,
        },
        customer_acceptance: null,
        setup_future_usage: 'on_session',
      },
      Response: {
        status: 200,
        body: {
          status: 'failed',
          error_code: 'card_declined',
          error_message: 'Your card was declined',
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
          status: 'succeeded',
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
          status: 'partially_captured',
          amount: 6000,
          amount_capturable: 0,
          amount_received: 2000,
        },
      },
    },

    Void: {
      Request: {},
      Response: {
        status: 200,
        body: {
          status: 'cancelled',
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
          status: 'pending',
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
          status: 'pending',
        },
      },
    },

    SyncRefund: {
      Response: {
        status: 200,
        body: {
          status: 'succeeded',
        },
      },
    },
  },
};
