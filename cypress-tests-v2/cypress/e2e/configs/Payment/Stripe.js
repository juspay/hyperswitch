export const connectorDetails = {
  revenue_recovery_pm: {
    RecordFailedScheduleRetry: {
      Request: {
        amount_details: {
          order_amount: 6540,
          currency: "USD",
        },
        merchant_reference_id: "cypress_rr_inv_failed_schedule",
        transaction_status: "failure",
        action: "schedule_failed_payment",
        payment_method_type: "card",
        payment_method_sub_type: "credit",
        connector_customer_id: "cust_cypress_rr_1",
        connector_transaction_id: "ch_cypress_rr_failed_1",
        payment_method_data: {
          primary_processor_payment_method_token: "pm_cypress_rr_token_1",
          payment_method_metadata: {
            last4: "4242",
            card_exp_month: "12",
            card_exp_year: "2030",
            card_network: "Visa",
          },
        },
        error: {
          code: "card_declined",
          message: "The card was declined.",
          reason: "insufficient_funds",
        },
      },
      Response: {
        status: 200,
        body: {
          intent_status: "failed",
        },
      },
    },
    RecordFailedForCancel: {
      Request: {
        amount_details: {
          order_amount: 4200,
          currency: "USD",
        },
        merchant_reference_id: "cypress_rr_inv_for_cancel",
        transaction_status: "failure",
        action: "schedule_failed_payment",
        payment_method_type: "card",
        payment_method_sub_type: "credit",
        connector_customer_id: "cust_cypress_rr_2",
        connector_transaction_id: "ch_cypress_rr_failed_2",
        payment_method_data: {
          primary_processor_payment_method_token: "pm_cypress_rr_token_2",
          payment_method_metadata: {
            last4: "4242",
            card_exp_month: "12",
            card_exp_year: "2030",
          },
        },
        error: {
          code: "card_declined",
          message: "The card was declined.",
        },
      },
      Response: {
        status: 200,
        body: {
          intent_status: "failed",
        },
      },
    },
    RecordChargedExternal: {
      Request: {
        amount_details: {
          order_amount: 5100,
          currency: "USD",
        },
        merchant_reference_id: "cypress_rr_inv_charged_external",
        transaction_status: "charged",
        action: "success_payment_external",
        payment_method_type: "card",
        payment_method_sub_type: "credit",
        connector_customer_id: "cust_cypress_rr_3",
        connector_transaction_id: "ch_cypress_rr_charged_1",
        payment_method_data: {
          primary_processor_payment_method_token: "pm_cypress_rr_token_3",
          payment_method_metadata: {
            last4: "4242",
            card_exp_month: "12",
            card_exp_year: "2030",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          intent_status: "succeeded",
        },
      },
    },
    RecordMissingRequiredFields: {
      Request: {
        amount_details: {
          currency: "USD",
        },
        transaction_status: "failure",
        action: "schedule_failed_payment",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
          },
        },
      },
    },
    RecordInvalidReferenceId: {
      Request: {
        amount_details: {
          order_amount: 1000,
          currency: "USD",
        },
        merchant_reference_id: "bad ref!!",
        transaction_status: "failure",
        action: "schedule_failed_payment",
        payment_method_type: "card",
        payment_method_sub_type: "credit",
        connector_customer_id: "cust_cypress_rr_bad_ref",
        connector_transaction_id: "ch_cypress_rr_bad_ref",
        payment_method_data: {
          primary_processor_payment_method_token: "pm_cypress_rr_token_bad",
          payment_method_metadata: {},
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            code: "IR_06",
          },
        },
      },
    },
    RecordUnknownMca: {
      Request: {
        amount_details: {
          order_amount: 1000,
          currency: "USD",
        },
        merchant_reference_id: "cypress_rr_inv_unknown_mca",
        billing_merchant_connector_id: "mca_unknown_does_not_exist",
        payment_merchant_connector_id: "mca_unknown_does_not_exist",
        transaction_status: "failure",
        action: "schedule_failed_payment",
        payment_method_type: "card",
        payment_method_sub_type: "credit",
        connector_customer_id: "cust_cypress_rr_unknown_mca",
        connector_transaction_id: "ch_cypress_rr_unknown_mca",
        payment_method_data: {
          primary_processor_payment_method_token: "pm_cypress_rr_token_x",
          payment_method_metadata: {},
        },
      },
      Response: {
        status: 404,
        body: {
          error: {
            type: "object_not_found",
            code: "HE_02",
          },
        },
      },
    },
    RecordMissingProfileHeader: {
      Request: {
        amount_details: {
          order_amount: 1000,
          currency: "USD",
        },
        merchant_reference_id: "cypress_rr_inv_no_profile_header",
        transaction_status: "failure",
        action: "schedule_failed_payment",
        payment_method_type: "card",
        payment_method_sub_type: "credit",
        connector_customer_id: "cust_cypress_rr_no_prof",
        connector_transaction_id: "ch_cypress_rr_no_prof",
        payment_method_data: {
          primary_processor_payment_method_token: "pm_cypress_rr_token_np",
          payment_method_metadata: {},
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            code: "IR_06",
          },
        },
      },
    },
    RecordUnauthorized: {
      Request: {
        amount_details: {
          order_amount: 1000,
          currency: "USD",
        },
        merchant_reference_id: "cypress_rr_inv_unauthorized",
        transaction_status: "failure",
        action: "schedule_failed_payment",
        payment_method_type: "card",
        payment_method_sub_type: "credit",
        connector_customer_id: "cust_cypress_rr_unauth",
        connector_transaction_id: "ch_cypress_rr_unauth",
        payment_method_data: {
          primary_processor_payment_method_token: "pm_cypress_rr_token_unauth",
          payment_method_metadata: {},
        },
      },
      Response: {
        status: 401,
        body: {
          error: {
            type: "invalid_request",
            code: "IR_01",
          },
        },
      },
    },
    WorkflowRetrieve: {
      Response: {
        status: 200,
        body: {},
      },
    },
    WorkflowRetrieveUnknown: {
      Response: {
        status: 404,
        body: {
          error: {
            type: "object_not_found",
            code: "HE_02",
          },
        },
      },
    },
    WorkflowRetrieveUnauthorized: {
      Response: {
        status: 401,
        body: {
          error: {
            type: "invalid_request",
            code: "IR_01",
          },
        },
      },
    },
    WorkflowNotScheduled: {
      Response: {
        status: 404,
        body: {
          error: {
            type: "object_not_found",
            code: "HE_02",
          },
        },
      },
    },
    ResumeCalculateWorkflow: {
      Request: {
        revenue_recovery_task: "CALCULATE_WORKFLOW",
        status: "new",
        business_status: "RETRY_SCHEDULED",
        retry_count: 1,
      },
      Response: {
        status: 200,
        body: {
          status: "new",
          business_status: "RETRY_SCHEDULED",
        },
      },
    },
    CancelSuccess: {
      Response: {
        status: 200,
        body: {
          intent_status: "cancelled",
        },
      },
    },
    CancelUnknown: {
      Response: {
        status: 404,
        body: {
          error: {
            type: "object_not_found",
            code: "HE_02",
          },
        },
      },
    },
    CancelIntentNotCancellable: {
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            code: "IR_16",
          },
        },
      },
    },
    CancelRetryInProgress: {
      Configs: {
        TRIGGER_SKIP: true,
        IGNORE: true,
      },
      Request: {
        skip_reason:
          "RetryInProgress requires an in-flight CALCULATE/EXECUTE task in process_started/processing; not deterministically expressible via recorded API calls alone",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            code: "IR_16",
          },
        },
      },
    },
    CancelAlreadyTerminal: {
      Configs: {
        TRIGGER_SKIP: true,
        IGNORE: true,
      },
      Request: {
        skip_reason:
          "AlreadyTerminal requires a CALCULATE task business_status in the terminal set (CALCULATE_WORKFLOW_FINISH/CANCELLED/RETRIES_EXCEEDED/FAILURE/GLOBAL_FAILURE); not deterministically expressible via recorded API calls alone",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            code: "IR_16",
          },
        },
      },
    },
    MaxRetryExhausted: {
      Configs: {
        TRIGGER_SKIP: true,
        IGNORE: true,
      },
      Request: {
        skip_reason:
          "Max retry count exhaustion requires repeated simulated failures past billing MCA max_retry_count and worker-driven transitions to RETRIES_EXCEEDED; not expressible via recorded API calls alone",
      },
      Response: {
        status: 200,
        body: {},
      },
    },
  },
};
