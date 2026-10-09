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

const bank_payout_method_data = {
  bank: {
    country_code: "SE",
    account_number: "69706212",
    bank_number: "6112",
    iban: null,
  },
};

// Malformed (non-numeric) Swedish account number. Verified behavior against
// integ: the UCS intake transformer cannot map an unmappable account string
// to a recipient, so it demands raw payout_connector_metadata and the create
// is rejected up front with a 400. Well-formed numbers map inline and settle
// as "success" in the sandbox (e.g. "99999999" completes), so the intake
// rejection is the only observable invalid-account rejection path.
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
      RetrieveAfterFulfill: {
        Response: {
          status: 200,
          body: {
            status: "success",
            payout_type: "bank",
          },
        },
      },
      InvalidAccountNumber: {
        Request: {
          amount: 10,
          currency: "EUR",
          payout_type: "bank",
          payout_method_data: invalid_bank_payout_method_data,
          billing: billing,
        },
        Response: {
          status: 400,
          body: {
            error: {
              type: "invalid_request",
              // Intake-phase rejection of the unmappable account number;
              // metadata-free-text in the message drifts across deployments,
              // so only the stable signature (code) is asserted.
              code: "IR_04",
            },
          },
        },
      },
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
      // No SavePayoutMethod key: POST /payment_methods cannot vault a
      // trustly-typed bank transfer (v1 validate() allows trustly only under
      // bank_redirect) - recipients get saved via a recurring payout instead
      // (see 00005-SavePayout.cy.js).
      Token: {
        Request: {
          amount: 10,
          currency: "EUR",
          payout_token: "token",
          payout_type: "bank",
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
    },
  },
};
