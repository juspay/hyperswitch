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
              // UCS validates the trustly recipient at intake: the malformed
              // (non-numeric) Swedish account number cannot be mapped, so the
              // transfer demands raw payout_connector_metadata and rejects
              // the create instead of failing asynchronously with a 624.
              message:
                "Missing required param: Missing required field: payout_connector_metadata. Trustly Payout Transfer - missing required field",
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
