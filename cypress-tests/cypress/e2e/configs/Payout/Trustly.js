// Trustly payout billing identity matching the integ-verified sandbox flows:
// Swedish customer, EUR payout.
const billing = {
  address: {
    city: "Stockholm",
    country: "SE",
    line1: "Main street 1",
    line2: "Main street 2",
    zip: "SE-11253",
    state: "SE",
    first_name: "Steve",
    last_name: "Smith",
  },
  email: "abc@gmail.com",
  phone: {
    number: "709876543",
    country_code: "+46",
  },
};

// Trustly payouts are submitted as bank (IBAN-form) payout_method_data: the
// untagged `bank` enum deserializes to TrustlyBankTransfer from the
// country_code/account_number/bank_number triplet. This is the verified
// sandbox working set (POST /payouts/create, /confirm, /fulfill, GET sync).
// Trustly payouts execute only on the UCS path (Trustly is a UCS-only
// connector), so 00004-BankTransfer.cy.js installs the
// trustly_trustly_PoCreate/PoFulfill/PoSync rollout configs before any of
// these steps - on the direct path Trustly payouts return 501 IR_00
// ("Selected payment method through Trustly is not implemented").
const bank_payout_method_data = {
  bank: {
    country_code: "SE",
    account_number: "69706212",
    bank_number: "6112",
    iban: null,
  },
};

// Sandbox-rejected account number (alphabetic) - the PSP responds with
// ERROR_INVALID_BANK_ACCOUNT_NUMBER and the payout lands in `failed`.
const invalid_bank_payout_method_data = {
  bank: {
    country_code: "SE",
    account_number: "AASSSRMFKF",
    bank_number: "6112",
    iban: null,
  },
};

export const connectorDetails = {
  bank_transfer_pm: {
    open_banking: {
      // Create without confirm: no connector is selected yet, so the payout
      // waits in requires_confirmation (verified: POST /payouts/create with
      // confirm=false).
      Create: {
        Request: {
          amount: 10,
          currency: "EUR",
          payout_type: "bank",
          payout_method_data: bank_payout_method_data,
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
      // Create with confirm=true and auto_fulfill=false: the payout is
      // confirmed and awaits manual fulfillment (verified trace).
      Confirm: {
        Request: {
          amount: 10,
          currency: "EUR",
          payout_type: "bank",
          payout_method_data: bank_payout_method_data,
          billing: billing,
        },
        Response: {
          status: 200,
          body: {
            status: "requires_fulfillment",
            payout_type: "bank",
          },
        },
      },
      // Auto-fulfill create and manual POST /payouts/{id}/fulfill both end
      // in `initiated` once Trustly accepts the payout (verified traces).
      Fulfill: {
        Request: {
          amount: 10,
          currency: "EUR",
          payout_type: "bank",
          payout_method_data: bank_payout_method_data,
          billing: billing,
        },
        Response: {
          status: 200,
          body: {
            status: "initiated",
            payout_type: "bank",
          },
        },
      },
      // Once Trustly settles the payout its sandbox fires the
      // payoutconfirmation webhook (processed through the UCS Webhooks
      // rollout); PoSync then reports the payout as success. The spec polls
      // until a terminal status before asserting this.
      RetrieveAfterFulfill: {
        Response: {
          status: 200,
          body: {
            status: "success",
            payout_type: "bank",
          },
        },
      },
      // Alphabetic account number: Trustly rejects the bank account and the
      // payout ends in `failed` with the 624 connector error (verified).
      InvalidAccountNumber: {
        Request: {
          amount: 10,
          currency: "EUR",
          payout_type: "bank",
          payout_method_data: invalid_bank_payout_method_data,
          billing: billing,
        },
        Response: {
          status: 200,
          body: {
            status: "failed",
            error_code: "624",
            error_message: "ERROR_INVALID_BANK_ACCOUNT_NUMBER",
          },
        },
      },
      // Billing address is optional for Trustly payouts: create+confirm with
      // auto_fulfill goes straight to `initiated` without it (verified).
      ConfirmWithoutBilling: {
        Request: {
          amount: 10,
          currency: "EUR",
          payout_type: "bank",
          payout_method_data: bank_payout_method_data,
          billing: null,
        },
        Response: {
          status: 200,
          body: {
            status: "initiated",
            payout_type: "bank",
          },
        },
      },
      // Create Recipient / payout-token reuse are not exercised for Trustly
      // (no SAVED_* connector-list membership); placeholders keep the config
      // shape consistent with the other payout connectors.
      SavePayoutMethod: {
        Configs: {
          TRIGGER_SKIP: true,
        },
        Request: {
          payment_method: "bank_transfer",
          payment_method_type: "trustly",
          bank_transfer: bank_payout_method_data.bank,
        },
      },
      Token: {
        Configs: {
          TRIGGER_SKIP: true,
        },
        Request: {
          payout_token: "token",
          payout_type: "bank",
        },
      },
    },
  },
};
