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

const bank_transfer_data = {
  payout_method_type: "sepa",
  iban: "DE94500700101234580002",
  bic: "DEUTDEDB237",
  bank_name: "Deutsche Bank",
  bank_country_code: "DE",
  account_holder_name: "John Doe",
};

// Debtor (ordering party) name is mandatory for Deutsche Bank payouts.
const source_bank_data = {
  payout_method_type: "sepa",
  iban: "DE83215730130100853100",
  bic: "DEUTDEDB237",
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

export const connectorDetails = {
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
          // `account_holder_name` intentionally omitted — the router
          // validates the debtor name up front and rejects with 400 `IR_04`.
          source_bank_data: {
            payout_method_type: "sepa",
            iban: "DE83215730130100853100",
            bic: "DEUTDEDB237",
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
