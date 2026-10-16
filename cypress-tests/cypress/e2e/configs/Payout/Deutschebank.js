const billing = {
  address: {
    city: "Frankfurt",
    country: "DE",
    line1: "Taunusanlage 12",
    zip: "60325",
    state: "HE",
    first_name: "John",
    last_name: "Doe",
  },
};

// The real iban/bic (and account_holder_name for the beneficiary) are
// injected at runtime from the gitignored creds.json by
// injectPayoutBankTransferDetails — see Utils.js. Only the
// payout_method_type marker lives in committed config.
const bank_transfer_data = {
  payout_method_type: "sepa",
};

// Debtor (ordering party) name is mandatory for Deutsche Bank payouts.
// account_holder_name is a non-sensitive literal kept here (not in
// creds.json) because its presence/absence is what the positive vs.
// CreateWithoutSourceAccountHolderName negative case tests; iban/bic are
// injected at runtime the same way as above.
const source_bank_data = {
  payout_method_type: "sepa",
  account_holder_name: "John Doe",
};

const create_payout_request = {
  currency: "EUR",
  payout_type: "bank",
  payout_method_data: {
    bank_transfer: bank_transfer_data,
  },
  source_bank_data: source_bank_data,
  billing: billing,
  entity_type: "Individual",
  recurring: false,
  description: "any-purpose",
  phone_country_code: "+49",
};

// Create with auto-fulfill stays `pending`; success is only reached after
// PoSync with `force_sync=true`.
const pending_payout_response = {
  status: 200,
  body: {
    status: "pending",
    payout_type: "bank",
    connector: "deutschebank",
    currency: "EUR",
  },
};

const card_data = {
  card_number: "4111111111111111",
  expiry_month: "3",
  expiry_year: "2030",
  card_holder_name: "John Smith",
};

export const connectorDetails = {
  // Only Fulfill is overridden here. Create and Confirm both inherit
  // Commons.js's card_pm defaults unchanged — confirmed live (2026-10-16)
  // that deutschebank's actual Confirm response (200, "requires_fulfillment")
  // already matches that default exactly, and Create is never exercised by
  // any spec so there's nothing to verify an override against.
  // SavePayoutMethod and Token were previously duplicated/guessed here too;
  // dropped in favor of Commons's defaults — SavePayoutMethod's default is
  // byte-identical to what was here, and Token's old value (501/IR_04
  // "Missing required param: connector_customer_id") was a fabricated guess
  // never checked against the live API, so Commons's generic
  // not-implemented default is the more honest fallback until it's verified.
  card_pm: {
    // This key does double duty in 00003-CardTest.cy.js: the auto-fulfill
    // context uses it as the create-call Request/Response (same
    // /payouts/create endpoint Confirm uses, confirmed live to return this
    // shape). The manual-fulfill context separately reuses this same
    // Response to assert the actual POST /payouts/{id}/fulfill call's
    // result, which hasn't been verified live yet and may legitimately
    // differ (e.g. "pending"/"success" instead of "requires_fulfillment").
    Fulfill: {
      Request: {
        payout_method_data: {
          card: card_data,
        },
        currency: "EUR",
        payout_type: "card",
        recurring: true,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_fulfillment",
          payout_type: "card",
          connector: "deutschebank",
          error_code: null,
          error_message: null,
        },
      },
    },
  },
  bank_transfer_pm: {
    sepa_bank_transfer: {
      Create: {
        Request: create_payout_request,
        Response: pending_payout_response,
      },
      Confirm: {
        Request: create_payout_request,
        Response: pending_payout_response,
      },
      Fulfill: {
        Request: create_payout_request,
        Response: pending_payout_response,
      },
      // PoSync returns a transient HTTP 408 while the transfer settles,
      // hence the DELAY before the first attempt.
      Sync: {
        Configs: {
          DELAY: {
            STATUS: true,
            TIMEOUT: 30000,
          },
        },
        Request: {},
        Response: {
          status: 200,
          body: {
            status: "success",
            payout_type: "bank",
          },
        },
      },
      SyncIdempotent: {
        Request: {},
        Response: {
          status: 200,
          body: {
            status: "success",
            payout_type: "bank",
          },
        },
      },
      SyncNonExistentPayout: {
        Request: {},
        Response: {
          status: 404,
          body: {
            error: {
              type: "invalid_request",
              message: "Payout does not exist in our records",
              code: "HE_02",
            },
          },
        },
      },
      CreateWithoutSourceAccountHolderName: {
        Request: {
          currency: "EUR",
          payout_type: "bank",
          payout_method_data: {
            bank_transfer: bank_transfer_data,
          },
          // `account_holder_name` intentionally omitted — on the UCS path,
          // the connector validates the debtor name up front and rejects the
          // request with 400 `IR_04`. iban/bic are injected at runtime same
          // as create_payout_request's source_bank_data.
          source_bank_data: {
            payout_method_type: "sepa",
          },
          billing: billing,
          entity_type: "Individual",
          recurring: false,
          description: "any-purpose",
          phone_country_code: "+49",
        },
        Response: {
          status: 400,
          body: {
            error: {
              type: "invalid_request",
              message:
                "Missing required param: Missing required field: source_bank_data.sepa.account_holder_name. Deutsche Bank requires the debtor (ordering party) name on `source_bank_data.sepa.account_holder_name`",
              code: "IR_04",
            },
          },
        },
      },
    },
  },
};
