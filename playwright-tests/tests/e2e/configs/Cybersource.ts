/**
 * Cybersource Connector Configuration
 *
 * Test data and expected responses for Cybersource connector
 * Ported from Cypress Cybersource.js - Simplified for POC
 */

import type { ConnectorConfig } from './ConnectorTypes';

// Customer acceptance data (defined locally to avoid circular dependency)
const customerAcceptance = {
  acceptance_type: 'offline',
  accepted_at: '1963-05-03T04:07:52.723Z',
  online: {
    ip_address: '127.0.0.1',
    user_agent:
      'Mozilla/5.0 (iPhone; CPU iPhone OS 18_5 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/22F76 [FBAN/FBIOS;FBAV/520.0.0.38.101;FBBV/756351453;FBDV/iPhone14,7;FBMD/iPhone;FBSN/iOS;FBSV/18.5;FBSS/3;FBID/phone;FBLC/fr_FR;FBOP/5;FBRV/760683563;IABMV/1]',
  },
};

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

    PaymentIntentOffSession: {
      Request: {
        currency: 'USD',
        amount: 6000,
        authentication_type: 'no_three_ds',
        customer_acceptance: null,
        setup_future_usage: 'off_session',
      },
      Response: {
        status: 200,
        body: {
          status: 'requires_payment_method',
          setup_future_usage: 'off_session',
        },
      },
    },

    PaymentMethodIdMandateNo3DSAutoCapture: {
      Request: {
        payment_method: 'card',
        payment_method_data: {
          card: cybersourceNo3DSCardDetails,
        },
        currency: 'USD',
        mandate_data: null,
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: 'succeeded',
        },
      },
    },

    PaymentMethodIdMandateNo3DSManualCapture: {
      Request: {
        payment_method: 'card',
        payment_method_data: {
          card: cybersourceNo3DSCardDetails,
        },
        currency: 'USD',
        mandate_data: null,
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: 'requires_capture',
        },
      },
    },

    PaymentMethodIdMandate3DSAutoCapture: {
      Request: {
        payment_method: 'card',
        payment_method_data: {
          card: cybersourceThreeDSCardDetails,
        },
        currency: 'USD',
        mandate_data: null,
        authentication_type: 'three_ds',
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: 'requires_customer_action',
        },
      },
    },

    PaymentMethodIdMandate3DSManualCapture: {
      Request: {
        payment_method: 'card',
        payment_method_data: {
          card: cybersourceThreeDSCardDetails,
        },
        mandate_data: null,
        authentication_type: 'three_ds',
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: 'requires_customer_action',
        },
      },
    },

    MITAutoCapture: {
      Request: {},
      Response: {
        status: 200,
        body: {
          status: 'succeeded',
        },
      },
    },

    MITManualCapture: {
      Request: {},
      Response: {
        status: 200,
        body: {
          status: 'requires_capture',
        },
      },
    },

    MITWithoutBillingAddress: {
      Request: {},
      Response: {
        status: 200,
        body: {
          status: 'succeeded',
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
          status: 'processing',  // Cybersource captures are async
          amount: 6000,
          amount_capturable: 6000,  // Stays at 6000 during processing
          amount_received: null,     // null until processed
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
          status: 'processing',  // Cybersource partial captures are also async
          amount: 6000,
          amount_capturable: 6000,  // Stays at 6000 during processing
          amount_received: null,     // null until processed
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
          status: 'pending',  // Cybersource refunds are async, stay pending
        },
      },
    },
  },
};
