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
 *
 * The Fulfill keys assert the deterministic shipped-environment behavior: the
 * OSS dummy connector base_url ("dev") is not a valid URL, so the fulfill call
 * fails inside the router with 500 HE_00 and the payout silently stays in
 * requires_fulfillment. The RetrieveAfterFulfill keys assert that silent
 * failure state (status unchanged, no error code/message on the payout).
 */
export const connectorDetails = {
  bank_transfer_pm: {
    payshap: {
      Create: {
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
            status: "requires_confirmation",
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
          status: 500,
          body: {
            error: {
              type: "api",
              message: "Something went wrong",
              code: "HE_00",
            },
          },
        },
      },
      RetrieveAfterFulfill: {
        Response: {
          status: 200,
          body: {
            status: "requires_fulfillment",
            error_code: null,
            error_message: null,
          },
        },
      },
    },
    payshap_proxy: {
      Create: {
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
          status: 500,
          body: {
            error: {
              type: "api",
              message: "Something went wrong",
              code: "HE_00",
            },
          },
        },
      },
      RetrieveAfterFulfill: {
        Response: {
          status: 200,
          body: {
            status: "requires_fulfillment",
            error_code: null,
            error_message: null,
          },
        },
      },
    },
  },
};
