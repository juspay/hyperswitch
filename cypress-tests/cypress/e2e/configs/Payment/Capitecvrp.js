// Capitec VRP Connector Configuration
// This connector uses Open Banking VRP (Variable & Recurring Payments) flow
// Currency: ZAR (South African Rand)
// Flow: Consent creation -> Customer approval -> Payment action

import { customerAcceptance } from "./Commons";

// Test client identifiers from Capitec QA environment
// From Capitec Pay VRP Test Plan v1.1 - QA Test Clients (Section 2.2)
// Veronica Fox: Cell 0609603632, ID 8906244547089, Account 2409425506
// Tshepo Moreki: Cell 0609603633, ID 8906241825082, Account 2409425514

const testClientCellphone = {
  identifier_key: "CELLPHONE",
  identifier_value: "0609603632", // Veronica Fox test user (QA)
};

const testClientIdNumber = {
  identifier_key: "IDNUMBER",
  identifier_value: "8906244547089", // Veronica Fox test user (QA)
};

const testClientAccountNumber = {
  identifier_key: "ACCOUNTNUMBER",
  identifier_value: "2409425506", // Veronica Fox test user (QA)
};

// Alternative test client - Tshepo Moreki
const testClientCellphoneAlt = {
  identifier_key: "CELLPHONE",
  identifier_value: "0609603633", // Tshepo Moreki test user (QA)
};

const testClientIdNumberAlt = {
  identifier_key: "IDNUMBER",
  identifier_value: "8906241825082", // Tshepo Moreki test user (QA)
};

const testClientAccountNumberAlt = {
  identifier_key: "ACCOUNTNUMBER",
  identifier_value: "2409425514", // Tshepo Moreki test user (QA)
};

// Test merchant from QA environment
const testMerchant = "CapitecPayTest";

// Standard billing address for South Africa
const southAfricaBillingAddress = {
  address: {
    line1: "123 Main Street",
    line2: "Sandton",
    city: "Johannesburg",
    state: "Gauteng",
    zip: "2196",
    country: "ZA",
    first_name: "Veronica",
    last_name: "Fox",
  },
  phone: {
    number: "609603632",
    country_code: "+27",
  },
};

// Single use mandate data for once-off VRP consent
export const singleUseMandateDataZAR = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    single_use: {
      amount: 10000, // R100.00 in cents
      currency: "ZAR",
    },
  },
};

// Multi use mandate data for recurring VRP consent
export const multiUseMandateDataZAR = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    multi_use: {
      amount: 50000, // R500.00 in cents
      currency: "ZAR",
    },
  },
};

// Open banking payment method configuration for Capitec VRP
const openBankingCapitecEnabled = [
  {
    payment_method: "open_banking",
    payment_method_types: [
      {
        payment_method_type: "open_banking_capitec",
        payment_experience: "redirect_to_url",
        card_networks: null,
        accepted_currencies: ["ZAR"],
        accepted_countries: ["ZA"],
        minimum_amount: 100, // R1.00 minimum
        maximum_amount: 100000000, // R1,000,000 maximum
        recurring_enabled: true,
        installment_payment_enabled: false,
      },
    ],
  },
];

export const connectorDetails = {
  // Payment methods enabled for connector creation
  payment_methods_enabled: openBankingCapitecEnabled,

  // Open banking payment method configuration
  open_banking_pm: {
    // Payment Intent creation for open banking
    PaymentIntent: {
      Request: {
        currency: "ZAR",
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },

    // Once-off consent creation (single use mandate)
    OnceOffConsent: {
      Configs: {
        TRIGGER_SKIP: false, // Enabled for QA testing
      },
      Request: {
        payment_method: "open_banking",
        payment_method_type: "open_banking_capitec",
        payment_method_data: {
          open_banking: {
            open_banking_capitec: {
              client_identifier: testClientCellphone,
              minimum_amount: 5000, // R50.00
              maximum_amount: 15000, // R150.00
              product_description: "Test VRP Payment",
              beneficiary_statement_description: "Test Merchant",
              client_statement_description: "Test Purchase",
            },
          },
        },
        currency: "ZAR",
        payment_type: "setup_mandate", // Triggers SetupMandate flow for consent creation
        setup_future_usage: "off_session",
        mandate_data: singleUseMandateDataZAR,
        billing: southAfricaBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action", // Pending consent approval
        },
      },
    },

    // Recurring consent creation (multi use mandate)
    RecurringConsent: {
      Configs: {
        TRIGGER_SKIP: false, // Enabled for QA testing
      },
      Request: {
        payment_method: "open_banking",
        payment_method_type: "open_banking_capitec",
        payment_method_data: {
          open_banking: {
            open_banking_capitec: {
              client_identifier: testClientCellphone,
              minimum_amount: 1000, // R10.00
              maximum_amount: 50000, // R500.00
              product_description: "Monthly Subscription",
              recurrence: {
                first_payment_date: "2026-02-01",
                interval: "MONTHLY",
                occurrences: 12,
              },
            },
          },
        },
        currency: "ZAR",
        payment_type: "setup_mandate", // Triggers SetupMandate flow for consent creation
        setup_future_usage: "off_session",
        mandate_data: multiUseMandateDataZAR,
        billing: southAfricaBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action", // Pending consent approval
        },
      },
    },

    // Payment action after consent approval
    PaymentAction: {
      Configs: {
        TRIGGER_SKIP: false, // Enabled for QA testing
      },
      Request: {
        payment_method: "open_banking",
        payment_method_type: "open_banking_capitec",
        currency: "ZAR",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },

    // Sync consent status
    SyncConsent: {
      Configs: {
        TRIGGER_SKIP: false, // Enabled for QA testing
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action", // Or "authorized" after approval
        },
      },
    },

    // Revoke consent
    RevokeConsent: {
      Configs: {
        TRIGGER_SKIP: false, // Enabled for QA testing
      },
      Response: {
        status: 200,
        body: {
          status: "revoked",
        },
      },
    },

    // Once-off consent with ID Number identifier (Test Plan 3.1 TC1)
    OnceOffConsentIdNumber: {
      Configs: {
        TRIGGER_SKIP: false,
      },
      Request: {
        payment_method: "open_banking",
        payment_method_type: "open_banking_capitec",
        payment_method_data: {
          open_banking: {
            open_banking_capitec: {
              client_identifier: testClientIdNumber,
              minimum_amount: 5000,
              maximum_amount: 15000,
              product_description: "Test VRP Payment ID",
              beneficiary_statement_description: "Test Merchant",
              client_statement_description: "Test Purchase",
            },
          },
        },
        currency: "ZAR",
        payment_type: "setup_mandate", // Triggers SetupMandate flow for consent creation
        setup_future_usage: "off_session",
        mandate_data: singleUseMandateDataZAR,
        billing: southAfricaBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },

    // Once-off consent with Account Number identifier (Test Plan 3.1 TC1)
    OnceOffConsentAccountNumber: {
      Configs: {
        TRIGGER_SKIP: false,
      },
      Request: {
        payment_method: "open_banking",
        payment_method_type: "open_banking_capitec",
        payment_method_data: {
          open_banking: {
            open_banking_capitec: {
              client_identifier: testClientAccountNumber,
              minimum_amount: 5000,
              maximum_amount: 15000,
              product_description: "Test VRP Payment Account",
              beneficiary_statement_description: "Test Merchant",
              client_statement_description: "Test Purchase",
            },
          },
        },
        currency: "ZAR",
        payment_type: "setup_mandate", // Triggers SetupMandate flow for consent creation
        setup_future_usage: "off_session",
        mandate_data: singleUseMandateDataZAR,
        billing: southAfricaBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },

    // Recurring consent with DAILY interval (Test Plan 3.2 TC2)
    RecurringConsentDaily: {
      Configs: {
        TRIGGER_SKIP: false,
      },
      Request: {
        payment_method: "open_banking",
        payment_method_type: "open_banking_capitec",
        payment_method_data: {
          open_banking: {
            open_banking_capitec: {
              client_identifier: testClientCellphone,
              minimum_amount: 1000,
              maximum_amount: 10000,
              product_description: "Daily Subscription",
              recurrence: {
                first_payment_date: "2026-02-01",
                interval: "DAILY",
                occurrences: 30,
              },
            },
          },
        },
        currency: "ZAR",
        payment_type: "setup_mandate", // Triggers SetupMandate flow for consent creation
        setup_future_usage: "off_session",
        mandate_data: multiUseMandateDataZAR,
        billing: southAfricaBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },

    // Recurring consent with WEEKLY interval (Test Plan 3.2 TC2)
    RecurringConsentWeekly: {
      Configs: {
        TRIGGER_SKIP: false,
      },
      Request: {
        payment_method: "open_banking",
        payment_method_type: "open_banking_capitec",
        payment_method_data: {
          open_banking: {
            open_banking_capitec: {
              client_identifier: testClientCellphone,
              minimum_amount: 1000,
              maximum_amount: 25000,
              product_description: "Weekly Subscription",
              recurrence: {
                first_payment_date: "2026-02-01",
                interval: "WEEKLY",
                occurrences: 52,
              },
            },
          },
        },
        currency: "ZAR",
        payment_type: "setup_mandate", // Triggers SetupMandate flow for consent creation
        setup_future_usage: "off_session",
        mandate_data: multiUseMandateDataZAR,
        billing: southAfricaBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },

    // Recurring consent with FORTNIGHTLY interval (Test Plan 3.2 TC2)
    RecurringConsentFortnightly: {
      Configs: {
        TRIGGER_SKIP: false,
      },
      Request: {
        payment_method: "open_banking",
        payment_method_type: "open_banking_capitec",
        payment_method_data: {
          open_banking: {
            open_banking_capitec: {
              client_identifier: testClientCellphone,
              minimum_amount: 1000,
              maximum_amount: 30000,
              product_description: "Fortnightly Subscription",
              recurrence: {
                first_payment_date: "2026-02-01",
                interval: "FORTNIGHTLY",
                occurrences: 26,
              },
            },
          },
        },
        currency: "ZAR",
        payment_type: "setup_mandate", // Triggers SetupMandate flow for consent creation
        setup_future_usage: "off_session",
        mandate_data: multiUseMandateDataZAR,
        billing: southAfricaBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },

    // Recurring consent with BIANNUALLY interval (Test Plan 3.2 TC2)
    RecurringConsentBiannually: {
      Configs: {
        TRIGGER_SKIP: false,
      },
      Request: {
        payment_method: "open_banking",
        payment_method_type: "open_banking_capitec",
        payment_method_data: {
          open_banking: {
            open_banking_capitec: {
              client_identifier: testClientCellphone,
              minimum_amount: 10000,
              maximum_amount: 100000,
              product_description: "Biannual Subscription",
              recurrence: {
                first_payment_date: "2026-07-01",
                interval: "BIANNUALLY",
                occurrences: 4,
              },
            },
          },
        },
        currency: "ZAR",
        payment_type: "setup_mandate", // Triggers SetupMandate flow for consent creation
        setup_future_usage: "off_session",
        mandate_data: multiUseMandateDataZAR,
        billing: southAfricaBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },

    // Recurring consent with ANNUALLY interval (Test Plan 3.2 TC2)
    RecurringConsentAnnually: {
      Configs: {
        TRIGGER_SKIP: false,
      },
      Request: {
        payment_method: "open_banking",
        payment_method_type: "open_banking_capitec",
        payment_method_data: {
          open_banking: {
            open_banking_capitec: {
              client_identifier: testClientCellphone,
              minimum_amount: 50000,
              maximum_amount: 200000,
              product_description: "Annual Subscription",
              recurrence: {
                first_payment_date: "2026-01-01",
                interval: "ANNUALLY",
                occurrences: 3,
              },
            },
          },
        },
        currency: "ZAR",
        payment_type: "setup_mandate", // Triggers SetupMandate flow for consent creation
        setup_future_usage: "off_session",
        mandate_data: multiUseMandateDataZAR,
        billing: southAfricaBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
  },

  // Card payment method - not supported for Capitec VRP
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "ZAR",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    No3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true, // Cards not supported
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
          },
        },
      },
    },
    No3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true, // Cards not supported
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
          },
        },
      },
    },
    "3DSAutoCapture": {
      Configs: {
        TRIGGER_SKIP: true, // Cards not supported
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
          },
        },
      },
    },
    "3DSManualCapture": {
      Configs: {
        TRIGGER_SKIP: true, // Cards not supported
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
          },
        },
      },
    },
    Capture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
          },
        },
      },
    },
    PartialCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
          },
        },
      },
    },
    Void: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
          },
        },
      },
    },
    Refund: {
      Configs: {
        TRIGGER_SKIP: true, // Refunds not supported for VRP
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Refunds are not supported for Capitec VRP",
          },
        },
      },
    },
    PartialRefund: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Refunds are not supported for Capitec VRP",
          },
        },
      },
    },
    SyncRefund: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Refunds are not supported for Capitec VRP",
          },
        },
      },
    },
    manualPaymentRefund: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Refunds are not supported for Capitec VRP",
          },
        },
      },
    },
    manualPaymentPartialRefund: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Refunds are not supported for Capitec VRP",
          },
        },
      },
    },
    // Mandate flows - use open banking instead
    MandateSingleUseNo3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
          },
        },
      },
    },
    MandateSingleUseNo3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
          },
        },
      },
    },
    MandateMultiUseNo3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
          },
        },
      },
    },
    MandateMultiUseNo3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
          },
        },
      },
    },
    ZeroAuthMandate: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
          },
        },
      },
    },
    ZeroAuthPaymentIntent: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
          },
        },
      },
    },
    SaveCardUseNo3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
          },
        },
      },
    },
    SaveCardUseNo3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
          },
        },
      },
    },
    PaymentMethodIdMandateNo3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
          },
        },
      },
    },
    PaymentMethodIdMandateNo3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
          },
        },
      },
    },
    MITAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
          },
        },
      },
    },
    MITManualCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
          },
        },
      },
    },
  },
};
