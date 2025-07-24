import { getCustomExchange } from "./Modifiers";

const billing = {
  address: {
    line1: "1467",
    line2: "Harrison Street",
    line3: "Harrison Street",
    city: "San Fransico",
    state: "CA",
    zip: "94122",
    country: "US",
    first_name: "John",
    last_name: "Doe",
  },
};

const browser_info = {
  user_agent:
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/70.0.3538.110 Safari/537.36",
  accept_header:
    "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8",
  language: "nl-NL",
  color_depth: 24,
  screen_height: 723,
  screen_width: 1536,
  time_zone: 0,
  java_enabled: true,
  java_script_enabled: true,
  ip_address: "127.0.0.1",
};

const successfulNoThreeDsCardDetailsRequest = {
  card_number: "4242424242424242",
  card_exp_month: "10",
  card_exp_year: "30",
  card_holder_name: "morino",
  card_cvc: "737",
};

const successfulThreeDsTestCardDetailsRequest = {
  card_number: "4000000000001091",
  card_exp_month: "10",
  card_exp_year: "30",
  card_holder_name: "morino",
  card_cvc: "737",
};

const failedNoThreeDsCardDetails = {
  card_number: "4242424242424242",
  card_exp_month: "10",
  card_exp_year: "30",
  card_holder_name: "REFUSED13",
  card_cvc: "737",
};

const paymentMethodDataNoThreeDsResponse = {
  card: {
    last4: "4242",
    card_type: "CREDIT",
    card_network: "Visa",
    card_issuer: "STRIPE PAYMENTS UK LIMITED",
    card_issuing_country: "UNITEDKINGDOM",
    card_isin: "424242",
    card_extended_bin: null,
    card_exp_month: "10",
    card_exp_year: "30",
    card_holder_name: "morino",
    payment_checks: null,
    authentication_data: null,
  },
  billing: null,
};

const paymentMethodDataThreeDsResponse = {
  card: {
    last4: "1091",
    card_type: "CREDIT",
    card_network: "Visa",
    card_issuer: "INTL HDQTRS-CENTER OWNED",
    card_issuing_country: "UNITEDSTATES",
    card_isin: "400000",
    card_extended_bin: null,
    card_exp_month: "10",
    card_exp_year: "30",
    card_holder_name: "morino",
    payment_checks: null,
    authentication_data: null,
  },
  billing: null,
};

const customerAcceptance = {
  acceptance_type: "offline",
  accepted_at: "1963-05-03T04:07:52.723Z",
  online: {
    ip_address: "125.0.0.1",
    user_agent: "amet irure esse",
  },
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

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "USD",
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
    No3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNoThreeDsCardDetailsRequest,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: billing,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          payment_method: "card",
          payment_method_type: "credit",
          attempt_count: 1,
          payment_method_data: paymentMethodDataNoThreeDsResponse,
        },
      },
    },
    No3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNoThreeDsCardDetailsRequest,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method: "card",
          payment_method_type: "credit",
          attempt_count: 1,
          payment_method_data: paymentMethodDataNoThreeDsResponse,
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
        },
      },
    },
    PartialCapture: {
      Request: {
        amount_to_capture: 2000,
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulNoThreeDsCardDetailsRequest,
        },
        currency: "USD",
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "partially_captured",
          amount: 6000,
          amount_capturable: 0,
        },
      },
    },
    Void: getCustomExchange({
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "cancelled",
        },
      },
      ResponseCustom: {
        body: {
          type: "invalid_request",
          message:
            "You cannot cancel this payment because it has status processing",
          code: "IR_16",
        },
      },
    }),
    VoidAfterConfirm: {
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    SaveCardUseNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNoThreeDsCardDetailsRequest,
        },
        currency: "USD",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        body: {
          status: "requires_capture",
        },
      },
    },
    SaveCardUseNo3DSManualCaptureOffSession: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNoThreeDsCardDetailsRequest,
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
    SaveCardConfirmManualCaptureOffSession: {
      Request: {
        setup_future_usage: "off_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    SaveCardUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNoThreeDsCardDetailsRequest,
        },
        currency: "USD",
        setup_future_usage: "on_session",
        browser_info,
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    SaveCardUseNo3DSAutoCaptureOffSession: {
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulNoThreeDsCardDetailsRequest,
        },
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    SaveCardUse3DSAutoCaptureOffSession: {
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulThreeDsTestCardDetailsRequest,
        },
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
    "3DSManualCapture": {
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulThreeDsTestCardDetailsRequest,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
        browser_info,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          setup_future_usage: "on_session",
          payment_method_data: paymentMethodDataThreeDsResponse,
        },
      },
    },
    "3DSAutoCapture": {
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulThreeDsTestCardDetailsRequest,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        browser_info,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          setup_future_usage: "on_session",
          payment_method_data: paymentMethodDataThreeDsResponse,
        },
      },
    },
    CaptureCapturedAmount: {
      Request: {
        Request: {
          payment_method: "card",
          payment_method_data: {
            card: successfulNoThreeDsCardDetailsRequest,
          },
          currency: "EUR",
          customer_acceptance: null,
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "This Payment could not be captured because it has a payment.status of succeeded. The expected state is requires_capture, partially_captured_and_capturable, processing",
            code: "IR_14",
          },
        },
      },
    },
    ConfirmSuccessfulPayment: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNoThreeDsCardDetailsRequest,
        },
        customer_acceptance: null,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "You cannot confirm this payment because it has status succeeded",
            code: "IR_16",
          },
        },
      },
    },
    Refund: {
      Request: {
        amount: 6000,
      },
      Response: {
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
        body: {
          status: "succeeded",
        },
      },
    },
    SyncRefund: {
      Response: {
        body: {
          status: "succeeded",
        },
      },
    },
    MandateSingleUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNoThreeDsCardDetailsRequest,
        },
        currency: "USD",
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
        payment_method: "card",
        payment_method_data: {
          card: successfulNoThreeDsCardDetailsRequest,
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
    },
    MandateMultiUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNoThreeDsCardDetailsRequest,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
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
        payment_method_data: {
          card: successfulNoThreeDsCardDetailsRequest,
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
    },
    PaymentIntentOffSession: {
      Request: {
        amount: 6000,
        authentication_type: "no_three_ds",
        currency: "USD",
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
          card: successfulNoThreeDsCardDetailsRequest,
        },
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
          net_amount: 6050,
        },
      },
    },
    ZeroAuthMandate: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNoThreeDsCardDetailsRequest,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
      },
      Response: {
        trigger_skip: true,
        status: 200,
        body: {
          error_code: "boardingError",
          error_message:
            "There has been a problem with your boarding and you cannot use this API yet, please contact support.",
          status: "failed",
          payment_method_id: null,
        },
      },
    },
    ZeroAuthPaymentIntent: {
      Request: {
        amount: 0,
        setup_future_usage: "off_session",
        currency: "USD",
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
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulNoThreeDsCardDetailsRequest,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          error_code: "boardingError",
          error_message:
            "There has been a problem with your boarding and you cannot use this API yet, please contact support.",
          status: "failed",
          payment_method_id: null,
        },
      },
    },
    No3DSFailPayment: {
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: failedNoThreeDsCardDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_code: "13",
          error_message: "INVALID AMOUNT",
          unified_code: "UE_9000",
          unified_message: "Something went wrong",
        },
      },
    },
    PaymentMethodIdMandateNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNoThreeDsCardDetailsRequest,
        },
        currency: "USD",
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
    PaymentMethodIdMandateNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNoThreeDsCardDetailsRequest,
        },
        currency: "USD",
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
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDsTestCardDetailsRequest,
        },
        currency: "USD",
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
    PaymentMethodIdMandate3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDsTestCardDetailsRequest,
        },
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
    DDCRaceConditionServerSide: {
      ...getCustomExchange({
        Request: {
          payment_method: "card",
          payment_method_type: "debit",
          payment_method_data: {
            card: successfulThreeDsTestCardDetailsRequest,
          },
          currency: "USD",
          customer_acceptance: null,
          setup_future_usage: "on_session",
          browser_info,
        },
        Response: {
          status: 200,
          body: {
            status: "requires_customer_action",
          },
        },
      }),
      DDCConfig: {
        completeUrlPath: "/redirect/complete/worldpay",
        collectionReferenceParam: "collectionReference",
        firstSubmissionValue: "",
        secondSubmissionValue: "race_condition_test_ddc_123",
        expectedError: {
          status: 400,
          body: {
            error: {
              code: "IR_07",
              type: "invalid_request",
              message:
                "Invalid value provided: collection_reference not allowed in AuthenticationPending state",
            },
          },
        },
      },
    },
    DDCRaceConditionClientSide: {
      ...getCustomExchange({
        Request: {
          payment_method: "card",
          payment_method_type: "debit",
          payment_method_data: {
            card: successfulThreeDsTestCardDetailsRequest,
          },
          currency: "USD",
          customer_acceptance: null,
          setup_future_usage: "on_session",
          browser_info,
        },
        Response: {
          status: 200,
          body: {
            status: "requires_customer_action",
          },
        },
      }),
      DDCConfig: {
        redirectUrlPath: "/payments/redirect",
        collectionReferenceParam: "collectionReference",
        delayBeforeSubmission: 2000,
        raceConditionScript: `
          <script>
            console.log("INJECTING_RACE_CONDITION_TEST");
            
            // Track submission attempts and ddcProcessed flag behavior
            window.testResults = {
              submissionAttempts: 0,
              actualSubmissions: 0,
              blockedSubmissions: 0
            };
            
            // Override the submitCollectionReference function to test race conditions
            var originalSubmit = window.submitCollectionReference;
            
            window.submitCollectionReference = function(collectionReference) {
              window.testResults.submissionAttempts++;
              console.log("SUBMISSION_ATTEMPT_" + window.testResults.submissionAttempts + ": " + collectionReference);
              
              // Check if ddcProcessed flag would block this
              if (window.ddcProcessed) {
                window.testResults.blockedSubmissions++;
                console.log("SUBMISSION_BLOCKED_BY_DDC_PROCESSED_FLAG");
                return;
              }
              
              window.testResults.actualSubmissions++;
              console.log("SUBMISSION_PROCEEDING: " + collectionReference);
              
              if (originalSubmit) {
                return originalSubmit(collectionReference);
              }
            };
            
            // Submit first value at configured timing
            setTimeout(function() {
              console.log("FIRST_SUBMISSION_TRIGGERED_AT_100MS");
              window.submitCollectionReference("");
            }, 100);
            
            // Submit second value at configured timing (should be blocked)
            setTimeout(function() {
              console.log("SECOND_SUBMISSION_ATTEMPTED_AT_200MS");
              window.submitCollectionReference("test_ddc_123");
            }, 200);
          </script>
        `,
      },
    },
  },
  pm_list: {
    PmListResponse: {
      pmListDynamicFieldWithoutBilling: {
        payment_methods: [
          {
            payment_method: "card",
            payment_method_types: [
              {
                payment_method_type: "credit",
                card_networks: [{ eligible_connectors: ["worldpay"] }],
                required_fields: {
                  "billing.address.first_name": {
                    required_field:
                      "payment_method_data.billing.address.first_name",
                    display_name: "card_holder_name",
                    field_type: "user_full_name",
                    value: null,
                  },
                  "payment_method_data.card.card_exp_year": {
                    required_field: "payment_method_data.card.card_exp_year",
                    display_name: "card_exp_year",
                    field_type: "user_card_expiry_year",
                    value: null,
                  },
                  "payment_method_data.card.card_number": {
                    required_field: "payment_method_data.card.card_number",
                    display_name: "card_number",
                    field_type: "user_card_number",
                    value: null,
                  },
                  "payment_method_data.card.card_exp_month": {
                    required_field: "payment_method_data.card.card_exp_month",
                    display_name: "card_exp_month",
                    field_type: "user_card_expiry_month",
                    value: null,
                  },
                },
              },
            ],
          },
        ],
      },
      pmListDynamicFieldWithBilling: {
        payment_methods: [
          {
            payment_method: "card",
            payment_method_types: [
              {
                payment_method_type: "credit",
                card_networks: [{ eligible_connectors: ["worldpay"] }],
                required_fields: {
                  "payment_method_data.card.card_number": {
                    required_field: "payment_method_data.card.card_number",
                    display_name: "card_number",
                    field_type: "user_card_number",
                    value: null,
                  },
                  "billing.address.first_name": {
                    required_field:
                      "payment_method_data.billing.address.first_name",
                    display_name: "card_holder_name",
                    field_type: "user_full_name",
                    value: "joseph",
                  },
                  "payment_method_data.card.card_exp_month": {
                    required_field: "payment_method_data.card.card_exp_month",
                    display_name: "card_exp_month",
                    field_type: "user_card_expiry_month",
                    value: null,
                  },
                  "payment_method_data.card.card_exp_year": {
                    required_field: "payment_method_data.card.card_exp_year",
                    display_name: "card_exp_year",
                    field_type: "user_card_expiry_year",
                    value: null,
                  },
                },
              },
            ],
          },
        ],
      },
      pmListDynamicFieldWithNames: {
        payment_methods: [
          {
            payment_method: "card",
            payment_method_types: [
              {
                payment_method_type: "credit",
                card_networks: [
                  {
                    eligible_connectors: ["worldpay"],
                  },
                ],
                required_fields: {
                  "billing.address.first_name": {
                    required_field:
                      "payment_method_data.billing.address.first_name",
                    display_name: "card_holder_name",
                    field_type: "user_full_name",
                    value: "joseph",
                  },
                  "payment_method_data.card.card_exp_year": {
                    required_field: "payment_method_data.card.card_exp_year",
                    display_name: "card_exp_year",
                    field_type: "user_card_expiry_year",
                    value: null,
                  },
                  "payment_method_data.card.card_number": {
                    required_field: "payment_method_data.card.card_number",
                    display_name: "card_number",
                    field_type: "user_card_number",
                    value: null,
                  },
                  "payment_method_data.card.card_exp_month": {
                    required_field: "payment_method_data.card.card_exp_month",
                    display_name: "card_exp_month",
                    field_type: "user_card_expiry_month",
                    value: null,
                  },
                },
              },
            ],
          },
        ],
      },
      pmListDynamicFieldWithEmail: {
        payment_methods: [
          {
            payment_method: "card",
            payment_method_types: [
              {
                payment_method_type: "credit",
                card_networks: [
                  {
                    eligible_connectors: ["worldpay"],
                  },
                ],
                required_fields: {
                  "billing.address.first_name": {
                    required_field:
                      "payment_method_data.billing.address.first_name",
                    display_name: "card_holder_name",
                    field_type: "user_full_name",
                    value: "joseph",
                  },
                  "payment_method_data.card.card_exp_year": {
                    required_field: "payment_method_data.card.card_exp_year",
                    display_name: "card_exp_year",
                    field_type: "user_card_expiry_year",
                    value: null,
                  },
                  "payment_method_data.card.card_number": {
                    required_field: "payment_method_data.card.card_number",
                    display_name: "card_number",
                    field_type: "user_card_number",
                    value: null,
                  },
                  "payment_method_data.card.card_exp_month": {
                    required_field: "payment_method_data.card.card_exp_month",
                    display_name: "card_exp_month",
                    field_type: "user_card_expiry_month",
                    value: null,
                  },
                },
              },
            ],
          },
        ],
      },
    },
  },
};
