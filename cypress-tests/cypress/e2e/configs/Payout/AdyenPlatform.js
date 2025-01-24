const billing = {
  address: {
    line1: "Raadhuisplein",
    line2: "92",
    city: "Hoogeveen",
    state: "FL",
    zip: "7901 BW",
    country: "NL",
    first_name: "John",
    last_name: "Doe",
  },
  phone: {
    number: "9123456789",
    country_code: "+31",
  },
};

export const connectorDetails = {
  bank_transfer_pm: {
    sepa: {
      Create: {
        Request: {
          payout_type: "bank",
          priority: "instant",
          payout_method_data: {
            bank: {
              iban: "NL57INGB4654188101",
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
          payout_type: "bank",
          priority: "instant",
          payout_method_data: {
            bank: {
              iban: "NL57INGB4654188101",
            },
          },
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
          payout_type: "bank",
          priority: "instant",
          payout_method_data: {
            bank: {
              iban: "NL57INGB4654188101",
            },
          },
          billing: billing,
          recurring: true,
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
        Request: {
          payment_method: "bank_transfer",
          payment_method_type: "sepa",
          bank_transfer: {
            iban: "NL57INGB4654188101",
          },
        },
        Response: {
          status: 200,
          body: {
            payment_method: "bank_transfer",
            payment_method_type: "sepa",
          },
        },
      },
      Token: {
        Request: {
          payout_token: "token",
          payout_type: "bank",
          priority: "instant",
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
