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
          status: 200,
          body: {
            status: "failed",
            error_code: "624",
            error_message: "ERROR_INVALID_BANK_ACCOUNT_NUMBER",
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
