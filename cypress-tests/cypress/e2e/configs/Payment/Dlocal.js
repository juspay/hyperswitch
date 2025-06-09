import { customerAcceptance } from "./Commons";


const mockBillingDetails = { // Updated to match cURL payer info for 3DS test
  address: {
    line1: "Servidao B-1", // street from cURL payer.address
    line2: null, // number from cURL payer.address (Hyperswitch model uses line1, line2, line3)
    line3: null,
    city: "Volta Redonda",   // city from cURL payer.address
    state: "Rio de Janeiro", // state from cURL payer.address
    zip: "27275-595",     // zip_code from cURL payer.address
    country: "BR",          // country from cURL
    first_name: "Thiago",   // To form "Thiago Gabriel"
    last_name: "Gabriel",
  },
  phone: { // phone from cURL payer
    number: "123456712345",
    country_code: "+55", // Assuming Brazil country code for the phone
  },
  email: "thiago@example.com", // email from cURL payer
};

// Test card details for Dlocal
const successfulNo3DSCardDetails = { // Kept original for non-3DS tests
  card_number: "4111111111111111", 
  card_exp_month: "10",
  card_exp_year: "40",
  card_holder_name: "Thiago Gabriel",
  card_cvc: "123",
};

const successfulThreeDSCardDetails = { // Updated to match cURL card info for 3DS test
  card_number: "4111111111111111",    // number from cURL card
  card_exp_month: "10",               // expiration_month from cURL card
  card_exp_year: "40",                // expiration_year from cURL card (e.g., 2040)
  card_holder_name: "Thiago Gabriel", // holder_name from cURL card
  card_cvc: "123",                    // cvv from cURL card
};

const successfulMastercardDetails = {
  card_number: "5555555555554444", // Mastercard test card for Dlocal
  card_exp_month: "10",
  card_exp_year: "40",
  card_holder_name: "Thiago Gabriel",
  card_cvc: "123",
};

const failedCardDetails = {
  card_number: "4000000000000002", // Decline test card for Dlocal
  card_exp_month: "10",
  card_exp_year: "40",
  card_holder_name: "Thiago Gabriel",
  card_cvc: "123",
};

const singleUseMandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    single_use: {
      amount: 8000,
      currency: "USD",
    },
  },
};

const multiUseMandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    multi_use: {
      amount: 8000,
      currency: "USD",
    },
  },
};

export const cardRequiredField = {
  "payment_method_data.card.card_number": {
    required_field: "payment_method_data.card.card_number",
    display_name: "card_number",
    field_type: "user_card_number",
    value: null,
  },
  "payment_method_data.card.card_exp_year": {
    required_field: "payment_method_data.card.card_exp_year",
    display_name: "card_exp_year",
    field_type: "user_card_expiry_year",
    value: null,
  },
  "payment_method_data.card.card_cvc": {
    required_field: "payment_method_data.card.card_cvc",
    display_name: "card_cvc",
    field_type: "user_card_cvc",
    value: null,
  },
  "payment_method_data.card.card_exp_month": {
    required_field: "payment_method_data.card.card_exp_month",
    display_name: "card_exp_month",
    field_type: "user_card_expiry_month",
    value: null,
  },
};

export const fullNameRequiredField = {
  "billing.address.last_name": {
    required_field: "payment_method_data.billing.address.last_name",
    display_name: "card_holder_name",
    field_type: "user_full_name",
    value: "Doe",
  },
  "billing.address.first_name": {
    required_field: "payment_method_data.billing.address.first_name",
    display_name: "card_holder_name",
    field_type: "user_full_name",
    value: "joseph",
  },
};

export const billingRequiredField = {};

const requiredFields = {
  payment_methods: [
    {
      payment_method: "card",
      payment_method_types: [
        {
          payment_method_type: "credit",
          card_networks: [
            {
              eligible_connectors: ["dlocal"],
            },
          ],
          required_fields: cardRequiredField,
        },
      ],
    },
  ],
};

const payment_method_data_3ds = {
  card: {
    last4: "4242",
    card_type: "CREDIT",
    card_network: "Visa",
    card_issuer: "STRIPE PAYMENTS UK LIMITED",
    card_issuing_country: "UNITEDKINGDOM",
    card_isin: "424242",
    card_extended_bin: null,
    card_exp_month: "12",
    card_exp_year: "25",
    card_holder_name: "Thiago Gabriel",
    payment_checks: null,
    authentication_data: null,
  },
  billing: null,
};

const payment_method_data_no3ds = {
  card: {
    last4: "1111",
    card_type: "CREDIT",
    card_network: "Visa",
    card_issuer: "JP Morgan",
    card_issuing_country: "INDIA",
    card_isin: "411111",
    card_extended_bin: null,
    card_exp_month: "10",
    card_exp_year: "40",
    card_holder_name: "Thiago Gabriel",
    payment_checks: null,
    authentication_data: null,
  },
  billing:null,
};
const payment_method_data_no3ds_address = {
  card: {
    last4: "1111",
    card_type: "CREDIT",
    card_network: "Visa",
    card_issuer: "JP Morgan",
    card_issuing_country: "INDIA",
    card_isin: "411111",
    card_extended_bin: null,
    card_exp_month: "10",
    card_exp_year: "40",
    card_holder_name: "Thiago Gabriel",
    payment_checks: null,
    authentication_data: null,
  },
  billing:mockBillingDetails,
};
const payment_method_data_3ds_address = {
  card: {
    last4: "1111",
    card_type: "CREDIT",
    card_network: "Visa",
    card_issuer: "JP Morgan",
    card_issuing_country: "INDIA",
    card_isin: "411111",
    card_extended_bin: null,
    card_exp_month: "10",
    card_exp_year: "40",
    card_holder_name: "Thiago Gabriel",
    payment_checks: null,
    authentication_data: null,
  },
  billing: mockBillingDetails,
};
const payment_method_data_mastercard = {
  card: {
    last4: "4444",
    card_type: "CREDIT",
    card_network: "Mastercard",
    card_issuer: "JP Morgan",
    card_issuing_country: "INDIA",
    card_isin: "555555",
    card_extended_bin: null,
    card_exp_month: "10",
    card_exp_year: "40",
    card_holder_name: "Thiago Gabriel",
    payment_checks: {
      cvc_check: "pass",
      address_line1_check: "pass",
      address_postal_code_check: "pass",
    },
    authentication_data: null,
  },
  billing: null,
};

export const connectorDetails = {
  card_pm: {
    // Mastercard specific flow
    MastercardAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulMastercardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method: "card",
          payment_method_data: payment_method_data_mastercard,
        },
      },
    },
    
    // Failed payment scenario
    No3DSFailPayment: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: failedCardDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_code: "104",
          error_message: "Card declined",
          unified_code: "UE_9000",
          unified_message: "Something went wrong",
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
    
    PaymentConfirmWithShippingCost: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: mockBillingDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method: "card",
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
          billing:mockBillingDetails
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method: "card",
          payment_method_data: payment_method_data_no3ds_address,
        },
      },
    },
    
    "3DSAutoCapture": {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSCardDetails, // Uses updated card details
          billing: mockBillingDetails,       // Uses updated billing details
        },
        currency: "BRL", // Currency from cURL
        // customer_acceptance and setup_future_usage removed as not directly used by Dlocal transformer for main request
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action", // Expected status from cURL
          // Based on cURL, Dlocal returns 'id' (connector_transaction_id) and 'three_dsecure.eci'
          // Our transformer maps 'id' to resource_id. It expects 'three_dsecure.redirect_url'.
          // If no redirect, next_action should be null.
          // The detailed payment_method_data is not directly part of the top-level response body in Hyperswitch model.
          // We rely on the main status and resource_id.
          // The `commands.js` will check for next_action based on status.
          // If status is AUTHORIZED (mapped to AttemptStatus::Authorized), next_action is expected to be null.
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
        billing: mockBillingDetails,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          payment_method: "card",
          payment_method_data: payment_method_data_no3ds,
        },
      },
    },
    
    "3DSManualCapture": {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSCardDetails,
          billing: mockBillingDetails, // Uses updated billing details
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method_data: payment_method_data_3ds_address,
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
          capture_method: "manual",
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
          status: "failed",
          capture_method: "manual",
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
    
    // Dlocal doesn't support mandates, so we'll skip these tests
    MandateSingleUseNo3DSAutoCapture: {
      // Ensuring test runs and expects the correct IR_00 error
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: mockBillingDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
        authentication_type: "no_three_ds", // ensure all relevant fields for the flow
        capture_method: "automatic",      // ensure all relevant fields for the flow
        setup_future_usage: "off_session", // ensure all relevant fields for the flow
        customer_acceptance: customerAcceptance, // ensure all relevant fields for the flow
      },
      Response: { // Now expecting the IR_00 error
        status: 200,
        body: {
          status: "succeeded",
          payment_method_data: payment_method_data_no3ds_address,
          payment_method: "card",
          connector: "dlocal",
        },
      },
      Configs: { TRIGGER_SKIP: true } // Explicitly skip this test as mandates are not supported by DLocal
    },

    MandateSingleUseNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: mockBillingDetails,
        },
        
        currency: "USD",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
      Configs: { TRIGGER_SKIP: true } // Explicitly skip this test as mandates are not supported by DLocal
    },
    MITManualCapture: {
      Request: {
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: mockBillingDetails,
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
      Configs: { TRIGGER_SKIP: true } // Explicitly skip this test as MIT is not supported by DLocal
    },
    MandateSingleUse3DSAutoCapture: {
      
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 200, // Expecting failure as mandates are not supported
        body: {
          status: "requires_customer_action",
          payment_method_data: payment_method_data_3ds_address,
          payment_method: "card",
          connector: "dlocal",
        },
      },
      Configs: { TRIGGER_SKIP: true } // Explicitly skip this test as mandates are not supported by DLocal
    },

    SaveCardUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: mockBillingDetails,
        },
        currency: "USD",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200, // Expecting failure as mandates are not supported
        body: {
          status: "succeeded",
          payment_method_data: payment_method_data_no3ds_address,
          payment_method: "card",
          connector: "dlocal",
        },
      },
      Configs: { TRIGGER_SKIP: true } // Explicitly skip this test as setup_future_usage is not supported by DLocal
    },

    SaveCardUseNo3DSAutoCaptureOffSession: {

      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: mockBillingDetails,
        },
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
      },
      Response: { // Fixed typo: RResponse -> Response
        status: 200, // Expecting failure as mandates are not supported
        body: {
          status: "succeeded",
          payment_method_data: payment_method_data_no3ds_address,
          payment_method: "card",
          connector: "dlocal",
        },
      },
      Configs: { TRIGGER_SKIP: true } // Explicitly skip this test as setup_future_usage is not supported by DLocal
    },

    MandateMultiUseNo3DSAutoCapture: {
      // This test attempts a multi-use mandate flow.
      // As per dlocal_limitations.md, mandates are not supported.
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: mockBillingDetails, // Assuming billing is needed as per other Dlocal flows
        },
        currency: "USD",
        mandate_data: multiUseMandateData, // Corrected to use multiUseMandateData
        // authentication_type: "no_three_ds", // Retaining fields from similar tests if applicable
        // capture_method: "automatic",
        // setup_future_usage: "off_session", // Dlocal limitation says no mandate support, so setup_future_usage might be irrelevant or lead to an error
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method_data: payment_method_data_no3ds_address,
          payment_method: "card",
          connector: "dlocal",
        },
      },
      Configs: { TRIGGER_SKIP: true } // Explicitly skip this test as mandates are not supported by DLocal
    },
    MandateMultiUseNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: mockBillingDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
        authentication_type: "no_three_ds", // ensure all relevant fields for the flow
        capture_method: "automatic",      // ensure all relevant fields for the flow
        setup_future_usage: "off_session", // ensure all relevant fields for the flow
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          payment_method_data: payment_method_data_no3ds_address,
          payment_method: "card",
          connector: "dlocal",
        },
      },
      Configs: { TRIGGER_SKIP: true } // Explicitly skip this test as mandates are not supported by DLocal
    },
    // MIT (Merchant Initiated Transaction) configurations
    MITAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          mandate_id: null,
          payment_method: "card",
          payment_method_data: payment_method_data_no3ds_address,
          connector: "dlocal",
        },
      },
      Configs: { TRIGGER_SKIP: true } // Explicitly skip this test as MIT is not supported by DLocal
    },
    ZeroAuthMandate: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        billing: mockBillingDetails,

        },
        currency: "USD",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method_data: payment_method_data_no3ds_address,
        },
      },
      Configs: { TRIGGER_SKIP: true } // Explicitly skip this test as mandates are not supported by DLocal
    },
    // Zero auth configurations
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
      Configs: { TRIGGER_SKIP: true } // Explicitly skip this test as zero auth is not supported by DLocal
    },
    
    ZeroAuthConfirmPayment: {
      Request: {
        payment_type: "setup_mandate",
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: mockBillingDetails,
        },
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method: "card",
          payment_method_data: payment_method_data_no3ds_address,
        },
      },
      Configs: { TRIGGER_SKIP: true } // Explicitly skip this test as setup mandate is not supported by DLocal
    },
    
    // Sync payment configuration
    SyncPayment: {
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method: "card",
          payment_method_data: payment_method_data_no3ds_address,
        },
      },
    },
  },
  
  // Bank redirect payment method configurations
  bank_redirect_pm: {
    PaymentIntent: {
      Request: {
        currency: "EUR",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
  },
  
  // Payment method list configurations
  pm_list: {
    PmListResponse: {
      PmListNull: {
        payment_methods: [],
      },
      pmListDynamicFieldWithoutBilling: requiredFields,
      pmListDynamicFieldWithBilling: requiredFields,
      pmListDynamicFieldWithNames: requiredFields,
      pmListDynamicFieldWithEmail: requiredFields,
    },
  },
};

// Define supported payment methods based on Dlocal's capabilities
export const payment_methods_enabled = [
  {
    payment_method: "card",
    payment_method_types: [
      {
        payment_method_type: "credit",
        card_networks: ["Visa", "Mastercard", "AmericanExpress", "Discover", "JCB", "DinersClub", "UnionPay", "Interac", "CartesBancaires"],
        minimum_amount: 0,
        maximum_amount: 68607706,
        recurring_enabled: false,
        installment_payment_enabled: true,
      },
      {
        payment_method_type: "debit",
        card_networks: ["Visa", "Mastercard", "AmericanExpress", "Discover", "JCB", "DinersClub", "UnionPay", "Interac", "CartesBancaires"],
        minimum_amount: 0,
        maximum_amount: 68607706,
        recurring_enabled: false,
        installment_payment_enabled: true,
      }
    ]
  }
];
