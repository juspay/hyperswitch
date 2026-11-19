const billing = {
  address: {
    line1: "123 Main St",
    city: "Cape Town",
    state: "Western Cape",
    zip: "8001",
    country: "ZA",
    first_name: "John",
    last_name: "Doe",
  },
};

/*
 * Sensitive payout bank details (bank_account_number, account_holder_name,
 * bank_name, shap_id) are intentionally NOT defined here. They are injected at
 * runtime from the gitignored `creds.json` (`<connector>_payout` ->
 * `payout_bank_transfer`) by `injectGotymePayoutBankTransfer` in
 * `cypress/e2e/configs/Payout/Utils.js`. The configs below only declare WHICH
 * payout_method_type is used.
 */
export const connectorDetails = {
  bank_transfer_pm: {
    payshap: {
      Create: {
        Request: {
          amount: 1000,
          currency: "ZAR",
          payout_type: "bank",
          description: "Test Payout",
          billing: billing,
        },
        Response: {
          status: 200,
          body: {
            status: "requires_payout_method_data",
            payout_type: "bank",
          },
        },
      },
      Confirm: {
        Request: {
          amount: 1000,
          currency: "ZAR",
          payout_type: "bank",
          payout_method_data: {
            bank_transfer: {
              payout_method_type: "payshap",
            },
          },
          billing: billing,
        },
        Response: {
          status: 200,
          body: {
            status: "requires_fulfillment",
            payout_type: "bank",
            connector: "gotyme_sanlam",
          },
        },
      },
      Fulfill: {
        Response: {
          status: 200,
          body: {
            status: "initiated",
            amount: 1000,
          },
        },
      },
      RetrieveAfterFulfill: {
        Response: {
          status: 200,
          body: {
            status: "initiated",
          },
        },
      },
    },
    /*
     * FRM (pre-FRM) scenarios for the sanlam_payshield -> gotyme_sanlam
     * payout chain (see cypress/e2e/spec/Payout/00009-PayoutFrm.cy.js).
     * The Payshield sandbox flags an amount over 1,000,000 cents as fraud;
     * frm_message details (frm_status, frm_score, frm_error) are asserted
     * separately via cy.verifyPayoutFrmDetails since they contain
     * non-deterministic ids and can't be part of a fixed deep-equal
     * Response body.
     *
     * These scenarios run create+confirm in a single call, so
     * payout_method_data has to be present on the request (unlike the
     * two-step `payshap` scenario above, where it's only added on Confirm).
     * As with `payshap`, only payout_method_type is declared here — the
     * actual bank account fields are injected at request time by
     * injectGotymePayoutBankTransfer (see the file-level comment above).
     */
    // These frm_* scenarios run create+confirm in a single call (no
    // separate Confirm/Fulfill step), so Request/Response sit directly on
    // the scenario instead of nested under a Create key.
    frm_legit: {
      Request: {
        amount: 1000,
        currency: "ZAR",
        payout_type: "bank",
        description: "Test Payout",
        payout_method_data: {
          bank_transfer: {
            payout_method_type: "payshap",
          },
        },
        billing: billing,
      },
      Response: {
        status: 200,
        body: {
          payout_type: "bank",
          status: "initiated",
          connector: "gotyme_sanlam",
        },
      },
      // Asserted via cy.verifyPayoutFrmDetails against a follow-up GET, since
      // frm_message carries non-deterministic ids and can't be part of the
      // fixed deep-equal Response.body above.
      FrmDetails: {
        frm_message: { frm_name: "sanlam_payshield", frm_status: "legit" },
        status: "initiated",
        connector: "gotyme_sanlam",
      },
    },
    frm_fraud: {
      Request: {
        amount: 10000000,
        currency: "ZAR",
        payout_type: "bank",
        description: "Test Payout",
        payout_method_data: {
          bank_transfer: {
            payout_method_type: "payshap",
          },
        },
        billing: billing,
      },
      Response: {
        status: 200,
        body: {
          payout_type: "bank",
          connector: null,
          merchant_connector_id: null,
          status: "failed",
          error_code: "fraud",
        },
      },
      FrmDetails: {
        status: "failed",
        error_code: "fraud",
        connector: null,
        frm_message: { frm_status: "fraud" },
        greaterThan: { "frm_message.frm_score": 0 },
      },
    },
    frm_transaction_failure_fail_closed: {
      Request: {
        amount: 1000,
        currency: "ZAR",
        payout_type: "bank",
        description: "Test Payout",
        payout_method_data: {
          bank_transfer: {
            payout_method_type: "payshap",
          },
        },
        billing: billing,
      },
      Response: {
        status: 200,
        body: {
          payout_type: "bank",
          connector: null,
          merchant_connector_id: null,
          status: "failed",
          error_code: "transaction_failure",
        },
      },
      FrmDetails: {
        status: "failed",
        error_code: "transaction_failure",
        connector: null,
        frm_message: { frm_status: "transaction_failure" },
      },
    },
    frm_transaction_failure_fail_open: {
      Request: {
        amount: 1000,
        currency: "ZAR",
        payout_type: "bank",
        description: "Test Payout",
        payout_method_data: {
          bank_transfer: {
            payout_method_type: "payshap",
          },
        },
        billing: billing,
      },
      Response: {
        status: 200,
        body: {
          payout_type: "bank",
          status: "initiated",
          connector: "gotyme_sanlam",
        },
      },
      FrmDetails: {
        status: "initiated",
        connector: "gotyme_sanlam",
        frm_message: { frm_status: "transaction_failure" },
        notEqual: { error_code: "transaction_failure" },
      },
    },
    payshap_proxy: {
      Create: {
        Request: {
          amount: 2000,
          currency: "ZAR",
          payout_type: "bank",
          billing: billing,
        },
        Response: {
          status: 200,
          body: {
            status: "requires_confirmation",
            payout_type: "bank",
          },
        },
      },
      Confirm: {
        Request: {
          amount: 2000,
          currency: "ZAR",
          payout_type: "bank",
          payout_method_data: {
            bank_transfer: {
              payout_method_type: "payshap_proxy",
            },
          },
          billing: billing,
        },
        Response: {
          status: 200,
          body: {
            status: "requires_fulfillment",
            payout_type: "bank",
            connector: "gotyme_sanlam",
          },
        },
      },
      Fulfill: {
        Response: {
          status: 200,
          body: {
            status: "initiated",
            amount: 2000,
          },
        },
      },
      RetrieveAfterFulfill: {
        Response: {
          status: 200,
          body: {
            status: "initiated",
            amount: 2000,
          },
        },
      },
    },
  },
};
